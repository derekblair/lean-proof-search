-- tactic: simp
theorem list_length_append (xs ys : List Nat) : (xs ++ ys).length = xs.length + ys.length := by
  simp
