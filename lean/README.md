# Lean targets, specification and proofs

This project uses Lean 4.34.1 with core/Std only, no Mathlib or external Lean dependencies. Results describe a **simulated environment, synthetic data**. The same candidate function is executed in fixed tests and named in the theorem. Run from the repository root with native Lean binaries on PATH:

```sh
(cd lean && lake build && lake env lean SpecTests.lean)
(cd lean/targets/spend && lake build)
cargo test --manifest-path judge/Cargo.toml --locked --offline
python3 scripts/check-spend-proof-leak.py --self-test
```

A standalone build is insufficient: Lean can compile a `sorry` proof with a warning. The judge also checks source policy, trusted bytes, fixed hashes/theorem type, kernel proof and axiom dependencies. Allowed axioms are `propext`, `Quot.sound`, `Classical.choice`. Each subprocess has a default 120-second limit, configurable by positive integer `FIXSEAL_LEAN_TIMEOUT_SECS`; timeouts reject as `PROOF_TIMEOUT`.

Path lives here; spend is a separate project in `targets/spend`. Default Lake builds use `after_honest`. To add a candidate, copy to a fresh name, edit only its Target/Proof files, keep tests byte-identical and judge via `--lean lean VERSION` (insert `--subject spend` for spend). The judge compiles the named directory in a fresh project; names do not select hardcoded implementations. Never change the shared build configuration or spec to make a candidate pass.

Path's theorem is `Spec.Correct Target.join` (containment and acceptance of ordinary relative requests). Spend's is `SpendSpec.Correct Target.authorize` (acceptance iff mathematical sum is within limit). Spec hashes and precise trust boundaries are in [ASSUMPTIONS](../ASSUMPTIONS.md). Fixed shared code includes parsing/execution Main, proof Audit, Lake configuration and toolchain; spend also includes `TargetAudit.lean`, which rejects Nat/Int/BitVec/Fin constants in the elaborated Target body, types and let bindings before execution. UInt64 literals are allowed; Proof is exempt. It does not recursively audit trusted core implementations.

The fixed wrapper/import/theorem name and type must be retained. Core tactics such as `intro`, `cases`, `induction`, `rw`, `simp`, `omega`, `constructor`, `exact`, `apply`, `have`, `show`, `unfold`, `split`, `by_cases`, `decide` are supported. Extra declarations/imports, environment mutation, IO, unsafe/external code, `sorry`, new axioms and native decision tactics are rejected. This is a restricted source interface, not a general Lean sandbox. Path Target lists allow literal strings and parentheses only. See [AGENTS](../AGENTS.md) for exact wrappers and commands.

The proof leak scanner covers text of every extension and encountered symlinks, excluding specified build/generated directories and post-experiment archives. For independent fixer copies remove all reference `after_honest` directories, `examples/fixers` and `docs/fixer`. [Archived attempts](../examples/fixers/README.md) are replay evidence, not a new blind experiment. Current judges: path-7/spend-4. Historical details follow.

T010 spend-4 also exports supported elaborated UInt64/Bool expressions through TargetAudit. Unsupported fragments fail closed. The judge shares a Rust interpreter with the spending contract and compares it with Lean on a fixed 1,028-input set. The exporter/interpreter are trusted, outside the proof; comparison is a check, not a proof. [Expression and guard guide](../docs/guard.md).

## Detailed reference

# Lean targets, spec and proofs

The T002 dummy path traversal is implemented in Lean, with the same function tested and proved. All results describe a **simulated environment, synthetic data**. [ASSUMPTIONS.md](../ASSUMPTIONS.md) defines scope and trust.

## Check in 3 commands

Run at the repository root with installed Lean 4.34.1, Rust toolchains and dependency caches. No downloads are needed. Put cargo / rustc on PATH.

```sh
export FIXSEAL_LEAN_BIN="${FIXSEAL_LEAN_BIN:-$HOME/.elan/toolchains/leanprover--lean4---v4.34.1/bin}"
export PATH="$FIXSEAL_LEAN_BIN:$PATH"
(cd lean && lake build)
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean lean after_honest
```

