#!/usr/bin/env python3
"""Run a NEAR AI Cloud (OpenAI-compatible) model as a Fixseal repair agent.

Standard library only. The model gets two tools, ``bash`` and ``done``.
``bash`` runs inside the prepared work copy only and rejects commands that
use the network. ``done`` runs the judge; a non-accept verdict is returned
to the model so it can keep fixing. Every judge run is appended to a JSON
Lines log, and a Markdown record in the style of docs/fixer/FX3-*.md is
written at the end. The API key is read only from NEARAI_API_KEY and is
never written to logs, records or exception text.
"""
import argparse
import json
import os
import re
import shlex
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path

# Confirmed on the public quickstart / OpenAI-compatibility pages at
# https://docs.near.ai/cloud/ and https://docs.near.ai/cloud/guides/openai-compatibility
DEFAULT_BASE_URL = "https://cloud-api.near.ai/v1"
# Model used in the quickstart example; override with --model / NEARAI_MODEL.
DEFAULT_MODEL = "zai-org/GLM-5.1-FP8"
API_KEY_ENV = "NEARAI_API_KEY"
REDACTED = "***REDACTED***"

DONE_NUDGE = "The judge accepted your candidate or you wrote the repair record. Call done now to submit it officially; bash alone does not finish the run."

NUDGE = "No bash / done call yet. Continue working, then call done when finished."

# Commands that reach the network (or install/fetch code) are rejected before
# execution. This is a guardrail for a cooperative agent, not a sandbox.
BLOCKED_COMMANDS = {
    "curl", "wget", "http", "https", "nc", "ncat", "netcat", "socat",
    "ssh", "scp", "sftp", "ftp", "telnet", "rsh",
    "pip", "pip3", "pipx", "npm", "npx", "yarn", "pnpm", "bun", "uv", "rye",
    "poetry", "conda", "gem", "brew", "apt", "apt-get", "yum", "dnf",
    "pacman", "zypper", "apk", "mas",
    "docker", "docker-compose", "podman", "kubectl", "vagrant",
    "gh", "glab", "hub", "rustup", "elan", "near", "near-cli", "open",
}
# argv prefixes (argv0, argv1[, argv2]) that fetch over the network.
BLOCKED_PREFIXES = [
    ("git", "clone"), ("git", "fetch"), ("git", "pull"), ("git", "push"),
    ("git", "ls-remote"), ("git", "remote"), ("git", "submodule"),
    ("cargo", "install"), ("cargo", "add"), ("cargo", "search"),
    ("cargo", "login"), ("cargo", "publish"), ("cargo", "owner"),
    ("cargo", "yank"),
    ("lake", "update"), ("lake", "upgrade"), ("lake", "exe", "cache"),
    ("go", "get"), ("go", "install"), ("go", "mod"),
    ("python", "-m", "pip"), ("python3", "-m", "pip"),
    ("python", "-m", "venv"), ("python3", "-m", "venv"),  # ensurepip bootstrap
]
WRAPPER_COMMANDS = {"env", "nice", "time", "command", "builtin", "exec", "xargs"}
ALLOWED_ABSOLUTE = {"/dev/null", "/dev/stdout", "/dev/stderr"}

_secret = ""


def redact(text):
    """Remove the API key from anything that may be logged or returned."""
    if not _secret:
        return text
    return text.replace(_secret, REDACTED)


def utc_now():
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def truncate(text, head=2000, tail=6000):
    if len(text) <= head + tail + 100:
        return text
    return text[:head] + "\n...[truncated]...\n" + text[-tail:]


def split_words(segment):
    try:
        return shlex.split(segment, posix=True)
    except ValueError:
        return None


