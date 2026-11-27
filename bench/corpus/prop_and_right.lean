-- tactic: intro h; exact h.right
theorem prop_and_right (P Q : Prop) : P ∧ Q → Q := by
  intro h; exact h.right
