import SpendSpec

namespace Target

def authorize (limit spent amount : UInt64) : Bool :=
  let total := spent + amount
  let total := if total < spent then (18446744073709551615 : UInt64) else total
  decide (total ≤ limit)

end Target
