-- tactic: intro h; exact h.left
theorem prop_and_left (P Q : Prop) : P ∧ Q → P := by
  intro h; exact h.left
