# Repository instructions for coding agents

Read [README.md](README.md), [JUDGES.md](JUDGES.md) and [ASSUMPTIONS.md](ASSUMPTIONS.md) before evaluation or edits. In a development copy, also read `docs/HANDOFF.md` if present and the active task instruction; these internal documents are intentionally absent from a public export.

## Constraints

- Do not commit, push, send messages, publish, install packages or use external network access without explicit user authorization. Prepared evaluation commands use `--locked --offline`. Only localhost Ollama is permitted by the development task; do not call other services. The explicitly requested local NEAR demo may use loopback on a host where bind is allowed.
- Preserve existing staged/unstaged work and reference logs. Use ignored `demo-artifacts/` for generated bundles/logs. Do not operate on the git index.
- Keep existing naming/comment style. Describe synthetic results as **simulated environment, synthetic data**. Do not claim the code is secure; state that the kernel checked a proof against the fixed specification and fixed tests were rerun. Do not add company, customer or personal names (NyxFoundation design credit is allowed).
- Never bypass checks by weakening tests/specs, adding `sorry`, ignoring failing tests, or changing expected hashes. Record environmental failures explicitly.

## Protected reference inputs

A fixer may change **only its new candidate's `Target.lean` and `Proof.lean`**. Keep its regression file byte-identical to the baseline. Do not edit or shadow:

- `lean/Spec.lean` and `judge/src/lean.rs`'s `SPEC_SHA256`: **`f68c72b5bf0028485f7e6b79f1d6ba3befb69273efed35cb6f395f9b9e5f6c8b`**.
- `lean/Path.lean`, `lean/Main.lean`, `lean/Audit.lean`, `lean/lakefile.toml`, `lean/lean-toolchain`.
- `lean/before/`, `lean/corpus.json`, `lean/policy.json`, existing fixtures/tests, or the judge used to evaluate your candidate.

These are the pinned reference/approval inputs, not evidence that a human approval ceremony has already occurred. An operator must review the spec content and hash before real use. Candidate work never authorizes updating this hash. Task-specific maintenance of the judge requires explicit scope, as in the timeout task; it does not authorize changing the fixed spec.

The judge compares all six protected Lean runtime/build files with trusted bytes and checks the spec hash. It builds fresh temporary projects; do not add `.olean`, executables, build files or extra imports to a candidate. A shadow `Spec.lean` is rejected.

## Commands to run

Use installed Rust/Cargo 1.99.1 and Lean 4.34.1. If needed, set `FIXSEAL_LEAN_BIN` to the actual installed `bin` directory and put it on PATH for direct `lake` commands. No toolchain downloads. From the repository root:

```sh
(cd lean && lake build && lake env lean SpecTests.lean)
(cd lean/targets/spend && lake build)
python3 scripts/check-spend-proof-leak.py --self-test
cargo fmt --manifest-path judge/Cargo.toml -- --check
cargo clippy --manifest-path judge/Cargo.toml --locked --offline --all-targets -- -D warnings
cargo test --manifest-path judge/Cargo.toml --locked --offline
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test fix_proof_tests
mkdir -p demo-artifacts
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean lean after_honest > demo-artifacts/bundle.json
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/bundle.json
```

Expected honest verdict: `accept`, reasons `[]`, proof verified `true`; replay ends `VERIFIED`. Rejected bundles can also be reproducible: check verdict separately. Current judges are `0.1.0/lean-path-7` and `0.1.0/lean-spend-4`. Every Lean build/audit/execution subprocess has a default 120-second wall-clock limit, configurable with positive integer `FIXSEAL_LEAN_TIMEOUT_SECS`; timeout is `PROOF_TIMEOUT`, not `PROOF_REJECTED`. Use matching settings for replay; never conceal a timeout as a proof pass.

For contract changes in authorized maintenance, also run its fmt, clippy, `--lib` and `--test local_demo` checks. Sandbox integration requires supplied wasm/sandbox plus host bind permissions. Use `scripts/demo.sh --check` then `scripts/demo.sh` only within the user's network permissions. Do not attempt a toolchain/runtime download or a bind restriction workaround. See [JUDGES.md](JUDGES.md).

## Add and judge a candidate

Create a fresh name, e.g. `after_example`; do not overwrite an existing fixture:

```sh
cp -R lean/after_honest lean/after_example
```

Edit only `lean/after_example/Target.lean` and `Proof.lean`. Leave `tests/regression.json` unchanged. Then:

```sh
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean lean after_example
cargo run --manifest-path judge/Cargo.toml --locked --offline -- bundle --lean lean after_example > demo-artifacts/after_example.json
cargo run --manifest-path judge/Cargo.toml --locked --offline -- verify demo-artifacts/after_example.json
```

Create `demo-artifacts/` first if it is absent. Direct judging exits 0 for accept, 1 for reject, 2 for malformed invocation/input. Bundle export exits 0 even for a rejected report. Inspect its verdict/reasons. Judge takes source files from the named directory and compiles them; the version name does not select a hardcoded implementation. Do not edit the shared Lake configuration: standalone `lake build` builds `after_honest`; judging an arbitrary version creates its own project.

## Subjects and Lean syntax

