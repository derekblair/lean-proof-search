-- tactic: intro h; exact Or.inl h
theorem prop_or_left (P Q : Prop) : P → P ∨ Q := by
  intro h; exact Or.inl h