def _check_tokens(argv, workdir):
    """Return a rejection message or None for a single simple command."""
    i = 0
    while i < len(argv) and re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", argv[i]):
        i += 1
    argv = argv[i:]
    while argv and os.path.basename(argv[0]) in WRAPPER_COMMANDS:
        argv = argv[1:]
        while argv and re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", argv[0]):
            argv = argv[1:]
    if not argv:
        return None
    name = os.path.basename(argv[0])
    if name in BLOCKED_COMMANDS:
        return "rejected: '%s' is not allowed (no network access)" % name
    for prefix in BLOCKED_PREFIXES:
        if tuple(argv[: len(prefix)]) == prefix:
            return "rejected: '%s' is not allowed (no network access)" % " ".join(prefix)
    root = os.path.realpath(workdir)
    for token in argv[1:]:
        # Strip shell redirection prefixes like "2>" or ">>" before the check.
        target = re.sub(r"^[0-9&]*[<>]+", "", token)
        if target in ("..",) or target.startswith("../") or "/../" in target or target.endswith("/.."):
            return "rejected: paths outside the work directory are not allowed"
        if target.startswith("~"):
            return "rejected: home-relative paths are not allowed"
        if re.match(r"^\$\{?[A-Za-z_]", target):
            return "rejected: variable expansion in paths is not allowed"
        if target.startswith("/") and target not in ALLOWED_ABSOLUTE:
            resolved = os.path.realpath(target)
            if resolved != root and not resolved.startswith(root + os.sep):
                return "rejected: absolute paths outside the work directory are not allowed"
    return None


def command_rejection(command, workdir):
    """Return a rejection message if the shell command is not allowed."""
    for segment in re.split(r"[|&;\n]+", command):
        words = split_words(segment)
        if not words:
            continue
        message = _check_tokens(words, workdir)
        if message:
            return message
    return None


def child_env():
    """Environment for model-invoked commands and the judge.

    The API key and similar variables are removed so a command such as
    ``env`` cannot copy them into the transcript.
    """
    drop = re.compile(r"(?i)(^NEARAI|_API_KEY$|_TOKEN$|_SECRET$|_PASSWORD$)")
    return {k: v for k, v in os.environ.items() if not drop.search(k)}


def run_bash(command, workdir, timeout):
    rejection = command_rejection(command, workdir)
    if rejection:
        return rejection
    try:
        proc = subprocess.run(
            command, shell=True, cwd=workdir, env=child_env(),
            capture_output=True, text=True, errors="replace", timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return "command timed out after %d seconds" % timeout
    output = proc.stdout + proc.stderr
    return "exit %d\n%s" % (proc.returncode, truncate(output))


def extract_errors(text, limit=6):
    lines = [ln for ln in text.splitlines() if "error" in ln.lower()]
    return lines[:limit]


class Judge:
    """Runs the fixseal judge inside the work copy."""

    def __init__(self, workdir, subject, version, timeout, cargo="cargo"):
        self.workdir = workdir
        self.subject = subject
        self.version = version
        self.timeout = timeout
        self.cargo = cargo

    def _args(self, bundle=False):
        args = [self.cargo, "run", "--manifest-path", "judge/Cargo.toml",
                "--locked", "--offline", "--"]
        if bundle:
            args.append("bundle")
        args.append("--lean")
        if self.subject != "path":
            args += ["--subject", self.subject]
        args += ["lean", self.version]
        return args

    def _run(self, args):
        started = time.monotonic()
        try:
            proc = subprocess.run(
                args, cwd=self.workdir, env=child_env(),
                capture_output=True, text=True, errors="replace",
                timeout=self.timeout,
            )
            return proc.returncode, proc.stdout, proc.stderr, time.monotonic() - started
        except subprocess.TimeoutExpired:
            return None, "", "judge timed out after %d seconds" % self.timeout, \
                time.monotonic() - started

    def judge(self):
        """Return (report_dict_or_None, raw_output, seconds, exit_code)."""
        code, stdout, stderr, seconds = self._run(self._args())
        report = None
        for line in stdout.splitlines():
            line = line.strip()
            if line.startswith("{"):
                try:
                    report = json.loads(line)
                except json.JSONDecodeError:
                    pass
        return report, stdout + stderr, seconds, code

    def bundle(self, bundle_path):
        code, stdout, _, seconds = self._run(self._args(bundle=True))
        if code == 0 and stdout.strip():
            Path(bundle_path).write_text(redact(stdout))
            return True, seconds
        return False, seconds

    def verify(self, bundle_path):
        code, stdout, stderr, seconds = self._run(
            [self.cargo, "run", "--manifest-path", "judge/Cargo.toml",
             "--locked", "--offline", "--", "verify", str(bundle_path)])
        return code, stdout + stderr, seconds


TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "bash",
            "description": "Run a shell command inside the prepared work "
                           "directory. Network commands are rejected.",
            "parameters": {
                "type": "object",
                "properties": {"command": {"type": "string"}},
                "required": ["command"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "done",
            "description": "Submit the candidate for official judging. Call "
                           "only when the fix is ready; a reject verdict is "
                           "returned so you can keep fixing.",
            "parameters": {
                "type": "object",
                "properties": {"summary": {"type": "string"}},
            },
        },
    },
]

