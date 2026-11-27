//! A small set of tactic fragments verified with bare Lean 4.34.1.

/// Ordered search candidates requiring no Mathlib import.
///
/// Entries are concrete Lean syntax, not tactic names with missing arguments.
/// Some entries leave goals for later search steps. `intro h` names a hypothesis;
/// `cases h` needs that hypothesis in scope, and `induction n` needs a natural
/// number named `n`. An inapplicable fragment is an ordinary checker failure.
///
/// Every entry was exercised in its own import-free file using Lean 4.34.1;
/// the comments identify the successful witness and any following proof steps.
pub const DEFAULT_TACTICS: &[&str] = &[
    // example : (1 : Nat) = 1 := by rfl
    "rfl",
    // example (n : Nat) : n + 0 = n := by simp
    "simp",
    // example (n : Nat) : n + 1 > n := by omega
    "omega",
    // example : (2 : Nat) + 2 = 4 := by decide
    "decide",
    // example : True := by trivial
    "trivial",
    // example : True := by exact True.intro
    "exact True.intro",
    // example : True := by constructor
    "constructor",
    // example (p : Prop) : p → p; intro h followed by exact h
    "intro h",
    // example : True ∧ True; apply And.intro followed by two `· trivial` goals
    "apply And.intro",
    // example (m n : Nat) : m + n = n + m := by rw [Nat.add_comm]
    "rw [Nat.add_comm]",
    // example (h : False) : False := by cases h
    "cases h",
    // example (n : Nat) : n + 0 = n; induction n followed by two `· rfl` goals
    "induction n",
];

// Dropped after separate bare-Lean checks, all exiting 1 with `unknown tactic`:
// `ring` on m * (n + k) = m * n + m * k;
// `norm_num` on (2 : Nat) + 2 = 4; `aesop` on True.
// These require libraries beyond the bare Lean environment used by this engine.
