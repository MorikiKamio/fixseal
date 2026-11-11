import copy
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("judge_check", ROOT / "scripts/judge_check.py")
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.bundle = checker.load((checker.EVIDENCE / "spend-honest.json").read_bytes())

    def test_packaged_hashes_and_bundles(self):
        checker.hashes()
        checker.bundles()

    def test_input_report_result_and_expression_tampering_fail(self):
        for field in ("source", "input_hashes", "inputs_sha256", "verdict", "checks",
                      "proof", "report_sha256", "policy_expr", "policy_expr_sha256", "extra"):
            data = copy.deepcopy(self.bundle)
            report = data["report"]
            if field == "source":
                data["input"]["candidate"]["Target.lean"][0] ^= 1
            elif field == "input_hashes":
                report[field]["version"] = "0" * 64
            elif field == "checks":
                report[field]["regressions_passed"] = -1
            elif field == "proof":
                report[field]["verified"] = False
            elif field == "report_sha256":
                data[field] = "0" * 64
            elif field == "extra":
                data[field] = True
            else:
                report[field] = "tampered"
            with self.subTest(field=field), self.assertRaises(ValueError):
                checker.validate_bundle(data)

    def test_duplicate_fields_and_invalid_byte_arrays_fail(self):
        with self.assertRaises(ValueError):
            checker.load('{"format":"one","format":"two"}')
        for values in ([True], [-1], [256], "bytes"):
            with self.assertRaises(ValueError):
                checker.bytes_array(values)

    def test_manifest_missing_file_or_changed_bytes_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            evidence = root / "docs/evidence"
            evidence.mkdir(parents=True)
            (evidence / "SHA256SUMS").write_text("0" * 64 + "  absent.json\n")
            with patch.object(checker, "ROOT", root), patch.object(checker, "EVIDENCE", evidence):
                with self.assertRaises(OSError):
                    checker.hashes()
                (root / "absent.json").write_text("different")
                with self.assertRaises(ValueError):
                    checker.hashes()

    def test_unfilled_testnet_never_invokes_curl(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "docs").mkdir()
            (root / "docs/testnet-evidence.md").write_text('- Judge-check contract account: ``\n')
            with patch.object(checker, "ROOT", root), patch.object(checker.subprocess, "run") as run:
                self.assertEqual(checker.testnet(), 3)
                run.assert_not_called()

    def test_testnet_view_only_transport_skip_rpc_failure_and_policy_mismatch(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "docs").mkdir()
            record_id = self.bundle["report_sha256"]
            (root / "docs/testnet-evidence.md").write_text(
                f'- Judge-check contract account: `guard.testnet`\n'
                f'- Judge-check accepted record ID: `{record_id}`\n')
            report = self.bundle["report"]
            summary = {key: report[key] for key in ("judge_version", "policy_expr", "policy_expr_sha256")}
            summary.update(subject="spend", verdict="accept", spec_sha256=report["proof"]["approved_spec_sha256"])
            state = {key: report[key] for key in ("policy_expr", "policy_expr_sha256")}
            state.update(record_id=record_id, spec_sha256=summary["spec_sha256"])
            record = {"bundle_hash": record_id, "summary": json.dumps(summary)}

            def response(value):
                return subprocess.CompletedProcess([], 0, checker.wire({
                    "result": {"result": list(checker.wire(value))}}))

            with patch.object(checker, "ROOT", root), patch.object(checker.shutil, "which", return_value="curl"):
                with patch.object(checker.subprocess, "run", return_value=subprocess.CompletedProcess([], 6)):
                    self.assertEqual(checker.testnet(), 3)
                with patch.object(checker.subprocess, "run", return_value=subprocess.CompletedProcess(
                        [], 0, b'{"error":{"message":"missing"}}')):
                    with self.assertRaises(ValueError):
                        checker.testnet()
                with patch.object(checker.subprocess, "run", side_effect=[response(state), response(record)]) as run:
                    checker.testnet()
                    methods = []
                    for call in run.call_args_list:
                        payload = checker.load(call.kwargs["input"])
                        self.assertEqual(payload["method"], "query")
                        self.assertEqual(payload["params"]["request_type"], "call_function")
                        self.assertEqual(payload["params"]["finality"], "final")
                        methods.append(payload["params"]["method_name"])
                    self.assertEqual(methods, ["guard_state", "get_fix_proof"])
                with patch.object(checker.subprocess, "run", side_effect=[response({}), response(record)]):
                    with self.assertRaises((ValueError, KeyError)):
                        checker.testnet()


class RunnerTests(unittest.TestCase):
    def test_missing_tools_skip_without_downloads(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "python3").symlink_to(shutil.which("python3"))
            env = dict(os.environ, PATH=tmp, FIXSEAL_LEAN_BIN=tmp)
            result = subprocess.run(["/bin/bash", str(ROOT / "scripts/judge-check.sh")],
                                    env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("PASS=3 FAIL=0 SKIP=11", result.stdout)
            self.assertIn("elan toolchain install leanprover/lean4:v4.34.1", result.stdout)

    def test_docker_never_builds_pulls_or_enables_network(self):
        with tempfile.TemporaryDirectory() as tmp:
            marker = Path(tmp) / "args"
            binary = Path(tmp) / "docker"
            binary.write_text(f'#!/bin/sh\nprintf "%s\\n" "$@" > "{marker}"\n')
            binary.chmod(0o755)
            env = dict(os.environ, PATH=tmp + os.pathsep + os.environ["PATH"])
            result = subprocess.run(["bash", str(ROOT / "scripts/judge-check.sh"), "--stage", "b", "--docker"],
                                    env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            args = marker.read_text().splitlines()
            self.assertEqual(args[0], "run")
            self.assertIn("--pull=never", args)
            self.assertIn("--network=none", args)
            self.assertIn("--read-only", args)
            self.assertIn("readonly", args[args.index("--mount") + 1])

    def test_stage_a_does_not_invoke_rust_lean_docker_or_rpc(self):
        with tempfile.TemporaryDirectory() as tmp:
            marker = Path(tmp) / "invoked"
            for tool in ("cargo", "rustc", "lean", "lake", "docker", "curl"):
                path = Path(tmp) / tool
                path.write_text(f'#!/bin/sh\necho {tool} >> "{marker}"\nexit 99\n')
                path.chmod(0o755)
            env = dict(os.environ, PATH=tmp + os.pathsep + os.environ["PATH"])
            result = subprocess.run(["bash", str(ROOT / "scripts/judge-check.sh"), "--stage", "a"],
                                    env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("PASS=2 FAIL=0 SKIP=1", result.stdout)
            self.assertFalse(marker.exists())

    def test_failed_rust_checks_return_nonzero_and_continue(self):
        with tempfile.TemporaryDirectory() as tmp:
            for tool in ("cargo", "rustc"):
                path = Path(tmp) / tool
                path.write_text("#!/bin/sh\nexit 101\n")
                path.chmod(0o755)
            env = dict(os.environ, PATH=tmp + os.pathsep + os.environ["PATH"])
            result = subprocess.run(["bash", str(ROOT / "scripts/judge-check.sh"), "--stage", "b"],
                                    env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            for check_id in ("B1", "B2", "B3"):
                self.assertIn(f"FAIL {check_id}", result.stdout)
            self.assertIn("FAIL=3", result.stdout)


if __name__ == "__main__":
    unittest.main()