SYSTEM_PROMPT = """You are a repair agent inside a prepared copy of the Fixseal \
repository. The work directory is a full repository copy with the reference \
answers withheld.

Tools:
- bash(command): run a shell command inside the work directory. Commands \
that use the network or leave the work directory are rejected.
- done(summary): run the official judge on your candidate. A non-accept \
verdict and its details are returned so you can keep fixing. Call done only \
when the candidate is ready.

Edit only the files your task instructions allow. Replies without a tool \
call do not execute anything; keep acting with tools until done."""


class ChatClient:
    """Minimal OpenAI-compatible chat completions client."""

    def __init__(self, base_url, api_key, model, timeout=600):
        self.base_url = base_url.rstrip("/")
        self.url = self.base_url + "/chat/completions"
        self.api_key = api_key
        self.model = model
        self.timeout = timeout

    def model_pricing(self):
        request = urllib.request.Request(
            self.base_url + "/models",
            headers={"Authorization": "Bearer " + self.api_key})
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                models = json.loads(response.read().decode())
            model = next(item for item in models["data"] if item.get("id") == self.model)
            return {"source": self.base_url + "/models", "ts": utc_now(),
                    "model": self.model, "pricing": model.get("pricing")}
        except (OSError, ValueError, KeyError, TypeError, StopIteration) as error:
            return {"source": self.base_url + "/models", "model": self.model,
                    "error": redact(str(error))}

    def chat(self, messages, tools):
        body = {"model": self.model, "messages": messages, "tools": tools}
        data = json.dumps(body).encode()
        last_error = None
        for attempt in range(4):
            request = urllib.request.Request(
                self.url, data=data, method="POST",
                headers={
                    "Content-Type": "application/json",
                    "Authorization": "Bearer " + self.api_key,
                },
            )
            try:
                with urllib.request.urlopen(request, timeout=self.timeout) as resp:
                    return json.loads(resp.read().decode())
            except urllib.error.HTTPError as error:
                detail = error.read().decode(errors="replace")[:500]
                last_error = "HTTP %d: %s" % (error.code, redact(detail))
                if error.code not in (429, 500, 502, 503, 504):
                    break
            except (urllib.error.URLError, TimeoutError, OSError) as error:
                last_error = redact(str(error))
            time.sleep(min(5 * 2 ** attempt, 60))
        raise RuntimeError("chat completion failed: " + last_error)


def usage_of(response):
    usage = response.get("usage") or {}
    prompt = usage.get("prompt_tokens")
    completion = usage.get("completion_tokens")
    cached = (usage.get("prompt_tokens_details") or {}).get("cached_tokens")
    return {
        "prompt_tokens": prompt or 0,
        "completion_tokens": completion or 0,
        "total_tokens": usage.get("total_tokens") or ((prompt or 0) + (completion or 0)),
        "cached_tokens": cached or 0,
        "cache_usage_missing": int(cached is None),
        "usage_missing": int(prompt is None or completion is None),
    }


