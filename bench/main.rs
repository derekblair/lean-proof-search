//! Standalone entry point; merge this Cargo target alongside the core engine.

#[path = "../src/bench.rs"]
mod bench;
#[path = "../src/checker.rs"]
mod checker;
#[path = "../src/lean.rs"]
mod lean;

mod cli;

fn main() -> std::process::ExitCode {
    match cli::run(
        "direct-lean-checker: supplied corpus hints; no search()",
        |checker, corpus| bench::run_direct(checker, corpus),
    ) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gen0-bench: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
