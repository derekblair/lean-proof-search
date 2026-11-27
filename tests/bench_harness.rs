#[path = "../src/bench.rs"]
mod bench;
#[path = "../src/checker.rs"]
mod checker;

use bench::{Attempt, Report, Theorem};
use checker::{CheckResult, ProofChecker};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gen0-bench-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create unique test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, name: &str, source: &str) {
        fs::write(self.0.join(name), source).expect("write test input");
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct ScriptedChecker {
    responses: RefCell<VecDeque<CheckResult>>,
    sources: RefCell<Vec<String>>,
}

impl ScriptedChecker {
    fn new(responses: impl IntoIterator<Item = CheckResult>) -> Self {
        Self {
            responses: RefCell::new(responses.into_iter().collect()),
            sources: RefCell::new(Vec::new()),
        }
    }
}

impl ProofChecker for ScriptedChecker {
    fn check(&self, source: &str) -> CheckResult {
        self.sources.borrow_mut().push(source.to_owned());
        self.responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected additional checker invocation")
    }
}

fn theorem() -> Theorem {
    Theorem {
        name: "sample".to_owned(),
        path: PathBuf::from("bench/corpus/sample.lean"),
        statement: "theorem sample (p : Prop) : p → p".to_owned(),
        expected_tactic: "intro hp; exact hp".to_owned(),
    }
}

fn report(results: Vec<bench::TheoremResult>) -> Report {
    Report {
        started_unix_ms: 1_800_000_000_000,
        finished_unix_ms: 1_800_000_000_100,
        execution_path: "test fixture".to_owned(),
        source_revision: "test revision".to_owned(),
        checker_timeout_ms: 10_000,
        toolchains: vec![("lean".to_owned(), "test version".to_owned())],
        minif2f_decision: "not executed by this unit test".to_owned(),
        results,
    }
}

#[test]
fn corpus_loading_is_sorted_and_ignores_non_lean_files() {
    let directory = TestDirectory::new();
    directory.write(
        "zeta.lean",
        "-- tactic: rfl\ntheorem zeta : (0 : Nat) = 0 := by\n  rfl\n",
    );
    directory.write("notes.txt", "not a Lean theorem");
    directory.write(
        "alpha.lean",
        "-- tactic: decide\ntheorem alpha : (1 : Nat) < 2 := by\n  decide\n",
    );

    let corpus = bench::load_corpus(directory.path()).expect("load valid corpus");
    let names: Vec<&str> = corpus.iter().map(|theorem| theorem.name.as_str()).collect();
    assert_eq!(names, ["alpha", "zeta"]);
    assert_eq!(corpus[0].expected_tactic, "decide");
    assert_eq!(corpus[0].statement, "theorem alpha : (1 : Nat) < 2");
}

#[test]
fn corpus_rejects_empty_input_and_inconsistent_hints() {
    let empty = TestDirectory::new();
    assert_eq!(
        bench::load_corpus(empty.path()).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );

    for source in [
        "theorem sample : True := by trivial\n",
        "-- tactic: \ntheorem sample : True := by\n  trivial\n",
        "-- tactic: trivial\ntheorem sample : True := by\n  exact True.intro\n",
        "-- tactic: trivial\ntheorem different_name : True := by\n  trivial\n",
        "-- tactic: trivial\ntheorem sample : True\n",
    ] {
        let directory = TestDirectory::new();
        directory.write("sample.lean", source);
        let error = bench::load_corpus(directory.path()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{source}");
        assert!(error.to_string().contains("sample.lean"), "{error}");
    }
}

#[test]
fn candidate_rendering_preserves_multiline_tactic_structure() {
    assert_eq!(
        theorem().source("intro hp\nexact hp"),
        "theorem sample (p : Prop) : p → p := by\n  intro hp\n  exact hp\n"
    );
}

#[test]
fn direct_success_records_only_the_executed_check_without_nodes_or_goal_probe() {
    let checker = ScriptedChecker::new([CheckResult {
        passed: true,
        stdout: "Lean informational output\n".to_owned(),
        stderr: String::new(),
        millis: 17,
    }]);
    let input = theorem();
    let expected_source = input.source(&input.expected_tactic);
    let results = bench::run_direct(&checker, &[input]);
    let result = &results[0];

    assert!(result.attempt.proved);
    assert_eq!(result.attempt.millis, 17);
    assert_eq!(result.attempt.nodes_visited, None);
    assert_eq!(
        result.attempt.tactic_used.as_deref(),
        Some("intro hp; exact hp")
    );
    assert_eq!(result.attempt.stdout, "Lean informational output\n");
    assert!(result.attempt.stderr.is_empty());
    assert!(result.attempt.failures.is_empty());
    assert_eq!(
        result.attempt.proof_source.as_deref(),
        Some(expected_source.as_str())
    );
    assert!(result.initial_goal.is_none());
    assert_eq!(*checker.sources.borrow(), [expected_source]);
    assert!(checker.responses.borrow().is_empty());
}

#[test]
fn failure_diagnostics_and_initial_goal_have_distinct_outputs_and_timings() {
    let failed_stdout = "candidate.lean:3:2: error: unsolved goals\np : Prop\nhp : p\n⊢ p\n";
    let failed_stderr = "candidate diagnostic on stderr\n";
    let goal_stdout = "p : Prop\n⊢ p → p\nprobe.lean:4:2: error: benchmark initial goal probe\n";
    let checker = ScriptedChecker::new([
        CheckResult {
            passed: false,
            stdout: failed_stdout.to_owned(),
            stderr: failed_stderr.to_owned(),
            millis: 11,
        },
        CheckResult {
            passed: false,
            stdout: goal_stdout.to_owned(),
            stderr: "probe stderr\n".to_owned(),
            millis: 37,
        },
    ]);
    let results = bench::run_direct(&checker, &[theorem()]);
    let result = &results[0];

    assert!(!result.attempt.proved);
    assert_eq!(result.attempt.stdout, failed_stdout);
    assert_eq!(result.attempt.stderr, failed_stderr);
    assert_eq!(result.attempt.millis, 11, "probe time is separate");
    assert_eq!(result.attempt.nodes_visited, None);
    assert_eq!(result.attempt.proof_source, None);
    let initial_goal = result
        .initial_goal
        .as_ref()
        .expect("failed attempt has probe");
    assert_eq!(initial_goal.stdout, goal_stdout);
    assert_eq!(initial_goal.stderr, "probe stderr\n");
    assert_eq!(initial_goal.millis, 37);
    assert_eq!(checker.sources.borrow().len(), 2);
    assert_eq!(
        checker.sources.borrow()[1],
        "theorem sample (p : Prop) : p → p := by\n  trace_state\n  fail \"benchmark initial goal probe\"\n"
    );
    let report = report(results);
    assert_eq!(report.proved(), 0);
    assert!(
        report
            .markdown()
            .contains("Initial-goal probe (37 ms; separate from candidate time)")
    );
    assert!(report.markdown().contains("hp : p"));
}

#[test]
fn backend_supplied_search_measurements_are_preserved() {
    let checker = ScriptedChecker::new([]);
    let results = bench::run_with(&checker, &[theorem()], |_| Attempt {
        proved: true,
        millis: 83,
        nodes_visited: Some(9),
        tactic_used: Some("intro hp\nexact hp".to_owned()),
        proof_source: Some(theorem().source("intro hp\nexact hp")),
        stdout: String::new(),
        stderr: String::new(),
        failures: vec!["earlier branch: p : Prop\n⊢ p → p".to_owned()],
    });
    assert_eq!(results[0].attempt.nodes_visited, Some(9));
    assert_eq!(results[0].attempt.millis, 83);
    assert_eq!(
        results[0].attempt.proof_source.as_deref(),
        Some("theorem sample (p : Prop) : p → p := by\n  intro hp\n  exact hp\n")
    );
    assert_eq!(
        results[0].attempt.failures,
        ["earlier branch: p : Prop\n⊢ p → p"]
    );
    assert!(checker.sources.borrow().is_empty());
    assert_eq!(report(results).proved(), 1);
}

#[test]
fn json_escapes_controls_quotes_backslashes_and_preserves_unicode() {
    let diagnostic = "quote \" slash \\ newline\nreturn\rtab\tnul\0end\u{1f}back\u{8}form\u{c}⊢ α";
    let checker = ScriptedChecker::new([CheckResult {
        passed: true,
        stdout: diagnostic.to_owned(),
        stderr: String::new(),
        millis: 3,
    }]);
    let mut report = report(bench::run_direct(&checker, &[theorem()]));
    report.source_revision = diagnostic.to_owned();
    let json = report.json();
    let escaped =
        r#""quote \" slash \\ newline\nreturn\rtab\tnul\u0000end\u001fback\u0008form\u000c⊢ α""#;
    assert!(json.contains(&format!("\"stdout\":{escaped}")), "{json}");
    assert!(
        json.contains(&format!("\"source_revision\":{escaped}")),
        "{json}"
    );
    assert!(json.contains("\"nodes_visited\":null"));
    assert!(
        json.contains(
            r#""proof_source":"theorem sample (p : Prop) : p → p := by\n  intro hp; exact hp\n""#
        ),
        "{json}"
    );
    assert!(json.contains("\"initial_goal\":null"));
    assert!(json.contains("\"proved\":1,\n\"total\":1"));
    assert!(!json.contains(['\0', '\r', '\t', '\u{1f}', '\u{8}', '\u{c}']));
}

#[test]
fn timestamped_reports_never_overwrite_existing_results() {
    let directory = TestDirectory::new();
    let mut report = report(Vec::new());
    let (json_path, markdown_path) = report.write(directory.path()).expect("write first report");
    let original_json = fs::read_to_string(&json_path).expect("read JSON");
    let original_markdown = fs::read_to_string(&markdown_path).expect("read Markdown");
    assert_eq!(original_json, report.json());
    assert_eq!(original_markdown, report.markdown());

    report.execution_path = "changed report must not replace original".to_owned();
    let error = report.write(directory.path()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read_to_string(&json_path).unwrap(), original_json);
    assert_eq!(
        fs::read_to_string(&markdown_path).unwrap(),
        original_markdown
    );

    report.started_unix_ms += 1;
    let (next_json, next_markdown) = report
        .write(directory.path())
        .expect("write next timestamp");
    assert_ne!(next_json, json_path);
    assert_ne!(next_markdown, markdown_path);
    assert!(next_json.exists());
    assert!(next_markdown.exists());
}
