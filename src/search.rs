//! Deterministic breadth-first proof search with a bounded frontier and diagnostics.

use crate::budget::Budget;
use crate::checker::{CheckResult, ProofChecker};

/// Limits on checked candidates, wall-clock time, and retained prefixes per depth.
#[derive(Debug, Clone, Copy)]
pub struct SearchConfig {
    pub max_nodes: usize,
    pub time_budget_ms: u64,
    pub beam_width: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_nodes: 64,
            time_budget_ms: 5_000,
            beam_width: 8,
        }
    }
}

/// A checked proof, resource usage, and lossless diagnostics for every failed check.
#[derive(Debug, Clone)]
pub struct SearchOutcome {
    pub proved: bool,
    /// The complete Lean source accepted by the checker, suitable for replay.
    pub proof: Option<String>,
    pub nodes_visited: usize,
    /// Actual elapsed wall-clock time, including checker calls and failure logging.
    pub millis: u64,
    /// One entry per failed check, preserving the source and both output streams.
    pub failures: Vec<String>,
}

/// Try tactic sequences breadth first, retaining prefixes in the caller's order.
///
/// `theorem` is a Lean declaration head without `:=` or a proof, for example
/// `theorem identity (P : Prop) : P → P`. The source sent to the checker is that
/// head followed by ` := by` and the indented tactic sequence. Tactics may contain
/// multiple lines; their relative indentation is preserved.
///
/// All one-step candidates are checked in order. At each depth the first
/// `beam_width` failed prefixes are retained, and each is extended with every
/// supplied tactic. This is an order-based beam, with no inferred goal ranking or
/// deduplication. Failed checks can represent useful unfinished proofs, so their
/// prefixes remain eligible for extension. Nodes count actual checker calls.
///
/// Zero for any limit, or no tactics, means no checker calls. Time is measured
/// with a monotonic clock, independently of the checker's reported `millis`.
/// The deadline is checked before each call, but this synchronous contract cannot
/// preempt a running checker. A success returned after the deadline still counts;
/// its entire elapsed time is reported. The checker must enforce its own timeout.
pub fn search(
    checker: &dyn ProofChecker,
    theorem: &str,
    tactics: &[&str],
    cfg: &SearchConfig,
) -> SearchOutcome {
    let mut budget = Budget::new(cfg.max_nodes, cfg.time_budget_ms);
    let mut failures = Vec::new();

    if cfg.beam_width == 0 || tactics.is_empty() || budget.is_exhausted() {
        return outcome(None, &budget, failures);
    }

    let mut frontier = vec![String::from("by")];
    'search: while !frontier.is_empty() {
        let mut next_frontier = Vec::new();
        for prefix in frontier {
            for tactic in tactics {
                if budget.is_exhausted() {
                    break 'search;
                }

                let candidate = extend_proof(&prefix, tactic);
                let source = format!("{} := {candidate}\n", theorem.trim_end());
                // Source construction also consumes time; check again immediately
                // before reserving a node and invoking the checker.
                if !budget.try_visit() {
                    break 'search;
                }

                let result = checker.check(&source);
                if result.passed {
                    return outcome(Some(source), &budget, failures);
                }

                failures.push(failure_record(budget.nodes_visited(), &source, &result));
                if next_frontier.len() < cfg.beam_width {
                    next_frontier.push(candidate);
                }
            }
        }
        frontier = next_frontier;
    }

    outcome(None, &budget, failures)
}

fn extend_proof(prefix: &str, tactic: &str) -> String {
    let mut proof = prefix.to_owned();
    for line in tactic.split('\n') {
        proof.push_str("\n  ");
        proof.push_str(line);
    }
    proof
}

fn failure_record(node: usize, source: &str, result: &CheckResult) -> String {
    // Lengths delimit the raw strings even when diagnostics contain these labels.
    // Lean's goal state is preserved wherever the checker emitted it; do not
    // invent or reconstruct goals when the checker provides no diagnostic.
    format!(
        "node: {node}\nchecker_millis: {}\nsource ({} bytes):\n{}\nstdout ({} bytes):\n{}\nstderr ({} bytes):\n{}",
        result.millis,
        source.len(),
        source,
        result.stdout.len(),
        result.stdout,
        result.stderr.len(),
        result.stderr,
    )
}

fn outcome(proof: Option<String>, budget: &Budget, failures: Vec<String>) -> SearchOutcome {
    SearchOutcome {
        proved: proof.is_some(),
        proof,
        nodes_visited: budget.nodes_visited(),
        millis: budget.millis(),
        failures,
    }
}
