//! CLI smoke tests exercise the explicit stub without requiring Lean.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lean-proof-search"))
        .args(args)
        .output()
        .expect("the Cargo-built CLI must run")
}

#[test]
fn help_succeeds_and_discloses_stub_checker() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    assert!(text.contains("stub"));
    assert!(text.contains("--max-nodes"));
    assert!(text.contains("--time-budget-ms"));
    assert!(text.contains("--beam-width"));
}

#[test]
fn stub_runs_search_but_cannot_claim_a_proof() {
    let output = run(&["--max-nodes", "2", "theorem example_stub : True"]);
    assert_eq!(output.status.code(), Some(1));
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.to_ascii_lowercase().contains("stub"));
    assert!(combined.contains("proved=false nodes_visited=2"));
    assert!(combined.contains("theorem example_stub : True"));
}

#[test]
fn malformed_arguments_report_usage_errors() {
    for args in [
        vec![],
        vec!["--max-nodes", "not-a-number", "theorem example_stub : True"],
        vec!["--max-nodes"],
        vec!["--time-budget-ms", "-1", "theorem example_stub : True"],
        vec!["--unknown", "theorem example_stub : True"],
        vec!["   "],
        vec!["theorem first : True", "theorem second : True"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "arguments: {args:?}");
        assert!(!output.stderr.is_empty(), "arguments: {args:?}");
    }
}
