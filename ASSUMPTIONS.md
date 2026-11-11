# Proof scope and trust assumptions

Fixseal reports a **simulated environment, synthetic data**. The Lean kernel checks a proof against a fixed specification and the judge reruns fixed tests. This does not establish that arbitrary code or real filesystem access is secure. Local attestation is mock.

A later [operator host result](docs/t010-host-check.md#host-result-operator-2026-11-10-1420-jst) reports a current-source wasm rebuild and successful local guard sandbox payments/key checks. The T013 coding task inspected the supplied logs offline; it did not rerun that build or chain execution. Historical statements below describe the earlier tasks. On 2026-11-10 the guard scenario also ran on NEAR testnet with disposable accounts and mock attestation ([run](docs/testnet-run-2026-11-10.md)). Real TEE and WASI remain unconfirmed.

For path, `Spec.Correct Target.join` is the conjunction of `Contained` and `AcceptsRelative`. Every returned normalized path must have the allowed root as a prefix. For relative requests with ordinary root/request components (neither empty, `.` nor `..`), the function must return the normalized concatenation, including for empty lists. Rejecting everything therefore fails. The theorem quantifies over every request, including its `raw` field, but does not prove that raw strings correspond to parsed components, that components contain no slash, or that lexical normalization matches OS behavior.

For spend, `SpendSpec.Correct Target.authorize` requires, for every UInt64 input, acceptance exactly when the mathematical sum of spent and amount is within limit. It covers neither keys, currency units, concurrent state updates nor payments. Target policy combines source inspection with an audit of the elaborated definition: constants in the Nat, Int, BitVec and Fin families are rejected, including inferred type arguments and unused let bindings. UInt64 literals remain permitted. This audit inspects the definition's expression, not the transitive implementations of trusted UInt64/core functions. Proof arithmetic is unrestricted by this Target policy.

Trusted inputs include the reviewed specification, the judge, baseline, corpus, original regressions, policy, Lean 4.34.1 kernel and standard library/axioms, Lake, compiler/runtime, native C toolchain and host OS. Path parsing, URL decoding, JSON I/O, Rust reference arithmetic and hashing are outside the theorem. Symlinks, mounts, permissions, case sensitivity, Windows paths, TOCTOU and actual file I/O are outside the model. The fixed spec hashes are `f68c72b5bf0028485f7e6b79f1d6ba3befb69273efed35cb6f395f9b9e5f6c8b` (path) and `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe` (spend). An operator must review their content and hashes before real use; pinned hashes do not record human approval.

Only candidate Target/Proof files may change. Runtime/build inputs are compared with trusted bytes, and fresh temporary projects replace candidate build artifacts. Fixed wrappers, source restrictions, theorem types and an axiom audit permit only `propext`, `Quot.sound`, `Classical.choice`. This is a restricted source interface, not a general Lean sandbox. Each build/audit/execution subprocess has a default 120-second limit; positive integer `FIXSEAL_LEAN_TIMEOUT_SECS` overrides it. Timeouts produce `PROOF_TIMEOUT`, are not cached, and require matching settings on replay. CPU, memory and output limits remain outside this protection.

`verify` rechecks input/result hashes and the complete judge report. `VERIFIED` also applies to reproducible rejections: inspect the verdict. Optional chain hash/summary comparisons do not authenticate offline readback JSON, caller or timestamp. The contract stores authorized agent claims and does not run Lean. The operator's [FIX5 host record](docs/demo-output-host-fix5.txt), path-7/spend-3, completed all ten versions with matched readbacks/replays and demo exit 0. This task performed no chain run. Real TEE, WASI, testnet and supplied wasm/source correspondence remain unconfirmed.

The proof leak scanner checks all text files regardless of extension and rejects encountered symlinks, with documented generated/archive exclusions. It detects copied fragments and named hints, not all semantic paraphrases. Independent fixer copies must exclude all reference `after_honest` directories and the post-experiment `examples/fixers` and `docs/fixer` records. See [replay records](examples/fixers/README.md).

## T010 exported policy and spending guard

The proof is about Lean's `Target.authorize`. The contract evaluates the exported expression using `shared/policy_expr.rs`, the same source included by the judge. Both the fixed exporter and interpreter are trusted and outside the proof. Comparing actual Lean outputs on the 12 attacks, 16 regressions and 1,000 boundary triples is a fixed-input check, not a proof of correspondence. Unsupported exports are rejected; any comparison disagreement is EXPORT_MISMATCH. Anyone can rerun `fixseal-judge verify` to re-export, hash and check the bundle/proof with trusted reference inputs.

The contract trusts the authorized record submitter's verdict/spec/expression claims, without a TEE or verifier quorum (N=0). Owner policy installation checks accept/spend, pinned spec hash, canonical expression hash/content and allowlisted spend-4 judge. Full-access keys and owner code upgrades can bypass the policy and remain trusted. Restricted agent keys allow only guard_spend on the intended receiver, zero deposit; access-key gas allowance does not itself limit payment arguments or recipients. The contract's own funds pay recipients. Successful transfer callbacks update total_paid; pending reservations remain separate, and callback failure requires investigation. A formula replacement cannot recover past payments or reconstruct the true cumulative amount from wrapped spent. [API and limits](docs/guard.md), [host checks](docs/t010-host-check.md), [testnet evidence template](docs/testnet-evidence.md). T010 chain execution was unconfirmed at implementation time; see the later operator host result above.

## Detailed reference

# Proof scope and trust assumptions

Fixseal uses dummy paths in a **simulated environment, synthetic data**; it opens no real files. On 2026-11-10 the operator completed recording, readback and replay on a local NEAR sandbox: all 5 path-5 versions matched / exit 0 ([record](docs/demo-output-host-path5.txt)); all 10 T008 path-7 / spend-2 versions matched / exit 0 ([record](docs/demo-output-host-t008.txt)); all 10 FIX5 path-7 / spend-3 versions matched / demo exit 0 ([record](docs/demo-output-host-fix5.txt)). Bind permissions block the restricted agent environment. Local attestation is mock, not a real TEE.

## Properties proved

`Correct Target.join` in `lean/Spec.lean` combines `Contained` and `AcceptsRelative`. Every returned normalized path has the allowed root as a prefix. For relative requests whose root and request.parts contain no empty strings, `.` or `..`, the function must return `some (normalize (root ++ request.parts))`. Empty lists are included, so rejecting everything fails. Acceptance of absolute paths or components containing empty strings, `.` or `..` is outside this property; original regressions also run.

Inputs have `absolute`, `parts` and `raw` (used to reproduce the hardcoded demo fix). The theorem quantifies over all values, not just the corpus. It does not establish the contents of `raw`, slash-free components or correspondence between strings and components. `Path.parse` decodes URLs once and splits on `/`. `Spec.normalize` defines the lexical semantics for review; correspondence with OS behavior is unproved.

`after_honest/Proof.lean` uses no `sorry` or additional axioms. `Audit.lean` fixes the theorem type to the same `Target.join` called by the executable. The current proof depends on standard Lean axioms `propext` and `Quot.sound`; the judge permits only these 2 and `Classical.choice`.

## Trusted components

- Human review and pinning of the specification. T004 pins SHA-256 `f68c72b5bf0028485f7e6b79f1d6ba3befb69273efed35cb6f395f9b9e5f6c8b` in the judge and compares the supplied spec hash. This implementation reference does not generate an approval history. The operator must review the content and hash before use.
- Lean 4.34.1 kernel, standard library, axioms, compiler, runtime, Lake and native C toolchain, plus the fixed judging environment. Correctness down to machine code is unproved.
- `Path.lean` string parsing, UTF-8 checks and URL decoding; `Main.lean` JSON I/O and path reconstruction; judge execution, reason codes, hashes and policy checks. Attack, regression and boundary tests check these outside the theorem.
- OS and filesystem. Symlinks, mounts, permissions, case sensitivity, Windows paths, TOCTOU and real file I/O are outside the model. Lexical containment alone does not establish correct OS behavior.
- Operator-pinned judge binary, baseline, corpus, regressions, policy and their hashes. Variant attacks are public in this demo. Replacing arbitrary judging inputs is outside this agreement.

## Judging limits

Lean mode is native only. It builds `lake build pathcheck` with trusted helpers in a fresh temporary directory and runs attacks and original regressions. Only candidates passing behavioral checks proceed to `lake build` and axiom checks. Existing attack, regression and policy reasons are retained. Forbidden constructs are detected first and add `PROOF_CHEATED` even if other checks fail.

Fixers may edit only `Target.lean` and `Proof.lean`. Spec, runtime, helpers, audit, build configuration and toolchain are checked against trusted bytes; supplied build artifacts are unused. Fixed wrappers and forbidden-token checks allow core expressions and general proof tactics, but reject extra imports, macros, initialize, IO, unsafe/external implementations, `sorry`, `axiom`, `native_decide`, etc. before execution. Valid general Lean programs can also fail these restrictions. This is not a general Lean sandbox.

In-process caches use source/input SHA-256; kernel caches distinguish both implementation and proof. Candidate `.olean` files and executables are untrusted. Caches, build logs, temporary paths and timestamps are excluded from judging JSON. Each Lean build, axiom check and execution has a default 120-second wall-clock limit. Positive integer `FIXSEAL_LEAN_TIMEOUT_SECS` overrides it; invalid values and 0 use 120 seconds. Timeouts reject as `PROOF_TIMEOUT` and are not cached. Other cache keys include the limit. Unix terminates the process group and reaps the direct child. CPU, memory and output limits need separate controls. Replay requires the same toolchain and limit.

A missing toolchain causes rejection; acceptance is not guaranteed across arbitrary environments.

## T004 verification and chain records

`verify` recomputes input/report/result hashes and rechecks Lean restrictions, trusted code and proofs. Reproducible rejections also produce VERIFIED. Optional chain readback checks the result hash and verdict / judge version / aggregate input hash summary. Offline JSON does not authenticate record existence, caller or timestamp; no RPC is used. Replacing both inputs and expectations can create another consistent bundle, so pin the trusted judge, baseline / corpus / policy and result-hash provenance. The contract stores authorized agent claims; it does not check proofs or verdicts.

## T006 spend proof scope

`Correct Target.authorize` in `lean/targets/spend/SpendSpec.lean` requires, for every UInt64 input, equivalence between true authorization and `spent.toNat + amount.toNat ≤ limit.toNat`. The reference hash is `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe`. No proof lemmas are provided. The Lean kernel checks a proof against this fixed specification; human approval history, keys, currency units, concurrency, real payments and wallet behavior are outside the theorem.

Spend Main / Audit / Lake / toolchain bytes are trusted and fixed; candidate build settings/artifacts are unused. I/O conversion and judge u128 reference arithmetic are checked by tests outside the proof. The same 3 standard axioms are allowed. Subject is included in input hashes and cache keys; verify reruns that subject. The spend scanner detects copied proofs and named lemma hints, not all semantic paraphrases. See [FX3 records](examples/fixers/README.md) for blind experiments. Post-experiment records are excluded from scanning and future fixer copies.

T007 spend-2 checks forbidden Target identifiers (including namespace/dot forms) and representation unpacking before execution. Proof is exempt. This source policy does not fully analyze general Lean types/semantics. T008 restricts after_cheat Target to UInt64/Bool: attacks 0/12 and regressions 16/16 pass, but its invalid proof rejects with PROOF_CHEATED alone.

## T010 additional assumptions

The proof covers Lean Target.authorize; the contract evaluates its exported expression with a shared Rust interpreter. The fixed exporter/interpreter are trusted, outside the proof. Comparison on 1,028 inputs is a fixed-input check, not a proof. The record submitter is trusted; no TEE or verifier quorum is used. Anyone can rerun fixseal-judge verify. Owner, full-access keys and upgrades remain trusted; function-call receiver/method restrictions and gas allowance do not limit payment arguments or recipients. Replacing a rule cannot undo past transfers. Check real payments and callbacks on the host.