def estimate_cost(metadata, tokens):
    """Use explicit USD units only; unknown schemas never become a guessed bill."""
    pricing = metadata.get("pricing") or {}
    result = {"source": metadata, "usage": dict(tokens), "usd": None,
              "kind": "unavailable"}
    try:
        if pricing.get("currency", "USD").upper() != "USD":
            raise ValueError("unsupported currency")
        # OpenAI-compatible prompt/completion prices are USD per token.
        if "prompt" in pricing and "completion" in pricing:
            input_rate, output_rate = pricing["prompt"], pricing["completion"]
            cache_rate = pricing.get("input_cache_read")
            scale = {"usd_per_token": Decimal(1),
                     "usd_per_million_tokens": Decimal(1000000)}[
                         pricing.get("unit", "usd_per_token")]
        else:
            input_rate, output_rate = pricing["input"], pricing["output"]
            cache_rate = pricing.get("cache_read")
            scale = {"usd_per_token": Decimal(1),
                     "usd_per_million_tokens": Decimal(1000000)}[pricing["unit"]]
        rates = [Decimal(str(rate)) / scale for rate in (input_rate, output_rate)]
        cached_rate = Decimal(str(cache_rate)) / scale if cache_rate is not None else rates[0]
        if (any(not r.is_finite() or r < 0 for r in rates + [cached_rate])
                or cached_rate > rates[0]):
            raise ValueError("invalid rate")
        if (tokens.get("usage_missing")
                or any(type(tokens[key]) is not int or tokens[key] < 0
                       for key in ("prompt_tokens", "completion_tokens"))):
            raise ValueError("incomplete/invalid usage")
        cached = tokens.get("cached_tokens", 0)
        if not 0 <= cached <= tokens["prompt_tokens"]:
            raise ValueError("invalid cached usage")
        discount = (not tokens.get("cache_usage_missing", 1) and cache_rate is not None)
        cost = Decimal(tokens["prompt_tokens"]) * rates[0] + Decimal(tokens["completion_tokens"]) * rates[1]
        if discount:
            cost += Decimal(cached) * (cached_rate - rates[0])
        result.update(usd=str(cost), kind="estimate" if discount else "upper_bound",
                      cache_discount_applied=discount)
    except (AttributeError, KeyError, TypeError, ValueError, InvalidOperation):
        result["reason"] = "missing/unsupported pricing units or incomplete usage"
    return result


def completion_hint(command, result):
    accepted = bool(re.search(r'(?:"verdict"\s*:\s*"accept"|verdict=accept|\baccept\s+\[\])', result, re.IGNORECASE))
    record = "docs/fixer/" in command and bool(re.search(
        r'(?:>|write_text|write_bytes|tee\s|cp\s|mv\s)', command))
    return accepted or (record and result.startswith("exit 0"))


def summarize_report(report, raw_output):
    """Short judge summary fed back to the model and recorded per attempt."""
    if report is None:
        errors = extract_errors(raw_output)
        tail = errors if errors else truncate(raw_output, head=0, tail=800).splitlines()
        return "judge did not emit a report: " + " | ".join(tail)
    parts = ["verdict=%s" % report.get("verdict"),
             "reasons=%s" % ",".join(report.get("reason_codes") or []),
             "attacks=%s/%s regressions=%s/%s" % (
                 report.get("checks", {}).get("candidate_attacks_working"),
                 report.get("checks", {}).get("attacks_total"),
                 report.get("checks", {}).get("regressions_passed"),
                 report.get("checks", {}).get("regressions_total"))]
    if report.get("policy_violations"):
        parts.append("policy=" + "; ".join(report["policy_violations"])[:300])
    errors = extract_errors(raw_output)
    if errors:
        parts.append("diagnostics=" + " | ".join(errors)[:500])
    return "; ".join(parts)


