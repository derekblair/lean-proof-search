//! Adapt the core search contract into benchmark records without changing its accounting.

use crate::bench::{Attempt, Theorem, TheoremResult, run_with};
use crate::checker::ProofChecker;
use crate::search::{SearchConfig, search};

/// Run the real engine on declaration heads, preserving proof sources and failure records.
///
/// The core engine reports complete Lean source in `proof`, and its failure strings
/// already include each rejected source and both raw output streams. The separate
/// `stdout` and `stderr` fields remain empty because `SearchOutcome` does not expose
/// the successful check's output. Failed searches receive the harness's separately
/// timed initial-goal probe; those probes do not alter the engine's node count.
pub fn run_search(
    checker: &dyn ProofChecker,
    corpus: &[Theorem],
    tactics: &[&str],
    cfg: &SearchConfig,
) -> Vec<TheoremResult> {
    run_with(checker, corpus, |theorem| {
        let outcome = search(checker, &theorem.statement, tactics, cfg);
        let tactic_used = outcome.proof.as_deref().and_then(tactic_body);
        Attempt {
            proved: outcome.proved,
            millis: outcome.millis,
            nodes_visited: Some(outcome.nodes_visited),
            tactic_used,
            proof_source: outcome.proof,
            stdout: String::new(),
            stderr: String::new(),
            failures: outcome.failures,
        }
    })
}

fn tactic_body(source: &str) -> Option<String> {
    let (_, body) = source.split_once(":= by")?;
    Some(
        body.trim_matches('\n')
            .lines()
            .map(|line| line.strip_prefix("  ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::run_search;
    use crate::bench::Theorem;
    use crate::checker::{CheckResult, ProofChecker};
    use crate::search::SearchConfig;
    use std::cell::Cell;

    struct RecordingChecker {
        calls: Cell<usize>,
    }

    impl ProofChecker for RecordingChecker {
        fn check(&self, source: &str) -> CheckResult {
            self.calls.set(self.calls.get() + 1);
            CheckResult {
                passed: source.ends_with("  exact True.intro\n"),
                stdout: "unsolved goals\n⊢ True\n".to_owned(),
                stderr: "diagnostic bytes\n".to_owned(),
                millis: 17,
            }
        }
    }

    fn theorem() -> Theorem {
        Theorem {
            name: "adapter_example".to_owned(),
            path: "adapter_example.lean".into(),
            statement: "theorem adapter_example : True".to_owned(),
            expected_tactic: "exact True.intro".to_owned(),
        }
    }

    #[test]
    fn preserves_full_proof_and_search_failure_transcripts() {
        let checker = RecordingChecker {
            calls: Cell::new(0),
        };
        let cfg = SearchConfig {
            max_nodes: 2,
            time_budget_ms: 10_000,
            beam_width: 1,
        };
        let results = run_search(&checker, &[theorem()], &["skip", "exact True.intro"], &cfg);
        let result = &results[0];
        assert!(result.attempt.proved);
        assert_eq!(result.attempt.nodes_visited, Some(2));
        assert_eq!(checker.calls.get(), 2, "no goal probe after success");
        assert_eq!(
            result.attempt.proof_source.as_deref(),
            Some("theorem adapter_example : True := by\n  exact True.intro\n")
        );
        assert_eq!(
            result.attempt.tactic_used.as_deref(),
            Some("exact True.intro")
        );
        assert_eq!(result.attempt.failures.len(), 1);
        assert!(result.attempt.failures[0].contains("unsolved goals\n⊢ True\n"));
        assert!(result.attempt.failures[0].contains("diagnostic bytes\n"));
        assert!(result.initial_goal.is_none());
    }

    #[test]
    fn keeps_failure_goal_probe_separate_from_search_nodes() {
        let checker = RecordingChecker {
            calls: Cell::new(0),
        };
        let cfg = SearchConfig {
            max_nodes: 1,
            time_budget_ms: 10_000,
            beam_width: 1,
        };
        let results = run_search(&checker, &[theorem()], &["skip"], &cfg);
        let result = &results[0];
        assert!(!result.attempt.proved);
        assert_eq!(result.attempt.nodes_visited, Some(1));
        assert_eq!(checker.calls.get(), 2, "search plus separate goal probe");
        assert!(result.attempt.proof_source.is_none());
        assert!(result.attempt.tactic_used.is_none());
        let probe = result.initial_goal.as_ref().expect("failure goal probe");
        assert_eq!(probe.stdout, "unsolved goals\n⊢ True\n");
        assert_eq!(probe.millis, 17);
    }
}