Path remains at `lean/`; spend is at `lean/targets/spend/`. Use `--lean --subject spend lean VERSION` for spend, also after `bundle`. `verify` reads the hashed subject from the bundle. Existing commands default to path. The demo runs both subjects sequentially; `scripts/demo.sh --subject path|spend` selects one. Run it only with network permission for loopback NEAR; `--check` performs file presence checks without opening sockets.

Spend Target expressions are limited to UInt64 and Bool operations: `target_forbidden_identifiers` rejects `toNat`/other numeric representations (including qualified and dot forms) and scope/alias/syntax definitions, and representation unpacking is rejected with `POLICY_VIOLATION`; Proof may freely use `toNat`. The trusted `TargetAudit.lean` also inspects the elaborated Target body, including type arguments and let bindings, for Nat/Int/BitVec/Fin constants and rejects them with `POLICY_VIOLATION` before execution. UInt64 literals are allowed; trusted core implementations are not recursively expanded.

The spend fixed specification is `SpendSpec.Correct Target.authorize`: acceptance iff the mathematical sum of the two UInt64 inputs is at most the limit. Its fixed SHA-256 is `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe`. All six shared spend files (SpendSpec, Main, Audit, TargetAudit, lakefile, toolchain), baseline, corpus, policy and regression tests are protected reference inputs. Operator review of content and hash is required before real use. T006 authorizes new subject/judge maintenance; it does not authorize changing the path Spec or its reference hash.

Target wrappers are fixed to the existing path declaration or:

```lean
import SpendSpec
namespace Target
def authorize (limit spent amount : UInt64) : Bool :=
  -- candidate expression
end Target
```

Proofs start with `import Target` and `theorem patch_sound : Spec.Correct Target.join := by` for path, or `theorem patch_sound : SpendSpec.Correct Target.authorize := by` for spend. General core tactics including `intro`, `cases`, `induction`, `rw`, `simp`, `omega`, `constructor`, `exact`, `apply`, `have`, `show`, `unfold`, `split`, `by_cases` and `decide` are supported, with arbitrary ASCII core lemma/local names. Arithmetic, comparison and logical notation are supported. Path target lists retain their restriction to literal strings and parentheses, with empty lists allowed; expressions inside lists are rejected. Strings use JSON-compatible escapes. Attack literal inspection remains a supplemental policy, not a general obfuscation detector.

Additional declarations/imports, environment mutation, metaprogram syntax, macros, IO, initialization, external/unsafe implementations, `sorry`, `admit`, new axioms, `native_decide`, `bv_decide`, `implemented_by`, `extern`, `opaque` and `set_option` are rejected before execution. See `expression_allowed` and the lexer in [judge/src/lean.rs](judge/src/lean.rs). The theorem's name/type, trusted runtime bytes and Spec hash are fixed; axiom audit allows only `propext`, `Quot.sound`, `Classical.choice`. This is a restricted source interface, not a general Lean sandbox.

For spend, the reference proof and any proof-specific hints belong only in `after_honest`, except for the explicitly archived post-experiment records under `examples/fixers/` and `docs/fixer/`. Other fixtures, tests and docs must not copy them. Tests needing a proof must read its file. Run the leak scanner above; it excludes generated artifacts and those two post-experiment directories, and checks other source, fixtures and docs. When preparing an independent fixer copy, always exclude `examples/fixers/`, `docs/fixer/` and every `after_honest` reference directory. A public export includes answers and is not a blind-run copy; the fixture suite itself is not such an experiment.

Use `fixers/nearai/` to run a model only against an answer-withheld copy; its offline unit tests use synthetic temporary copies and never call the API.

## Completion and public export

In a development copy, update the task table in HANDOFF, the active task's final results section, and progress if those files exist. Report changed files, actual tests, measured synthetic results and unresolved issues. Public exports use the development allowlist as described in [docs/PUBLIC.md](docs/PUBLIC.md); new candidates need explicit inclusion for submission. Never send or publish an export without authorization.

- Internal QA reports under `docs/qa/` are excluded from the leak scan: they are never exported and must never be copied into a blind fixer copy.

## T010 spending guard maintenance

T010 explicitly authorizes judge/contract maintenance and extending the trusted spend TargetAudit with expression export. It does not authorize changing either fixed specification/hash, baseline, corpus, policy, candidate fixtures or regressions. Spend-4 uses a small typed AST and the shared interpreter in `shared/policy_expr.rs`; unsupported elaborated expressions fail with POLICY_VIOLATION. Reports/bundles/spend summaries bind canonical policy_expr and policy_expr_sha256; verify re-exports and checks both. EXPORT_MISMATCH rejects any disagreement on attacks, regressions and the fixed 1,000 boundary triples.

The Lean proof concerns Lean's Target.authorize. The contract interprets the exported expression. The exporter and interpreter are fixed trusted code, outside the proof; fixed-input comparison is a check, not a proof. The contract trusts the record submitter and does not use a TEE; anyone can rerun fixseal-judge verify. Owner/full-access/upgrade authority remains trusted. See [guard API, key setup and limits](docs/guard.md).

Run contract guard_tests, lib/fix_proof_tests/local_demo, both crates' fmt/clippy, judge tests, both Lean builds and the leak/export checks. The opt-in local_guard_demo checks real sandbox receipt/balance effects and restricted keys on a permitted host. See [T010 host checks](docs/t010-host-check.md). Do not run testnet-guard.sh --execute without separate external-network/signing authorization; --check uses no RPC. Keep credentials in the OS keychain, never in script output/files.
