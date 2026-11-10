#!/usr/bin/env bash
set -euo pipefail

FIXSEAL_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$FIXSEAL_ROOT"
FIXSEAL_DEMO_LOG="${FIXSEAL_DEMO_LOG:-$FIXSEAL_ROOT/demo-artifacts/demo-output.txt}"
mkdir -p "$(dirname -- "$FIXSEAL_DEMO_LOG")"
trap 'status=$?; if [ "$status" -ne 0 ]; then echo "DEMO FAILED (exit $status): see docs/SETUP.md and docs/BLOCKED.md." | tee -a "$FIXSEAL_DEMO_LOG"; fi' EXIT

demo_main() {

FIXSEAL_DEMO_SUBJECTS=(path spend)
FIXSEAL_DEMO_CHECK=false
FIXSEAL_DEMO_GUARD=true
while [ "$#" -gt 0 ]; do
    case "$1" in
        --guard) FIXSEAL_DEMO_SUBJECTS=(); FIXSEAL_DEMO_GUARD=true; shift ;;
        --check) FIXSEAL_DEMO_CHECK=true; shift ;;
        --subject)
            case "${2:-}" in
                path|spend) FIXSEAL_DEMO_SUBJECTS=("$2"); FIXSEAL_DEMO_GUARD=false ;;
                all) FIXSEAL_DEMO_SUBJECTS=(path spend); FIXSEAL_DEMO_GUARD=true ;;
                *) echo 'Usage: scripts/demo.sh [--check] [--guard | --subject path|spend|all]'; exit 2 ;;
            esac
            shift 2 ;;
        *) echo 'Usage: scripts/demo.sh [--check] [--guard | --subject path|spend|all]'; exit 2 ;;
    esac
done

export FIXSEAL_LEAN_BIN="${FIXSEAL_LEAN_BIN:-$HOME/.elan/toolchains/leanprover--lean4---v4.34.1/bin}"
export PATH="$FIXSEAL_LEAN_BIN:$HOME/.cargo/bin:$PATH"
export CARGO_NET_OFFLINE=true
export CARGO_TARGET_DIR="$FIXSEAL_ROOT/agent-contract/target"
export NO_PROXY="127.0.0.1,localhost"
export no_proxy="$NO_PROXY"
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy

echo 'Fixseal: 仮想空間での検証 (local NEAR sandbox; no account or external network)'
if [ -z "${NEAR_SANDBOX_BIN_PATH:-}" ]; then
    shopt -s nullglob
    FIXSEAL_SANDBOX_CANDIDATES=(
        "$FIXSEAL_ROOT"/agent-contract/target/debug/build/near-sandbox-*/out/.near/near-sandbox-2.11.0/near-sandbox
        "$HOME"/.near/near-sandbox-2.11.0/near-sandbox
    )
    for candidate in "${FIXSEAL_SANDBOX_CANDIDATES[@]}"; do
        if [ -x "$candidate" ]; then
            export NEAR_SANDBOX_BIN_PATH="$candidate"
            break
        fi
    done
fi
if [ -z "${NEAR_SANDBOX_BIN_PATH:-}" ] || [ ! -x "$NEAR_SANDBOX_BIN_PATH" ]; then
    echo 'Missing executable near-sandbox 2.11.0. Set NEAR_SANDBOX_BIN_PATH to a preinstalled binary. Downloads are disabled.'
    exit 2
fi
if [[ "$NEAR_SANDBOX_BIN_PATH" != /* ]]; then
    export NEAR_SANDBOX_BIN_PATH="$FIXSEAL_ROOT/$NEAR_SANDBOX_BIN_PATH"
fi
if [ ! -f agent-contract/target/near/shade_contract_template.wasm ]; then
    echo 'Missing prebuilt agent-contract/target/near/shade_contract_template.wasm. No build/download will be attempted.'
    exit 2
fi
if [ ! -x "$FIXSEAL_LEAN_BIN/lake" ] || [ ! -x "$FIXSEAL_LEAN_BIN/lean" ]; then
    echo 'Missing installed Lean 4.34.1 toolchain. Automatic toolchain downloads are disabled.'
    exit 2
fi
if ! command -v cargo >/dev/null; then
    echo 'Missing cargo on PATH. See docs/SETUP.md.'
    exit 2
fi
if [ "$FIXSEAL_DEMO_CHECK" = true ]; then
    echo 'Required executables and contract wasm found (dependency caches and socket permissions are not checked).'
    return
fi
echo "Sandbox binary: $NEAR_SANDBOX_BIN_PATH"
"$NEAR_SANDBOX_BIN_PATH" --version
(cd lean && lake build && lake env lean SpecTests.lean)
cargo build --manifest-path judge/Cargo.toml --locked --offline --target-dir "$FIXSEAL_ROOT/judge/target"
export FIXSEAL_JUDGE_BIN="$FIXSEAL_ROOT/judge/target/debug/fixseal-judge"
for subject in ${FIXSEAL_DEMO_SUBJECTS[@]+"${FIXSEAL_DEMO_SUBJECTS[@]}"}; do
    echo "Running subject: $subject"
    if [ "$subject" = spend ]; then
        (cd lean/targets/spend && lake build)
    fi
    FIXSEAL_DEMO_SUBJECT="$subject" cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test local_demo local_fix_proof_demo -- --ignored --nocapture
done
if [ "$FIXSEAL_DEMO_GUARD" = true ]; then
    (cd lean/targets/spend && lake build)
    cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --test local_demo local_guard_demo -- --ignored --nocapture
fi
}

demo_main "$@" 2>&1 | tee "$FIXSEAL_DEMO_LOG"
