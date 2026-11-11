#!/usr/bin/env bash
# Read-only evidence checks, followed by offline builds and replay. Never signs.
set -uo pipefail
ROOT="$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd)"
cd "$ROOT" || exit 2
stage=all
docker_mode=0
allow_testnet=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --stage)
      [ "$#" -ge 2 ] || { echo 'Missing --stage a|b|c|all' >&2; exit 2; }
      stage="$2"; shift ;;
    --docker) docker_mode=1 ;;
    --allow-testnet-read) allow_testnet=1 ;;
    --help)
      echo 'Usage: scripts/judge-check.sh [--stage a|b|c|all] [--docker] [--allow-testnet-read]'
      echo 'Default: no network, no installs. Read-only testnet RPC requires explicit opt-in.'
      exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done
case "$stage" in a|b|c|all) ;; *) echo 'Invalid stage' >&2; exit 2 ;; esac
command -v python3 >/dev/null || { echo 'FAIL A0 python3 is required'; exit 1; }
run_dir="$(python3 -c 'import pathlib,tempfile; p=pathlib.Path("demo-artifacts/judge-check"); p.mkdir(parents=True,exist_ok=True); print(tempfile.mkdtemp(prefix="run-",dir=p))')" || exit 1
passed=0
failed=0
skipped=0
start="$(python3 -c 'import time; print(time.monotonic())')"
skip() { echo "SKIP($2) $1"; skipped=$((skipped + 1)); }
check() {
  local id="$1" label="$2" began rc elapsed
  shift 2
  began="$(python3 -c 'import time; print(time.monotonic())')"
  "$@" >"$run_dir/$id.log" 2>&1
  rc=$?
  elapsed="$(python3 -c 'import sys,time; print(f"{time.monotonic()-float(sys.argv[1]):.3f}s")' "$began")"
  if [ "$rc" -eq 0 ]; then
    echo "PASS $id $label ($elapsed)"
    passed=$((passed + 1))
  elif [ "$rc" -eq 3 ]; then
    local reason
    reason="$(<"$run_dir/$id.log")"
    skip "$id" "$reason"
  else
    echo "FAIL $id $label (exit=$rc, $elapsed; $run_dir/$id.log)"
    failed=$((failed + 1))
  fi
}
if [ "$stage" = all ] || [ "$stage" = a ]; then
  check A1 'packaged evidence SHA-256' python3 scripts/judge_check.py hashes
  check A2 'bundle structure and input/report/expression hashes; no proof replay' python3 scripts/judge_check.py bundles
  if [ "$allow_testnet" -eq 1 ]; then
    check A3 'testnet guard_state/get_fix_proof views' python3 scripts/judge_check.py testnet
  else
    skip A3 'network disabled; operator may opt in with --allow-testnet-read'
  fi
fi
if [ "$stage" != a ]; then
  if [ "$docker_mode" -eq 1 ]; then
    # Use only a prebuilt local image. Build/provision it separately with permission.
    if ! command -v docker >/dev/null; then
      skip D1 'Docker missing; B/C not run'
    else
      check D1 'local Docker image runs the same script on current sources' docker run --rm --pull=never --network=none --read-only --tmpfs /tmp:exec --tmpfs /work:exec --mount "type=bind,source=$ROOT,target=/source,readonly" --mount "type=bind,source=$run_dir,target=/results" --env "FIXSEAL_CHECK_STAGE=$stage" fixseal-judge:rust1.99.1-lean4.34.1
      while IFS= read -r line; do
        case "$line" in PASS\ *|FAIL\ *|SKIP\(*|Summary:*) echo "$line" ;; esac
      done <"$run_dir/D1.log"
    fi
  else
    lean_bin="${FIXSEAL_LEAN_BIN:-$HOME/.elan/toolchains/leanprover--lean4---v4.34.1/bin}"
    if [ -x "$lean_bin/lean" ] && [ -x "$lean_bin/lake" ]; then
      export FIXSEAL_LEAN_BIN="$lean_bin"
      export PATH="$lean_bin:$PATH"
    fi
    has_lean=0
    if command -v lean >/dev/null && command -v lake >/dev/null; then
      # Avoid elan's automatic toolchain download: inspect real binaries only.
      if python3 scripts/judge_check.py lean-ready >"$run_dir/lean-ready.log" 2>&1; then
        has_lean=1
      fi
    fi
    has_rust=0
    if command -v cargo >/dev/null && command -v rustc >/dev/null; then
      has_rust=1
    fi
    if [ "$stage" = all ] || [ "$stage" = b ]; then
      if [ "$has_rust" -eq 1 ]; then
        check B1 'Rust judge tests (no Lean required)' cargo test --manifest-path judge/Cargo.toml --locked --offline --lib --test judge --test timeout
        check B2 'shared expression interpreter tests' cargo test --manifest-path judge/Cargo.toml --locked --offline --lib policy_expr::tests
        check B3 'contract unit/guard/record/summary tests' cargo test --manifest-path agent-contract/Cargo.toml --locked --offline --lib --test guard_tests --test fix_proof_tests --test local_demo
      else
        for id in B1 B2 B3; do skip "$id" 'Rust/Cargo missing; provide Rust 1.99.1 and locked caches'; done
      fi
      check B5 'NEAR AI fixer offline tool tests (no API or sockets)' python3 fixers/nearai/test_run_fixer.py
      if command -v node >/dev/null; then
        check B6 'viewer policy evaluator vs Rust vectors (node --test)' node --test viewer/
      else
        skip B6 'node unavailable; viewer policy evaluator test not run'
      fi
      skip B4 'live sandbox is separate: scripts/demo.sh; requires rebuilt wasm and loopback permission'
    fi
    if [ "$stage" = all ] || [ "$stage" = c ]; then
      if [ "$has_lean" -eq 0 ]; then
        echo 'Lean setup (operator only, not executed): elan toolchain install leanprover/lean4:v4.34.1'
        for id in C1 C2 C3 C4 C5; do skip "$id" 'real Lean/Lake 4.34.1 binaries unavailable; no downloads attempted'; done
      elif [ "$has_rust" -eq 0 ]; then
        skip C1 'Rust/Cargo missing'
        check C2 'both lake builds and fixed spec tests' python3 scripts/judge_check.py lean-builds
        for id in C3 C4 C5; do skip "$id" 'Rust/Cargo missing'; done
      else
        check C1 'complete judge cargo test --locked --offline' cargo test --manifest-path judge/Cargo.toml --locked --offline
        check C2 'both lake builds and fixed spec tests' python3 scripts/judge_check.py lean-builds
        check C3 'all path/spend fixture verdicts and reasons' python3 scripts/judge_check.py versions "$run_dir"
        check C4 'fresh bundles -> verify plus packaged bundle replay' python3 scripts/judge_check.py replay "$run_dir"
        check C5 'three archived AI fixers replay (FX3 first proofs reconstructed)' cargo test --manifest-path judge/Cargo.toml --locked --offline --test fixers
      fi
    fi
  fi
fi
python3 - "$passed" "$failed" "$skipped" "$start" "$run_dir" <<'PY'
import sys,time
p,f,s,began,directory=sys.argv[1:]
print(f'Summary: PASS={p} FAIL={f} SKIP={s}; {time.monotonic()-float(began):.3f}s; logs={directory}')
PY
[ "$failed" -eq 0 ]
