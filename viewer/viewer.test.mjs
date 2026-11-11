// Offline DOM/RPC checks for the static viewer. No sockets or browser packages.
import "./policy.test.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import { webcrypto } from "node:crypto";
import PolicyExpr from "./policy.js";

const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
const script = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)][0][1];

function viewer(search = "?contract=live.testnet&live=1") {
  const elements = new Map();
  class Element {
    constructor() {
      this._html = ""; this._text = ""; this.children = [];
      this.dataset = {}; this.style = {}; this.value = "100";
      const classes = new Set();
      this.classList = { add: (c) => classes.add(c), remove: (c) => classes.delete(c), contains: (c) => classes.has(c) };
    }
    set innerHTML(value) {
      for (const [, id] of this._html.matchAll(/id="([^"]+)"/g)) elements.delete(id);
      this._html = value; this.children = [];
    }
    get innerHTML() { return this._html; }
    set textContent(value) { this.innerHTML = ""; this._text = String(value); }
    get textContent() { return this._text; }
    get firstElementChild() { return this.children[0]; }
    get lastElementChild() { return this.children.at(-1); }
    prepend(child) { child.parent = this; this.children.unshift(child); }
    remove() { this.parent.children.splice(this.parent.children.indexOf(this), 1); }
    addEventListener() {}
  }
  for (const [, id] of html.matchAll(/id="([^"]+)"/g)) elements.set(id, new Element());
  elements.get("rule-note")._html = '<code id="rule-record">–</code>';
  const document = { hidden: false, getElementById: (id) => elements.get(id) || null,
    createElement: () => new Element(), querySelectorAll: () => [] };
  const requests = [], intervals = [];
  let snapshot = { height: 100, state: null, history: [], records: [] };
  let gate;
  const context = vm.createContext({
    document, location: { search }, URLSearchParams, TextEncoder, TextDecoder,
    Uint8Array, crypto: webcrypto, PolicyExpr, btoa: (s) => Buffer.from(s).toString("base64"),
    setInterval: (fn) => intervals.push(fn),
    fetch: async (_url, options) => {
      const { method, params } = JSON.parse(options.body);
      assert.equal(method, "query");
      requests.push(params);
      if (params.method_name === "guard_state" && gate) await gate;
      let result = { block_hash: `block-${snapshot.height}`, block_height: snapshot.height };
      if (params.request_type === "call_function") {
        const value = { guard_state: snapshot.state, guard_history: snapshot.history, list_fix_proofs: snapshot.records }[params.method_name];
        result.result = [...Buffer.from(JSON.stringify(value))];
      } else if (params.request_type === "view_account") result.code_hash = "1";
      else result.keys = [{ public_key: "ed25519:public", access_key: { permission: { FunctionCall: {
        receiver_id: new URLSearchParams(search).get("contract") || "fixseal-guard.testnet", method_names: ["guard_spend"],
      } } } }];
      return { json: async () => ({ result }) };
    },
  });
  vm.runInContext(script, context);
  return { elements, requests, intervals, document, context,
    set: (value) => { snapshot = value; }, gate: (value) => { gate = value; },
    load: () => vm.runInContext("load()", context),
    ready: async () => {
      for (let i = 0; i < 100; i++) {
        if (!vm.runInContext("loading", context)) return;
        await new Promise((resolve) => setImmediate(resolve));
      }
      assert.fail("viewer did not finish loading");
    },
    feed: () => elements.get("feed").children.map((e) => e.innerHTML).join("\n"),
  };
}

const state = (spent, paid) => ({ limit: "1000", spent, total_paid: paid,
  policy_expr: PolicyExpr.VULNERABLE, policy_expr_sha256: "<img>", record_id: "<img>", spec_sha256: "<img>" });
const record = (id, verdict) => ({ bundle_hash: id, summary: JSON.stringify({ verdict,
  policy_expr: PolicyExpr.VULNERABLE, judge_version: "spend-4", spec_sha256: "<img>" }),
  block_timestamp_ms: "1000", caller: "<img>" });
const history = (id) => ({ record_id: id, previous_policy_expr_sha256: "<img>",
  policy_expr_sha256: "<img>", block_timestamp_ms: "1000" });

test("viewer rejects invalid account parameters without RPC", async () => {
  for (const id of ["", "a", "a..testnet", "a_-b.testnet", "a.-b", ".abc", "a".repeat(65), "ab\n", "<img>", "Ab.testnet"]) {
    for (const param of ["contract", "agent"]) {
      const v = viewer(`?${param}=${encodeURIComponent(id)}`);
      await v.ready();
      assert.equal(v.requests.length, 0, `${param}=${JSON.stringify(id)}`);
      assert.match(v.elements.get("status").innerHTML, /Invalid/);
    }
  }
  for (const id of ["ab", "a-b_c.testnet", "a".repeat(64)]) {
    const v = viewer(`?contract=${id}`);
    await v.ready();
    assert.equal(v.requests.length, 5);
  }
});

test("viewer reloads after init, pins all reads, escapes chain fields and reports only diffs", async () => {
  const v = viewer();
  await v.ready();
  assert.equal(v.elements.get("rule-now").textContent, "Guard not initialized yet");
  v.set({ height: 101, state: { ...state("500", "500"), record_id: null }, history: [], records: [] });
  await v.load();
  assert.match(v.feed(), /Guard initialized/);
  assert.match(v.elements.get("rule-note").innerHTML, /no accept record/);
  v.set({ height: 102, state: state("100", "1100"), history: [history("<img>"), history("next")],
    records: [record("<img>", "reject"), record("next", "accept")] });
  await v.load();
  assert.match(v.elements.get("status").textContent, /Final block 102/);
  assert.match(v.feed(), /Total paid increased by 600 — cumulative total 1,100 exceeds cap 1,000/);
  assert.equal((v.feed().match(/Rule replaced/g) || []).length, 2);
  assert.match(v.feed(), /Judge record added: reject/);
  assert.match(v.feed(), /Judge record added: accept/);
  assert.match(v.elements.get("checks").innerHTML, /n\/a/);
  for (const e of v.elements.values()) assert.doesNotMatch(e.innerHTML, /<img>/);
  assert.match(v.elements.get("history").innerHTML, /&lt;img&gt;/);
  const before = v.feed();
  await v.load();
  assert.equal(v.feed(), before, "unchanged state must not claim a refusal or payment");
  for (let i = 0; i < v.requests.length; i += 5) {
    assert.equal(v.requests[i].finality, "final");
    for (const request of v.requests.slice(i + 1, i + 5)) {
      assert.match(request.block_id, /^block-/);
      assert.equal(request.finality, undefined);
    }
  }
  v.set({ height: 101, state: state("0", "0"), history: [], records: [] });
  await v.load();
  assert.equal(v.feed(), before, "older snapshots must not regress the displayed state");
  assert.match(v.elements.get("status").textContent, /Final block 102/);
  const count = v.requests.length;
  v.document.hidden = true;
  v.intervals[0]();
  assert.equal(v.requests.length, count);
});

test("viewer serializes overlapping refreshes", async () => {
  const v = viewer();
  await v.ready();
  let release;
  v.gate(new Promise((resolve) => { release = resolve; }));
  const count = v.requests.length;
  const first = v.load();
  await v.load();
  assert.equal(v.requests.length, count + 1);
  release();
  await first;
  assert.equal(v.requests.length, count + 5);
});

test("viewer keeps the simulation banner and avoids security certification claims", () => {
  assert.match(html, /Simulated environment, synthetic data · NEAR testnet · mock attestation · not a security certification/);
  assert.doesNotMatch(html, /verified secure|audited|proof of safety/i);
});
