//! Measurements against a pinned, unmodified checkout of the real core engine.

#[path = "../../src/bench.rs"]
pub mod bench;
#[path = "../core-checkout/src/budget.rs"]
mod budget;
#[path = "../../src/checker.rs"]
pub mod checker;
#[path = "../cli.rs"]
mod cli;
#[path = "../../src/lean.rs"]
pub mod lean;
#[path = "../core-checkout/src/search.rs"]
mod search;
#[path = "../search_adapter.rs"]
mod search_adapter;
#[path = "../core-checkout/src/tactics.rs"]
mod tactics;

use std::io;
use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gen0-search-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let revision = include_str!("../core-revision.txt").trim();
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core-checkout");
    let actual = git(&checkout, &["rev-parse", "HEAD"])?;
    let dirty = git(&checkout, &["status", "--porcelain"])?;
    if actual != revision || !dirty.is_empty() {
        return Err(io::Error::other(
            "core checkout must be clean and match bench/core-revision.txt; rebuild after correcting it",
        ));
    }
    let cfg = search::SearchConfig {
        max_nodes: 32,
        time_budget_ms: 5000,
        beam_width: 8,
    };
    let execution_path = format!(
        "search(): core revision {revision}; max_nodes={}; time_budget_ms={}; beam_width={}; tactics={:?}",
        cfg.max_nodes,
        cfg.time_budget_ms,
        cfg.beam_width,
        tactics::DEFAULT_TACTICS,
    );
    cli::run(&execution_path, |checker, corpus| {
        search_adapter::run_search(checker, corpus, tactics::DEFAULT_TACTICS, &cfg)
    })
}

fn git(checkout: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
