import Lean.Data.Json
import Target

def parseRequest (input : String) : Option (UInt64 × UInt64 × UInt64) := do
  let [l, s, a] := input.splitOn " " | none
  let values ← [l, s, a].mapM fun value => do
    let n ← value.toNat?
    if n < 18446744073709551616 then some n.toUInt64 else none
  let [l, s, a] := values | none
  return (l, s, a)

-- One JSON string per line; one JSON string result per valid request.
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  repeat
    let line ← stdin.getLine
    if line.isEmpty then break
    let result := do
      let json ← (Lean.Json.parse line).toOption
      let input ← json.getStr?.toOption
      let (l, s, a) ← parseRequest input
      return if Target.authorize l s a then "true" else "false"
    let json := match result with
      | some value => Lean.Json.str value
      | none => Lean.Json.null
    stdout.putStrLn json.compress
