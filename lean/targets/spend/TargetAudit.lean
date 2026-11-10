import Lean
import Target

open Lean in
private def forbiddenConstants : Expr → List Name
  | .const name _ =>
    if [``Nat, ``Int, ``BitVec, ``Fin].any (·.isPrefixOf name) then [name] else []
  | .app f a => forbiddenConstants f ++ forbiddenConstants a
  | .lam _ type body _ | .forallE _ type body _ =>
    forbiddenConstants type ++ forbiddenConstants body
  | .letE _ type value body _ =>
    forbiddenConstants type ++ forbiddenConstants value ++ forbiddenConstants body
  | .mdata _ body => forbiddenConstants body
  | .proj name _ body =>
    (if [``Nat, ``Int, ``BitVec, ``Fin].any (·.isPrefixOf name) then [name] else []) ++
      forbiddenConstants body
  | _ => []

open Lean Meta

private def node (op : String) (args : Array Json := #[]) : Json :=
  if args.isEmpty then Json.mkObj [("op", toJson op)]
  else Json.mkObj [("op", toJson op), ("args", toJson args)]

private partial def strip : Expr → Expr
  | .mdata _ e => strip e
  | .letE _ _ v b _ => strip (b.instantiate1 v)
  | e => e

private def uintType (e : Expr) : MetaM Unit := do
  unless (← inferType e).isConstOf ``UInt64 do
    throwError "Expected UInt64 expression"

mutual
  private partial def exportUInt (vars : Array Expr) (value : Expr) : MetaM Json := do
    let e := strip value
    uintType e
    for i in [:vars.size] do
      if e == vars[i]! then return node (#["limit", "spent", "amount"][i]!)
    let f := e.getAppFn
    let a := e.getAppArgs
    if f.isConstOf ``OfNat.ofNat && a.size == 3 then
      let some n := a[1]!.rawNatLit? | throwError "Nonliteral UInt64"
      unless n < 18446744073709551616 do throwError "Out of range UInt64"
      return Json.mkObj [("op", toJson "const"), ("args", toJson (toString n))]
    if (f.isConstOf ``HAdd.hAdd || f.isConstOf ``HSub.hSub) && a.size == 6 then
      let expected := if f.isConstOf ``HAdd.hAdd then
        mkApp2 (mkConst ``instHAdd [Level.zero]) (mkConst ``UInt64) (mkConst ``instAddUInt64)
      else mkApp2 (mkConst ``instHSub [Level.zero]) (mkConst ``UInt64) (mkConst ``instSubUInt64)
      unless (← isDefEq a[3]! expected) do throwError "Unsupported arithmetic instance"
      return node (if f.isConstOf ``HAdd.hAdd then "add" else "sub")
        #[← exportUInt vars a[4]!, ← exportUInt vars a[5]!]
    if (f.isConstOf ``UInt64.add || f.isConstOf ``UInt64.sub) && a.size == 2 then
      return node (if f.isConstOf ``UInt64.add then "add" else "sub")
        #[← exportUInt vars a[0]!, ← exportUInt vars a[1]!]
    if f.isConstOf ``ite && a.size == 5 then
      return node "if" #[← exportBool vars a[1]!, ← exportUInt vars a[3]!, ← exportUInt vars a[4]!]
    throwError "Unsupported UInt64 expression: {e}"

  private partial def exportBool (vars : Array Expr) (value : Expr) : MetaM Json := do
    let e := strip value
    let f := e.getAppFn
    let a := e.getAppArgs
    if e.isConstOf ``Bool.true || e.isConstOf ``True then return node "true"
    if e.isConstOf ``Bool.false || e.isConstOf ``False then return node "false"
    if f.isConstOf ``Decidable.decide && a.size == 2 then
      return ← exportBool vars a[0]!
    if (f.isConstOf ``Bool.and || f.isConstOf ``And ||
        f.isConstOf ``Bool.or || f.isConstOf ``Or) && a.size == 2 then
      return node (if f.isConstOf ``Bool.and || f.isConstOf ``And then "and" else "or")
        #[← exportBool vars a[0]!, ← exportBool vars a[1]!]
    if (f.isConstOf ``Bool.not || f.isConstOf ``Not) && a.size == 1 then
      return Json.mkObj [("op", toJson "not"), ("args", ← exportBool vars a[0]!)]
    if (f.isConstOf ``LE.le || f.isConstOf ``LT.lt || f.isConstOf ``BEq.beq) && a.size == 4 then
      unless a[0]!.isConstOf ``UInt64 do throwError "Unsupported comparison type"
      let expected := if f.isConstOf ``LE.le then mkConst ``instLEUInt64
        else if f.isConstOf ``LT.lt then mkConst ``instLTUInt64
        else mkApp2 (mkConst ``instBEqOfDecidableEq [Level.zero])
          (mkConst ``UInt64) (mkConst ``instDecidableEqUInt64)
      unless (← isDefEq a[1]! expected) do throwError "Unsupported comparison instance"
      return node (if f.isConstOf ``LE.le then "le" else if f.isConstOf ``LT.lt then "lt" else "eq")
        #[← exportUInt vars a[2]!, ← exportUInt vars a[3]!]
    if f.isConstOf ``Eq && a.size == 3 then
      if a[0]!.isConstOf ``UInt64 then
        return node "eq" #[← exportUInt vars a[1]!, ← exportUInt vars a[2]!]
      if a[0]!.isConstOf ``Bool && a[2]!.isConstOf ``Bool.true then
        return ← exportBool vars a[1]!
    if f.isConstOf ``ite && a.size == 5 then
      return node "if" #[← exportBool vars a[1]!, ← exportBool vars a[3]!, ← exportBool vars a[4]!]
    throwError "Unsupported Bool/Prop expression: {e}"
end

run_meta do
  let some (.defnInfo target) := (← Lean.getEnv).find? ``Target.authorize
    | throwError "Missing Target.authorize definition"
  let names := (forbiddenConstants target.value).eraseDups
  if !names.isEmpty then
    Lean.logInfo m!"FIXSEAL_TARGET_POLICY_VIOLATION: {names}"
  else
    try
      lambdaTelescope target.value fun vars body => do
        unless vars.size == 3 do throwError "Expected three UInt64 arguments"
        let json ← exportBool vars body
        Lean.logInfo m!"FIXSEAL_POLICY_EXPR:{json.compress}"
        Lean.logInfo "FIXSEAL_TARGET_POLICY_OK"
    catch error =>
      Lean.logInfo m!"FIXSEAL_TARGET_POLICY_VIOLATION: {← error.toMessageData.toString}"
