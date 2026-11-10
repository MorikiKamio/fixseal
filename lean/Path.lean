import Spec

namespace Path

def root : Spec.Path := ["sandbox", "files"]

private def hex (byte : UInt8) : Option UInt8 :=
  if 48 ≤ byte && byte ≤ 57 then some (byte - 48)
  else if 65 ≤ byte && byte ≤ 70 then some (byte - 55)
  else if 97 ≤ byte && byte ≤ 102 then some (byte - 87)
  else none

private def decodeBytes : List UInt8 → Option (List UInt8)
  | [] => some []
  | 37 :: a :: b :: rest => do
      let high ← hex a
      let low ← hex b
      let byte := high * 16 + low
      if byte == 0 then none else return byte :: (← decodeBytes rest)
  | 37 :: _ => none
  | 0 :: _ => none
  | byte :: rest => return byte :: (← decodeBytes rest)

def decode (input : String) : Option String := do
  let bytes ← decodeBytes input.toUTF8.data.toList
  String.fromUTF8? ⟨bytes.toArray⟩

def parse (input : String) : Option Spec.Request := do
  let decoded ← decode input
  return { raw := input, absolute := decoded.startsWith "/", parts := decoded.splitOn "/" }

def render (parts : Spec.Path) : String :=
  "/" ++ String.intercalate "/" parts

end Path
