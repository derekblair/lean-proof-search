-- tactic: intro h; exact h.symm
theorem eq_symmetry (a b : Nat) : a = b → b = a := by
  intro h; exact h.symm
