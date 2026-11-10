# Testnet guard evidence template

No testnet transaction has been executed for T010. This is a template, not evidence of success.
The operator script copies this file to a fresh `demo-artifacts/testnet-guard/` directory and appends public transaction hashes and final receipt outcomes, including expected rejections. Keep a partial run as failed/incomplete; a transaction hash alone is not success.

- Run date: pending operator run
- Network: testnet
- Judge-check contract account: ``
- Judge-check accepted record ID: ``
- Environment: **simulated environment, synthetic data** / 仮想空間での検証
- Rebuilt wasm SHA-256 and source/build provenance: pending
- Judge: `0.1.0/lean-spend-4`
- Fixed specification: `e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe`
- Accepted record ID and policy expression SHA-256: see the generated bundle and final guard-state.json
- Sender/recipient balance checks, complete receipt outcomes: pending
- Key permission check: exactly guard_spend, intended receiver, zero attached deposit
- Secrets/seed/private keys: never include

The contract trusts the record submitter; it does not run Lean or use a TEE. Anyone can rerun `fixseal-judge verify` on the saved bundles. The Lean kernel checks a proof about the Lean Target against the fixed specification; the exporter and shared interpreter are trusted, and comparison on fixed inputs is a check rather than a proof.

The two judge-check fields stay empty until a separately authorized testnet run. With explicit read-only network permission, `scripts/judge-check.sh --stage a --allow-testnet-read` calls only `guard_state` and `get_fix_proof` on the final testnet state. Empty fields or unavailable transport produce A3 SKIP; RPC errors, missing records or mismatched policy data fail. The default command never contacts RPC. No keys or signing are used.
