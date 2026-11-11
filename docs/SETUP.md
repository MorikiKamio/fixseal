# Setup and offline reproduction

A clone contains source and lockfiles, not toolchains, Cargo caches, sandbox executables or contract wasm. Use Rust/Cargo 1.99.1, Lean/Lake 4.34.1, a native C compiler and Python 3. Both lockfiles need matching cached dependencies. No external Lean packages are needed. Node, Docker and a testnet account are unnecessary for the prepared offline route.

Point `FIXSEAL_LEAN_BIN` at the installed native Lean 4.34.1 `bin` directory and add it to PATH for direct Lake commands. The judge uses native binaries rather than an auto-downloading proxy. From the repository root:

```sh
export FIXSEAL_LEAN_BIN="${FIXSEAL_LEAN_BIN:-$HOME/.elan/toolchains/leanprover--lean4---v4.34.1/bin}"
export PATH="$FIXSEAL_LEAN_BIN:$PATH"
(cd lean && lake build && lake env lean SpecTests.lean)
(cd lean/targets/spend && lake build)
cargo test --manifest-path judge/Cargo.toml --locked --offline
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test fix_proof_tests
```

The separate chain route needs preinstalled near-sandbox 2.11.0, contract caches, `agent-contract/target/near/shade_contract_template.wasm`, and permission to bind/listen on localhost. Set `NEAR_SANDBOX_BIN_PATH` to an existing absolute executable path to prevent runtime downloads. `scripts/demo.sh --check` checks presence only; `scripts/demo.sh` performs both subjects' deploy/register/submit/readback/replay sequence. It requires explicit loopback permission. Socket permission errors are environment failures, not successful tests. The [T008 operator host record](demo-output-host-t008.txt) covers path-7/spend-2 (ten matched, exit 0); FIX5 spend-3 awaits operator reconfirmation. All figures describe a **simulated environment, synthetic data**.

With the matching Rust wasm target already installed, rebuild the contract offline using:

```sh
cargo build --manifest-path agent-contract/Cargo.toml --target-dir agent-contract/target --target wasm32-unknown-unknown --release --locked --offline
mkdir -p agent-contract/target/near
cp agent-contract/target/wasm32-unknown-unknown/release/shade_contract_template.wasm agent-contract/target/near/shade_contract_template.wasm
```

Initial preparation requires separately authorized downloads on a capable machine: Rust 1.99.1 and its wasm target, Lean 4.34.1, locked Cargo dependencies and the sandbox for the target OS/CPU. Transfer prepared caches/binaries into restricted environments. The historical installation examples below were not executed here; do not run them when network access or installation is prohibited. Supplied wasm/source correspondence and a cold setup remain unconfirmed. See [JUDGES](../JUDGES.md) for expected outputs and troubleshooting.

## Detailed reference and historical preparation commands

# Initial setup and offline reproduction

A clone excludes toolchains, dependency caches, NEAR sandbox and built contracts. This guide separates initial provisioning from prepared-environment execution. R1 performed no downloads/installations; initial setup and contract rebuild below were unexecuted.

## Versions and prerequisites

| Resource | Supported / checked version | Required route |
| --- | --- | --- |
| Rust / Cargo | 1.99.1 (installed); edition 2024 support required | Judge / contract tests |
| Lean / Lake | Fixed `leanprover/lean4:v4.34.1`; no external Lean packages | Lean judge |
| C compiler | Installed native compiler | Lean executable |
| Cargo dependencies | Each `Cargo.lock`; near-sdk 5.26.1 / near-api 0.8.6 / near-sandbox crate 0.3.9 | Each crate |
| NEAR sandbox executable | 2.11.0 | Chain demo / integration tests |
| Rust wasm standard library | `wasm32-unknown-unknown` for the selected Rust | Initial contract build |
| Contract wasm | `agent-contract/target/near/shade_contract_template.wasm` | Chain demo / integration tests |
| Python | 3 (standard library only) | Development export / helper tests |

Recorded runs use macOS arm64 and existing host resources; other OSes were untested at this stage. Node, Docker, Shade CLI and testnet / Phala accounts are unnecessary for local Fixseal checks.

