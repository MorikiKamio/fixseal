import json
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts/live-fix.sh"

FAKE_JUDGE = """\
#!/bin/sh
# Fake judge: accept iff the candidate Target contains 'good'.
target="lean/targets/spend/after_fx3/Target.lean"
if [ "$1" = "bundle" ]; then
    printf '{"format":"fixseal-bundle-v1","input":{"subject":"spend"},"report":{"verdict":"accept","reason_codes":[],"proof":{"verified":true}}}\\n'
    exit 0
fi
if [ "$1" = "verify" ]; then
    test -f "$2" || exit 2
    echo "VERIFIED"
    exit 0
fi
if grep -q good "$target" 2>/dev/null; then
    echo '{"verdict":"accept","reason_codes":[],"checks":{"attacks_total":12,"candidate_attacks_working":0,"regressions_total":16,"regressions_passed":16},"proof":{"verified":true}}'
    exit 0
fi
echo '{"verdict":"reject","reason_codes":["PROOF_REJECTED"],"checks":{"attacks_total":12,"candidate_attacks_working":0,"regressions_total":16,"regressions_passed":16},"proof":{"verified":false}}'
echo "Proof.lean:5:1: error: omega could not prove the goal" >&2
exit 1
"""

FAKE_FIXER = """\
#!/bin/sh
# Fake fixer: first call writes 'bad', second writes 'good'.
mkdir -p lean/targets/spend/after_fx3
if [ -f .tries ]; then
    echo good > lean/targets/spend/after_fx3/Target.lean
else
    echo bad > lean/targets/spend/after_fx3/Target.lean
    touch .tries
fi
test -f fixer-brief.md
"""

NEVER_FIXER = """\
#!/bin/sh
mkdir -p lean/targets/spend/after_fx3
echo bad > lean/targets/spend/after_fx3/Target.lean
"""


class LiveFixTests(unittest.TestCase):
    def test_blind_copy_excludes_viewer_reference_expressions(self):
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp) / "blind"
            proc = subprocess.run([str(ROOT / "fixers/nearai/make_blind_copy.sh"), str(work)],
                                  cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
            for name in ("viewer", "examples/fixers", "docs", "lean/targets/spend/after_honest"):
                self.assertFalse((work / name).exists(), name)

    def run_live_fix(self, tmp, fixer, judge_text=FAKE_JUDGE, extra=None):
        work = Path(tmp) / "work"
        out = Path(tmp) / "out"
        (work / "lean/targets/spend").mkdir(parents=True)
        judge = Path(tmp) / "fake-judge.sh"
        judge.write_text(judge_text)
        judge.chmod(0o755)
        fixer_path = Path(tmp) / "fixer.sh"
        fixer_path.write_text(fixer)
        fixer_path.chmod(0o755)
        env = {
            "PATH": "/usr/bin:/bin",
            "FIXSEAL_LIVE_WORK": str(work),
            "FIXSEAL_LIVE_OUT": str(out),
            "FIXSEAL_LIVE_JUDGE": str(judge),
            "FIXSEAL_FIXER_CMD": str(fixer_path),
        }
        env.update(extra or {})
        return subprocess.run(["bash", str(SCRIPT)], env=env, cwd=ROOT,
                              capture_output=True, text=True), out, work

    def test_reject_then_accept_writes_bundle_and_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            proc, out, work = self.run_live_fix(tmp, FAKE_FIXER)
            self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
            self.assertIn("REJECT PROOF_REJECTED attacks 0/12 regressions 16/16", proc.stdout)
            self.assertIn("ACCEPT attacks 0/12 regressions 16/16", proc.stdout)
            self.assertIn("Proof.lean:5:1: error:", proc.stdout)
            accepted = (out / "accepted.json").read_text()
            self.assertIn('"verdict":"accept"', accepted)
            # The first reject was fed back into the copy for the next attempt.
            report = json.loads((work / "live-fix-report.json").read_text())
            self.assertEqual(report["verdict"], "accept")

    def test_three_rejects_exit_nonzero_without_bundle(self):
        with tempfile.TemporaryDirectory() as tmp:
            proc, out, work = self.run_live_fix(tmp, NEVER_FIXER)
            self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
            self.assertEqual(proc.stdout.count("REJECT PROOF_REJECTED"), 3)
            self.assertIn("no accept after 3 attempts", proc.stdout)
            self.assertFalse((out / "accepted.json").exists())
            # Feedback file still holds the last reject for inspection.
            self.assertEqual(
                json.loads((work / "live-fix-report.json").read_text())["verdict"], "reject")

    def test_rejected_bundle_is_not_published_as_accepted(self):
        judge = FAKE_JUDGE.replace('"verdict":"accept","reason_codes":[],"proof"',
                                   '"verdict":"reject","reason_codes":[],"proof"')
        with tempfile.TemporaryDirectory() as tmp:
            proc, out, _ = self.run_live_fix(tmp, FAKE_FIXER, judge)
            self.assertNotEqual(proc.returncode, 0)
            self.assertFalse((out / "accepted.json").exists())
            self.assertIn("Exported bundle is not", proc.stderr)

    def test_failed_verify_does_not_publish_accepted_bundle(self):
        judge = FAKE_JUDGE.replace('echo "VERIFIED"', 'exit 2')
        with tempfile.TemporaryDirectory() as tmp:
            proc, out, _ = self.run_live_fix(tmp, FAKE_FIXER, judge)
            self.assertNotEqual(proc.returncode, 0)
            self.assertFalse((out / "accepted.json").exists())

    def test_attempt_limit_rejects_zero_and_more_than_three(self):
        for value in ("0", "4", "-1", "abc"):
            with tempfile.TemporaryDirectory() as tmp:
                proc, _, _ = self.run_live_fix(tmp, NEVER_FIXER,
                                              extra={"FIXSEAL_LIVE_ATTEMPTS": value})
                self.assertEqual(proc.returncode, 2)

    def test_version_is_passed_to_the_fixer_prompt(self):
        fixer = "#!/bin/sh\ngrep -q 'after_custom' fixer-brief.md\n"
        with tempfile.TemporaryDirectory() as tmp:
            proc, _, work = self.run_live_fix(tmp, fixer,
                                            extra={"FIXSEAL_LIVE_VERSION": "after_custom"})
            self.assertIn("after_custom", (work / "fixer-instruction.md").read_text())
            self.assertNotIn("fixer command failed", proc.stdout)

    def test_existing_accepted_bundle_is_preserved(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out"
            out.mkdir()
            (out / "accepted.json").write_text("existing evidence")
            proc, _, _ = self.run_live_fix(tmp, FAKE_FIXER)
            self.assertEqual(proc.returncode, 2)
            self.assertEqual((out / "accepted.json").read_text(), "existing evidence")


if __name__ == "__main__":
    unittest.main()
