// Shared evaluator for the guard's stored policy expressions. Semantics
// match shared/policy_expr.rs: u64 wrapping add/sub, le/lt/eq/and/or/not/if
// and canonical decimal const values. Serde's sequence form is normalized
// to the canonical object form the contract stores. No I/O.
(function (global) {
"use strict";
var MOD = 1n << 64n;
var MAX = MOD - 1n;
var MAX_DEPTH = 48;
var MAX_BYTES = 32768;

var VULNERABLE =
  '{"op":"le","args":[{"op":"add","args":[{"op":"spent"},{"op":"amount"}]},{"op":"limit"}]}';

function fail(message) { throw new Error(message); }
function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
function node(value) {
  if (!isObject(value) || typeof value.op !== "string" ||
      Object.keys(value).some(function (k) { return k !== "op" && k !== "args"; }))
    fail("Invalid policy expression");
  return value;
}
function pair(args, kind, depth, visit) {
  if (!Array.isArray(args) || args.length !== 2) fail("Invalid policy expression");
  visit(args[0], kind, depth + 1);
  visit(args[1], kind, depth + 1);
}
function checkConst(value) {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value) || (BigInt(value) > MAX || BigInt(value).toString() !== value))
    fail("Invalid UInt64 constant");
}
function uint(e, depth) {
  if (depth > MAX_DEPTH) fail("Policy expression too deep");
  var args = node(e).args;
  switch (e.op) {
    case "limit": case "spent": case "amount":
      if (args !== undefined && args !== null) fail("Invalid policy expression");
      return;
    case "const":
      checkConst(args);
      return;
    case "add": case "sub":
      pair(args, "uint", depth, visit);
      return;
    case "if":
      if (!Array.isArray(args) || args.length !== 3) fail("Invalid policy expression");
      visit(args[0], "bool", depth + 1);
      visit(args[1], "uint", depth + 1);
      visit(args[2], "uint", depth + 1);
      return;
    default:
      fail("Invalid policy expression");
  }
}
function bool(e, depth) {
  if (depth > MAX_DEPTH) fail("Policy expression too deep");
  var args = node(e).args;
  switch (e.op) {
    case "true": case "false":
      if (args !== undefined && args !== null) fail("Invalid policy expression");
      return;
    case "le": case "lt": case "eq":
      pair(args, "uint", depth, visit);
      return;
    case "and": case "or":
      pair(args, "bool", depth, visit);
      return;
    case "not":
      visit(args, "bool", depth + 1);
      return;
    case "if":
      if (!Array.isArray(args) || args.length !== 3) fail("Invalid policy expression");
      visit(args[0], "bool", depth + 1);
      visit(args[1], "bool", depth + 1);
      visit(args[2], "bool", depth + 1);
      return;
    default:
      fail("Invalid policy expression");
  }
}
function visit(value, kind, depth) {
  if (kind === "uint") uint(value, depth); else bool(value, depth);
}

function normalize(e, depth) {
  if (depth > MAX_DEPTH) fail("Policy expression too deep");
  if (Array.isArray(e) && e.length === 2) e = { op: e[0], args: e[1] };
  if (!isObject(e)) return e;
  if (e.op === "not") e.args = normalize(e.args, depth + 1);
  else if (Array.isArray(e.args)) e.args = e.args.map(function (a) { return normalize(a, depth + 1); });
  return e;
}

function parse(json) {
  if (typeof json !== "string" || new TextEncoder().encode(json).length > MAX_BYTES)
    fail("Policy expression too large");
  var expr;
  try {
    expr = JSON.parse(json);
  } catch (e) {
    fail("Invalid policy expression");
  }
  // serde rejects duplicate fields; JSON.parse alone keeps the last value.
  var stack = [];
  var tokens = /"(?:\\.|[^"\\])*"|[{}\[\]]/g;
  var token;
  while ((token = tokens.exec(json))) {
    var text = token[0];
    if (text === "{") stack.push(new Set());
    else if (text === "[") stack.push(null);
    else if (text === "}" || text === "]") stack.pop();
    else if (/^\s*:/.test(json.slice(tokens.lastIndex))) {
      var keys = stack[stack.length - 1], key = JSON.parse(text);
      if (keys.has(key)) fail("Invalid policy expression");
      keys.add(key);
    }
  }
  expr = normalize(expr, 0);
  bool(expr, 0);
  return expr;
}

