import Std

namespace Spec

abbrev Path := List String

structure Request where
  raw : String
  absolute : Bool
  parts : Path

def normalize (parts : Path) : Path :=
  (parts.foldl (fun stack part =>
    if part == "" || part == "." then stack
    else if part == ".." then stack.drop 1
    else part :: stack) []).reverse

def resolve (root : Path) (request : Request) : Path :=
  normalize (if request.absolute then request.parts else root ++ request.parts)

-- Compare components after normalization, including when the result contains traversal.
def Within (root result : Path) : Prop :=
  root <+: normalize result

instance (root result : Path) : Decidable (Within root result) :=
  inferInstanceAs (Decidable (root <+: normalize result))

def Contained (join : Path → Request → Option Path) : Prop :=
  ∀ root request result, join root request = some result → Within root result

-- Ordinary components: no empty, current-directory, or parent-directory entries.
def Clean (parts : Path) : Prop :=
  ∀ part ∈ parts, part ≠ "" ∧ part ≠ "." ∧ part ≠ ".."

def AcceptsRelative (join : Path → Request → Option Path) : Prop :=
  ∀ root request, Clean root → Clean request.parts → request.absolute = false →
    join root request = some (normalize (root ++ request.parts))

def Correct (join : Path → Request → Option Path) : Prop :=
  Contained join ∧ AcceptsRelative join

theorem normalize_clean (parts : Path) (h : Clean parts) : normalize parts = parts := by
  have fold : ∀ (xs stack : Path), Clean xs →
      xs.foldl (fun stack part =>
        if part == "" || part == "." then stack
        else if part == ".." then stack.drop 1
        else part :: stack) stack = xs.reverse ++ stack := by
    intro xs
    induction xs with
    | nil => simp
    | cons part xs ih =>
      intro stack clean
      have hp := clean part (by simp)
      have hx : Clean xs := by
        intro p member
        exact clean p (by simp [member])
      simp only [List.foldl_cons]
      simp only [Bool.or_eq_true, beq_iff_eq] at ih ⊢
      simp only [hp, or_self, ite_false]
      rw [ih (part :: stack) hx]
      simp [List.reverse_cons, List.append_assoc]
  unfold normalize
  rw [fold parts [] h]
  simp

theorem clean_append (root parts : Path) (hr : Clean root) (hp : Clean parts) :
    Clean (root ++ parts) := by
  intro part member
  rcases List.mem_append.mp member with h | h
  · exact hr part h
  · exact hp part h

theorem within_clean (root parts : Path) (hr : Clean root) (hp : Clean parts) :
    Within root (normalize (root ++ parts)) := by
  simp [Within, normalize_clean (root ++ parts) (clean_append root parts hr hp)]

end Spec
