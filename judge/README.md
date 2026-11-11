# Judge: English evaluation guide

The local judge evaluates synthetic path and spending examples: **simulated environment, synthetic data**. Use installed Rust/Cargo 1.99.1, Lean 4.34.1, a native C compiler and cached locked dependencies. No account, chain, Ollama or external service is needed. Run from the repository root:

```sh
mkdir -p demo-artifacts
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean lean after_honest > demo-artifacts/bundle.json
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/bundle.json
```

For spend insert `--subject spend` after `--lean` during bundle generation. Replay reads the subject from hashed inputs. Expected honest verdict: `accept`, reasons `[]`, proof verified `true`; replay ends `VERIFIED`. Reproducible rejected bundles also verify: inspect the verdict. Direct judging exits 0 for accept, 1 for reject, 2 for malformed CLI/input; bundle export exits 0 even for reject. The VM-only historical route is `cargo run --manifest-path judge/Cargo.toml --locked --offline -- target after_honest`.

Reports contain judge version, validation scope, verdict/reasons, per-input and aggregate SHA-256, fixed attack/regression counts, policy violations and proof/spec status. Result hashes cover canonical `Report::to_json()` bytes including the final LF. Bundles contain inputs/report/result hash and optional chain readback. `verify` recomputes hashes, rejudges, compares the complete report and checks optional hash/summary fields; it does not authenticate offline chain evidence.

Current Lean judges are path-7 and spend-4. Candidates may edit only Target/Proof and must retain baseline regressions byte-for-byte. The judge checks fixed spec hashes and trusted runtime/build bytes, uses fresh temporary projects, executes fixed attacks and original regressions, then checks the fixed theorem type and allowed axioms. Spend additionally audits the elaborated Target expression for Nat/Int/BitVec/Fin constants before execution; Proof may use these types. Trusted UInt64/core implementations are not recursively expanded. Source restrictions remain supplemental; attack-literal matching is not general obfuscation detection. All Lean subprocesses default to 120 seconds; use matching `FIXSEAL_LEAN_TIMEOUT_SECS` for replay. A timeout is `PROOF_TIMEOUT`, never proof success.

See [JUDGES](../JUDGES.md) for all reason codes, [AGENTS](../AGENTS.md) for candidate syntax, [ASSUMPTIONS](../ASSUMPTIONS.md) for exact properties/trust boundaries and [setup](../docs/SETUP.md) for prerequisites. Run the judge suite with `cargo test --manifest-path judge/Cargo.toml --locked --offline`. The historical implementation/format details follow.

Spend-4 adds a typed expression export from the elaborated Target and compares it with the shared Rust interpreter on 1,028 inputs. Reports/spend summaries include canonical JSON as a policy_expr string and policy_expr_sha256; replay re-exports and checks both. The new EXPORT_MISMATCH reason rejects a comparison disagreement. The fixed exporter/interpreter and the record submitter remain trusted; the Lean proof covers Target.authorize, while fixed-input correspondence is a check, not a proof. No TEE is used. [Wire format, guard and keys](../docs/guard.md).

## Detailed reference

# Fixseal judge

Local judging core for dummy path traversal: results apply to **these attacks and this judge** in a **simulated environment, synthetic data**. No contract, account, Ollama, TEE or external network is used.

Run at the repository root with Rust, cached dependencies and the appropriate PATH:

```sh
cargo test --manifest-path judge/Cargo.toml --locked --offline
cargo run --manifest-path judge/Cargo.toml --locked --offline -- target after_honest
```

Use `before` / `after_hardcode` / `after_weaken_test` as the final argument to reproduce rejection. Stdout is one JSON object plus newline. Exit codes: accept 0, reject 1, read/CLI/JSON transport errors 2 (explanation on stderr, empty stdout). Invalid judging content yields `POLICY_VIOLATION` JSON.

## Targets and execution

Each version under `../target/` has `path.fn` and `tests/regression.json`. Baseline URL-decodes, joins the root and normalizes, allowing escape. Honest checks containment under `/sandbox/files` after normalization. Hardcode rejects one public attack. Weaken_test rejects all inputs and removes normal regressions.

`path.fn` is a small instruction language interpreted by fixed Rust code, without Rust compilation or subprocesses, to support source execution in WASI. The judge reads source and executes instructions in order; version names do not select implementations.

