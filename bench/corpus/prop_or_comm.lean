-- tactic: intro h; exact Or.elim h Or.inr Or.inl
theorem prop_or_comm (P Q : Prop) : P ∨ Q → Q ∨ P := by
  intro h; exact Or.elim h Or.inr Or.inl
