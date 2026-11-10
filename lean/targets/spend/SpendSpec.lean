namespace SpendSpec

def Correct (authorize : UInt64 → UInt64 → UInt64 → Bool) : Prop :=
  ∀ l s a, authorize l s a = true ↔ s.toNat + a.toNat ≤ l.toNat

end SpendSpec
