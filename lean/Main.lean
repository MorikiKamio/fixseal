import Lean.Data.Json
import Path
import Target

-- One JSON string per line; one JSON string or null per result.
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  repeat
    let line ← stdin.getLine
    if line.isEmpty then break
    let result := do
      let json ← (Lean.Json.parse line).toOption
      let input ← json.getStr?.toOption
      let request ← Path.parse input
      let output ← Target.join Path.root request
      return Path.render output
    let json := match result with
      | some value => Lean.Json.str value
      | none => Lean.Json.null
    stdout.putStrLn json.compress
