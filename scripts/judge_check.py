#!/usr/bin/env python3
"""Standard-library evidence checks; Lean replay is an explicit later stage."""

import base64
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent
EVIDENCE = ROOT / "examples/fixers/evidence"
REPORT_FIELDS = (
    "judge_version", "validation_scope", "verdict", "reason_codes", "input_hashes",
    "inputs_sha256", "checks", "proof", "policy_violations", "policy_expr",
    "policy_expr_sha256", "export_checked_inputs",
)
CHECK_FIELDS = (
    "attacks_total", "baseline_attacks_working", "candidate_attacks_working",
    "regressions_total", "regressions_passed",
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def wire(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON field")
        result[key] = value
    return result


def load(data):
    return json.loads(data, object_pairs_hook=unique_object)


def bytes_array(value):
    require(isinstance(value, list) and all(type(n) is int and 0 <= n <= 255 for n in value),
            "expected byte array")
    return bytes(value)


def validate_bundle(bundle):
    require(set(bundle) in (
        {"format", "mode", "input", "report", "report_sha256"},
        {"format", "mode", "input", "report", "report_sha256", "chain"},
    ), "bundle fields")
    require(bundle["format"] == "fixseal-bundle-v1" and bundle["mode"] == "lean", "format/mode")
    data, report = bundle["input"], bundle["report"]
    require(set(data) == {"version", "baseline", "candidate", "corpus", "policy"}
            | ({"subject"} if "subject" in data else set()), "input fields")
    subject = data.get("subject", "path")
    require(subject in ("path", "spend"), "subject")
    require(isinstance(data["version"], str) and re.fullmatch(r"[A-Za-z0-9_-]+", data["version"]), "version")
    files = {"version": data["version"].encode(), "corpus.json": bytes_array(data["corpus"]),
             "policy.json": bytes_array(data["policy"])}
    if subject != "path":
        files["subject"] = subject.encode()
    for prefix in ("baseline", "candidate"):
        require(isinstance(data[prefix], dict) and data[prefix], "source tree")
        for name, values in data[prefix].items():
            require(not name.startswith("/") and not any(p in ("", ".", "..") for p in name.split("/"))
                    and "\\" not in name and "\0" not in name, "source path")
            files[f"{prefix}/{name}"] = bytes_array(values)
    combined = bytearray(b"fixseal-inputs-v1\0")
    hashes = {}
    for name, values in sorted(files.items()):
        encoded = name.encode()
        combined += len(encoded).to_bytes(8, "big") + encoded
        combined += len(values).to_bytes(8, "big") + values
        hashes[name] = sha(values)
    require(report["input_hashes"] == hashes, "individual input hashes")
    require(report["inputs_sha256"] == sha(combined), "combined input hash")
    require(set(REPORT_FIELDS[:9]) <= set(report) <= set(REPORT_FIELDS), "report fields")
    require(report["judge_version"] == f'0.1.0/lean-{"spend-4" if subject == "spend" else "path-7"}', "judge version")
    require(report["validation_scope"] == "仮想空間での検証", "scope")
    require(report["verdict"] in ("accept", "reject"), "verdict")
    for field in ("reason_codes", "policy_violations"):
        require(isinstance(report[field], list) and all(isinstance(v, str) for v in report[field]), field)
    checks = report["checks"]
    require(set(checks) == set(CHECK_FIELDS), "check fields")
    require(all(type(n) is int and n >= 0 for n in checks.values()), "check counts")
    require(checks["candidate_attacks_working"] <= checks["attacks_total"]
            and checks["baseline_attacks_working"] <= checks["attacks_total"]
            and checks["regressions_passed"] <= checks["regressions_total"], "check bounds")
    proof = report["proof"]
    require(set(proof) == {"verified", "approved_spec_sha256"} and type(proof["verified"]) is bool, "proof fields")
    spec = ROOT / ("lean/Spec.lean" if subject == "path" else "lean/targets/spend/SpendSpec.lean")
    require(proof["approved_spec_sha256"] == sha(spec.read_bytes()), "spec hash")
    if report["verdict"] == "accept":
        require(proof["verified"] and not report["reason_codes"] and not report["policy_violations"], "accept status")
    ordered = {key: report[key] for key in REPORT_FIELDS if key in report}
    ordered["input_hashes"] = dict(sorted(hashes.items()))
    ordered["checks"] = {key: checks[key] for key in CHECK_FIELDS}
    ordered["proof"] = {key: proof[key] for key in ("verified", "approved_spec_sha256")}
    if "policy_expr" in report:
        expression = report["policy_expr"]
        require(isinstance(expression, str) and wire(load(expression)).decode() == expression, "expression JSON")
        require(report["policy_expr_sha256"] == sha(expression.encode()), "expression hash")
    if "export_checked_inputs" in report:
        require(type(report["export_checked_inputs"]) is int and report["export_checked_inputs"] >= 0, "export count")
    if subject == "spend" and report["verdict"] == "accept":
        require(set(REPORT_FIELDS[9:]) <= set(report), "accepted spend export fields")
    require(bundle["report_sha256"] == sha(wire(ordered) + b"\n"), "report hash")
    if bundle.get("chain") is not None:
        record = bundle["chain"]
        require(set(record) == {"bundle_hash", "summary", "caller", "block_timestamp_ms"}, "chain fields")
        require(all(isinstance(value, str) for value in record.values()), "chain types")
        require(record["bundle_hash"] == bundle["report_sha256"], "chain result hash")
        summary = {key: report[key] for key in ("judge_version", "inputs_sha256", "verdict")}
        if subject == "spend":
            summary.update(subject="spend", spec_sha256=proof["approved_spec_sha256"],
                           policy_expr=expression, policy_expr_sha256=report["policy_expr_sha256"])
        require(record["summary"] == wire(dict(sorted(summary.items()))).decode(), "chain summary")


def hashes():
    lines = (EVIDENCE / "SHA256SUMS").read_text().splitlines()
    require(lines, "empty evidence manifest")
    seen = set()
    for line in lines:
        digest, name = line.split("  ", 1)
        require(re.fullmatch(r"[0-9a-f]{64}", digest) and name not in seen, "manifest entry")
        path = Path(name)
        require(not path.is_absolute() and all(p not in ("", ".", "..") for p in name.split("/")), "manifest path")
        require(not any((ROOT / Path(*path.parts[:i])).is_symlink() for i in range(1, len(path.parts) + 1)), "manifest symlink")
        require(sha((ROOT / path).read_bytes()) == digest, f"evidence hash mismatch: {name}")
        seen.add(name)
    require({"examples/fixers/evidence/path-honest.json", "examples/fixers/evidence/spend-honest.json"} <= seen, "missing evidence bundles")
    print(f"{len(seen)} packaged evidence files match; integrity only, not provenance authentication")


def bundles():
    for path in sorted(EVIDENCE.glob("*-honest.json")):
        validate_bundle(load(path.read_bytes()))
        print(f"{path.name}: structure/hashes match; claimed accept; Lean not executed")
    require(len(list(EVIDENCE.glob("*-honest.json"))) == 2, "expected two evidence bundles")


def lean_ready():
    for tool in ("lean", "lake"):
        executable = shutil.which(tool)
        require(executable is not None, "Lean/Lake unavailable")
        require(Path(executable).resolve().name != "elan", "elan shim requires prepared real binaries")
        output = subprocess.check_output([executable, "--version"], text=True, timeout=10)
        require("4.34.1" in output, "Lean/Lake version must be 4.34.1")


def run(args, **kwargs):
    print("+ " + " ".join(map(str, args)), flush=True)
    return subprocess.run(args, check=True, **kwargs)


def lean_builds():
    for directory in (ROOT / "lean", ROOT / "lean/targets/spend"):
        run(["lake", "build"], cwd=directory)
    run(["lake", "env", "lean", "SpecTests.lean"], cwd=ROOT / "lean")


def judge(*args, **kwargs):
    return run(["cargo", "run", "--manifest-path", str(ROOT / "judge/Cargo.toml"),
                "--locked", "--offline", "--", *args], **kwargs)


def versions(directory):
    directory = Path(directory)
    expected = {
        "before": ("reject", ["ATTACK_STILL_WORKS"]),
        "after_honest": ("accept", []),
        "after_hardcode": ("reject", ["POLICY_VIOLATION", "ATTACK_STILL_WORKS"]),
        "after_cheat": ("reject", ["PROOF_CHEATED"]),
        "after_reject_all": ("reject", ["REGRESSION_FAILED"]),
    }
    for subject, extra in (("path", {"after_weaken_test": ("reject", ["POLICY_VIOLATION"])}),
                           ("spend", {"after_saturate": ("reject", ["ATTACK_STILL_WORKS"])})):
        fixtures = dict(expected, **extra)
        root = ROOT / ("lean" if subject == "path" else "lean/targets/spend")
        require({p.name for p in root.iterdir() if p.is_dir() and (p / "Target.lean").exists()} == set(fixtures),
                "fixture list changed; review expected verdicts")
        for version, (verdict, reasons) in fixtures.items():
            path = directory / f"{subject}-{version}.json"
            # Bundle export exits 0 for rejections too; inspect the complete expected verdict/reasons.
            with path.open("w") as output:
                judge("bundle", "--lean", "--subject", subject, "lean", version, stdout=output)
            bundle = load(path.read_bytes())
            validate_bundle(bundle)
            report = bundle["report"]
            require((report["verdict"], report["reason_codes"]) == (verdict, reasons), f"unexpected {subject}/{version}: {report}")
            require(report["proof"]["verified"] == (version == "after_honest"), "proof status")
            print(subject, version, verdict, reasons, report["checks"], flush=True)


def replay(directory):
    directory = Path(directory)
    for subject in ("path", "spend"):
        path = directory / f"fresh-{subject}.json"
        with path.open("w") as output:
            judge("bundle", "--lean", "--subject", subject, "lean", "after_honest", stdout=output)
        validate_bundle(load(path.read_bytes()))
        judge("verify", str(path))
        judge("verify", str(EVIDENCE / f"{subject}-honest.json"))


def testnet():
    text = (ROOT / "docs/testnet-evidence.md").read_text()
    settings = []
    for label in ("Judge-check contract account", "Judge-check accepted record ID"):
        match = re.search(rf"^- {label}: `([^`]*)`$", text, re.M)
        settings.append(match[1] if match else "")
    account, record_id = settings
    if not account or not record_id:
        print("testnet account/record ID unfilled in docs/testnet-evidence.md")
        return 3
    require(re.fullmatch(r"[a-z0-9][a-z0-9_.-]{0,62}[a-z0-9]", account), "invalid contract account")
    require(re.fullmatch(r"[0-9a-f]{64}", record_id), "invalid record ID")
    if not shutil.which("curl"):
        print("curl unavailable for read-only RPC")
        return 3
    responses = {}
    for method, args in (("guard_state", {}), ("get_fix_proof", {"bundle_hash": record_id})):
        payload = {"jsonrpc": "2.0", "id": method, "method": "query", "params": {
            "request_type": "call_function", "finality": "final", "account_id": account,
            "method_name": method, "args_base64": base64.b64encode(wire(args)).decode()}}
        response = subprocess.run([
            "curl", "--silent", "--show-error", "--proto", "=https", "--connect-timeout", "5",
            "--max-time", "15", "--header", "Content-Type: application/json", "--data-binary", "@-",
            "https://rpc.testnet.near.org",
        ], input=wire(payload), capture_output=True)
        if response.returncode:
            # Transport unavailability is a skip; RPC errors and inconsistent data fail below.
            print(f"testnet transport unavailable (curl exit {response.returncode})")
            return 3
        rpc = load(response.stdout)
        require("error" not in rpc and "result" in rpc and "error" not in rpc["result"], f"{method} RPC failure")
        responses[method] = load(bytes_array(rpc["result"]["result"]))
    state, record = responses["guard_state"], responses["get_fix_proof"]
    require(isinstance(state, dict) and isinstance(record, dict), "uninitialized guard or missing record")
    require(record["bundle_hash"] == record_id, "record ID mismatch")
    summary = load(record["summary"])
    reference = load((EVIDENCE / "spend-honest.json").read_bytes())["report"]
    require(summary["verdict"] == "accept" and summary["subject"] == "spend", "record verdict/subject")
    require(summary["spec_sha256"] == reference["proof"]["approved_spec_sha256"], "record spec")
    require(summary["judge_version"] == reference["judge_version"], "record judge")
    require(sha(summary["policy_expr"].encode()) == summary["policy_expr_sha256"], "record expression hash")
    require(state["record_id"] == record_id and state["spec_sha256"] == summary["spec_sha256"], "active record/spec")
    require(state["policy_expr"] == summary["policy_expr"]
            and state["policy_expr_sha256"] == summary["policy_expr_sha256"], "active policy mismatch")
    print("guard_state/get_fix_proof: final read-only responses agree; no signing or transactions")


def main():
    command = sys.argv[1]
    actions = {"hashes": hashes, "bundles": bundles, "lean-ready": lean_ready,
               "lean-builds": lean_builds, "testnet": testnet}
    if command in ("versions", "replay"):
        (versions if command == "versions" else replay)(sys.argv[2])
    else:
        return actions[command]()


if __name__ == "__main__":
    try:
        sys.exit(main() or 0)
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"Check failed: {error}", file=sys.stderr)
        sys.exit(1)
