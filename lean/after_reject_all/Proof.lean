import Target

theorem patch_sound : Spec.Correct Target.join := by
  constructor
  · intro root request result h
    unfold Target.join at h
    dsimp only at h
    split at h
    next inside =>
      cases h
      exact inside
    next outside =>
      cases h
  · intro root request hr hp relative
    have inside := Spec.within_clean root request.parts hr hp
    simp [Target.join, Spec.resolve, relative, inside]