`lake build` builds the honest implementation, proof, audit and executable. The final judge command checks the spec hash, forbidden syntax, clean build, axioms, attacks and original regressions, then emits accept JSON. Lean alone accepts `sorry` with a warning: **a successful `lake build` is insufficient for acceptance**.

Run all tests with:

```sh
cargo test --manifest-path judge/Cargo.toml --locked --offline
```

## Files and editing scope

| File | Role |
| --- | --- |
| `Spec.lean` | Specification for human review: component lists, lexical normalization, root containment and universal properties. Fixers must not edit it. |
| `after_honest/Target.lean` | Normalizes, checks root containment and returns the path. |
| `after_honest/Proof.lean` | Proof of `Spec.Correct Target.join` (containment + ordinary relative input acceptance), without `sorry` or extra axioms. |
| `before/Target.lean` | Vulnerable implementation allowing escape from the root; no proof. |
| `after_hardcode/Target.lean` | Rejects only public attack strings; variants pass. No proof. |
| `after_cheat/` | Honest implementation with a `sorry` proof: builds, but the judge rejects it. |
| `after_weaken_test/` | Honest implementation/proof with weakened regressions; the judge uses originals and rejects test changes. |
| `after_reject_all/` | Always returns none; fails regressions. Its copied honest proof also fails the new spec. |
| `SpecTests.lean` | Examples showing reject-all passes the old spec but fails the new one; run `lake env lean SpecTests.lean`. |
| `Path.lean` / `Main.lean` | Single URL decoding, component splitting and JSON Lines I/O; outside the proof. |
| `Audit.lean` | Fixes the theorem type to the executed function and prints axiom dependencies. |
| `corpus.json` / each version's `tests/regression.json` | T002 dummy attacks/regressions; the judge uses baseline regressions. |
| `lakefile.toml` / `lean-toolchain` | Std/Init and Lean standard JSON only; no external packages or Mathlib. |

The default Lake project reads the 2 after_honest files. Change the final judge argument to `before`, `after_hardcode` or `after_cheat` to test others. The judge compiles the named source rather than selecting an implementation by name. Shared spec/runtime/build inputs are fixed and included in each input hash; shadow Spec files in candidates are rejected.

The judge directly uses `~/.elan/toolchains/leanprover--lean4---v4.34.1/bin/lake` (or the real binary in `FIXSEAL_LEAN_BIN`) to avoid elan downloads. Candidate build settings/artifacts are unused. Fixed imports/types, wrappers, forbidden tokens and axiom checks restrict the accepted source interface.

Spec reference SHA-256 is `f68c72b5bf0028485f7e6b79f1d6ba3befb69273efed35cb6f395f9b9e5f6c8b`, pinned in `judge/src/lean.rs`. Before operation, a human must review and approve the content and pin it with the judge binary. Fixers cannot update it.

## Run the attack corpus directly

After building, `lean/.lake/build/bin/pathcheck` reads JSON strings from stdin and emits a JSON path string or `null` per 1 input line. No real files are opened. This sends the 11 T002 attacks directly to the honest implementation:

```sh
python3 - <<'PY' | lean/.lake/build/bin/pathcheck
import json
from pathlib import Path
corpus = json.loads(Path('target/corpus.json').read_text())
for attack in corpus['public'] + corpus['variants']:
    print(json.dumps(attack))
PY
```

All outputs are `null`. The judge runs every version in this format and counts escapes using the existing Rust lexical path check. The proof is not replaced by a Rust model.

## Reasons and expectations

**simulated environment, synthetic data**: baseline attacks succeed 11/11 for every version. Original regressions pass 2/11 for reject_all and 11/11 for the others.

| Version | Verdict | Candidate attacks working | Reasons |
| --- | --- | --- | --- |
| before | reject | 11/11 | ATTACK_STILL_WORKS |
| after_honest | accept | 0/11 | None; kernel check passed. |
| after_hardcode | reject | 10/11 | POLICY_VIOLATION, ATTACK_STILL_WORKS |
| after_cheat | reject | 0/11 | PROOF_CHEATED |
| after_weaken_test | reject | 0/11 | POLICY_VIOLATION |
| after_reject_all | reject | 0/11 | REGRESSION_FAILED (separate kernel check: PROOF_REJECTED) |