| Instruction | Behavior |
| --- | --- |
| `decode_percent` | Decode `%HH` once as UTF-8; reject malformed encoding, UTF-8 and NUL. |
| `join_root` | Prepend the allowed root to relative paths; leave absolute paths unchanged. |
| `normalize` | Lexically normalize POSIX `/`, `.` and `..`. |
| `require_within_root` | Require the normalized path to equal or descend from the allowed root. |
| `reject_exact STRING` | Reject an exact match with the current string. |
| `reject_all` | Reject every input. |
| `return_path` | Return the current path; required as the final instruction. |

Blank lines and lines starting with `#` are comments. Maximum 64 instructions; no loops, file access or external execution. Paths are virtual strings; no real files are opened. OS-specific separators, filesystem symlinks and other applications' double decoding are outside the model.

## Inputs and verdicts

Directory input `TARGET_ROOT VERSION` reads the following. Absolute paths, directory iteration order and mtime are excluded from results.

- All files in `TARGET_ROOT/before/`: trusted baseline source and regressions.
- All files in `TARGET_ROOT/VERSION/`: candidate, including added files in differences and hashes.
- `TARGET_ROOT/corpus.json`: nonempty `public` and `variants` arrays of attack strings.
- `TARGET_ROOT/policy.json`: exact `editable_paths` / `immutable_paths` and `forbid_attack_literals`.
- The `VERSION` string.

Current VM policy permits editing only `path.fn`; `tests/regression.json` is immutable. Unauthorized additions, deletions, changes and missing required files reject. Original regressions run even if candidate tests change. Candidate corpus/policy copies are unauthorized additions.

Attack-literal checks search editable files for corpus strings and their once-decoded forms. Lean mode decodes permitted string escapes and detects literal lists of URL-decoded, `/`-split attack components. Parentheses/comments compare as the same value (R2). R3 limits Target list elements to string literals and parentheses; `let` / `if` expressions and nested lists reject before execution as `PROOF_CHEATED` (empty lists allowed). Proof `simp [...]` has separate syntax rules. This is not arbitrary obfuscation detection or general program analysis. Baseline, corpus and policy are operator-fixed trusted inputs; replacing them changes the input hash.

| Reason code | Condition |
| --- | --- |
| `VULN_NOT_REPRODUCED` | Baseline does not reproduce every attack, including invalid baseline source. |
| `ATTACK_STILL_WORKS` | Candidate returns any normalized path outside the allowed root. |
| `REGRESSION_FAILED` | Candidate fails any original regression. |
| `POLICY_VIOLATION` | Difference, literal, required-input or input-content rules fail. |

Multiple reasons can coexist. Accept requires no reasons. Empty attack or regression sets cannot produce acceptance.

## Hashes and determinism

JSON includes `judge_version`, `validation_scope`, `verdict`, `reason_codes`, `input_hashes`, `inputs_sha256`, counts and policy violations. VM version is `0.1.0/path-vm-1`; implementation/semantic changes require a version update.

`input_hashes` is a name-sorted map of SHA-256 over each input's raw bytes. Names are `baseline/RELATIVE_PATH`, `candidate/RELATIVE_PATH`, `corpus.json`, `policy.json`, `version`. JSON input whitespace and final newlines are hashed; empty directories and metadata are not.

Aggregate `inputs_sha256` is SHA-256 over `fixseal-inputs-v1` + NUL followed, in name order, by name byte length (u64 big endian), name (UTF-8), content byte length (u64 big endian), content. Framing captures boundaries and renames. No external time, randomness or network is consulted. JSON field/array order and final newline are fixed.

`--stdin` accepts `input::Bundle` JSON: `version` string; `baseline` / `candidate` maps of relative filenames to byte arrays; `corpus` / `policy` byte arrays. Hashes cover embedded raw bytes, not transport JSON whitespace/key order. Equal directory/stdin inputs yield equal outputs.

## WASI and validation status

```sh
cargo build --manifest-path judge/Cargo.toml --target wasm32-wasip2 --locked --offline
```

The code uses standard Rust I/O and file reads without WASI-specific calls. Intended WASI usage supplies a bundle via `--stdin` or preopens a directory. On 2026-11-10 the build stopped at missing target standard libraries (E0463); nothing was installed. WASM build/runtime, native-output agreement and TEE execution remain unconfirmed.

