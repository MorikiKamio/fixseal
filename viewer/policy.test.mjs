// Compares viewer/policy.js with the Rust evaluator's recorded outputs in
// policy-vectors.json (generated and checked by judge/tests/policy_expr.rs).
// Run: node --test viewer/policy.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import PolicyExpr from "./policy.js";

const vectors = JSON.parse(readFileSync(new URL("./policy-vectors.json", import.meta.url), "utf8"));

test("vectors exercise the same inputs the exporter check uses", () => {
  assert.equal(vectors.format, "fixseal-policy-vectors-v1");
  assert.equal(vectors.inputs.length, 1028);
  assert.ok(vectors.cases.length >= 3);
});

for (const c of vectors.cases) {
  test(`case ${c.name}: expression and hash`, () => {
    const expr = PolicyExpr.parse(c.expr);
    assert.equal(PolicyExpr.canonical(expr), c.expr);
    assert.equal(createHash("sha256").update(c.expr, "utf8").digest("hex"), c.expr_sha256);
  });
  test(`case ${c.name}: every recorded output matches`, () => {
    const expr = PolicyExpr.parse(c.expr);
    const bits = Buffer.from(c.expected, "hex");
    for (let i = 0; i < vectors.inputs.length; i++) {
      const [limit, spent, amount] = vectors.inputs[i].split(" ");
      const expected = Boolean((bits[i >> 3] >> (i % 8)) & 1);
      assert.equal(
        PolicyExpr.evalJson(c.expr, limit, spent, amount),
        expected,
        `${vectors.inputs[i]}`,
      );
      assert.equal(PolicyExpr.eval(expr, BigInt(limit), BigInt(spent), BigInt(amount)), expected);
    }
  });
}

test("u64 input validation", () => {
  for (const bad of ["", "01", "+1", "-1", "1.5", "abc", "18446744073709551616", "0x10", "1\n", "0\n", 1, 1n, true, null]) {
    assert.throws(() => PolicyExpr.u64(bad), /Expected a u64/);
  }
  assert.equal(PolicyExpr.u64("0"), 0n);
  assert.equal(PolicyExpr.u64("18446744073709551615"), (1n << 64n) - 1n);
});

test("malformed expressions are rejected", () => {
  const bad = [
    "not json",
    "1",
    "{}",
    '{"op":"true","extra":1}',
    '{"op":"unknown"}',
    '{"op":"le","args":[{"op":"true"},{"op":"amount"}]}',
    '{"op":"le","args":[{"op":"amount"}]}',
    '{"op":"and","args":[{"op":"true"},{"op":"false"},{"op":"true"}]}',
    '{"op":"const","args":"01"}',
    '{"op":"le","args":[{"op":"const","args":"18446744073709551616"},{"op":"limit"}]}',
    '{"op":"le","args":[{"op":"const","args":1},{"op":"limit"}]}',
    '{"op":"amount","args":[]}',
    '{"op":"add","args":{"op":"amount"}}',
    '{"op":"le","args":[{"op":"const","args":"1\\n"},{"op":"limit"}]}',
    "x".repeat(32769),
  ];
  for (const json of bad) {
    assert.throws(() => PolicyExpr.parse(json), undefined, json.slice(0, 40));
  }
  let deep = { op: "true" };
  for (let i = 0; i < 49; i++) deep = { op: "not", args: deep };
  assert.throws(() => PolicyExpr.parse(JSON.stringify(deep)), /too deep/);
});

test("wrapping and lazy-if semantics match u64", () => {
  const max = "18446744073709551615";
  const add = '{"op":"eq","args":[{"op":"add","args":[{"op":"amount"},{"op":"const","args":"1"}]},{"op":"const","args":"0"}]}';
  assert.equal(PolicyExpr.evalJson(add, "0", "0", max), true);
  const sub = '{"op":"eq","args":[{"op":"sub","args":[{"op":"const","args":"0"},{"op":"amount"}]},{"op":"const","args":"' + max + '"}]}';
  assert.equal(PolicyExpr.evalJson(sub, "0", "0", "1"), true);
});

test("trace renders the intermediate values", () => {
  const lines = PolicyExpr.trace(PolicyExpr.parse(PolicyExpr.VULNERABLE), 1000n, 600n, 18446744073709551116n);
  assert.equal(lines.length, 1);
  assert.match(lines[0], /spent \+ amount = 100 \(wrapped\) ≤ limit = 1000 → true/);
});

test("const validation applies inside UInt64 expressions", () => {
  for (const value of ["01", "+1", "-1", "1.0", "1\n", " 1", "18446744073709551616", 1, null]) {
    const expr = { op: "eq", args: [{ op: "const", args: value }, { op: "amount" }] };
    assert.throws(() => PolicyExpr.parse(JSON.stringify(expr)), /Invalid UInt64 constant/);
  }
  for (const json of [
    '{"op":"false","op":"true"}',
    '{"op":"le","args":[{"op":"amount"},{"op":"limit"}],"args":[{"op":"spent"},{"op":"limit"}]}',
    '{"op":"false","\\u006fp":"true"}',
  ]) assert.throws(() => PolicyExpr.parse(json), /Invalid policy expression/);
});

test("both If types and boolean operators evaluate only the selected branch", () => {
  const unreachable = new Proxy({}, { get() { throw new Error("unchosen branch evaluated"); } });
  const yes = { op: "true" }, no = { op: "false" };
  assert.equal(PolicyExpr.eval({ op: "if", args: [yes, yes, unreachable] }, 0n, 0n, 0n), true);
  assert.equal(PolicyExpr.eval({ op: "if", args: [no, unreachable, no] }, 0n, 0n, 0n), false);
  assert.equal(PolicyExpr.eval({ op: "and", args: [no, unreachable] }, 0n, 0n, 0n), false);
  assert.equal(PolicyExpr.eval({ op: "or", args: [yes, unreachable] }, 0n, 0n, 0n), true);
  for (const [condition, a, b] of [[yes, { op: "amount" }, unreachable], [no, unreachable, { op: "amount" }]]) {
    const expr = { op: "eq", args: [{ op: "if", args: [condition, a, b] }, { op: "amount" }] };
    assert.equal(PolicyExpr.eval(expr, 0n, 0n, 7n), true);
  }
});

test("Serde unit-null and sequence forms normalize to the same policy", () => {
  for (const json of ['{"op":"true","args":null}', '{"args":null,"op":"true"}', '["true",null]']) {
    assert.equal(PolicyExpr.evalJson(json, "0", "0", "0"), true);
    assert.equal(PolicyExpr.canonical(PolicyExpr.parse(json)), '{"op":"true"}');
  }
  const json = '["le",[["amount",null],{"op":"limit","args":null}]]';
  assert.equal(PolicyExpr.evalJson(json, "1000", "0", "600"), true);
  assert.equal(PolicyExpr.canonical(PolicyExpr.parse(json)), '{"op":"le","args":[{"op":"amount"},{"op":"limit"}]}');
  for (const bad of ['["true"]', '["true",null,null]', '{"op":"true","args":[]}']) {
    assert.throws(() => PolicyExpr.parse(bad));
  }
});
