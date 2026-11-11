# Environment status: English summary

The operator's [FIX5 host record](demo-output-host-fix5.txt) completed path-7/spend-3 on 2026-11-10: both Lean builds, ten matched readbacks/replays and demo exit 0. The operator also reports 66 judge tests and 72 contract tests passing; full suite output is not part of the linked demo log. All results describe a **simulated environment, synthetic data**. This task does not run the chain.

Restricted agent environments may lack localhost bind permission and fail with `Operation not permitted`; this is not a completed chain test. Prepared offline Lean/judge and contract mock tests need no socket. Chain tests require preinstalled near-sandbox 2.11.0, matching caches, supplied contract wasm and explicit loopback permission. `scripts/demo.sh --check` checks file presence only. Use [SETUP](SETUP.md) for setup/wasm commands and [JUDGES](../JUDGES.md) for expected output. Never work around bind restrictions or download tools without authorization. WASI, real TEE/testnet, CI execution and wasm/source correspondence remain unconfirmed. Historical notes below retain their original context.

## Detailed reference

# Environment status and resumption requirements

Updated: 2026-11-10 (T009). Initial setup, wasm generation and offline commands are collected in [SETUP.md](SETUP.md).

## Validation provenance

- On 2026-11-10 the operator completed path-5 in the host shell with `exit=0`. [demo-output-host-path5.txt](demo-output-host-path5.txt) records deploy → whitelist/register → submit → get → verify for all 5 versions, `matched` / `VERIFIED`. **simulated environment, synthetic data**.
- At this historical stage, judges were `0.1.0/lean-path-7` and `0.1.0/lean-spend-3`. The T006 host record (path-7 / spend-1, all 10 versions matched) is preserved below. [T008 host record](demo-output-host-t008.txt) confirms all 10 path-7 / spend-2 versions matched / exit 0. [FIX5 operator host record](demo-output-host-fix5.txt) confirms all 10 path-7 / spend-3 versions matched / demo exit 0. This task permitted only localhost Ollama and did not start NEAR. Agent-environment localhost bind fails with `Operation not permitted (os error 1)`; this is not a completed chain run.
- Offline judge tests, Lean builds/spec tests and mocked contract unit tests run with existing caches. Sandbox integration failures are not converted to ignored tests to claim success.

## Missing binaries or wasm

The demo reports missing resources and exits 2 without downloads/installations. `scripts/demo.sh --check` checks presence only. [SETUP.md](SETUP.md) lists versions and build/provisioning steps. A clone alone does not enable an initial fully offline build.

Set `NEAR_SANDBOX_BIN_PATH` to the absolute path of an installed sandbox 2.11.0. Without it, contract-test dependencies may attempt runtime downloads; Cargo `--offline` alone does not prevent subprocess networking.

## Missing socket permissions

```text
Local sandbox could not start: Error while binding socket to a port 0:
Operation not permitted (os error 1)
```

Rerun with identical code/resources in a normal shell permitting localhost bind / listen. Do not change permissions or bypass restrictions. No testnet account/private key is needed. External peers, telemetry and RPC fallbacks are disabled.

```sh
scripts/demo.sh
```

Each run writes `demo-artifacts/demo-output.txt` and replay JSON under `demo-artifacts/path/*.json` / `demo-artifacts/spend/*.json`, preserving public reference logs. Check both `matched` for all 10 versions (5 per subject) and process exit 0.

## Outstanding checks at that stage

- Supplied wasm/source correspondence. R1 lacked wasm standard libraries and could not rebuild.
- Judge WASI build/runtime: missing `wasm32-wasip2` standard library. Native Lean subprocesses do not support WASI.
- Real TEE / testnet. Local attestation is mock.
- Chain execution of anonymized public test wasm. FIX5 host logs use development resources; they do not establish public-copy execution.
- Contract clippy diagnostics were resolved in R2; T005 all-targets / -D warnings also passed. The dependency wasmparser future-incompatibility notice remains.
- Incomplete third-party license inventory and current judge / Lean SBOM. Missing license originals were unavailable locally and were not fetched.

## 2026-11-10 operator host check (path-5)
After R3, lake build, all judge/contract tests and scripts/demo.sh passed outside the restricted sandbox (demo exit 0; all 5 chain/verify results matched). Log: docs/demo-output-host-path5.txt

## 2026-11-10 operator host check (path-6, after T005)
lake build, 47 judge tests, 72 contract tests and demo.sh passed (exit 0; all 5 versions matched). Log: docs/demo-output-host-path6.txt

## 2026-11-10 operator host check (after T006, path-7 / spend-1)
Both lake builds, proof leak scan, 57 judge tests, 72 contract tests and both demo.sh subjects passed (exit 0; all 10 versions matched). Log: docs/demo-output-host-t006.txt

## 2026-11-10 operator host check (after FIX5)
Internal docs/qa/ reports were excluded from the leak scan (never exported or included in blind copies). The [demo log](demo-output-host-fix5.txt) confirms both lake builds and all 10 demo versions matched / exit 0. The operator reported 66 judge and 72 contract tests passing; full test details are not in that log.
