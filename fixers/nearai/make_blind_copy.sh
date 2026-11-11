#!/bin/sh
# Create an answer-withheld copy of this repository for an independent
# fixer run, then verify that no reference-proof fragments, lemma hints or
# symlinks remain (equivalent to scripts/check-spend-proof-leak.py, whose
# reference file is absent inside the copy by design).
#
# usage: fixers/nearai/make_blind_copy.sh DEST
set -eu

SRC=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
DEST=${1:-}
if [ -z "$DEST" ]; then
    echo "usage: $0 DEST" >&2
    exit 2
fi
if [ -e "$DEST" ]; then
    echo "refusing to overwrite existing $DEST" >&2
    exit 2
fi
mkdir -p "$DEST"

# Exclusions follow AGENTS.md: internal docs, tests, build caches, the
# contract crate, viewer reference expressions, post-experiment records and every after_honest reference
# directory. .git is excluded because history contains the answers.
tar -C "$SRC" \
    --exclude='./.git' \
    --exclude='./docs' \
    --exclude='./judge/tests' \
    --exclude='./judge/target' \
    --exclude='./agent-contract' \
    --exclude='./viewer' \
    --exclude='./examples/fixers' \
    --exclude='./demo-artifacts' \
    --exclude='./submission' \
    --exclude='./lean/after_honest' \
    --exclude='./lean/targets/spend/after_honest' \
    --exclude='./lean/.lake' \
    --exclude='./lean/targets/spend/.lake' \
    --exclude='node_modules' \
    --exclude='__pycache__' \
    --exclude='.env' \
    --exclude='.DS_Store' \
    -cf - . | tar -C "$DEST" -xf -

# Structural check: excluded paths absent, required inputs present.
for p in docs judge/tests agent-contract viewer examples/fixers demo-artifacts \
         lean/after_honest lean/targets/spend/after_honest \
         lean/.lake lean/targets/spend/.lake; do
    if [ -e "$DEST/$p" ] || [ -L "$DEST/$p" ]; then
        echo "error: excluded path present in copy: $p" >&2
        exit 1
    fi
done
for p in AGENTS.md judge/Cargo.toml judge/src lean/Spec.lean \
         lean/targets/spend/SpendSpec.lean lean/targets/spend/before \
         lean/targets/spend/corpus.json lean/targets/spend/policy.json \
         scripts/check-spend-proof-leak.py; do
    if [ ! -e "$DEST/$p" ]; then
        echo "error: required input missing from copy: $p" >&2
        exit 1
    fi
done

# Fragment check equivalent to scripts/check-spend-proof-leak.py: the
# reference proof is read from the source repository, then every text file
# and symlink inside the copy is scanned.
python3 - "$SRC" "$DEST" <<'PY'
import os
import re
import sys
from pathlib import Path

src, root = Path(sys.argv[1]), Path(sys.argv[2])
honest = Path("lean/targets/spend/after_honest/Proof.lean")
proof = (src / honest).read_text().split(":= by", 1)[1]
lines = [re.sub(r"\s+", "", line) for line in proof.splitlines() if line.strip()]
fragments = {a + b for a, b in zip(lines, lines[1:]) if len(a + b) >= 50}
names = set(re.findall(r"\b(?:UInt64|Nat|Bool)\.[A-Za-z0-9_]+", proof))
skip = {".git", ".lake", "node_modules", "__pycache__"}
generated = {("demo-artifacts",), ("submission",), ("judge", "target")}
found = []
for directory, dirs, files in os.walk(root, followlinks=False):
    base = Path(directory)
    for name in list(dirs) + files:
        path = base / name
        relative = path.relative_to(root)
        if path.is_symlink():
            found.append(str(relative) + " (symlink)")
            if name in dirs:
                dirs.remove(name)
            continue
        if name in dirs:
            if name in skip or relative.parts in generated:
                dirs.remove(name)
            continue
        if not path.is_file():
            found.append(str(relative) + " (special file)")
            continue
        data = path.read_bytes()
        if data.startswith((b"\xff\xfe\x00\x00", b"\x00\x00\xfe\xff")):
            text = data.decode("utf-32", errors="replace")
        elif data.startswith((b"\xff\xfe", b"\xfe\xff")):
            text = data.decode("utf-16", errors="replace")
        elif b"\x00" in data:
            continue
        else:
            text = data.decode("utf-8", errors="replace")
        normalized = re.sub(r"\s+", "", text)
        if any(f in normalized for f in fragments) or any(n in text for n in names):
            found.append(str(relative))
if found:
    print("Spend proof fragments or lemma hints in copy: " + ", ".join(sorted(found)))
    sys.exit(1)
print("OK blind copy: no reference proof fragments, lemma hints or symlinks")
PY

echo "blind copy ready: $DEST"
