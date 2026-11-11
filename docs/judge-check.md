# One-command review

From the repository root, run `scripts/judge-check.sh --stage a` to inspect saved
evidence without Rust, Lean or network access. Run `scripts/judge-check.sh` for
all locally available stages. Nothing is installed, signed or submitted. All
examples use a **simulated environment, synthetic data**.

Each check prints one `PASS ID`, `FAIL ID`, or `SKIP(reason) ID` line, followed by
a summary and elapsed time. Detailed stdout/stderr stays in a fresh ignored
`demo-artifacts/judge-check/run-*` directory; prior logs are not overwritten.
Exit 0 means no executed check failed, including a run with skips. Exit 1 means
at least one check failed; invalid options exit 2. Inspect the skips before
claiming that the whole evaluation ran. The [README table](../README.md#full-evaluation--do-x-see-y)
maps IDs to claims and prerequisites.

## Stages and interpretation

- **A1:** SHA-256 of the 16 packaged evidence files: two bundles, two fixed specs,
  two historical host logs, two provenance notes and eight archived Lean files.
  The manifest authenticates neither the author nor the origin of the files.
- **A2:** Python independently checks the bundle fields, byte arrays, per-file
  hashes, length-framed combined-input hash, report serialization/hash and
  exported-expression JSON/hash. It does not execute Lean, interpret the policy
  or establish that an asserted proof status is true. C4 reruns those bundles.
- **A3:** disabled by default. With separate external-network permission,
  populate the two judge-check fields in [testnet evidence](testnet-evidence.md)
  and use `--stage a --allow-testnet-read`. Only final-state `guard_state` and
  `get_fix_proof` view queries go to the fixed testnet RPC. Empty fields, missing
  curl or transport unavailability skip; malformed responses, RPC errors,
  missing records and inconsistent policy data fail. No account creation,
  signer or transaction method exists in this check.
- **B1–B3:** offline Cargo tests. B1 selects Rust-only judge/lib/timeout suites;
  B2 explicitly selects shared interpreter tests; B3 runs contract lib, guard,
  proof-record and local-demo summary tests. The two live local-demo tests are
  ignored by their existing opt-in annotations. B4 explicitly records that
  omission; passing B3 is not evidence of a payment on-chain.
- **C1–C5:** the complete judge test command, both Lake builds and path spec
  examples, 12 fixture evaluations (both baselines included), fresh bundles and
  verification plus both saved bundles, then four archived AI attempts. C3 checks
  expected verdict, reasons and proof status rather than treating bundle export
  exit 0 as acceptance. `VERIFIED` can also reproduce a rejection.

Each stage can be selected using `--stage a|b|c|all`. Missing Rust or real
Lean/Lake 4.34.1 binaries produce explicit skips. The script never invokes an
elan shim to download a toolchain; when Lean is missing it prints one operator
installation command without executing it. For installed nonstandard paths, set
`FIXSEAL_LEAN_BIN` to the real Lean `bin`, and place Rust/Cargo 1.99.1 on PATH.
Locked caches and a native C compiler are needed for B/C. Existing test failures
are failures, including missing cached dependencies. A timeout remains
`PROOF_TIMEOUT`; use matching positive `FIXSEAL_LEAN_TIMEOUT_SECS` settings for
generation/replay (default 120 seconds per Lean subprocess).

## Measured time

Local macOS arm64, 2026-11-10, Rust/Cargo 1.99.1, Lean 4.34.1, existing dependency
and build caches. These are wall-clock measurements, not cold-build or install
estimates. All results are **simulated environment, synthetic data**.

| Check | Seconds | Observed result |
| --- | ---: | --- |
| Standalone stage A including reporting | 0.113 | 2 PASS, 1 SKIP |
| A1 / A2 in full run | 0.038 / 0.040 | 16 evidence files / two bundles |
| B1 | 10.034 | 25 tests pass, no Lean required |
| B2 | 0.036 | 2 interpreter tests pass (also included in B1) |
| B3 | 1.372 | 71 tests pass, 2 opt-in chain tests ignored |
| C1 | 178.744 | Complete judge suite: 72 tests pass |
| C2 | 0.596 | Both builds and SpecTests pass |
| C3 | 55.855 | All 12 versions retain expected verdict/reasons |
| C4 | 39.056 | Two fresh bundles and two packaged replays pass |
| C5 | 10.850 | Both archive tests pass, four attempts reproduced |
| Full script | 296.814 | 10 PASS, 0 FAIL, 2 SKIP (A3/B4) |
| Docker build / execution | Unmeasured | Not run; no image fetched or built |
| GitHub CI (Ubuntu 24.04) | judge 11m25s; contract-sandbox 10m27s | [Run 42805855129](https://github.com/MorikiKamio/fixseal/actions/runs/42805855129), 2026-11-10 |
| Testnet views (A3, `--allow-testnet-read`) | 6.8 s | PASS against `fixseal-guard.testnet`, 2026-11-10 ([run](testnet-run-2026-11-10.md)) |

Honest path: attacks 0/11, regressions 11/11. Honest spend: attacks 0/12,
regressions 16/16, fixed Lean/interpreter comparison on 1,028 inputs (including
1,000 boundary triples), accept / [] / proof=true. The kernel checked a proof
against the fixed specification and fixed tests were rerun. The exporter and
interpreter are trusted; fixed-input comparison is a check, not a proof.

## Docker option

The root Dockerfile is the service template. The separate
[review Dockerfile](../docker/judge.Dockerfile) pins Rust 1.99.1 and Lean 4.34.1.
Initial provisioning downloads toolchains/system packages and locked Rust
dependencies, so run the build only on a host with separate installation and
network authorization. It was not built or run during this offline task:

```sh
docker build -f docker/judge.Dockerfile -t fixseal-judge:rust1.99.1-lean4.34.1 .
scripts/judge-check.sh --docker
```

The second command never builds or pulls an image. It first runs A locally,
then D1 invokes the prebuilt image with `--pull=never --network=none`, a read-only
filesystem/source mount, writable temporary work directories and a results-only
log mount. The container
copies current mounted sources into scratch space and runs the same script for
B then C; cached registry files are copied to writable scratch space too. It
does not run the sandbox. Nested B/C summaries are printed too; their detailed
logs are retained under the host run's `container/` subdirectory. Docker
absence skips D1; an unavailable engine/image or failed container is a failure.
Rust/Lean versions are pinned; system package versions and base-image digest are
not pinned, so this is not a claim of a bit-reproducible toolchain image.

## CI and live demo

[The workflow](../.github/workflows/ci.yml) provisions dependencies on Ubuntu,
runs `scripts/judge-check.sh` unchanged, and retains its logs as an artifact.
A separate Linux job builds wasm from current source, provisions sandbox 2.11.0,
then runs contract integration and `scripts/demo.sh` (both subjects and guard).
The jobs fail on build/runtime/receipt errors; no skipped chain run is counted
as success. CI provisioning uses external downloads when a future operator runs
the workflow; none were performed here.

**CI has not yet run on GitHub.** First public run URL: **<to be filled>**. Local arm64
checks do not establish Linux or macOS CI success; no macOS CI job is configured.
Current guard wasm and chain effects still need [T010 host checks](t010-host-check.md).
The older FIX5 logs predate the guard. This script never signs, publishes or submits; with
`--allow-testnet-read` it only reads public testnet views ([run](testnet-run-2026-11-10.md)).

## T013 additions

B5 runs `python3 fixers/nearai/test_run_fixer.py` using only in-process fake
responses and temporary synthetic copies. It requires Python 3, no Rust/Lean,
API key, sockets or external traffic. C5 now runs three tests: four FX3 attempts
(first proofs reconstructed) and the byte-identical NX1 final source, under
current spend-4. NX1's original spend-3 shell accept and operator rejudge are
documented separately in [the curated record](fixer/NX1-nearai.md).

Interactive judge/bundle commands display a plain verdict summary on stderr;
use leading `--summary` to capture that line. Noninteractive JSON callers retain
the previous stdout and stderr behavior without the flag. No ANSI color is used.
The T011 timings above are historical; T013 measurements are appended after
local verification, without substituting them for CI or chain evidence.

### Local T013 measurement (2026-11-10)

**simulated environment, synthetic data**; macOS arm64, installed tools and warm
caches. No network, chain, Docker or CI run. Timings exclude initial provisioning.

| Check | Seconds | Result |
| --- | --- | --- |
| Full A/B/C run | 322.893 | 11 PASS, 0 FAIL, 2 SKIP (A3/B4) |
| C1 judge suite | 194.131 | 74 tests pass |
| C2 both Lean builds / path SpecTests | 0.615 | Pass |
| C3 all twelve fixture verdicts | 62.665 | Match |
| C4 fresh and packaged bundle replay | 40.933 | Four VERIFIED results |
| C5 archived AI repair replay | 13.083 | Three tests, five source pairs pass |
| Final stage B after fixer updates | 10.878 | 4 PASS, 0 FAIL, 1 SKIP; B5 runs 14 tests |
| Final A integrity check | 0.132 | 2 PASS, 0 FAIL, 1 SKIP; manifest covers 19 files |
| Fresh NX1 final bundle + verify | 16.041 | Accept / proof=true / VERIFIED, 0/12 attacks, 16/16 regressions |

The fresh NX1 measurement ran alongside fixture checks and performs 1,028 fixed
export comparisons. It is separate from historical NX1's spend-3 run and its
1,022-second API session. See [NX1 provenance and completion caveat](fixer/NX1-nearai.md).
The final fixer tests use mocked GET/POST responses and open no sockets.

After this offline run, the operator added a [current host result](t010-host-check.md#host-result-operator-2026-11-10-1420-jst) reporting a source-built wasm and successful guard sandbox run. Local raw logs also show ten path/spend matches/replays and a guard pass. This does not change B4's deliberate SKIP in the offline script; no chain call was made by this task. Raw host logs were not added to the public export.
