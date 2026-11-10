#!/usr/bin/env python3
"""Operator-only testnet guard demonstration; credentials stay in the OS keychain."""
import argparse
import base64
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
SPEC = "e3933e5161e3cbd034aa95cdf11659ffec4407c9697dbf405869d5a3fee5bebe"
JUDGE = "0.1.0/lean-spend-4"
ATTACK = str(2**64 - 500)


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def summary(report):
    fields = {key: report[key] for key in
              ("inputs_sha256", "judge_version", "verdict", "policy_expr", "policy_expr_sha256")}
    fields.update(subject="spend", spec_sha256=report["proof"]["approved_spec_sha256"])
    return encoded(fields)


def transaction_hash(output):
    clean = re.sub(r"\x1b\[[0-9;]*[A-Za-z]", "", output)
    matches = re.findall(r"Transaction (?:ID|hash):\s*([1-9A-HJ-NP-Za-km-z]{32,44})", clean, re.I)
    if len(set(matches)) != 1:
        raise RuntimeError("CLI did not return one transaction hash; inspect transaction status manually")
    return matches[0]


def check_outcome(result, expected_error=None):
    failures = [entry["outcome"]["status"]["Failure"]
                for entry in [result["transaction_outcome"], *result["receipts_outcome"]]
                if "Failure" in entry["outcome"]["status"]]
    if expected_error:
        if not failures or expected_error not in encoded(failures):
            raise RuntimeError("Expected guard rejection was not observed in final receipts")
    elif failures or "Failure" in result["status"]:
        raise RuntimeError("Transaction or receipt failed; stop and inspect the public transaction hash")


