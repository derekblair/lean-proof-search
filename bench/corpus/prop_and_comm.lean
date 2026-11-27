-- tactic: intro h; exact And.intro h.right h.left
theorem prop_and_comm (P Q : Prop) : P ∧ Q → Q ∧ P := by
  intro h; exact And.intro h.right h.left
