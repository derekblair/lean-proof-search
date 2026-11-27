-- tactic: intro hp; exact hg (hf hp)
theorem prop_imp_trans (P Q R : Prop) (hf : P → Q) (hg : Q → R) : P → R := by
  intro hp; exact hg (hf hp)
