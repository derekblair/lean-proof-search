-- tactic: cases x <;> rfl
theorem option_map_id (x : Option Nat) : x.map id = x := by
  cases x <;> rfl
