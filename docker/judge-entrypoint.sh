#!/usr/bin/env bash
set -euo pipefail
python3 - <<'PY'
from pathlib import Path
import shutil
source = Path('/source') if Path('/source').is_dir() else Path('/opt/fixseal')
def ignored(directory, names):
    skip = {'.git', '.lake', '.codex', '.agents', '.aws', 'node_modules', '__pycache__',
            'demo-artifacts', 'submission', '.env'}
    if Path(directory) in (source / 'judge', source / 'agent-contract'):
        skip.add('target')
    return [name for name in names if name in skip or name.startswith('.env.')]
shutil.copytree(source, '/work/repo', ignore=ignored)
shutil.copytree('/usr/local/cargo/registry', '/work/cargo/registry')
PY
export CARGO_HOME=/work/cargo
cd /work/repo
status=0
if [ "${FIXSEAL_CHECK_STAGE:-all}" = all ]; then
  bash scripts/judge-check.sh --stage b || status=1
  bash scripts/judge-check.sh --stage c || status=1
else
  bash scripts/judge-check.sh --stage "$FIXSEAL_CHECK_STAGE" || status=1
fi
if [ -d /results ]; then
  python3 - <<'PY'
import shutil
shutil.copytree('/work/repo/demo-artifacts/judge-check', '/results/container', dirs_exist_ok=True)
PY
fi
exit "$status"
