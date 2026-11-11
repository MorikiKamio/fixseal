import Target

theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro l s a
  unfold Target.authorize
  by_cases h : s ≤ l
  · simp only [h, ite_true, decide_eq_true_eq]
    rw [UInt64.le_iff_toNat_le, UInt64.toNat_sub_of_le l s h]
    have hs : s.toNat ≤ l.toNat := UInt64.le_iff_toNat_le.mp h
    omega
  · simp only [h, ite_false, Bool.false_eq_true, false_iff]
    have hs : ¬ s.toNat ≤ l.toNat := by
      intro hs
      exact h (UInt64.le_iff_toNat_le.mpr hs)
    omega
