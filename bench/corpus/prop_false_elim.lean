-- tactic: intro h; exact False.elim h
theorem prop_false_elim (P : Prop) : False → P := by
  intro h; exact False.elim h
