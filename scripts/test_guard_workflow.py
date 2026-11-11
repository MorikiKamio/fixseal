import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import types
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("testnet_guard", ROOT / "scripts/testnet_guard.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)

ENV = {
    "FIXSEAL_TESTNET_CONTRACT": "fixseal-live-1.testnet",
    "FIXSEAL_TESTNET_OWNER": "fixseal-owner.testnet",
    "FIXSEAL_TESTNET_AGENT": "fixseal-agent.testnet",
    "FIXSEAL_TESTNET_RECIPIENT": "fixseal-recipient.testnet",
    "FIXSEAL_TESTNET_OWNER_PUBLIC_KEY": "ed25519:" + "A" * 43,
    "FIXSEAL_TESTNET_AGENT_PUBLIC_KEY": "ed25519:" + "A" * 43,
    "FIXSEAL_TESTNET_SPEND_PUBLIC_KEY": "ed25519:" + "B" * 43,
    "FIXSEAL_TESTNET_SIGN_WITH": "legacy-keychain",
}


def report(verdict, **extra):
    data = {
        "judge_version": guard.JUDGE, "verdict": verdict, "reason_codes": [],
        "inputs_sha256": "a" * 64, "checks": {},
        "proof": {"verified": verdict == "accept", "approved_spec_sha256": guard.SPEC},
        "policy_expr": '{"op":"true"}', "policy_expr_sha256": "b" * 64,
    }
    data.update(extra)
    data["policy_expr_sha256"] = guard.hashlib.sha256(data["policy_expr"].encode()).hexdigest()
    return data


def bundle(version, verdict):
    return {"format": "fixseal-bundle-v1", "mode": "lean",
            "input": {"subject": "spend", "version": version},
            "report": report(verdict), "report_sha256": "c" * 64}


class GuardOperatorTests(unittest.TestCase):
    def test_summary_matches_judge_spend_wire_format(self):
        report = {"inputs_sha256": "aa", "judge_version": guard.JUDGE, "verdict": "accept",
                  "proof": {"approved_spec_sha256": guard.SPEC},
                  "policy_expr": '{"op":"true"}', "policy_expr_sha256": "bb"}
        expected = {key: value for key, value in report.items() if key != "proof"}
        expected.update(spec_sha256=guard.SPEC, subject="spend")
        self.assertEqual(guard.summary(report), json.dumps(expected, sort_keys=True, separators=(",", ":")))

    def test_receipt_failure_cannot_be_concealed_by_successful_transaction(self):
        result = {"status": {"SuccessValue": ""},
                  "transaction_outcome": {"outcome": {"status": {"SuccessReceiptId": "id"}}},
                  "receipts_outcome": [{"outcome": {"status": {"Failure": "Guard spending denied"}}}]}
        with self.assertRaises(RuntimeError):
            guard.check_outcome(result)
        guard.check_outcome(result, "Guard spending denied")
        with self.assertRaises(RuntimeError):
            guard.check_outcome(result, "Guard record missing")
        result["receipts_outcome"] = []
        with self.assertRaises(RuntimeError):
            guard.check_outcome(result, "Guard spending denied")
        guard.check_outcome(result)

    def test_cli_hash_parser_ignores_ansi_and_refuses_ambiguous_output(self):
        tx_hash = "A" * 44
        self.assertEqual(guard.transaction_hash(f"Transaction ID: \x1b[32m{tx_hash}\x1b[0m"), tx_hash)
        with self.assertRaises(RuntimeError):
            guard.transaction_hash("no transaction hash")
        with self.assertRaises(RuntimeError):
            guard.transaction_hash(f"Transaction ID: {tx_hash}\nTransaction hash: {'B' * 44}")

    def test_check_never_uses_rpc_signing_or_creates_output(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(guard.shutil, "which", return_value="prepared"), \
                patch.object(guard, "Testnet") as client, patch.object(guard, "judge") as judge, \
                patch("sys.argv", ["testnet_guard.py", "--check", "--output", str(Path(tmp) / "run")]):
            guard.main()
            client.assert_not_called()
            judge.assert_not_called()
            self.assertFalse((Path(tmp) / "run").exists())


class StepHeadlineTests(unittest.TestCase):
    def make(self, tmp, extra=None):
        with patch.dict(os.environ, {**ENV, **(extra or {})}, clear=True):
            args = types.SimpleNamespace(output=str(Path(tmp) / "out"))
            operator = guard.Testnet(args)
        operator.output.mkdir(parents=True, exist_ok=True)
        return operator

    def fake_call(self, operator, label, method="guard_spend", **kwargs):
        run = types.SimpleNamespace(
            returncode=0, stdout="Transaction ID: " + "9" * 44 + "\n", stderr="")
        failure = kwargs.get("expected_error")
        result = {"status": {"SuccessValue": ""},
                  "transaction_outcome": {"outcome": {"status": {"SuccessReceiptId": "id"}}},
                  "receipts_outcome": [{"outcome": {"status": {"Failure": failure}}}] if failure else []}
        with patch.object(guard.subprocess, "run", return_value=run), \
                patch.object(operator, "rpc", return_value=result):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                operator.call(label, method, {}, **kwargs)
            return out.getvalue().strip()

    def test_step_pause_must_be_a_nonnegative_number(self):
        with tempfile.TemporaryDirectory() as tmp:
            for bad in ("abc", "-1", "nan", "inf", "-inf", ""):
                if bad:
                    with self.assertRaises(RuntimeError), \
                            patch.dict(os.environ, {**ENV, "FIXSEAL_TESTNET_STEP_PAUSE": bad}, clear=True):
                        guard.Testnet(types.SimpleNamespace(output=str(Path(tmp) / "out")))
            for good, expected in (("4", 4.0), ("0.5", 0.5)):
                operator = self.make(tmp, {"FIXSEAL_TESTNET_STEP_PAUSE": good})
                self.assertEqual(operator.step_pause, expected)

    def test_headline_numbers_ten_steps_and_pauses_between(self):
        with tempfile.TemporaryDirectory() as tmp:
            operator = self.make(tmp, {"FIXSEAL_TESTNET_STEP_PAUSE": "4"})
            with patch.object(guard.time, "sleep") as sleep:
                first = self.fake_call(operator, "Spend 600", note="PAID")
                second = self.fake_call(operator, "Overflow attack: spend " + guard.ATTACK, note="PAID (the bug)")
                refused = self.fake_call(operator, "Replace the rule without a record",
                                         method="guard_set_policy", expected_error="Guard record missing")
            self.assertEqual(sleep.call_count, 2)
            sleep.assert_called_with(4.0)
            self.assertRegex(first, r"^\[1/10\] Spend 600 … PAID; tx=9{44}$")
            self.assertRegex(second, r"^\[2/10\] Overflow attack: spend 18446744073709551116 … PAID \(the bug\)")
            self.assertIn("refused: Guard record missing", refused)

    def test_load_accepted_bundle_requires_kernel_checked_spend_accept(self):
        good = bundle("after_fx3", "accept")
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "accepted.json"
            path.write_text(json.dumps(good))
            self.assertEqual(guard.load_accepted_bundle(path), good)
        for key, value in (("verdict", "reject"), ("judge_version", "0.1.0/lean-spend-2")):
            bad = bundle("after_fx3", "accept")
            bad["report"][key] = value
            with tempfile.TemporaryDirectory() as tmp, self.assertRaises(RuntimeError):
                path = Path(tmp) / "accepted.json"
                path.write_text(json.dumps(bad))
                guard.load_accepted_bundle(path)
        bad = bundle("after_fx3", "accept")
        bad["report"]["proof"]["verified"] = False
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(RuntimeError):
            path = Path(tmp) / "accepted.json"
            path.write_text(json.dumps(bad))
            guard.load_accepted_bundle(path)
        bad = bundle("after_fx3", "accept")
        bad["input"] = {"version": "after_fx3"}
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(RuntimeError):
            path = Path(tmp) / "accepted.json"
            path.write_text(json.dumps(bad))
            guard.load_accepted_bundle(path)

    def test_accepted_bundle_substitutes_generated_after_honest(self):
        supplied = bundle("after_fx3", "accept")
        calls = []

        def fake_judge(args):
            calls.append(list(args))
            if args[0] == "bundle":
                return json.dumps(bundle(args[-1], "reject"))
            return ""

        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "accepted.json"
            path.write_text(json.dumps(supplied))
            with patch.dict(os.environ, ENV, clear=True), \
                    patch.object(guard, "judge", side_effect=fake_judge), \
                    patch.object(guard, "Testnet") as client, \
                    patch.object(guard.shutil, "which", return_value="prepared"), \
                    patch("sys.argv", ["testnet_guard.py", "--execute", "--output",
                                       str(Path(tmp) / "out"), "--accepted-bundle", str(path)]):
                guard.main()
            bundles = client.return_value.run.call_args[0][0]
            self.assertEqual(bundles["after_honest"], supplied)
            generated = [a[-1] for a in calls if a[0] == "bundle"]
            self.assertEqual(generated, ["before", "after_saturate"])
            self.assertIn(["verify", str(path)], calls)


if __name__ == "__main__":
    unittest.main()
