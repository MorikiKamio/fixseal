"""Check spend fixtures, source and documentation without printing proof fragments."""
import argparse
import os
from pathlib import Path
import re
import tempfile


HONEST = Path("lean/targets/spend/after_honest/Proof.lean")
SKIP = {".git", ".lake", "node_modules", "__pycache__"}
GENERATED = {("demo-artifacts",), ("submission",), ("judge", "target"), ("agent-contract", "target")}
POST_EXPERIMENT = {("examples", "fixers"), ("docs", "fixer")}
# Internal QA reports: never exported (not in scripts/public-files.txt) and never copied into blind fixer copies.
INTERNAL_RECORDS = {("docs", "qa")}


def leaks(root):
    for i in range(1, len(HONEST.parts) + 1):
        if (root / Path(*HONEST.parts[:i])).is_symlink():
            return [str(Path(*HONEST.parts[:i])) + " (symlink)"]
    proof = (root / HONEST).read_text().split(":= by", 1)[1]
    lines = [re.sub(r"\s+", "", line) for line in proof.splitlines() if line.strip()]
    fragments = {a + b for a, b in zip(lines, lines[1:]) if len(a + b) >= 50}
    names = set(re.findall(r"\b(?:UInt64|Nat|Bool)\.[A-Za-z0-9_]+", proof))
    found = []
    def scan_error(error):
        raise error

    for directory, dirs, files in os.walk(root, followlinks=False, onerror=scan_error):
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
                if (name in SKIP or relative.parts in GENERATED
                        or relative.parts in POST_EXPERIMENT
                        or relative.parts in INTERNAL_RECORDS):
                    dirs.remove(name)
                continue
            if relative == HONEST:
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
            if any(fragment in normalized for fragment in fragments) or any(name in text for name in names):
                found.append(str(relative))
    return sorted(found)


def self_test(root):
    with tempfile.TemporaryDirectory() as directory:
        test_root = Path(directory)
        (test_root / HONEST).parent.mkdir(parents=True)
        proof = (root / HONEST).read_text()
        (test_root / HONEST).write_text(proof)
        assert not leaks(test_root)
        for destination in ["docs/example.md", "judge/tests/example.rs",
                            "lean/targets/spend/after_reject_all/Proof.lean",
                            "docs/copied.txt", ".github/workflows/copied.yml",
                            "config/copied.yaml", "docs/extensionless", "docs/copied.unknown",
                            "lean/targets/spend/demo-artifacts/Proof.lean",
                            "docs/submission/copied.txt"]:
            path = test_root / destination
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(proof)
            assert leaks(test_root) == [destination]
            path.unlink()
        for encoding in ["utf-16", "utf-32"]:
            path = test_root / "encoded.txt"
            path.write_bytes(proof.encode(encoding))
            assert leaks(test_root) == ["encoded.txt"]
            path.unlink()
        for destination in ["linked-proof", "linked-directory", "dangling"]:
            path = test_root / destination
            target = (test_root / HONEST if destination == "linked-proof" else
                      test_root / "docs" if destination == "linked-directory" else test_root / "missing")
            path.symlink_to(target, target_is_directory=destination == "linked-directory")
            assert leaks(test_root) == [destination + " (symlink)"]
            path.unlink()
        for destination in ["demo-artifacts/copied.txt", "submission/copied.txt"]:
            path = test_root / destination
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(proof)
        assert not leaks(test_root)
        path = test_root / "binary"
        path.write_bytes(b"\x00" + proof.encode())
        assert not leaks(test_root)
        path.unlink()
        path = test_root / "hint.md"
        path.write_text(re.findall(r"\bUInt64\.[A-Za-z0-9_]+", proof)[0])
        assert leaks(test_root) == ["hint.md"]
        path.unlink()
        for destination in ["examples/fixers/spend/agent/attempt-2/Proof.lean",
                            "docs/fixer/record.md"]:
            path = test_root / destination
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(proof)
        assert not leaks(test_root)
        path = test_root / "docs/fixer-notes.md"
        path.write_text(proof)
        assert leaks(test_root) == ["docs/fixer-notes.md"]
        path.unlink()
        reference = test_root / HONEST
        reference.unlink()
        reference.symlink_to(test_root / "missing-reference")
        assert leaks(test_root) == [str(HONEST) + " (symlink)"]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    if args.self_test:
        self_test(root)
    found = leaks(root)
    if found:
        raise SystemExit("Spend proof fragments or lemma hints outside after_honest/post-experiment records: " + ", ".join(found))
    print("OK spend proof isolation (all text files and symlinks; generated artifacts and post-experiment records excluded)")