## Initial provisioning (only where downloads are authorized)

Example with Rust, C compiler and elan already available. Never run during network-prohibited work. Provision offline toolchains, Cargo caches, sandbox and wasm from an environment matching OS / CPU / toolchain.

```sh
rustup toolchain install 1.99.1
rustup target add wasm32-unknown-unknown --toolchain 1.99.1
elan toolchain install leanprover/lean4:v4.34.1
cargo +1.99.1 fetch --manifest-path judge/Cargo.toml --locked
cargo +1.99.1 fetch --manifest-path agent-contract/Cargo.toml --locked
```

Use sandbox 2.11.0 as selected by `near-sandbox` crate 0.3.9. Its cached `src/runner/mod.rs` specifies this distribution URL format (not accessed in this session):

```sh
# Select the matching platform: Darwin-arm64 / Linux-x86_64 / Linux-aarch64
FIXSEAL_PLATFORM=Darwin-arm64
mkdir -p "$HOME/.near/near-sandbox-2.11.0"
curl --fail --location "https://s3-us-west-1.amazonaws.com/build.nearprotocol.com/nearcore/$FIXSEAL_PLATFORM/2.11.0/near-sandbox.tar.gz" -o /tmp/fixseal-near-sandbox.tar.gz
tar -xzf /tmp/fixseal-near-sandbox.tar.gz -C "$HOME/.near/near-sandbox-2.11.0"
export NEAR_SANDBOX_BIN_PATH="$HOME/.near/near-sandbox-2.11.0/near-sandbox"
"$NEAR_SANDBOX_BIN_PATH" --version
```

URL/platforms come from cached dependency source; current availability and download authenticity are unconfirmed. Use trusted distribution resources. That crate does not distribute macOS x86_64 binaries.

## Build contract wasm (offline after provisioning)

At the repository root, use Rust with wasm standard libraries and locked dependency caches. Set PATH so `cargo` / `rustc` select the same toolchain.

```sh
cargo build --manifest-path agent-contract/Cargo.toml --target-dir agent-contract/target --target wasm32-unknown-unknown --release --locked --offline
mkdir -p agent-contract/target/near
cp agent-contract/target/wasm32-unknown-unknown/release/shade_contract_template.wasm agent-contract/target/near/shade_contract_template.wasm
```

Contract metadata also records this alternative with cargo-near 0.19.0: `(cd agent-contract && CARGO_NET_OFFLINE=true cargo near build non-reproducible-wasm --locked --no-abi)`. R1 lacked wasm standard libraries; neither rebuild was confirmed. Supplied wasm/current-source correspondence was not established.

## Prepared-environment checks (no downloads)

Put `cargo` / `rustc` on PATH. If Lean is installed outside elan, set `FIXSEAL_LEAN_BIN` to the real Lean 4.34.1 bin directory. The judge directly uses this trusted toolchain without fallback to downloading proxies.

```sh
export FIXSEAL_LEAN_BIN="${FIXSEAL_LEAN_BIN:-$HOME/.elan/toolchains/leanprover--lean4---v4.34.1/bin}"
export PATH="$FIXSEAL_LEAN_BIN:$PATH"
(cd lean && lake build && lake env lean SpecTests.lean)
cargo test --manifest-path judge/Cargo.toml --locked --offline
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --lib
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test fix_proof_tests
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test local_demo
```

The final command checks summary format; chain execution is opt-in. These commands need no sandbox, wasm or localhost bind permission.

All contract integration tests and chain demos require extra resources and localhost bind / listen permission. When external networking is prohibited, specify an existing sandbox binary in addition to Cargo `--offline`; otherwise the `near-sandbox` crate may attempt runtime downloads.

```sh
export NEAR_SANDBOX_BIN_PATH="$HOME/.near/near-sandbox-2.11.0/near-sandbox"
scripts/demo.sh --check
cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --no-fail-fast
scripts/demo.sh
```

`--check` checks presence only; socket-permission failures are not test passes. Logs go to `demo-artifacts/demo-output.txt`, JSON to `demo-artifacts/*.json`. Distinguish the [path-5 host success record](demo-output-host-path5.txt) from [restricted-environment status](BLOCKED.md).
