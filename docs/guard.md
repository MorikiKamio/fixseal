# Proof-gated spending guard

T010 adds a contract-held spending expression and real yoctoNEAR transfer promises. Demonstrations use a **simulated environment, synthetic data**. Offline tests exercise the mock VM; sandbox payments, callback gas and testnet remain host checks. Existing path/spend record APIs remain available.

The Lean kernel checks a proof about Lean's `Target.authorize` against the fixed specification. The contract evaluates an exported expression using the same Rust interpreter source as the judge. The fixed exporter and interpreter are trusted and are outside that proof. Comparing their outputs on 1,028 fixed inputs is a **check, not a proof of correspondence**. The contract trusts the authorized record submitter's claims; no TEE is used. Anyone can independently rerun `fixseal-judge verify` with the saved inputs and reviewed judge/reference files.

## Expression wire format

`policy_expr` is a JSON **string containing canonical JSON**, not a nested object in the report or method arguments. Serialization uses UTF-8, no whitespace or trailing LF, and object fields in order `op`, then `args`. Its SHA-256 is `policy_expr_sha256`. This differs from the report hash, which includes a final LF.

| Type | Operations | args |
| --- | --- | --- |
| UInt64 | limit, spent, amount | omitted |
| UInt64 | const | canonical decimal string, 0 through 18446744073709551615 |
| UInt64 | add, sub | array of two UInt64 expressions; wraps at 2^64 |
| UInt64 | if | array: Bool condition, UInt64 then, UInt64 else |
| Bool | true, false | omitted |
| Bool | le, lt, eq | array of two UInt64 expressions |
| Bool | and, or | array of two Bool expressions |
| Bool | not | one Bool expression object |
| Bool | if | array of three Bool expressions |

The root is Bool. For example, the vulnerable baseline is:

```json
{"op":"le","args":[{"op":"add","args":[{"op":"spent"},{"op":"amount"}]},{"op":"limit"}]}
```

Parsing tolerates JSON key order/whitespace and canonicalizes before hashing. Unknown operations, fields, wrong operand types, noncanonical/out-of-range constants, input over 32 KiB or depth over 48 fail. The interpreter is `shared/policy_expr.rs`, included by judge and contract; evaluation performs no I/O.

The trusted `TargetAudit.lean` walks the elaborated definition, substitutes lets, and exports only supported UInt64/Bool operations with the expected core instances. Supported decidable propositions (UInt64 comparisons/equality, conjunction, disjunction, negation, Bool-to-Prop coercion and conditionals) normalize to Bool nodes. Unsupported constructs fail with `POLICY_VIOLATION`, even if they use UInt64 exclusively. The prior audit of forbidden numeric families remains. The judge compares exported evaluation with actual Lean execution on attacks 12 + regressions 16 + the 1,000 triples from `{0,1,2^32-1,2^32,2^32+1,2^63-1,2^63,2^63+1,2^64-2,2^64-1}`. One disagreement rejects with `EXPORT_MISMATCH`.

Current spend judge: `0.1.0/lean-spend-4`. Reports and spend summaries include the exported JSON and hash; `export_checked_inputs` records the comparison count. Spend summaries also include `subject: spend` and `spec_sha256`, alongside existing verdict/version/input hash fields. `verify` re-exports, rechecks the hash and complete report, and reruns kernel checking when applicable. Older spend bundles need regeneration with their original sources; path report/summary formats remain unchanged. The spec hash is unchanged.

## Contract API and trust

| Method | Caller / behavior |
| --- | --- |
| guard_init(limit, spec_sha256, policy_expr) | Owner only, once. U64 decimal string limit, reviewed spec hash and exactly the vulnerable baseline expression. Pins the spec; no reset. |
| guard_spend(to, amount) | Valid registered agent, zero attached deposit. Evaluates the active expression, reserves amount and wraps spent; false panics. Transfers amount yoctoNEAR from contract funds. |
| guard_on_transfer(amount) | Private callback. On success increments u128 total_paid; on failure releases only that reservation and reverses its wrapped spent update. |
| guard_set_policy(policy_expr, record_id) | Owner only. Requires existing, well-formed accept/spend record, pinned spec match, canonical expression hash/content match and allowlisted judge. |
| guard_state() | limit/spent as U64 strings; total_paid/pending_paid as U128 strings; pinned spec, active JSON/hash and evidence record ID. Null before initialization. |
| guard_history() | Successful replacements, prior/new hashes, record ID and block timestamp. |

The allowlist is fixed in contract source to spend-4. Legacy summaries cannot authorize replacement. Each missing/malformed record, verdict, subject, spec, hash, expression and judge mismatch has a distinct error. Rejected replacements leave state/history unchanged. Successful initialization, spending requests/settlements and replacements emit `EVENT_JSON` logs with standard `fixseal_guard`, version `1.0.0`.