class Testnet:
    def __init__(self, args):
        self.args = args
        self.output = Path(args.output)
        self.contract = os.environ["FIXSEAL_TESTNET_CONTRACT"]
        self.owner = os.environ["FIXSEAL_TESTNET_OWNER"]
        self.agent = os.environ["FIXSEAL_TESTNET_AGENT"]
        self.recipient = os.environ["FIXSEAL_TESTNET_RECIPIENT"]
        self.rpc_url = os.environ.get("FIXSEAL_TESTNET_RPC", "https://rpc.testnet.near.org")
        if not self.rpc_url.startswith("https://") or "@" in self.rpc_url:
            raise RuntimeError("Use a credential-free HTTPS testnet RPC URL")
        for account in [self.contract, self.owner, self.agent, self.recipient]:
            if not re.fullmatch(r"[a-z0-9][a-z0-9_.-]{0,62}[a-z0-9]", account):
                raise RuntimeError("Invalid public account ID")
        self.keys = {role: os.environ[f"FIXSEAL_TESTNET_{role}_PUBLIC_KEY"]
                     for role in ["OWNER", "AGENT", "SPEND"]}
        if any(not re.fullmatch(r"(?:ed25519|secp256k1):[1-9A-HJ-NP-Za-km-z]+", key)
               for key in self.keys.values()):
            raise RuntimeError("Expected public keys only")
        self.evidence = self.output / "testnet-evidence.md"

    def rpc(self, method, params):
        request = urllib.request.Request(self.rpc_url, data=encoded({
            "jsonrpc": "2.0", "id": "fixseal-guard", "method": method, "params": params
        }).encode(), headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(request, timeout=60) as response:
            value = json.load(response)
        if "error" in value:
            raise RuntimeError("Testnet RPC returned an error; stop and inspect public state")
        return value["result"]

    def query(self, request_type, **params):
        return self.rpc("query", {"request_type": request_type, "finality": "final", **params})

    def view(self, method, args=None):
        response = self.query("call_function", account_id=self.contract, method_name=method,
                              args_base64=base64.b64encode(encoded(args or {}).encode()).decode())
        return json.loads(bytes(response["result"]))

    def balance(self):
        return int(self.query("view_account", account_id=self.recipient)["amount"])

    def call(self, label, method, args, role="OWNER", expected_error=None):
        signer = self.owner if role == "OWNER" else self.agent
        command = ["near", "contract", "call-function", "as-transaction", self.contract, method,
                   "json-args", encoded(args), "prepaid-gas", "100 Tgas", "attached-deposit",
                   "0 NEAR", "sign-as", signer, "network-config", "testnet",
                   "sign-with-keychain", "--signer-public-key", self.keys[role], "send"]
        # Capture CLI diagnostics only in memory; never persist or display credential prompts.
        run = subprocess.run(command, text=True, capture_output=True, stdin=subprocess.DEVNULL,
                             timeout=180, check=False)
        tx_hash = transaction_hash(run.stdout + run.stderr)
        with self.evidence.open("a") as out:
            out.write(f"| {label} | `{tx_hash}` | pending final receipt check |\n")
        result = self.rpc("tx", {"tx_hash": tx_hash, "sender_account_id": signer, "wait_until": "FINAL"})
        check_outcome(result, expected_error)
        if run.returncode and not expected_error:
            raise RuntimeError("CLI reported failure; transaction hash retained for inspection")
        with self.evidence.open("a") as out:
            out.write(f"| {label} final | `{tx_hash}` | {'expected rejection' if expected_error else 'success'} |\n")
        print(f"{label}: {'rejected as expected' if expected_error else 'final receipts passed'}; tx={tx_hash}")

    def preflight(self):
        if self.rpc("status", {})["chain_id"] != "testnet":
            raise RuntimeError("RPC is not testnet")
        info = self.view("get_contract_info")
        if info["owner_id"] != self.owner or info["requires_tee"]:
            raise RuntimeError("Expected owner and local/mock mode on this demo contract")
        if self.view("guard_state") is not None:
            raise RuntimeError("Guard already initialized; use an unused demo contract")
        agent = self.view("get_agent", {"account_id": self.agent})
        if not agent or agent["validity"] != "Valid":
            raise RuntimeError("Prepare a valid registered mock agent before running")
        permission = self.query("view_access_key", account_id=self.agent,
                                public_key=self.keys["SPEND"])["permission"]
        fc = permission.get("FunctionCall", {}) if isinstance(permission, dict) else {}
        if fc.get("receiver_id") != self.contract or fc.get("method_names") != ["guard_spend"]:
            raise RuntimeError("Spend key must allow only guard_spend on this contract")
        for role, account in [("OWNER", self.owner), ("AGENT", self.agent)]:
            if self.query("view_access_key", account_id=account,
                          public_key=self.keys[role])["permission"] != "FullAccess":
                raise RuntimeError("Operator and record submitter keys must be prepared separately")
        self.balance()  # Require an existing recipient; no account creation is attempted.

    def run(self, bundles):
        self.preflight()
        baseline = bundles["before"]["report"]
        self.call("initialize vulnerable policy", "guard_init", {
            "limit": "1000", "spec_sha256": SPEC, "policy_expr": baseline["policy_expr"]})
        balance = self.balance()
        for amount in ["600", ATTACK]:
            self.call(f"vulnerable spend {amount}", "guard_spend", {
                "to": self.recipient, "amount": amount}, role="SPEND")
        before = self.view("guard_state")
        if int(before["total_paid"]) != 2**64 + 100 or before["spent"] != "100" or before["pending_paid"] != "0":
            raise RuntimeError("Overflow accounting did not match the demo")
        if self.balance() - balance != int(before["total_paid"]):
            raise RuntimeError("Recipient balance did not match actual payments")
        self.call("replacement without record", "guard_set_policy", {
            "policy_expr": bundles["after_saturate"]["report"]["policy_expr"], "record_id": "missing"},
            expected_error="Guard record missing")
        for version in ["after_saturate", "after_honest"]:
            evidence = bundles[version]
            report = evidence["report"]
            record_id = evidence["report_sha256"]
            self.call(f"submit {version}", "submit_fix_proof", {
                "bundle_hash": record_id, "summary": summary(report)}, role="AGENT")
            record = self.view("get_fix_proof", {"bundle_hash": record_id})
            if record["summary"] != summary(report) or record["bundle_hash"] != record_id or record["caller"] != self.agent:
                raise RuntimeError("Chain readback mismatch")
            evidence["chain"] = record
            path = self.output / f"{version}.json"
            path.write_text(encoded(evidence))
            judge(["verify", str(path)])
            self.call(f"install {version}", "guard_set_policy", {
                "policy_expr": report["policy_expr"], "record_id": record_id},
                expected_error="Guard record is not accept" if version == "after_saturate" else None)
        balance = self.balance()
        self.call("replay overflow attack", "guard_spend", {"to": self.recipient, "amount": ATTACK},
                  role="SPEND", expected_error="Guard spending denied")
        if self.balance() != balance or self.view("guard_state")["total_paid"] != before["total_paid"]:
            raise RuntimeError("Rejected attack changed payment state")
        self.call("legitimate spend", "guard_spend", {"to": self.recipient, "amount": "100"}, role="SPEND")
        after = self.view("guard_state")
        expected_hash = bundles["after_honest"]["report"]["policy_expr_sha256"]
        if (int(after["total_paid"]) != int(before["total_paid"]) + 100
                or self.balance() != balance + 100 or after["spent"] != "200"
                or after["policy_expr_sha256"] != expected_hash or len(self.view("guard_history")) != 1):
            raise RuntimeError("Final guard state or recipient balance mismatch")
        (self.output / "guard-state.json").write_text(encoded(after))
        with self.evidence.open("a") as out:
            out.write("\nRun status: complete; final guard state and recipient balance checks passed.\n")
        print("仮想空間での検証 / simulated environment, synthetic data: all guard steps passed")


def judge(args):
    run = subprocess.run(["cargo", "run", "--manifest-path", str(ROOT / "judge/Cargo.toml"),
                          "--locked", "--offline", "--", *args], cwd=ROOT,
                         capture_output=True, text=True, check=False)
    if run.returncode:
        raise RuntimeError("Offline judge/replay failed; no transaction should follow")
    return run.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--execute", action="store_true", help="operator-authorized testnet transactions")
    parser.add_argument("--check", action="store_true", help="local file/tool checks only (default)")
    parser.add_argument("--output", default=str(ROOT / "demo-artifacts/testnet-guard"))
    args = parser.parse_args()
    if args.execute and args.check:
        parser.error("choose --check or --execute")
    for tool in ["near", "cargo"]:
        if not shutil.which(tool):
            raise RuntimeError(f"Missing preinstalled {tool}")
    if not args.execute:
        print("Local tools found; no RPC/signing. See docs/guard.md for account, keychain and wasm prerequisites.")
        return
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=False)
    operator = Testnet(args)
    template = (ROOT / "docs/testnet-evidence.md").read_text()
    template = template.replace("No testnet transaction has been executed for T010. This is a template, not evidence of success.",
                                "Operator run started. This populated copy is incomplete until all final checks pass.")
    template = template.replace("Run date: pending operator run", "Run date: " + datetime.now(timezone.utc).isoformat())
    operator.evidence.write_text(template + f"\nContract: `{operator.contract}`; recipient: `{operator.recipient}`\n\n| Step | Transaction hash | Outcome |\n| --- | --- | --- |\n")
    bundles = {}
    for version in ["before", "after_saturate", "after_honest"]:
        evidence = json.loads(judge(["bundle", "--lean", "--subject", "spend", str(ROOT / "lean"), version]))
        report = evidence["report"]
        if report["judge_version"] != JUDGE or report["proof"]["approved_spec_sha256"] != SPEC:
            raise RuntimeError("Unexpected judge or specification")
        if report["verdict"] != ("accept" if version == "after_honest" else "reject"):
            raise RuntimeError("Unexpected offline verdict")
        if version == "after_honest" and (report["reason_codes"] or not report["proof"]["verified"]):
            raise RuntimeError("Accepted proof did not pass kernel checking")
        if hashlib.sha256(report["policy_expr"].encode()).hexdigest() != report["policy_expr_sha256"]:
            raise RuntimeError("Policy expression hash mismatch")
        path = output / f"{version}.json"
        path.write_text(encoded(evidence))
        judge(["verify", str(path)])
        bundles[version] = evidence
    operator.run(bundles)


if __name__ == "__main__":
    try:
        main()
    except RuntimeError as error:
        sys.exit(f"Guard demo stopped: {error}. Partial public evidence is retained.")
    except KeyError:
        sys.exit("Guard demo stopped: missing required public configuration or unexpected RPC schema; see docs/guard.md.")
    except (OSError, ValueError, subprocess.TimeoutExpired):
        # Do not echo subprocess arguments, environment or credential-store diagnostics.
        sys.exit("Guard demo stopped: tool/file/RPC error or timeout. Inspect public prerequisites and partial evidence.")
