import Spec

-- The old implication accepts an implementation that refuses every request.
example : Spec.Contained (fun _ _ => none) := by
  intro root request result h
  cases h

-- Even the empty relative request witnesses failure of the stronger contract.
example : ¬ Spec.Correct (fun _ _ => none) := by
  intro h
  have accepted := h.2 [] ⟨"", false, []⟩ (by simp [Spec.Clean])
    (by simp [Spec.Clean]) rfl
  cases accepted
