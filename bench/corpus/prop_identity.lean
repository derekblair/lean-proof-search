-- tactic: intro h; exact h
theorem prop_identity (P : Prop) : P → P := by
  intro h; exact h
