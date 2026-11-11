# fixers/nearai

A small Python 3 standard-library tool that runs a NEAR AI Cloud model as a Fixseal repair agent and records its patch, Lean proof and judge result. The operator runs the API with a key and separate network permission. Never use the development copy or public export directly as workdir: copies containing reference answers are rejected before execution.

## Usage (3 commands)

```sh
# 1. Create an answer-withheld copy, including a proof leak check
fixers/nearai/make_blind_copy.sh /path/to/blind-copy

# 2. Set the API key and run from the repository root
export NEARAI_API_KEY=sk-...
python3 fixers/nearai/run_fixer.py \
    --workdir /path/to/blind-copy \
    --instruction docs/tasks/<fixer-instruction-file>.md \
    --subject spend --version after_nearai \
    --out-dir demo-artifacts/nearai

# 3. Inspect results
cat demo-artifacts/nearai/judgements.jsonl   # Per-attempt judgements
cat demo-artifacts/nearai/record.md          # Record for docs/fixer/
```

- `--instruction`: operator-provided task text matching FX3 in `docs/tasks/`. Match `--version` to the candidate directory named in the instruction; the model creates that copy.
- Prerequisites: a working judge in the copy, Rust/Cargo, Lean 4.34.1, dependency caches and, if needed, `FIXSEAL_LEAN_BIN`. Judge command inside the copy: `cargo run --manifest-path judge/Cargo.toml --locked --offline --` followed by `--lean [--subject spend] lean VERSION`.
- Model tools are only `bash` and `done`. Bash runs inside workdir; network commands (curl, wget, git clone, pip, npm, lake update, etc.) reject before execution. Outside paths, home-relative paths and variable expansion in paths also reject. Commands have a default 600-second timeout per 1 command and truncated output.
- Each `done` call runs the judge 1 time. Rejection returns verdict, reasons and extracted diagnostics for another attempt. After detecting accept via bash or a docs/fixer/ record write, the next turn explicitly requests `done`; this is supplemental output/command detection. A model stopping without tools receives a 1-line reminder. `--max-turns` / `--max-seconds` / cumulative `--max-tokens` stop execution.
- If the turn cap arrives without done, the tool runs the judge 1 time and records `final_judge`. `done_called` and done-only `attempts` remain separate; max_turns remains the stop reason even on accept.
- On accept, bundle export and `verify` run automatically (disable with `--no-bundle-verify`) and record whether replay is VERIFIED.
- `NEARAI_API_KEY` is excluded from logs, records, exception text and subprocess environments.

## Main options

| Option | Default | Meaning |
| --- | --- | --- |
| `--base-url` | `https://cloud-api.near.ai/v1` (`NEARAI_BASE_URL`) | Gateway checked against docs.near.ai |
| `--model` | `zai-org/GLM-5.1-FP8` (`NEARAI_MODEL`) | Configurable quickstart example |
| `--subject` | `spend` | `path` or `spend` |
| `--max-turns` | 40 | Maximum API calls |
| `--max-seconds` | 14400 | Overall wall-clock limit |
| `--max-tokens` | 4000000 | Cumulative usage limit |
| `--command-timeout` | 600 | Seconds per 1 bash command |
| `--judge-timeout` | 1200 | Seconds per 1 judgement |

## Cost estimates

At startup, fetch `/models` (default `/v1/models`) from the same base URL and retain selected-model pricing, retrieval time and source. Accumulate usage; at completion, write USD `cost` estimates and the pricing/usage used into JSON Lines and the record. NX1 historical runs lack pricing snapshots; do not invent retrospective costs.

Supported pricing formats:

- `prompt` / `completion` (USD/token), optional `input_cache_read`.
- `input` / `output` with explicit `unit` (`usd_per_token` or `usd_per_million_tokens`), optional `cache_read`.

Apply cache discounts only when every response contains `prompt_tokens_details.cached_tokens` and a cache rate exists. Otherwise use the normal input rate as `upper_bound`, before cache discounts. Estimates are not invoices. Pricing failures, unsupported formats/units or missing usage produce `unavailable`, not 0 dollars. Implementation tests call no API; live-service pricing formats remain unconfirmed.

## Limits

- The bash denylist assists cooperative agents; it is not a complete sandbox. Obfuscated networking or shell expansion may escape it. The operator must choose the execution environment.
- Judge JSON lacks Lean diagnostics; recorded errors are limited to stderr `error` lines. The model can compile locally to inspect diagnostics, as in FX3.
- `record.md` is generated with FX3-style fields. Repair explanations are model self-reports requiring operator review before publication.
- Only non-streaming chat completions are implemented. `reasoning_content` is unused in records; Responses API and model-specific endpoints are unsupported.
- Time/token/API-error stops do not trigger automatic rejudging. Distinguish turn-cap `final_judge`, `done_called` and `stop_reason`.
- After NX1, bundle-path construction and verify-log close ordering were fixed. Verify failure is recorded and returns a failure exit code.

## Tests

```sh
python3 fixers/nearai/test_run_fixer.py
```

Tests use in-process fake clients and replaced HTTP responses from the standard library, without API calls or localhost bind. They cover judge fallback, reminders, discounted/upper-bound costs, key redaction and bundle/verify records. Results describe a **simulated environment, synthetic data**.
