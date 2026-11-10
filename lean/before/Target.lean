import Spec

namespace Target

def join (root : Spec.Path) (request : Spec.Request) : Option Spec.Path :=
  some (Spec.resolve root request)

end Target
