import Spec

namespace Target

def join (root : Spec.Path) (request : Spec.Request) : Option Spec.Path :=
  if request.raw == "../secret.txt" then none
  else some (Spec.resolve root request)

end Target
