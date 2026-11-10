#!/usr/bin/env bash
set -euo pipefail
set +x
FIXSEAL_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "$FIXSEAL_ROOT/scripts/testnet_guard.py" "$@"
