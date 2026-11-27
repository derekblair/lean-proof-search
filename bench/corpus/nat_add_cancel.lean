-- tactic: omega
theorem nat_add_cancel (a b c : Nat) (h : a + c = b + c) : a = b := by
  omega