See [demo-output-host-path5.txt](../docs/demo-output-host-path5.txt) for the local host run and [BLOCKED.md](../docs/BLOCKED.md) for environment status. Variant attacks are public dummy replay inputs, not a secret corpus.

## T003: Lean judging

T002 CLI/JSON are retained; `--lean` adds proof checks.

```sh
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean lean after_honest
```

`before` / `after_hardcode` / `after_cheat` reject. A clean Lean build runs original attacks/regressions and difference policy, then checks fixed Spec SHA-256, forbidden syntax, `lake build`, theorem type and axioms. Added reasons: `PROOF_MISSING` / `PROOF_REJECTED` / `PROOF_CHEATED`. Judge version: `0.1.0/lean-path-7`; `proof` contains check status and fixed Spec hash.

`--lean --stdin` uses the same Bundle format. Library calls: `input::load_lean_bundle` and `judge_lean`. Baseline/candidate include 6 shared files (Spec, Path, Main, Audit, Lake configuration, toolchain), hashed with implementation/proof/regressions. With corpus, policy and version, honest has 20 individual hashes. Framing matches T002.

Requires fixed Lean 4.34.1 and a native compiler. Existing behavior/policy reasons take precedence for proofless before/hardcode. Forbidden constructs add `PROOF_CHEATED` even if behavior fails. See [lean/README.md](../lean/README.md) for syntax and 3 commands and [ASSUMPTIONS.md](../ASSUMPTIONS.md) for limits. Lean subprocess execution is native; WASI / TEE execution is unsupported and unconfirmed.

## T004: Offline bundles and verify

```sh
mkdir -p demo-artifacts
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean lean after_honest > demo-artifacts/bundle.json
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/bundle.json
```

With a built binary, run `judge/target/debug/fixseal-judge verify <bundle.json>`. No account/network needed. VM uses `bundle target after_honest`. `bundle` exports valid rejected reports with exit 0; direct judging still uses accept=0 / reject=1.

Evidence fields: `format: "fixseal-bundle-v1"`, `mode: "lean" | "vm"`, `input: Bundle`, `report: REPORT_JSON`, `report_sha256`, optional `chain`. Result SHA-256 covers `Report::to_json()` in fixed struct order, UTF-8, with 1 final LF, not whole-bundle JSON ordering/whitespace. Individual/aggregate input framing is unchanged.

`verify` displays:

- Bundle format and individual/aggregate input hash agreement; mismatch stops before execution.
- Replayed verdict and complete report agreement, including version, reasons, counts and spec.
- Lean fixed spec/helpers/forbidden syntax checks, `lake build` and axiom checks. Proofs of behaviorally rejected candidates are also checked and their failures displayed.
- Result hash agreement and, when `chain` exists, `get_fix_proof.bundle_hash` and `summary` agreement.

`chain` is raw contract readback `{bundle_hash, caller, block_timestamp_ms, summary}`. Summary is a JSON string of `{inputs_sha256, judge_version, verdict}`, sorted by key without spaces. The demo records result SHA-256 as `bundle_hash` and report summary as summary. This CLI does not authenticate record existence, caller or timestamp. Use trusted readback provenance, judge binary and reference inputs.

Exit 0=`VERIFIED` (accept / reject reproduced), exit 1=`VERIFICATION FAILED` (mismatch), exit 2=file/JSON/argument error. Proofless hardcode, cheating proof and regression-failing reject_all can all have reproducible rejections. `VERIFIED` for a rejected report does not accept the repair.

For another Lean binary location set `FIXSEAL_LEAN_BIN`; no fallback to downloading proxies. See [setup](../docs/SETUP.md). Literal checks advanced path-2 to path-3 in R1, path-4 in R2 and path-5 in R3. T005 added wall-clock limits / `PROOF_TIMEOUT` as path-6. Old path-2 / path-3 / path-4 / path-5 bundles have different full reports and fail current replay. Input hashes, Spec and VM version were unchanged.

## Lean subprocess limits (path-6)

