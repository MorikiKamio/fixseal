# T010 host checks — pending

T010 performed offline source/native/mock checks only. No NEAR sandbox, demo body or testnet RPC/signing was run. Examples use a **simulated environment, synthetic data**. Never substitute an older wasm or a failed build for these checks.

## Rebuild current contract wasm

Use the prepared Rust/Cargo 1.99.1 toolchain, wasm32-unknown-unknown standard library, locked dependency cache, and a C compiler supporting wasm32. No runtime/toolchain downloads are part of these commands. See [SETUP](SETUP.md).

```sh
cargo build --manifest-path agent-contract/Cargo.toml --target wasm32-unknown-unknown --release --locked --offline
mkdir -p agent-contract/target/near
cp agent-contract/target/wasm32-unknown-unknown/release/shade_contract_template.wasm agent-contract/target/near/shade_contract_template.wasm
```

Expected: build exit 0; retain the wasm SHA-256 and source/build identity. T010's local attempt failed in ring 0.17.14's C compilation: the installed clang reported `No available targets are compatible with triple wasm32-unknown-unknown`. No compatible separately installed LLVM compiler was found. The previous supplied wasm was preserved. Supply a compatible compiler on the operator's host (for example select its existing clang with `CC_wasm32_unknown_unknown` and matching archiver as required), then rebuild; no installation or download was attempted here. Rust standard-library presence alone is insufficient.

## Native and offline checks

From the repository root, put installed Rust/Cargo and the Lean 4.34.1 real bin on PATH; set FIXSEAL_LEAN_BIN if needed. All commands must complete successfully:

```sh
(cd lean && lake build && lake env lean SpecTests.lean)
(cd lean/targets/spend && lake build && lake env lean TargetAudit.lean)
cargo fmt --manifest-path judge/Cargo.toml -- --check
cargo clippy --manifest-path judge/Cargo.toml --locked --offline --all-targets -- -D warnings
cargo test --manifest-path judge/Cargo.toml --locked --offline
cargo fmt --manifest-path agent-contract/Cargo.toml -- --check
cargo clippy --manifest-path agent-contract/Cargo.toml --locked --offline --all-targets -- -D warnings
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --lib
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test fix_proof_tests
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test guard_tests
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test local_demo
python3 scripts/check-spend-proof-leak.py --self-test
python3 -m unittest scripts/test_workflows.py scripts/test_guard_workflow.py
```

Expected: path-7/spend-4; reference honest proof accepted and kernel checked against the unchanged spec; fixed tests rerun. Spend correspondence count is 1,028, including 1,000 boundary triples. local_demo's two chain tests are opt-in; ordinary compilation/summary tests do not establish chain success.

## NEAR sandbox and guard demo

Only on an operator-authorized host with localhost bind/listen permission, the freshly rebuilt wasm and a preinstalled near-sandbox 2.11.0 binary. Do not work around bind restrictions or let the sandbox crate download a binary. No peers/telemetry/remote RPC fallback is configured by the demo.

```sh
export NEAR_SANDBOX_BIN_PATH="$HOME/.near/near-sandbox-2.11.0/near-sandbox"
scripts/demo.sh --check
FIXSEAL_DEMO_LOG=demo-artifacts/t010-host-guard.txt scripts/demo.sh --guard
FIXSEAL_DEMO_LOG=demo-artifacts/t010-host-all.txt scripts/demo.sh
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --no-fail-fast
```

Expected guard output, one observed step per line:

1. Register mock agent and install a key allowing only guard_spend on the intended receiver.
2. Initialize limit=1000, pay 600 then 18446744073709551116 yoctoNEAR. Wrapped spent=100, total_paid=18446744073709551716, pending_paid=0; recipient balance grows by exactly total_paid.
3. Missing record rejects with Guard record missing; reject saturation record rejects with Guard record is not accept.
4. Accept record installs the exact exported policy; chain readback/replay matches.
5. Same attack rejects with Guard spending denied; recipient balance/cumulative paid unchanged.
6. Legitimate 100 pays; spent=200, total_paid=18446744073709551816, history length=1; restricted key cannot call guard_set_policy.

Every transfer and callback receipt must succeed where expected. Confirm the callback's 12 Tgas is sufficient. Test missing-recipient/insufficient-contract-funds failure on disposable state if investigating callback accounting; unit tests simulate failure and out-of-order settlement, but this is not a chain result. `--guard` invokes local_guard_demo only; the default also retains ten path/spend matched/VERIFIED records. A callback returning false or failed receipt must not be counted as a successful payment demo. Preserve failed logs and record actual exit status. No guard host success log is available yet.

## Testnet operator check

First review [guard and key instructions](guard.md). Prepare existing funded demo accounts, a newly initialized contract with current wasm, approved mock measurements/PPID, a whitelisted valid registered agent and separate keychain-held full/restricted keys. Do not log authentication or export secrets. The script creates no accounts and deploys no contract.

```sh
scripts/testnet-guard.sh --check
# After separate permission for external testnet RPC/signing and test tokens:
scripts/testnet-guard.sh --execute
```

Expected: read-only preflight confirms chain_id=testnet, owner/mock mode, valid agent, uninitialized guard and restricted key receiver/methods. Three offline bundles are generated/verified before any transaction. The same guard sequence and recipient balance assertions complete. A fresh output directory contains bundles/readbacks, guard-state.json and a populated copy of [testnet-evidence.md](testnet-evidence.md), with each public transaction hash and final receipt outcome. Missing or unexpected failure stops the script. Partial evidence remains incomplete; do not retry initialization on the same guard. near-cli-rs 0.31.1 argument grammar was inspected with local --help; actual RPC/signing and transaction-output parsing remain host checks.

## Public export

From the development copy, choose a new ignored output path:

```sh
python3 scripts/export-public.py demo-artifacts/t010-public-host
python3 scripts/export-public.py --check demo-artifacts/t010-public-host
```

Expected: no stale/missing/extra files, all new shared interpreter, guard, tests/scripts and public documentation present, leak scanner passes. Never send or publish the export without authorization. Optional verifier quorum, actual TEE, WASI, specification approval history and license inventory gaps remain outside this implementation.

## Host result (operator, 2026-11-10 14:20 JST)

- wasm: the contract now includes `../shared/policy_expr.rs`, so the Docker build must mount the repository root: `docker run --rm -v $PWD:/host -w /host/agent-contract sourcescan/cargo-near:0.19.0-rust-1.86.0 cargo near build non-reproducible-wasm --locked --no-abi` (exit 0, sha256 9fb00594…4c718). Mounting only agent-contract fails with "couldn't read src/../../shared/policy_expr.rs".
- scripts/demo.sh: `--guard` failed under macOS bash 3.2 (`FIXSEAL_DEMO_SUBJECTS[@]: unbound variable` for the empty array); the operator changed the loop to `${FIXSEAL_DEMO_SUBJECTS[@]+"${FIXSEAL_DEMO_SUBJECTS[@]}"}`. Update SETUP/CI/docs to the root-mount build command.
- `scripts/demo.sh --guard` on the fresh wasm: exit 0, all six guard steps observed (log demo-artifacts/t010-host-guard.txt; anonymize before publishing). Full demo and contract test results: see demo-artifacts/all-run.log.
