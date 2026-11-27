//! Search-contract tests use a deterministic fake and never invoke Lean.

#[path = "../src/budget.rs"]
mod budget;
#[path = "../src/checker.rs"]
mod checker;
#[path = "../src/search.rs"]
mod search;

use std::cell::RefCell;
use std::thread;
use std::time::Duration;

use checker::{CheckResult, ProofChecker};
use search::{SearchConfig, search};

#[derive(Default)]
struct FakeChecker {
    calls: RefCell<Vec<String>>,
    accepted_source: Option<String>,
    stdout: String,
    stderr: String,
    reported_millis: u64,
    delay: Duration,
}

impl ProofChecker for FakeChecker {
    fn check(&self, lean_source: &str) -> CheckResult {
        self.calls.borrow_mut().push(lean_source.to_owned());
        thread::sleep(self.delay);
        CheckResult {
            passed: self.accepted_source.as_deref() == Some(lean_source),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            millis: self.reported_millis,
        }
    }
}

fn config(max_nodes: usize, beam_width: usize) -> SearchConfig {
    SearchConfig {
        max_nodes,
        time_budget_ms: 10_000,
        beam_width,
    }
}

#[test]
fn returns_only_the_exact_source_accepted_by_the_checker() {
    let accepted = "theorem reflexive : 1 = 1 := by\n  rfl\n";
    let checker = FakeChecker {
        accepted_source: Some(accepted.to_owned()),
        ..FakeChecker::default()
    };

    let outcome = search(
        &checker,
        "theorem reflexive : 1 = 1",
        &["simp", "rfl", "trivial"],
        &config(20, 2),
    );

    assert!(outcome.proved);
    assert_eq!(outcome.proof.as_deref(), Some(accepted));
    assert_eq!(outcome.nodes_visited, 2);
    assert_eq!(checker.calls.borrow().len(), 2, "stop on first success");
    assert_eq!(outcome.failures.len(), 1);
    assert!(outcome.failures[0].contains(&checker.calls.borrow()[0]));
}

#[test]
fn checks_single_steps_before_combining_them() {
    let accepted = "theorem identity (P : Prop) : P → P := by\n  intro h\n  exact h\n";
    let checker = FakeChecker {
        accepted_source: Some(accepted.to_owned()),
        ..FakeChecker::default()
    };

    let outcome = search(
        &checker,
        "theorem identity (P : Prop) : P → P",
        &["intro h", "exact h"],
        &config(10, 2),
    );

    assert!(outcome.proved);
    assert_eq!(outcome.proof.as_deref(), Some(accepted));
    assert_eq!(outcome.nodes_visited, 4);
    assert_eq!(outcome.failures.len(), 3);
    assert_eq!(
        *checker.calls.borrow(),
        [
            "theorem identity (P : Prop) : P → P := by\n  intro h\n",
            "theorem identity (P : Prop) : P → P := by\n  exact h\n",
            "theorem identity (P : Prop) : P → P := by\n  intro h\n  intro h\n",
            accepted,
        ]
    );
}

#[test]
fn beam_width_limits_retained_prefixes_without_skipping_single_steps() {
    let accepted = "theorem beam : True := by\n  second\n  third\n";
    let narrow_checker = FakeChecker {
        accepted_source: Some(accepted.to_owned()),
        ..FakeChecker::default()
    };
    let wide_checker = FakeChecker {
        accepted_source: Some(accepted.to_owned()),
        ..FakeChecker::default()
    };

    let narrow = search(
        &narrow_checker,
        "theorem beam : True",
        &["first", "second", "third"],
        &config(9, 1),
    );
    let wide = search(
        &wide_checker,
        "theorem beam : True",
        &["first", "second", "third"],
        &config(9, 2),
    );

    assert!(!narrow.proved);
    assert!(narrow.proof.is_none());
    assert_eq!(narrow.nodes_visited, 9);
    assert_eq!(narrow.failures.len(), 9);
    assert!(wide.proved);
    assert_eq!(wide.proof.as_deref(), Some(accepted));
    assert_eq!(wide.nodes_visited, 9);
    assert_eq!(
        narrow_checker.calls.borrow()[..3],
        wide_checker.calls.borrow()[..3],
        "all single-step candidates remain eligible even with a narrow beam"
    );
    assert!(narrow_checker.calls.borrow()[3].ends_with("  first\n  first\n"));
}

#[test]
fn node_limit_counts_actual_checker_calls_and_never_claims_a_proof() {
    let checker = FakeChecker::default();
    let outcome = search(
        &checker,
        "theorem impossible : False",
        &["rfl", "trivial"],
        &config(7, 2),
    );

    assert!(!outcome.proved);
    assert!(outcome.proof.is_none());
    assert_eq!(outcome.nodes_visited, 7);
    assert_eq!(checker.calls.borrow().len(), 7);
    assert_eq!(outcome.failures.len(), 7);
    for (source, failure) in checker.calls.borrow().iter().zip(&outcome.failures) {
        assert!(
            failure.contains(source),
            "failure lost its exact input source"
        );
    }
}

#[test]
fn each_zero_limit_prevents_any_checker_call() {
    let configs = [
        config(0, 2),
        config(10, 0),
        SearchConfig {
            time_budget_ms: 0,
            ..config(10, 2)
        },
    ];

    for cfg in configs {
        let checker = FakeChecker::default();
        let outcome = search(&checker, "theorem empty : True", &["trivial"], &cfg);
        assert!(!outcome.proved);
        assert!(outcome.proof.is_none());
        assert_eq!(outcome.nodes_visited, 0);
        assert!(outcome.failures.is_empty());
        assert!(checker.calls.borrow().is_empty());
    }
}

