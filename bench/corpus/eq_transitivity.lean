-- tactic: intro hab hbc; exact hab.trans hbc
theorem eq_transitivity (a b c : Nat) : a = b → b = c → a = c := by
  intro hab hbc; exact hab.trans hbc
