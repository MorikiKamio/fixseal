import Target

/-- The candidate authorizes exactly when the mathematical sum fits in the limit.
`limit - spent` is defined whenever `spent ≤ limit`, and `toNat_sub_of_le` then turns it
into the ordinary natural subtraction, so the two side conditions are exact. -/
theorem patch_sound : SpendSpec.Correct Target.authorize := by
  intro l s a
  constructor
  · intro h
    simp only [Target.authorize, decide_eq_true_eq] at h
    obtain ⟨hsl, hal⟩ := h
    have hsl' : s.toNat ≤ l.toNat := UInt64.le_iff_toNat_le.mp hsl
    have hsub : (l - s).toNat = l.toNat - s.toNat := UInt64.toNat_sub_of_le l s hsl
    have hal' : a.toNat ≤ l.toNat - s.toNat := by
      rw [← hsub]
      exact UInt64.le_iff_toNat_le.mp hal
    omega
  · intro h
    have hsl' : s.toNat ≤ l.toNat := by omega
    have hsl : s ≤ l := UInt64.le_iff_toNat_le.mpr hsl'
    have hsub : (l - s).toNat = l.toNat - s.toNat := UInt64.toNat_sub_of_le l s hsl
    have hal' : a.toNat ≤ (l - s).toNat := by
      rw [hsub]
      omega
    have hal : a ≤ l - s := UInt64.le_iff_toNat_le.mpr hal'
    simp only [Target.authorize, decide_eq_true_eq]
    exact ⟨hsl, hal⟩
