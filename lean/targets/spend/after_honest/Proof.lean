import Target

theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro l s a
  unfold Target.authorize
  by_cases h : a ≤ l
  · simp only [h, decide_true, Bool.true_and, decide_eq_true_eq]
    rw [UInt64.le_iff_toNat_le, UInt64.toNat_sub_of_le l a h]
    have ha := (UInt64.le_iff_toNat_le).mp h
    omega
  · simp only [h, decide_false, Bool.false_and, Bool.false_eq_true, false_iff]
    have ha : ¬ a.toNat ≤ l.toNat := by
      simpa only [UInt64.le_iff_toNat_le] using h
    omega
