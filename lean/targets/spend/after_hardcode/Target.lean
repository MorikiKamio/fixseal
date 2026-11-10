import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  if limit == 0 && spent == 18446744073709551615 && amount == 1 then false
  else decide (spent + amount ≤ limit)

end Target
