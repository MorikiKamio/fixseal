#!/usr/bin/env python3
"""Offline tests for the fixer loop and injectable API transport; no sockets."""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from io import BytesIO

sys.path.insert(0, str(Path(__file__).resolve().parent))
import run_fixer

KEY = "sk-test-SECRET-do-not-log"


def tool_call(name, args, call_id="call_1"):
    return {
        "choices": [{
            "message": {
                "role": "assistant", "content": None,
                "tool_calls": [{
                    "id": call_id, "type": "function",
                    "function": {"name": name, "arguments": json.dumps(args)},
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
    }


def text_message(content):
    return {
        "choices": [{"message": {"role": "assistant", "content": content},
                     "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
    }


ACCEPT = {
    "verdict": "accept", "reason_codes": [],
    "checks": {"attacks_total": 12, "baseline_attacks_working": 12,
               "candidate_attacks_working": 0,
               "regressions_total": 16, "regressions_passed": 16},
    "proof": {"verified": True, "approved_spec_sha256": "e3933e51"},
    "policy_violations": [],
}
REJECT = {
    "verdict": "reject", "reason_codes": ["PROOF_REJECTED"],
    "checks": {"attacks_total": 12, "baseline_attacks_working": 12,
               "candidate_attacks_working": 0,
               "regressions_total": 16, "regressions_passed": 16},
    "proof": {"verified": False, "approved_spec_sha256": "e3933e51"},
    "policy_violations": [],
}


class FakeJudge:
    def __init__(self, reports):
        self.reports = list(reports)
        self.calls = 0

    def judge(self):
        self.calls += 1
        report = self.reports.pop(0) if self.reports else ACCEPT
        raw = json.dumps(report)
        if report["verdict"] != "accept":
            raw += "\nProof.lean:6:6: error: invalid rewrite argument"
        return report, raw, 0.01, 0 if report["verdict"] == "accept" else 1


class FakeClient:
    """In-process transport; never opens sockets."""

    def __init__(self, script):
        self.script = script
        self.requests = []

    def chat(self, messages, tools):
        self.requests.append([dict(m) for m in messages])
        return self.script[len(self.requests) - 1](messages)


def make_client(script):
    client = FakeClient(script)
    return client, client.requests, []


def make_workdir():
    root = Path(tempfile.mkdtemp(prefix="nearai-test-"))
    (root / "lean" / "targets" / "spend" / "after_nearai").mkdir(parents=True)
    return root


def run_agent(script, reports, workdir, events):
    client, requests, auths = make_client(script)
    agent = run_fixer.Agent(client, str(workdir), FakeJudge(reports), workdir,
                            log=events.append)
    report = agent.run("fix the spend target")
    if hasattr(client, "_server"):
        client._server.shutdown()
        client._server.server_close()
    return agent, report, requests, auths


class RunFixerTests(unittest.TestCase):
    def setUp(self):
        run_fixer._secret = KEY
        os.environ[run_fixer.API_KEY_ENV] = KEY

    def tearDown(self):
        run_fixer._secret = ""
        os.environ.pop(run_fixer.API_KEY_ENV, None)

    def test_tool_call_round_trip(self):
        workdir = make_workdir()
        seen = {}

        def second(messages):
            seen["tool"] = messages[-1]
            return tool_call("done", {"summary": "ready"}, "call_2")

        script = [
            lambda m: tool_call("bash", {"command": "printf hello-ok > marker.txt && pwd"}),
            second,
        ]
        events = []
        agent, report, requests, _ = run_agent(script, [ACCEPT], workdir, events)
        self.assertEqual(report["verdict"], "accept")
        self.assertEqual(agent.stop_reason, "accepted")
        self.assertEqual((workdir / "marker.txt").read_text(), "hello-ok")
        self.assertEqual(seen["tool"]["role"], "tool")
        self.assertIn(os.path.realpath(workdir), seen["tool"]["content"])

    def test_reject_then_continue(self):
        workdir = make_workdir()
        seen = {}
        script = [
            lambda m: tool_call("done", {"summary": "first"}),
            lambda m: (seen.setdefault("feedback", m[-1]),
                       tool_call("done", {"summary": "second"}, "call_2"))[1],
        ]
        events = []
        agent, report, _, _ = run_agent(script, [REJECT, ACCEPT], workdir, events)
        self.assertEqual(report["verdict"], "accept")
        judges = [e for e in events if e.get("type") == "judge"]
        self.assertEqual(len(judges), 2)
        self.assertEqual(judges[0]["verdict"], "reject")
        self.assertIn("PROOF_REJECTED", seen["feedback"]["content"])

    def test_blocked_command_rejected(self):
        workdir = make_workdir()
        bad_commands = ["curl https://example.com",
                        "cat ../outside.txt",
                        "cat /etc/hosts",
                        "git clone https://x",
                        "pip install x",
                        "cd .. && ls"]
        script = [lambda m, bad=bad: tool_call("bash", {"command": bad})
                  for bad in bad_commands]
        script.append(lambda m: tool_call("done", {"summary": "ok"}, "call_done"))
        # capture the tool results as they are sent back
        captured = []
        original = script[-1]

        def capture_done(messages):
            captured.extend(m["content"] for m in messages if m.get("role") == "tool")
            return original(messages)
        script[-1] = capture_done
        agent, report, _, _ = run_agent(script, [ACCEPT], workdir, [])
        self.assertEqual(len(captured), 6)
        for content in captured:
            self.assertIn("rejected", content)
        self.assertFalse((workdir / "outside.txt").exists())

    def test_nudge_on_plain_stop(self):
        workdir = make_workdir()
        seen = {}

        def second(messages):
            seen["last"] = messages[-1]
            return tool_call("done", {"summary": "ok"}, "call_2")

        script = [lambda m: text_message("let me think"), second]
        agent, report, _, _ = run_agent(script, [ACCEPT], workdir, [])
        self.assertEqual(seen["last"]["role"], "user")
        self.assertEqual(seen["last"]["content"], run_fixer.NUDGE)

    def test_api_key_not_in_logs(self):
        workdir = make_workdir()
        (workdir / "lean/targets/spend/after_nearai/Target.lean").write_text("t")
        (workdir / "lean/targets/spend/after_nearai/Proof.lean").write_text("p")
        script = [
            lambda m: tool_call("bash", {"command": "env; printf %s \"$NEARAI_API_KEY\""}),
            lambda m: tool_call("done", {"summary": "ok"}, "call_2"),
        ]
        events = []
        agent, report, requests, auths = run_agent(script, [ACCEPT], workdir, events)
        transcript = "\n".join(agent.transcript)
        logged = transcript + "\n".join(json.dumps(e) for e in events)
        for request in requests:
            logged += json.dumps(request)
        self.assertNotIn(KEY, logged)
        for auth in auths:
            self.assertEqual(auth, "Bearer " + KEY)
        record = workdir / "record.md"
        run_fixer.write_record(
            record, model="test-model", subject="spend",
            version="after_nearai", workdir=str(workdir),
            judge_command=["judge"], attempts=agent.attempts,
            report=report, verify="VERIFIED", tokens=agent.tokens,
            seconds=1.0, done_summaries=agent.done_summaries,
            stop_reason="accepted")
        self.assertNotIn(KEY, record.read_text())

    def test_max_turns_stop(self):
        workdir = make_workdir()
        script = [lambda m: text_message("still thinking")] * 10
        client, requests, _ = make_client(script)
        agent = run_fixer.Agent(client, str(workdir), FakeJudge([]), workdir,
                                max_turns=2)
        agent.run("fix")
        if hasattr(client, "_server"):
            client._server.shutdown()
            client._server.server_close()
        self.assertEqual(agent.stop_reason, "max_turns")
        self.assertEqual(agent.api_calls, 2)
        self.assertFalse(agent.done_called)
        self.assertEqual(len(agent.attempts), 0)
        self.assertEqual(agent.final_judge["verdict"], "accept")
        self.assertEqual(agent.judge.calls, 1)

    def test_completion_nudges(self):
        for command, output in [
            ("cargo run -- judge", 'exit 0\n' + json.dumps(ACCEPT)),
            ("cargo run -- judge", "exit 0\nspend-3 accept []"),
            ("cargo run -- judge", "exit 0\nACCEPT  []  attacks 0/12  regressions 16/16"),
            ("cat > docs/fixer/NX1.md", "exit 0\n"),
            ("python3 -c 'Path(\"docs/fixer/NX1.md\").write_text(\"ready\")'", "exit 0\n"),
        ]:
            with self.subTest(command=command), patch.object(run_fixer, "run_bash", return_value=output):
                client = FakeClient([
                    lambda m: tool_call("bash", {"command": command}),
                    lambda m: tool_call("done", {"summary": "ready"}),
                ])
                agent = run_fixer.Agent(client, make_workdir(), FakeJudge([ACCEPT]), make_workdir())
                agent.run("fix")
                self.assertEqual(client.requests[1][-1]["content"], run_fixer.DONE_NUDGE)
                self.assertTrue(agent.done_called)
                self.assertIsNone(agent.final_judge)
                self.assertEqual(agent.judge.calls, 1)
        self.assertFalse(run_fixer.completion_hint("cat docs/fixer/NX1.md", "exit 0"))
        self.assertFalse(run_fixer.completion_hint("cat > docs/fixer/NX1.md", "rejected: path"))

    def test_fallback_rejection_and_timeout_stay_visible(self):
        for result in [(REJECT, "rejected", 0.1, 1), (None, "timed out", 0.1, None)]:
            judge = FakeJudge([])
            with patch.object(judge, "judge", return_value=result) as execute:
                agent = run_fixer.Agent(FakeClient([lambda m: text_message("waiting")]),
                                        make_workdir(), judge, make_workdir(), max_turns=1)
                report = agent.run("fix")
                execute.assert_called_once()
                self.assertEqual(report, result[0])
                self.assertEqual(agent.final_judge["judge_exit"], result[3])
                self.assertEqual(agent.stop_reason, "max_turns")

    def test_done_reject_has_no_fallback(self):
        judge = FakeJudge([REJECT])
        agent = run_fixer.Agent(FakeClient([lambda m: tool_call("done", {})]),
                                make_workdir(), judge, make_workdir(), max_turns=1)
        self.assertEqual(agent.run("fix"), REJECT)
        self.assertTrue(agent.done_called)
        self.assertIsNone(agent.final_judge)
        self.assertEqual(judge.calls, 1)

    def test_models_transport_and_cost(self):
        model = {"id": "test-model", "pricing": {"prompt": "0.000001", "completion": "0.000002", "input_cache_read": "0.0000001"}}
        client = run_fixer.ChatClient("https://unused.invalid/v1", KEY, "test-model")
        with patch.object(run_fixer.urllib.request, "urlopen", return_value=BytesIO(json.dumps({"data": [model]}).encode())) as fetch:
            metadata = client.model_pricing()
        self.assertEqual(fetch.call_args.args[0].full_url, "https://unused.invalid/v1/models")
        usage = run_fixer.usage_of({"usage": {"prompt_tokens": 100, "completion_tokens": 10}})
        cost = run_fixer.estimate_cost(metadata, usage)
        self.assertEqual(cost["kind"], "upper_bound")
        self.assertAlmostEqual(float(cost["usd"]), 0.00012)
        usage.update(cached_tokens=60, cache_usage_missing=0)
        cost = run_fixer.estimate_cost(metadata, usage)
        self.assertEqual(cost["kind"], "estimate")
        self.assertAlmostEqual(float(cost["usd"]), 0.000066)
        metadata["pricing"] = {"input": "1", "output": "2", "cache_read": "0.1", "unit": "usd_per_million_tokens"}
        self.assertAlmostEqual(float(run_fixer.estimate_cost(metadata, usage)["usd"]), 0.000066)
        del metadata["pricing"]["unit"]
        self.assertEqual(run_fixer.estimate_cost(metadata, usage)["kind"], "unavailable")
        with patch.object(run_fixer.urllib.request, "urlopen", side_effect=OSError(KEY)):
            self.assertNotIn(KEY, json.dumps(client.model_pricing()))

    def test_chat_transport_without_sockets(self):
        response = tool_call("done", {"summary": "ready"})
        client = run_fixer.ChatClient("https://unused.invalid/v1", KEY, "test-model")
        with patch.object(run_fixer.urllib.request, "urlopen", return_value=BytesIO(json.dumps(response).encode())) as fetch:
            actual = client.chat([{"role": "user", "content": "fix"}], run_fixer.TOOLS)
        self.assertEqual(actual, response)
        request = fetch.call_args.args[0]
        self.assertEqual(request.full_url, "https://unused.invalid/v1/chat/completions")
        self.assertEqual(request.get_method(), "POST")
        self.assertEqual(json.loads(request.data)["model"], "test-model")
        self.assertEqual(request.get_header("Authorization"), "Bearer " + KEY)

    def test_mixed_usage_never_assumes_cache_discount(self):
        agent = run_fixer.Agent(None, make_workdir(), FakeJudge([]), make_workdir())
        agent._record_tokens(run_fixer.usage_of({"usage": {"prompt_tokens": 100, "completion_tokens": 10, "prompt_tokens_details": {"cached_tokens": 60}}}))
        agent._record_tokens(run_fixer.usage_of({"usage": {"prompt_tokens": 100, "completion_tokens": 10}}))
        cost = run_fixer.estimate_cost({"pricing": {"prompt": "1", "completion": "2", "input_cache_read": "0.1"}}, agent.tokens)
        self.assertEqual(cost["kind"], "upper_bound")
        self.assertEqual(float(cost["usd"]), 240)
        agent._record_tokens(run_fixer.usage_of({}))
        self.assertIsNone(run_fixer.estimate_cost(cost["source"], agent.tokens)["usd"])

    def test_main_writes_fallback_cost_and_verified_record(self):
        workdir = make_workdir()
        (workdir / "judge").mkdir()
        (workdir / "judge/Cargo.toml").write_text("")
        instruction = workdir / "instruction.md"
        instruction.write_text("fix")
        client = FakeClient([lambda m: text_message("ready")])
        client.model_pricing = lambda: {"pricing": {"prompt": "0.1", "completion": "0.2"}}
        judge = FakeJudge([ACCEPT])
        judge.bundle = lambda path: (Path(path).write_text("{}") or True, 0.1)
        judge.verify = lambda path: (0, "VERIFIED\n", 0.1)
        judge._args = lambda: ["judge"]
        out = workdir / "out"
        with patch.object(run_fixer, "ChatClient", return_value=client), patch.object(run_fixer, "Judge", return_value=judge):
            code = run_fixer.main(["--workdir", str(workdir), "--instruction", str(instruction), "--out-dir", str(out), "--max-turns", "1"])
        self.assertEqual(code, 0)
        events = [json.loads(line) for line in (out / "judgements.jsonl").read_text().splitlines()]
        end = next(event for event in events if event["type"] == "end")
        self.assertFalse(end["done_called"])
        self.assertEqual(end["attempts"], 0)
        self.assertEqual(end["final_judge"]["verdict"], "accept")
        self.assertEqual(events[-1]["result"], "VERIFIED")
        self.assertIn("upper_bound", (out / "record.md").read_text())
        judge.verify = lambda path: (1, "VERIFICATION FAILED\n", 0.1)
        client.requests.clear()
        with patch.object(run_fixer, "ChatClient", return_value=client), patch.object(run_fixer, "Judge", return_value=judge):
            code = run_fixer.main(["--workdir", str(workdir), "--instruction", str(instruction), "--out-dir", str(out), "--max-turns", "1"])
        self.assertEqual(code, 1)
        self.assertIn("FAILED (exit 1)", (out / "record.md").read_text())

    def test_reference_answers_rejected_before_api(self):
        workdir = make_workdir()
        (workdir / "judge").mkdir()
        (workdir / "judge/Cargo.toml").write_text("")
        (workdir / "lean/after_honest").mkdir()
        with patch.object(run_fixer, "ChatClient") as client, patch("sys.stderr"):
            with self.assertRaises(SystemExit) as error:
                run_fixer.main(["--workdir", str(workdir), "--instruction", "unused"])
        self.assertEqual(error.exception.code, 2)
        client.assert_not_called()


if __name__ == "__main__":
    unittest.main()