class Agent:
    """Model-agnostic agent loop; the chat transport is injectable for tests."""

    def __init__(self, client, workdir, judge, out_dir,
                 max_turns=40, max_seconds=14400, max_tokens=4_000_000,
                 command_timeout=600, log=None):
        self.client = client
        self.workdir = workdir
        self.judge = judge
        self.out_dir = Path(out_dir)
        self.max_turns = max_turns
        self.max_seconds = max_seconds
        self.max_tokens = max_tokens
        self.command_timeout = command_timeout
        self.log = log or (lambda event: None)
        self.transcript = []
        self.tokens = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        self.tokens.update(cached_tokens=0, cache_usage_missing=0, usage_missing=0)
        self.attempts = []
        self.done_called = False
        self.final_judge = None
        self.done_summaries = []
        self.api_calls = 0
        self.stop_reason = None

    def _record_tokens(self, usage):
        for key in self.tokens:
            self.tokens[key] += usage.get(key, 0) or 0

    def _tool_message(self, call_id, content):
        return {"role": "tool", "tool_call_id": call_id, "content": content}

    def _run_judge(self, final=False):
        report, raw, seconds, code = self.judge.judge()
        entry = {
            "type": "final_judge" if final else "judge",
            "ts": utc_now(),
            "attempt": None if final else len(self.attempts) + 1,
            "seconds": round(seconds, 3),
            "judge_exit": code,
            "verdict": (report or {}).get("verdict"),
            "reason_codes": (report or {}).get("reason_codes"),
            "checks": (report or {}).get("checks"),
            "proof": (report or {}).get("proof"),
            "policy_violations": (report or {}).get("policy_violations"),
            "summary": summarize_report(report, raw),
            "turn": self.api_calls,
            "tokens": dict(self.tokens),
        }
        if final:
            self.final_judge = entry
        else:
            self.attempts.append(entry)
        self.log(entry)
        return report, entry

    def run(self, instruction_text):
        messages = [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": instruction_text},
        ]
        deadline = time.monotonic() + self.max_seconds
        final_report = None
        while True:
            if self.api_calls >= self.max_turns:
                self.stop_reason = "max_turns"
                break
            if time.monotonic() > deadline:
                self.stop_reason = "max_seconds"
                break
            if self.tokens["total_tokens"] > self.max_tokens:
                self.stop_reason = "max_tokens"
                break
            self.api_calls += 1
            try:
                response = self.client.chat(messages, TOOLS)
            except Exception as error:  # transport failures end the run
                self.stop_reason = "api_error: " + redact(str(error))
                break
            self._record_tokens(usage_of(response))
            message = (response.get("choices") or [{}])[0].get("message") or {}
            transcript_msg = {"role": "assistant",
                              "content": message.get("content"),
                              "tool_calls": message.get("tool_calls")}
            messages.append({k: v for k, v in transcript_msg.items() if v})
            self.transcript.append(redact(json.dumps(transcript_msg, ensure_ascii=False)))
            calls = message.get("tool_calls") or []
            if not calls:
                messages.append({"role": "user", "content": NUDGE})
                continue
            accepted = False
            hint = False
            for call in calls:
                function = call.get("function") or {}
                name = function.get("name")
                try:
                    args = json.loads(function.get("arguments") or "{}")
                except json.JSONDecodeError:
                    args = {}
                if name == "bash":
                    command = str(args.get("command", ""))
                    result = run_bash(command,
                                      self.workdir, self.command_timeout)
                    result = redact(result)
                    hint = hint or completion_hint(command, result)
                elif name == "done":
                    self.done_called = True
                    self.done_summaries.append(str(args.get("summary", "")))
                    report, entry = self._run_judge()
                    final_report = report
                    if report and report.get("verdict") == "accept":
                        accepted = True
                        result = "verdict=accept; the candidate was accepted."
                    else:
                        result = ("verdict is not accept; keep fixing.\n"
                                  + entry["summary"])
                else:
                    result = "unknown tool: %s" % name
                messages.append(self._tool_message(call.get("id", ""), result))
                self.transcript.append(redact(json.dumps(
                    {"role": "tool", "name": name, "content": result},
                    ensure_ascii=False)))
                if accepted:
                    break
            if accepted:
                self.stop_reason = "accepted"
                break
            if hint:
                messages.append({"role": "user", "content": DONE_NUDGE})
        if self.stop_reason == "max_turns" and not self.done_called:
            final_report, _ = self._run_judge(final=True)
        self.log({
            "type": "end", "ts": utc_now(),
            "stop_reason": self.stop_reason,
            "turns": self.api_calls,
            "tokens": dict(self.tokens),
            "attempts": len(self.attempts),
            "done_called": self.done_called,
            "final_judge": self.final_judge,
        })
        return final_report


def candidate_paths(workdir, subject, version):
    if subject == "spend":
        base = Path(workdir) / "lean" / "targets" / "spend" / version
    else:
        base = Path(workdir) / "lean" / version
    return base / "Target.lean", base / "Proof.lean"