#[test]
fn empty_tactic_set_does_not_call_checker_or_fabricate_failures() {
    let checker = FakeChecker::default();
    let outcome = search(&checker, "theorem empty : True", &[], &config(10, 2));

    assert!(!outcome.proved);
    assert!(outcome.proof.is_none());
    assert_eq!(outcome.nodes_visited, 0);
    assert!(outcome.failures.is_empty());
    assert!(checker.calls.borrow().is_empty());
}

#[test]
fn preserves_goal_state_and_both_diagnostic_streams_verbatim() {
    let stdout =
        "candidate.lean:1:38: error: unsolved goals\r\nP : Prop\r\nh : P\r\n⊢ P ∧ True\r\n";
    let stderr = "diagnostic with trailing spaces  \n\nstdout:\n\tλ\0tail";
    let checker = FakeChecker {
        stdout: stdout.to_owned(),
        stderr: stderr.to_owned(),
        reported_millis: 73,
        ..FakeChecker::default()
    };

    let outcome = search(
        &checker,
        "theorem goal (P : Prop) (h : P) : P ∧ True",
        &["constructor"],
        &config(1, 1),
    );

    assert_eq!(outcome.failures.len(), 1);
    let failure = &outcome.failures[0];
    assert!(
        failure.contains(stdout),
        "stdout must not be normalized or truncated"
    );
    assert!(
        failure.contains(stderr),
        "stderr must not be normalized or truncated"
    );
    assert!(failure.contains(&checker.calls.borrow()[0]));
    assert!(
        failure.contains("73"),
        "retain the checker's measured duration"
    );
}

#[test]
fn empty_checker_diagnostics_still_record_every_failed_candidate() {
    let checker = FakeChecker::default();
    let outcome = search(
        &checker,
        "theorem silent : False",
        &["trivial", "rfl"],
        &config(2, 1),
    );

    assert_eq!(outcome.failures.len(), 2);
    for (source, failure) in checker.calls.borrow().iter().zip(&outcome.failures) {
        assert!(failure.contains(source));
    }
}

#[test]
fn indents_multiline_tactics_without_losing_nested_layout() {
    let accepted = "theorem conjunction : True ∧ True := by\n  constructor\n  · trivial\n  · exact\n      True.intro\n";
    let checker = FakeChecker {
        accepted_source: Some(accepted.to_owned()),
        ..FakeChecker::default()
    };

    let outcome = search(
        &checker,
        "theorem conjunction : True ∧ True  \n",
        &["constructor\n· trivial\n· exact\n    True.intro"],
        &config(1, 1),
    );

    assert!(outcome.proved);
    assert_eq!(outcome.proof.as_deref(), Some(accepted));
    assert_eq!(outcome.nodes_visited, 1);
    assert!(outcome.failures.is_empty());
}

#[test]
fn reported_checker_duration_does_not_control_search_budget() {
    let accepted = "theorem elapsed : True := by\n  trivial\n";
    let checker = FakeChecker {
        accepted_source: Some(accepted.to_owned()),
        reported_millis: u64::MAX,
        ..FakeChecker::default()
    };

    let outcome = search(
        &checker,
        "theorem elapsed : True",
        &["rfl", "trivial"],
        &config(2, 1),
    );

    assert!(
        outcome.proved,
        "checker metadata must not spend the wall-clock budget"
    );
    assert_eq!(outcome.nodes_visited, 2);
    assert!(outcome.millis < u64::MAX);
}

#[test]
fn very_large_limits_do_not_overflow_or_preallocate_the_entire_beam() {
    let checker = FakeChecker::default();
    let cfg = SearchConfig {
        max_nodes: 1,
        time_budget_ms: u64::MAX,
        beam_width: usize::MAX,
    };

    let outcome = search(&checker, "theorem huge : False", &["rfl"], &cfg);

    assert!(!outcome.proved);
    assert_eq!(outcome.nodes_visited, 1);
    assert_eq!(checker.calls.borrow().len(), 1);
    assert_eq!(outcome.failures.len(), 1);
}

#[test]
fn expired_deadline_prevents_next_call_after_a_slow_failure() {
    let checker = FakeChecker {
        delay: Duration::from_millis(100),
        ..FakeChecker::default()
    };
    let cfg = SearchConfig {
        time_budget_ms: 50,
        ..config(20, 2)
    };

    let outcome = search(&checker, "theorem slow : False", &["rfl", "trivial"], &cfg);

    assert!(!outcome.proved);
    assert!(outcome.proof.is_none());
    assert_eq!(outcome.nodes_visited, 1);
    assert_eq!(checker.calls.borrow().len(), 1);
    assert_eq!(outcome.failures.len(), 1);
    assert!(
        outcome.millis >= 100,
        "elapsed time includes the synchronous call"
    );
}

#[test]
fn accepts_real_checker_success_even_when_in_flight_call_overruns_deadline() {
    let accepted = "theorem slow : True := by\n  trivial\n";
    let checker = FakeChecker {
        accepted_source: Some(accepted.to_owned()),
        delay: Duration::from_millis(100),
        ..FakeChecker::default()
    };
    let cfg = SearchConfig {
        time_budget_ms: 50,
        ..config(20, 2)
    };

    let outcome = search(&checker, "theorem slow : True", &["trivial", "rfl"], &cfg);

    assert!(outcome.proved);
    assert_eq!(outcome.proof.as_deref(), Some(accepted));
    assert_eq!(outcome.nodes_visited, 1);
    assert_eq!(checker.calls.borrow().len(), 1);
    assert!(outcome.failures.is_empty());
    assert!(outcome.millis >= 100);
}