Each build, axiom check and execution defaults to 120 seconds. `FIXSEAL_LEAN_TIMEOUT_SECS` accepts positive integer seconds; invalid values / 0 use default. Exceeding it rejects as `PROOF_TIMEOUT`, distinct from `PROOF_REJECTED`. Unix terminates the process group and reaps its direct child. Timeouts are uncached; other caches are limit-specific. Environment settings are outside input hashes, so replay requires matching toolchain/limit. [FIX5 host record](../docs/demo-output-host-fix5.txt): all 10 path-7 / spend-3 versions matched / demo exit 0.

## T006: Multiple subjects and general core proofs

Default path keeps its location, Spec and input hashes; syntax changes use `0.1.0/lean-path-7`. New `spend` lives at `lean/targets/spend/`, historical version `0.1.0/lean-spend-3`. Each subject has a fixed Spec/hash, runtime/audit, Lake configuration, toolchain, corpus, policy and Target/Proof wrappers.

```sh
mkdir -p demo-artifacts
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean --subject spend lean after_honest > demo-artifacts/spend.json
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/spend.json
```

Library: `load_lean_subject_bundle(root, version, Subject::Spend)`. Spend Bundle has `subject: "spend"`, included in input hashes. Omitted path subject retains the original transport/hashes. `--lean --stdin` and verify read the bundle subject; conflicting explicit selection is a CLI error. VM rejects spend. CLI `--subject` follows `--lean`.

Spend Target permits UInt64 / Bool operations; `target_forbidden_identifiers` applies only to Target. Namespace/dot components are checked, excluding comments/strings. Conversions, scope / alias / syntax definitions, numeric projections and representation patterns reject before execution with POLICY_VIOLATION. Proof toNat is unrestricted. After_cheat Target uses UInt64/Bool and passes attacks 0/12 / regressions 16/16, but rejects with PROOF_CHEATED alone. All 5 verdicts remain unchanged.

Spend corpus retains public / variants arrays, each containing base-10 UInt64 `limit spent amount` triples; regression expected values are strings `"true"` / `"false"`. Inputs are checked with u128 mathematical sums, then the actual Lean function is built/run. There are 12 attacks and 16 regressions. Literal checks detect complete sets of 3 numbers, not common boundary constants alone; arbitrary obfuscation is outside scope.

Proof accepts general core tactics/lemmas. Extra declarations/imports, dangerous execution and environment changes reject; fixed Spec/runtime/type and axiom checks remain. See [AGENTS.md](../AGENTS.md). This restricted source interface is not a general sandbox. Caches are separated by subject.

See [README table](../README.md#agent-spending-limit-overflow) for all 5 versions. In a simulated environment, synthetic data, only honest accepts; saturate allows 3 maximum-limit attacks and rejects. Keep proof bodies/hints only in honest; tests read that file. Judge tests also run `python3 scripts/check-spend-proof-leak.py --self-test`.

`scripts/demo.sh` runs path → spend; `--subject path|spend` selects one; `--check` checks presence only. FIX5 path-7 / spend-3 was [confirmed by the operator on the host](../docs/demo-output-host-fix5.txt). Run only on a host explicitly permitting loopback NEAR, never during network-prohibited work. Old path-6 bundles also need regeneration because full reports differ.

## T010: spend-4 export

TargetAudit traverses elaborated Target into a restricted expression tree. Canonical JSON orders op/args, has no spaces/final newline and uses base-10 numeric strings. Reports/spend summaries contain this JSON string and SHA-256. Summary adds subject/spec_sha256/policy_expr/policy_expr_sha256 to the existing 3 fields, sorted by key; path format is unchanged. Lean/shared interpreter outputs are compared on 12 attacks + 16 regressions + 1,000 boundary inputs; disagreement gives EXPORT_MISMATCH. Verify re-exports/checks the expression. See [guard guide](../docs/guard.md). Old spend-3 host records are historical, not current spend-4 chain checks.

## Terminal verdict summary

Direct judging and bundle export show a plain verdict/reasons/attack/regression summary on stderr when it is a terminal. Use a leading `--summary` for captured output, e.g. `fixseal-judge --summary --lean --subject spend lean after_honest`. Stdout remains JSON, and noninteractive callers without the flag retain empty summary stderr. No ANSI color is used, so `NO_COLOR` is respected. Lean diagnostics may precede the summary. `verify` retains its own final `VERIFIED` / failure status.
