import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  decide (spent + amount ≤ limit)

end Target
