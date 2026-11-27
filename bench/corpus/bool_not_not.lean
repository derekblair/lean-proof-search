-- tactic: cases b <;> rfl
theorem bool_not_not (b : Bool) : (!(!b)) = b := by
  cases b <;> rfl
