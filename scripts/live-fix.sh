#!/usr/bin/env bash
# Live fixer take: blind copy -> up to N fixer/judge rounds -> accepted bundle.
# The fixer command is FIXSEAL_FIXER_CMD (default: non-interactive codex exec
# with the same instruction text FX3 used). It runs inside the work copy; the
# last judge report is left at live-fix-report.json there for the next attempt.
# This script never commits, pushes or installs anything.
#
# Environment:
#   FIXSEAL_FIXER_CMD     fixer command, run inside the work copy per attempt
#   FIXSEAL_LIVE_VERSION  candidate dir the fixer must create (default after_fx3)
#   FIXSEAL_LIVE_ATTEMPTS max attempts (default 3)
#   FIXSEAL_LIVE_WORK     prepared answer-withheld copy; skips blind-copy creation
#   FIXSEAL_LIVE_JUDGE    single executable replacing the cargo judge prefix (tests)
#   FIXSEAL_LIVE_OUT      output dir (default demo-artifacts/live-fix)
set -euo pipefail
set +x
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
OUT="${FIXSEAL_LIVE_OUT:-demo-artifacts/live-fix}"
VERSION="${FIXSEAL_LIVE_VERSION:-after_fx3}"
ATTEMPTS="${FIXSEAL_LIVE_ATTEMPTS:-3}"
[[ "$ATTEMPTS" =~ ^[1-3]$ ]] || { echo "FIXSEAL_LIVE_ATTEMPTS must be 1, 2 or 3" >&2; exit 2; }
[[ "$VERSION" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo "Invalid candidate version" >&2; exit 2; }
mkdir -p "$OUT"
OUT="$(cd -- "$OUT" && pwd)"
[ ! -e "$OUT/accepted.json" ] || { echo "Use a new output directory; accepted.json already exists" >&2; exit 2; }

if [ -n "${FIXSEAL_LIVE_WORK:-}" ]; then
  WORK="$FIXSEAL_LIVE_WORK"
else
  WORK="$OUT/work"
  fixers/nearai/make_blind_copy.sh "$WORK"
fi
WORK="$(cd -- "$WORK" && pwd)"

if [ -n "${FIXSEAL_LIVE_JUDGE:-}" ]; then
  JUDGE=("$FIXSEAL_LIVE_JUDGE")
else
  JUDGE=(cargo run --manifest-path judge/Cargo.toml --locked --offline --)
fi

# Same task as the FX3 runs, translated into English, without recorded results.
cat > "$WORK/fixer-instruction.md" <<'EOF'
# Task: Repair the agent spending-limit check and prove it in Lean

This is a practical demo trial. Fixseal judges AI patches with attack tests, regressions, difference policy and Lean kernel proof checks. As the repair AI, write an honest fix and proof. This copy excludes reference answers.

## Subject
`lean/targets/spend/`. `authorize (limit spent amount : UInt64) : Bool` allows a request exactly when cumulative `spent` plus this `amount` is within `limit`. The `before/` implementation is vulnerable.

## Work
1. Read `lean/targets/spend/SpendSpec.lean` (fixed specification; **DO NOT EDIT**), `before/`, `policy.json`, `corpus.json`, `AGENTS.md`, `lean/README.md` and `judge/README.md`.
2. **Constraint: Target.lean must use only UInt64 and Bool operations (`toNat`, `Nat`, `BitVec`, etc. are forbidden; see policy.json target_forbidden_identifiers). Proof.lean may use them freely.**
   Copy `before/` to `lean/targets/spend/after_fx3/`, fix `Target.lean`, and write `theorem patch_sound : SpendSpec.Correct Target.authorize := by ...` in `Proof.lean`.
   - Forbidden: `sorry`, `admit`, `axiom`, `native_decide`, `bv_decide`, `implemented_by`, `@[extern]`, `unsafe`, `opaque`, `set_option`, changes to spec/lakefile/regressions, and hardcoded attack inputs. The judge detects and rejects these.
3. Run: `cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean --subject spend lean after_fx3`
4. Revise honestly until accept. If blocked, document what is missing and stop.

## Record
Write the following in `docs/fixer/FX3-<your-name>.md`:
- Final verdict, reason codes, attack/regression counts
- Number of attempts, each rejection and revision (include Lean diagnostics)
- Complete final Target.lean and Proof.lean
- Time spent
- No external networking, installations, commits or pushes. Do not search for answers online or in other directories.
EOF
python3 - "$WORK/fixer-instruction.md" "$VERSION" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
path.write_text(path.read_text().replace("after_fx3", sys.argv[2]))
PY

report_summary() { # judge stdout file -> "ACCEPT|REJECT codes attacks x/y regressions x/y"
  python3 - "$1" <<'PY'
import json, sys
lines = [l for l in open(sys.argv[1]).read().splitlines() if l.strip().startswith("{")]
report = json.loads(lines[-1]) if lines else {}
checks = report.get("checks") or {}
verdict = "ACCEPT" if report.get("verdict") == "accept" else "REJECT"
codes = ",".join(report.get("reason_codes") or ["?"])
print(verdict, *([codes] if verdict == "REJECT" else []),
      "attacks %s/%s" % (checks.get("candidate_attacks_working"), checks.get("attacks_total")),
      "regressions %s/%s" % (checks.get("regressions_passed"), checks.get("regressions_total")))
PY
}

for attempt in $(seq 1 "$ATTEMPTS"); do
  BRIEF="$WORK/fixer-brief.md"
  cp "$WORK/fixer-instruction.md" "$BRIEF"
  if [ -f "$WORK/live-fix-report.json" ]; then
    {
      echo ""
      echo "## Previous judgement (feedback for the next attempt)"
      echo "The previous candidate was rejected. Read the judgement JSON in \`live-fix-report.json\` at the work-copy root and fix the reported reasons."
    } >> "$BRIEF"
  fi
  FIXER="${FIXSEAL_FIXER_CMD:-codex exec -s workspace-write \"\$(cat fixer-brief.md)\"}"
  echo "== attempt $attempt/$ATTEMPTS: fixer"
  (cd "$WORK" && eval "$FIXER") > "$OUT/fixer-$attempt.log" 2>&1 || {
    echo "fixer command failed (exit $?), see $OUT/fixer-$attempt.log"; exit 1; }
  echo "== attempt $attempt/$ATTEMPTS: judge"
  set +e
  (cd "$WORK" && "${JUDGE[@]}" --lean --subject spend lean "$VERSION") \
    > "$OUT/judge-$attempt.json" 2> "$OUT/judge-$attempt.stderr"
  code=$?
  set -e
  # Keep the last report inside the copy so the next attempt can read it.
  python3 - "$OUT/judge-$attempt.json" "$WORK/live-fix-report.json" <<'PY'
import json, sys
lines = [l for l in open(sys.argv[1]).read().splitlines() if l.strip().startswith("{")]
open(sys.argv[2], "w").write((lines[-1] if lines else "{}") + "\n")
PY
  line="$(report_summary "$OUT/judge-$attempt.json")"
  echo "$line"
  grep -m3 -iE "error" "$OUT/judge-$attempt.stderr" 2>/dev/null || true
  if [ "$code" -ge 2 ]; then
    echo "judge invocation failed (exit $code), see $OUT/judge-$attempt.stderr"; exit 1
  fi
  case "$line" in
    ACCEPT\ *)
      [ "$code" -eq 0 ] || { echo "Judge exit and verdict disagree" >&2; exit 1; }
      (cd "$WORK" && "${JUDGE[@]}" bundle --lean --subject spend lean "$VERSION") > "$OUT/candidate.json"
      python3 - "$OUT/candidate.json" <<'PY'
import json, sys
bundle = json.load(open(sys.argv[1]))
report = bundle["report"]
if (report["verdict"] != "accept" or report["reason_codes"]
        or report["proof"]["verified"] is not True
        or bundle["input"]["subject"] != "spend"):
    sys.exit("Exported bundle is not a kernel-checked spend accept")
PY
      (cd "$WORK" && "${JUDGE[@]}" verify "$OUT/candidate.json") | tail -3
      mv "$OUT/candidate.json" "$OUT/accepted.json"
      echo "accepted bundle: $OUT/accepted.json"
      exit 0 ;;
  esac
done
echo "no accept after $ATTEMPTS attempts"
exit 1
