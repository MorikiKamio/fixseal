import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  decide (amount ≤ limit) && decide (spent ≤ limit - amount)

end Target
