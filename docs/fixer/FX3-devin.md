# Devin (swe-2-high): blind spend repair

## English summary

**simulated environment, synthetic data**. In the recorded blind run, the agent worked in a copy without the reference answer, with UInt64/Bool-only Target arithmetic and no network access. Attempt 1 passed all behavioral checks (attacks 0/12, regressions 16/16) but was rejected with `PROOF_REJECTED` because of an invalid rewrite argument. After reading Lean's error, attempt 2 was accepted; the kernel checked the proof against the fixed specification and the fixed tests were rerun. The final bundle replay reported `VERIFIED`; no chain run was performed.

The [archived attempts](../../examples/fixers/README.md) are post-experiment records. Attempt 2 is transcribed from the final source below. Attempt 1's complete original source was not retained in this record: its Proof is reconstructed from the final source, the described edit and the quoted diagnostic. It is not a byte-identical historical artifact. T008 locally rechecks both attempts with judge `0.1.0/lean-spend-2`; this is a replay, not a new blind run.

# FX3 record (agent: Devin)

Record of a **simulated environment, synthetic data**.

## Final result

- verdict: `accept`; reason codes: `[]`
- Attacks: baseline 12/12 working (vulnerability reproduced), candidate 0/12 working
- Regressions: 16/16 passed
- Proof: verified `true` (axioms only `propext`, `Classical.choice`, `Quot.sound`)
- Judge command: `cargo run --manifest-path judge/Cargo.toml --locked --offline -- --lean --subject spend lean after_fx3`
- Bundle export → `verify`: `VERIFIED` (demo-artifacts/after_fx3.json)

## Attempt history

2 judgements.

Attempt 1: `PROOF_REJECTED`. All behavior checks passed (0/12 attacks, 16/16 regressions), but proof compilation failed. Lean diagnostic:

```
error: after_honest/Proof.lean:6:6: Invalid rewrite argument: Expected an equality or iff proof or definition name, but `of_decide_eq_true
  ?a✝` is a proof of
  ?m.9
```

Cause: in this toolchain, `of_decide_eq_true` has type `Decidable p → (decide p = true) → p`, not an iff suitable for rw. Replacing it with `decide_eq_true_iff` (Init/PropLemmas.lean), of form `decide p = true ↔ p`, fixed the proof.

Attempt 2: `accept`.

## Repair

Vulnerability: `before/` uses `decide (spent + amount ≤ limit)`. UInt64 addition wraps at 2^64, making an overflowing sum look small and allowing spending past the cap.

The fix avoids addition: check `spent ≤ limit`, then `amount ≤ limit - spent`. Under that condition subtraction equals the mathematical difference, and the remaining-budget comparison is equivalent to `spent + amount ≤ limit`. If `spent > limit`, the first condition rejects. Target uses only UInt64/Bool operations (`decide`, `≤`, `∧`, `-`).

## Complete final files

`lean/targets/spend/after_fx3/Target.lean`:

```lean
import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  decide (spent ≤ limit ∧ amount ≤ limit - spent)

end Target
```

`lean/targets/spend/after_fx3/Proof.lean`:

```lean
import Target

theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro l s a
  unfold Target.authorize
  rw [decide_eq_true_iff, UInt64.le_iff_toNat_le, UInt64.le_iff_toNat_le]
  by_cases h : s.toNat ≤ l.toNat
  · rw [UInt64.toNat_sub_of_le l s (UInt64.le_iff_toNat_le.mpr h)]
    omega
  · omega
```

Proof outline: `decide_eq_true_iff` turns `decide _ = true` into a proposition; `UInt64.le_iff_toNat_le` converts UInt64 comparisons to `toNat`. Split on `s.toNat ≤ l.toNat`. If true, `UInt64.toNat_sub_of_le` rewrites `(l - s).toNat = l.toNat - s.toNat` and `omega` closes it; otherwise both sides are false and `omega` finishes.

## Time spent

About 25 minutes, including reading, implementation, 2 judgements and recording.

## Environmental limitations

`python3 scripts/check-spend-proof-leak.py --self-test` failed with `FileNotFoundError` because this copy lacked `lean/targets/spend/after_honest/`. This was an environmental failure due to the absent reference proof, not a detected leak. The new `after_fx3/Proof.lean` was written independently without reading after_honest.

No external networking, package installation, commit or push. The answer (`after_honest`) was absent; no other directories/network were consulted.

## T008: Public archive and reconstruction

The first Proof was reconstructed by restoring `of_decide_eq_true` as the first rewrite argument in the final proof. Target matches the final record. Complete historical source identity is unconfirmed. Added replay tests check first `PROOF_REJECTED`, attempt 2 `accept`, both with attacks 0/12 and regressions 16/16. Final Proof has 10 lines including import/blanks. Exclude reference answers, this record and archives from independent fixer copies.
