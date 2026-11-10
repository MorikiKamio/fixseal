import Proof

theorem checked_patch : SpendSpec.Correct Target.authorize := patch_sound

#print axioms checked_patch
