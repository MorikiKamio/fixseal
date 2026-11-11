import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  if spent ≤ limit then decide (amount ≤ limit - spent) else false

end Target
