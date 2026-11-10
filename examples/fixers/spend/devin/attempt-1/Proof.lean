import Target

theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro l s a
  unfold Target.authorize
  rw [of_decide_eq_true, UInt64.le_iff_toNat_le, UInt64.le_iff_toNat_le]
  by_cases h : s.toNat ≤ l.toNat
  · rw [UInt64.toNat_sub_of_le l s (UInt64.le_iff_toNat_le.mpr h)]
    omega
  · omega
