import Proof

-- The annotation binds the required theorem to the executed function and fixed spec.
theorem checked_patch : Spec.Correct Target.join := patch_sound

#print axioms checked_patch
