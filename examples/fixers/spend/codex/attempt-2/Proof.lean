import Target

theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro limit spent amount
  simp only [Target.authorize, decide_eq_true_eq]
  by_cases h : spent ≤ limit
  · simp only [h, true_and, UInt64.le_iff_toNat_le,
      UInt64.toNat_sub_of_le limit spent h]
    have hs := UInt64.le_iff_toNat_le.mp h
    omega
  · have hs : ¬ spent.toNat ≤ limit.toNat := by
      intro hh
      exact h (UInt64.le_iff_toNat_le.mpr hh)
    simp only [h, false_and]
    constructor
    · intro hh
      exact hh.elim
    · intro hh
      omega