def write_record(path, *, model, subject, version, workdir, judge_command,
                 attempts, report, verify, tokens, seconds, done_summaries,
                 stop_reason, final_judge=None, cost=None):
    """Write a docs/fixer/-style Markdown record."""
    target, proof = candidate_paths(workdir, subject, version)
    target_text = target.read_text() if target.is_file() else "(not found)"
    proof_text = proof.read_text() if proof.is_file() else "(not found)"
    checks = (report or {}).get("checks", {})
    verdict = (report or {}).get("verdict")
    reasons = (report or {}).get("reason_codes") or []
    lines = [
        "# NEAR AI Cloud (%s): blind %s repair" % (model, subject),
        "",
        "## English summary",
        "",
        "**simulated environment, synthetic data**. A NEAR AI Cloud model "
        "(`%s`, OpenAI-compatible API via `%s`) worked as the repair agent "
        "in an answer-withheld copy, edited only the candidate Target.lean "
        "and Proof.lean, and made %d done submission(s). A turn-limit "
        "fallback judgement is recorded separately below. Final verdict: `%s` (%s). This record was generated by "
        "fixers/nearai/run_fixer.py; review it before publication."
        % (model, DEFAULT_BASE_URL, len(attempts), verdict,
           ", ".join(reasons) if reasons else "no reason codes"),
        "",
        "# Record (NEAR AI Cloud / %s)" % model,
        "",
        "Record of a **simulated environment, synthetic data**.",
        "",
        "## Final result",
        "",
        "- verdict: `%s`; reason codes: `%s`" % (verdict, reasons),
        "- Attacks: candidate %s/%s working" % (checks.get("candidate_attacks_working"),
                                          checks.get("attacks_total")),
        "- Regressions: %s/%s passed" % (checks.get("regressions_passed"),
                                checks.get("regressions_total")),
        "- Proof: %s" % ((report or {}).get("proof")),
        "- Judge command: `%s`" % " ".join(judge_command),
        "- Bundle replay: %s" % (verify or "not run"),
        "- Stop reason: `%s`" % stop_reason,
        "- done_called: `%s`" % bool(done_summaries),
        "- final_judge (tool fallback, separate from done attempts): `%s`" % final_judge,
        "- Estimated cost (USD; upper_bound is before cache discounts): `%s`" % cost,
        "",
        "## Attempt history",
        "",
        "%d judgement(s)." % len(attempts),
        "",
    ]
    for entry in attempts:
        lines.append("Attempt %d: `%s` (%s, %.1f seconds, turn %s, cumulative tokens %s)"
                     % (entry["attempt"], entry["verdict"],
                        entry["reason_codes"], entry["seconds"],
                        entry["turn"], entry["tokens"]["total_tokens"]))
        lines.append("")
        lines.append("> %s" % entry["summary"])
        lines.append("")
    lines += [
        "## Complete final files",
        "",
        "`%s`:" % target.relative_to(workdir) if target.is_file() else "Target.lean:",
        "",
        "```lean", target_text.rstrip(), "```", "",
        "`%s`:" % proof.relative_to(workdir) if proof.is_file() else "Proof.lean:",
        "",
        "```lean", proof_text.rstrip(), "```", "",
        "## Model final report",
        "",
        (done_summaries[-1] if done_summaries else "(no done summary)"),
        "",
        "## Time and tokens",
        "",
        "- Total %.1f seconds (including API round trips, bash and judging)" % seconds,
        "- Tokens: prompt %d / completion %d / total %d"
        % (tokens["prompt_tokens"], tokens["completion_tokens"],
           tokens["total_tokens"]),
        "- Judgements: %d" % len(attempts),
        "",
        "## Environmental limitations",
        "",
        "- This record is generated; repair explanations are model self-reports. "
        "The operator must review it before publication.",
        "- Judge JSON lacks Lean diagnostics; error summaries include only "
        "lines extracted from judge stderr.",
        "- External calls use NEAR AI Cloud models / chat completions. No chain record was submitted.",
    ]
    Path(path).write_text(redact("\n".join(lines)))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workdir", required=True,
                        help="answer-withheld repository copy (see make_blind_copy.sh)")
    parser.add_argument("--instruction", required=True,
                        help="task instruction file given to the model (e.g. the FX3 text)")
    parser.add_argument("--subject", choices=["path", "spend"], default="spend")
    parser.add_argument("--version", default="after_nearai",
                        help="candidate directory name the model must create")
    parser.add_argument("--base-url", default=os.environ.get("NEARAI_BASE_URL", DEFAULT_BASE_URL))
    parser.add_argument("--model", default=os.environ.get("NEARAI_MODEL", DEFAULT_MODEL))
    parser.add_argument("--out-dir", default="demo-artifacts/nearai")
    parser.add_argument("--record", default=None,
                        help="Markdown record path (default: OUT_DIR/record.md)")
    parser.add_argument("--max-turns", type=int, default=40)
    parser.add_argument("--max-seconds", type=int, default=14400)
    parser.add_argument("--max-tokens", type=int, default=4_000_000)
    parser.add_argument("--command-timeout", type=int, default=600)
    parser.add_argument("--judge-timeout", type=int, default=1200)
    parser.add_argument("--no-bundle-verify", action="store_true",
                        help="skip bundle + verify after an accept verdict")
    args = parser.parse_args(argv)

    global _secret
    _secret = os.environ.get(API_KEY_ENV, "")
    if not _secret and not args.base_url.startswith(("http://127.", "http://localhost")):
        parser.error("%s is not set" % API_KEY_ENV)

    workdir = os.path.abspath(args.workdir)
    if not Path(workdir, "judge", "Cargo.toml").is_file():
        parser.error("workdir does not look like a repository copy: " + workdir)
    if (Path(workdir, "examples/fixers").exists()
            or Path(workdir, "docs/fixer").exists()
            or any(Path(workdir, "lean").rglob("after_honest"))):
        parser.error("workdir contains reference answers; use make_blind_copy.sh first")
    instruction = Path(args.instruction).read_text()

    out_dir = Path(args.out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    jsonl_path = out_dir / "judgements.jsonl"
    transcript_path = out_dir / "transcript.jsonl"
    record_path = args.record or str(out_dir / "record.md")

    client = ChatClient(args.base_url, _secret, args.model)
    judge = Judge(workdir, args.subject, args.version, args.judge_timeout)

    jsonl = open(jsonl_path, "a")
    transcript_file = open(transcript_path, "a")

    def log(event):
        jsonl.write(redact(json.dumps(event, ensure_ascii=False)) + "\n")
        jsonl.flush()

    log({"type": "start", "ts": utc_now(), "model": args.model,
         "base_url": args.base_url, "subject": args.subject,
         "version": args.version, "workdir": workdir})

    pricing = client.model_pricing()
    log({"type": "pricing", **pricing})
    agent = Agent(client, workdir, judge, out_dir,
                  max_turns=args.max_turns, max_seconds=args.max_seconds,
                  max_tokens=args.max_tokens,
                  command_timeout=args.command_timeout, log=log)
    started = time.monotonic()
    report = agent.run(instruction)
    seconds = time.monotonic() - started
    for line in agent.transcript:
        transcript_file.write(line + "\n")
    transcript_file.close()

    cost = estimate_cost(pricing, agent.tokens)
    log({"type": "cost", **cost})
    verify = None
    verify_ok = True
    if report and report.get("verdict") == "accept" and not args.no_bundle_verify:
        bundle_path = out_dir / ("bundle-%s.json" % args.version)
        if judge.bundle(bundle_path)[0]:
            code, output, _ = judge.verify(bundle_path)
            tail = [ln for ln in output.splitlines() if ln.strip()]
            verify_ok = code == 0 and bool(tail) and tail[-1] == "VERIFIED"
            verify = tail[-1] if verify_ok else "FAILED (exit %s): %s" % (code, " | ".join(tail[-3:]))
        else:
            verify_ok = False
            verify = "bundle failed"
        log({"type": "verify", "ts": utc_now(), "result": verify})

    jsonl.close()
    write_record(record_path, model=args.model, subject=args.subject,
                 version=args.version, workdir=workdir,
                 judge_command=judge._args(), attempts=agent.attempts,
                 report=report, verify=verify, tokens=agent.tokens,
                 seconds=seconds, done_summaries=agent.done_summaries,
                 stop_reason=agent.stop_reason or "completed",
                 final_judge=agent.final_judge, cost=cost)
    print("stop_reason:", agent.stop_reason)
    print("verdict:", (report or {}).get("verdict"))
    print("attempts:", len(agent.attempts))
    print("done_called:", agent.done_called)
    print("final_judge:", (agent.final_judge or {}).get("verdict"))
    print("record:", record_path)
    print("log:", jsonl_path)
    return 0 if (report or {}).get("verdict") == "accept" and verify_ok else 1


if __name__ == "__main__":
    sys.exit(main())
