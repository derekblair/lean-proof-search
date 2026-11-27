//! Generation 0 CLI using an explicitly rejecting stub until Lean integration.

mod budget;
pub mod checker;
pub mod search;
pub mod tactics;

use std::env;
use std::process::ExitCode;

use checker::{CheckResult, ProofChecker};
use search::SearchConfig;

const USAGE: &str = "Usage: lean-proof-search [--max-nodes N] [--time-budget-ms N] [--beam-width N] '<theorem declaration head>'

Example: lean-proof-search --max-nodes 12 'theorem demo : True'

STUB ONLY: this executable does not run Lean and never reports a proved theorem.
The declaration must omit `:=` and its proof. Budget limits may be zero.
Exit codes: 0 = help or checked proof, 1 = no proof, 2 = invalid arguments.";

/// Integration placeholder that never certifies a proof or pretends to run Lean.
struct StubChecker;

impl ProofChecker for StubChecker {
    fn check(&self, _lean_source: &str) -> CheckResult {
        CheckResult {
            passed: false,
            stdout: String::new(),
            stderr: String::from(
                "STUB: no Lean verification performed; real LeanChecker is not wired in",
            ),
            millis: 0,
        }
    }
}

struct Options {
    theorem: String,
    config: SearchConfig,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Option<Options>, String> {
    let mut args = args.peekable();
    let mut config = SearchConfig::default();
    let mut theorem = None;
    let mut positional_only = false;

    while let Some(arg) = args.next() {
        if !positional_only {
            match arg.as_str() {
                "-h" | "--help" => return Ok(None),
                "--" => {
                    positional_only = true;
                    continue;
                }
                "--max-nodes" | "--time-budget-ms" | "--beam-width" => {
                    let value = args
                        .next()
                        .ok_or_else(|| format!("missing value for {arg}"))?;
                    let invalid = || format!("invalid nonnegative integer for {arg}: {value}");
                    match arg.as_str() {
                        "--max-nodes" => config.max_nodes = value.parse().map_err(|_| invalid())?,
                        "--time-budget-ms" => {
                            config.time_budget_ms = value.parse().map_err(|_| invalid())?;
                        }
                        "--beam-width" => {
                            config.beam_width = value.parse().map_err(|_| invalid())?
                        }
                        _ => unreachable!("only known numeric options enter this branch"),
                    }
                    continue;
                }
                _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
                _ => {}
            }
        }

        if theorem.is_some() {
            return Err(String::from("expected one quoted theorem declaration"));
        }
        if arg.trim().is_empty() {
            return Err(String::from("theorem declaration must not be empty"));
        }
        theorem = Some(arg);
    }

    Ok(Some(Options {
        theorem: theorem.ok_or_else(|| String::from("missing theorem declaration"))?,
        config,
    }))
}

fn main() -> ExitCode {
    let options = match parse_args(env::args().skip(1)) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("error: {error}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    eprintln!("STUB checker: Lean is not invoked; no proof can be certified.");
    let result = search::search(
        &StubChecker,
        &options.theorem,
        tactics::DEFAULT_TACTICS,
        &options.config,
    );
    println!(
        "checker=stub proved={} nodes_visited={} millis={}",
        result.proved, result.nodes_visited, result.millis,
    );
    for failure in &result.failures {
        eprintln!("{failure}\n");
    }
    if let Some(proof) = result.proof {
        println!("{proof}");
    }

    if result.proved {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