T004 strengthens `Correct = Contained ∧ AcceptsRelative`. For relative inputs whose root and components exclude empty strings, `.` and `..`, `AcceptsRelative` requires the normalized concatenation. Empty lists are included, so reject-all fails. Acceptance of absolute paths or empty/`.`/`..` components is outside this property; regressions still run.

Each Lean build, axiom check and execution has a default 120-second wall-clock limit. Positive integer `FIXSEAL_LEAN_TIMEOUT_SECS` overrides it; exceeding it rejects as `PROOF_TIMEOUT`. Current judges: `0.1.0/lean-path-7` and `0.1.0/lean-spend-4`.

For candidates passing behavior checks, absent proof gives `PROOF_MISSING`; allowed syntax with a failed build/type check gives `PROOF_REJECTED`. `sorry`, extra axioms, forbidden tactics, changed spec/helpers or unsupported Lean syntax gives `PROOF_CHEATED`. Existing T002 violations retain their reasons without adding a missing-proof reason.

## References

Design credit: [NyxFoundation/nyxis](https://github.com/NyxFoundation/nyxis) (MIT), for separating human specification approval, AI implementation/proof and Lean kernel checking, with explicit trust assumptions. The design v0.3 reference [nyx-ssz](https://github.com/NyxFoundation/nyx-ssz) (MIT) also motivates stating unproved scope. License labels follow supplied task/design materials; network restrictions prevented fetching, browsing or reusing code. Rust/Lean correspondence (e.g. Aeneas) remains future work.

## Fixer syntax (T006)

Wrappers, imports, theorem name and type are fixed. General core tactics/lemmas, ASCII hypothesis names and arithmetic/comparison/logical notation are allowed, including `rw` / `induction` / `omega` / `by_cases` / `decide`. Extra declarations, environment changes, metaprograms, IO, unsafe, `sorry` / `native_decide` / `bv_decide` are rejected before execution. See [AGENTS.md](../AGENTS.md) and judge `expression_allowed` / lexer. Trusted bytes, Spec hash, fixed type, kernel and axiom checks remain.

Path Target lists allow only literal strings and parentheses, including empty lists. Expressions inside elements and nested lists are rejected. Strings use JSON-compatible escapes; nested comments are recognized. Literal checks supplement policy and do not detect arbitrary obfuscation.

## Spending limit overflow (spend)

Spend Target allows only UInt64 and Bool operations. `target_forbidden_identifiers` rejects `toNat`, other numeric representations and scope/alias/syntax definitions (including namespace/dot forms); representation unpacking also gives `POLICY_VIOLATION`. Proof may freely use `toNat`.

`targets/spend/` is an independent Lake project without Mathlib/external packages. `SpendSpec.lean` defines only equivalence between true authorization and a mathematical sum within the limit for `UInt64 → UInt64 → UInt64 → Bool`; it contains no proof lemmas. Fixed hash: `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe`. Human content/hash approval history has not been created.

```sh
(cd lean/targets/spend && lake build)
cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean --subject spend lean after_honest
python3 scripts/check-spend-proof-leak.py --self-test
```

Input is a JSON string containing 3 base-10 UInt64 values `limit spent amount`, separated by spaces; output is the JSON string `"true"` / `"false"`. Malformed/out-of-range values are rejected. The judge checks corpus/regression validity with u128 arithmetic. Executed `Target.authorize` is the function named by the theorem.

**simulated environment, synthetic data**: 12 attacks, 16 regressions (9 allow, 7 refuse). Baseline attacks succeed 12/12. Honest: attacks 0/12, regressions 16/16, accept. Hardcode: 11/12 attacks and a policy violation. Cheat: attacks 0/12, regressions 16/16, rejected with PROOF_CHEATED alone. Reject_all: regressions 7/16. Saturate: attacks 3/12, reject; saturation misses mathematical overflow at the maximum limit.

Spend reference proofs and proof-specific hints belong in `after_honest`. The scanner checks other versions, source and docs, with self-tests inserting copied fragments. Generated artifacts and post-experiment `examples/fixers/` / `docs/fixer/` records are excluded. Remove those records and every reference `after_honest` directory from independent fixer copies. See [FX3 records and replay](../examples/fixers/README.md).
