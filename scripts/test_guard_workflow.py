import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("testnet_guard", ROOT / "scripts/testnet_guard.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


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


if __name__ == "__main__":
    unittest.main()