State is stored under separate keys, preserving the existing contract-state layout and `submit_fix_proof` / `get_fix_proof` compatibility. History is a separate JSON vector, suitable for this small demo; replacement cost grows with history. The owner funds contract storage and transfers. `total_paid` counts successful transfer receipts once the callback succeeds; `pending_paid` remains reserved until settlement. A callback failure requires operator investigation and must not be called a completed payment/accounting run. The callback uses 12 Tgas, still requiring host verification. The demo checks receipt outcomes and recipient balance changes, not just the return value.

Replacing a formula does not undo past payments, reset wrapped spent, or prove historical accounting. All agents share this guard's limit/counters. Existing full-access keys and the owner's contract-upgrade authority remain trusted. Optional verifier quorum/attestations are not implemented (submitter trust, equivalent to N=0).

## Restricted function-call key

Use a separate registered agent account. Keep its full-access key with the operator and install an agent-held function-call key restricted to this contract receiver and **only** `guard_spend`. The agent must not receive a contract-account full-access key or the owner's key. NEAR access keys restrict receiver/methods and gas allowance; they do not inspect JSON arguments or cap transfer amounts. The guard checks amounts, while the agent can choose the recipient. Function-call keys require zero attached deposit; the contract sends its own funds, and gas is paid from the agent account/allowance. Registry expiry, revoked approval/whitelist and insufficient funds still cause failure.

With preinstalled near-cli-rs 0.31.1, install a public key whose private half is already held in a secure keychain, off camera:

```sh
near account add-key "$FIXSEAL_TESTNET_AGENT" grant-function-call-access \
  --allowance '0.1 NEAR' --contract-account-id "$FIXSEAL_TESTNET_CONTRACT" \
  --function-names guard_spend \
  use-manually-provided-public-key "$FIXSEAL_TESTNET_SPEND_PUBLIC_KEY" \
  network-config testnet sign-with-keychain send
```

This is an operator action requiring testnet authorization. Do not generate/export seeds in a logged terminal. The key must also be available for signing through the supported OS keychain flow. Inspect the final on-chain key permission. Revoking the key is the operator's `near account delete-keys` action, with syntax checked against installed CLI help. CLI argument grammar was inspected locally; actual signing is unconfirmed.

## Operator runs

On a host permitting loopback, rebuild the wasm from current source using [SETUP](SETUP.md), then run:

```sh
scripts/demo.sh --check
scripts/demo.sh --guard
scripts/demo.sh
```

`--guard` runs only the guard scenario; the default runs the existing ten path/spend records, then guard. `--subject path|spend` retains the previous subject-only behavior. Preflight checks files, not wasm freshness or binding. Detailed commands and expectations are in [T010 host checks](t010-host-check.md).

For testnet, first prepare funded existing contract/owner/agent/recipient accounts, a source-built initialized contract in local/mock mode, default measurement/PPID approvals, a whitelisted registered valid agent with sufficient expiry, and the restricted key above. No account creation, deployment or registration is hidden in the script. Use only disposable synthetic test accounts. Prepare OS keychain entries without printing keys/seeds.

Set the public environment variables `FIXSEAL_TESTNET_CONTRACT`, `FIXSEAL_TESTNET_OWNER`, `FIXSEAL_TESTNET_AGENT`, `FIXSEAL_TESTNET_RECIPIENT`, `FIXSEAL_TESTNET_OWNER_PUBLIC_KEY`, `FIXSEAL_TESTNET_AGENT_PUBLIC_KEY` (operator-held full access) and `FIXSEAL_TESTNET_SPEND_PUBLIC_KEY` (restricted). Optional `FIXSEAL_TESTNET_RPC` is a credential-free HTTPS testnet RPC. The CLI's `testnet` configuration and independent RPC must serve the same chain.

```sh
scripts/testnet-guard.sh --check
# Only after the operator authorizes testnet RPC/signing and test tokens:
scripts/testnet-guard.sh --execute
```

Default/`--check` only checks local tools. `--execute` makes a fresh output directory, generates and verifies three bundles offline, checks public state/key permissions, then runs the attack/replacement/replay sequence. It starts with `spent=0`, spends 600 then `2^64-500`, and checks actual recipient growth of `2^64+100` yoctoNEAR. The large amount is about 0.0000184467440737 NEAR. Missing/reject records fail, the accept record installs, the same attack fails and a legitimate 100 succeeds. Each transaction hash and final receipt outcome is appended to a copy of [the evidence template](testnet-evidence.md) in `demo-artifacts/testnet-guard/`. Bundle readbacks are replayed offline. No private key/seed is printed or written; CLI diagnostics remain in memory. Unexpected failure stops the sequence and preserves partial public evidence. Never reuse an already initialized guard or treat partial evidence as success. Testnet execution was not performed for T010.