function u64(value) {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) fail("Expected a u64");
  var n = BigInt(value);
  if (n > MAX || n.toString() !== value) fail("Expected a u64");
  return n;
}
function wrap(value) {
  return ((value % MOD) + MOD) % MOD;
}
function evalUint(e, limit, spent, amount) {
  var args = e.args;
  switch (e.op) {
    case "limit": return limit;
    case "spent": return spent;
    case "amount": return amount;
    case "const": return BigInt(args);
    case "add": return wrap(evalUint(args[0], limit, spent, amount) + evalUint(args[1], limit, spent, amount));
    case "sub": return wrap(evalUint(args[0], limit, spent, amount) - evalUint(args[1], limit, spent, amount));
    case "if": return evalUint(evalBool(args[0], limit, spent, amount) ? args[1] : args[2], limit, spent, amount);
    default: fail("Invalid policy expression");
  }
}
function evalBool(e, limit, spent, amount) {
  var args = e.args;
  var a, b;
  switch (e.op) {
    case "true": return true;
    case "false": return false;
    case "le": case "lt": case "eq":
      a = evalUint(args[0], limit, spent, amount);
      b = evalUint(args[1], limit, spent, amount);
      return e.op === "le" ? a <= b : e.op === "lt" ? a < b : a === b;
    case "and": return evalBool(args[0], limit, spent, amount) && evalBool(args[1], limit, spent, amount);
    case "or": return evalBool(args[0], limit, spent, amount) || evalBool(args[1], limit, spent, amount);
    case "not": return !evalBool(args, limit, spent, amount);
    case "if": return evalBool(evalBool(args[0], limit, spent, amount) ? args[1] : args[2], limit, spent, amount);
    default: fail("Invalid policy expression");
  }
}
function evalJson(json, limit, spent, amount) {
  return evalBool(parse(json), u64(limit), u64(spent), u64(amount));
}

var PREC = { or: 1, and: 2, le: 3, lt: 3, eq: 3, add: 4, sub: 4 };
function fmt(e, parent) {
  parent = parent || 0;
  var a = e.args, s, p = PREC[e.op] || 9;
  switch (e.op) {
    case "amount": case "limit": case "spent": case "true": case "false": return e.op;
    case "const": return a === "18446744073709551615" ? "U64_MAX" : String(a);
    case "and": s = a.map(function (x) { return fmt(x, p); }).join(" ∧ "); break;
    case "or": s = a.map(function (x) { return fmt(x, p); }).join(" ∨ "); break;
    case "le": s = fmt(a[0], p) + " ≤ " + fmt(a[1], p); break;
    case "lt": s = fmt(a[0], p) + " < " + fmt(a[1], p); break;
    case "eq": s = fmt(a[0], p) + " = " + fmt(a[1], p); break;
    case "add": s = fmt(a[0], p) + " + " + fmt(a[1], p); break;
    case "sub": s = fmt(a[0], p) + " − " + fmt(a[1], p + 1); break;
    case "not": return "¬(" + fmt(Array.isArray(a) ? a[0] : a) + ")";
    case "if": return "(if " + fmt(a[0]) + " then " + fmt(a[1]) + " else " + fmt(a[2]) + ")";
    default: return JSON.stringify(e);
  }
  return p < parent ? "(" + s + ")" : s;
}

// One line per comparison node, pre-order, for the "would pay" preview.
// Shows each side's wrapped u64 value and marks results that wrapped.
function uintLine(e, limit, spent, amount) {
  var v = evalUint(e, limit, spent, amount);
  var text = fmt(e) + " = " + v.toString();
  if (e.op === "add" && evalUint(e.args[0], limit, spent, amount) + evalUint(e.args[1], limit, spent, amount) >= MOD)
    text += " (wrapped)";
  if (e.op === "sub" && evalUint(e.args[0], limit, spent, amount) < evalUint(e.args[1], limit, spent, amount))
    text += " (wrapped)";
  return text;
}
function trace(e, limit, spent, amount, lines) {
  lines = lines || [];
  var a = e.args;
  if (e.op === "le" || e.op === "lt" || e.op === "eq") {
    var sign = e.op === "le" ? " ≤ " : e.op === "lt" ? " < " : " = ";
    lines.push(uintLine(a[0], limit, spent, amount) + sign +
      uintLine(a[1], limit, spent, amount) + " → " +
      (evalBool(e, limit, spent, amount) ? "true" : "false"));
    return lines;
  }
  var children = e.op === "not" ? [a] : a || [];
  children.forEach(function (child) { trace(child, limit, spent, amount, lines); });
  return lines;
}

function canonical(e) {
  var a = e.args;
  switch (e.op) {
    case "amount": case "limit": case "spent": case "true": case "false":
      return '{"op":"' + e.op + '"}';
    case "const":
      return '{"op":"const","args":' + JSON.stringify(a) + "}";
    case "not":
      return '{"op":"not","args":' + canonical(a) + "}";
    default:
      return '{"op":"' + e.op + '","args":[' + a.map(canonical).join(",") + "]}";
  }
}

var PolicyExpr = {
  VULNERABLE: VULNERABLE,
  U64_MAX: MAX,
  parse: parse,
  u64: u64,
  eval: evalBool,
  evalJson: evalJson,
  fmt: fmt,
  trace: trace,
  canonical: canonical,
};
if (typeof module !== "undefined" && module.exports) module.exports = PolicyExpr;
global.PolicyExpr = PolicyExpr;
})(typeof globalThis !== "undefined" ? globalThis : this);
