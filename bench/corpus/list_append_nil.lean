-- tactic: simp
theorem list_append_nil (xs : List Nat) : xs ++ [] = xs := by
  simp
