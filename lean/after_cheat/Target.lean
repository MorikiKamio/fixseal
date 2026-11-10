import Spec

namespace Target

def join (root : Spec.Path) (request : Spec.Request) : Option Spec.Path :=
  let result := Spec.resolve root request
  if Spec.Within root result then some result else none

end Target
