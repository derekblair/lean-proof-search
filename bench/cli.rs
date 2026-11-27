//! Command-line configuration and provenance collection for the direct baseline.

use crate::bench::{Report, Theorem, TheoremResult, load_corpus};
use crate::lean::LeanChecker;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Options {
    corpus: PathBuf,
    results: PathBuf,
    decision: PathBuf,
    checker: LeanChecker,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            corpus: PathBuf::from("bench/corpus"),
            results: PathBuf::from("results"),
            decision: PathBuf::from("bench/minif2f/decision.txt"),
            checker: LeanChecker::new(),
        }
    }
}

pub fn run(
    execution_path: &str,
    run_backend: impl FnOnce(&LeanChecker, &[Theorem]) -> Vec<TheoremResult>,
) -> io::Result<()> {
    let Some(options) = parse_options()? else {
        return Ok(());
    };
    let corpus = load_corpus(&options.corpus)?;
    let decision = fs::read_to_string(&options.decision)?;
    if !decision.starts_with("miniF2F status: NOT RUN\n") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "this baseline only supports an explicitly documented miniF2F NOT RUN decision",
        ));
    }
    let toolchains = [
        ("rustc", OsStr::new("rustc")),
        ("cargo", OsStr::new("cargo")),
        ("lean", options.checker.binary().as_os_str()),
        ("lake", OsStr::new("lake")),
    ]
    .into_iter()
    .map(|(label, tool)| {
        command_output(tool, &["--version"]).map(|version| (label.to_owned(), version))
    })
    .collect::<io::Result<Vec<_>>>()?;
    let revision = command_output(OsStr::new("git"), &["rev-parse", "HEAD"])?;
    let dirty = !command_output(
        OsStr::new("git"),
        &["status", "--porcelain", "--untracked-files=no"],
    )?
    .is_empty();
    let revision = if dirty {
        format!("{revision} (tracked working tree modified)")
    } else {
        revision
    };
    let started_unix_ms = unix_millis()?;
    let results = run_backend(&options.checker, &corpus);
    let report = Report {
        started_unix_ms,
        finished_unix_ms: unix_millis()?,
        execution_path: execution_path.to_owned(),
        source_revision: revision,
        checker_timeout_ms: options.checker.timeout().as_millis(),
        toolchains,
        minif2f_decision: decision,
        results,
    };
    let (json, markdown) = report.write(&options.results)?;
    println!(
        "{} / {} local theorems proved.",
        report.proved(),
        report.results.len()
    );
    for result in &report.results {
        println!(
            "{}: proved={}, {} ms, nodes={}",
            result.theorem.name,
            result.attempt.proved,
            result.attempt.millis,
            result
                .attempt
                .nodes_visited
                .map_or_else(|| "not applicable".to_owned(), |n| n.to_string())
        );
    }
    println!(
        "miniF2F: NOT RUN (see decision evidence).\nJSON: {}\nSummary: {}",
        json.display(),
        markdown.display()
    );
    Ok(())
}

fn parse_options() -> io::Result<Option<Options>> {
    let mut options = Options::default();
    let mut args = env::args_os().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            println!(
                "Usage: benchmark [--corpus DIR] [--results DIR] [--lean PATH] [--timeout-ms N] [--minif2f-decision FILE]\n\nDefaults: bench/corpus, results, /root/.elan/bin/lean, 10000 ms, bench/minif2f/decision.txt.\nThe gen0-bench binary checks supplied hints; gen0-search-bench uses the pinned core search engine and fixed budgets recorded in the report."
            );
            return Ok(None);
        }
        let value = args.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("missing value for {}", flag.to_string_lossy()),
            )
        })?;
        match flag.to_str() {
            Some("--corpus") => options.corpus = value.into(),
            Some("--results") => options.results = value.into(),
            Some("--lean") => options.checker = options.checker.with_binary(PathBuf::from(value)),
            Some("--minif2f-decision") => options.decision = value.into(),
            Some("--timeout-ms") => {
                let millis = value
                    .to_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .filter(|n| *n > 0)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "timeout must be a positive integer in milliseconds",
                        )
                    })?;
                options.checker = options.checker.with_timeout(Duration::from_millis(millis));
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown option {}", flag.to_string_lossy()),
                ));
            }
        }
    }
    Ok(Some(options))
}

fn unix_millis() -> io::Result<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(io::Error::other)
}

fn command_output(binary: &OsStr, args: &[&str]) -> io::Result<String> {
    let current_path = env::var_os("PATH").unwrap_or_default();
    let path = env::join_paths(
        [
            PathBuf::from("/root/.cargo/bin"),
            PathBuf::from("/root/.elan/bin"),
        ]
        .into_iter()
        .chain(env::split_paths(&current_path)),
    )
    .map_err(io::Error::other)?;
    let output = Command::new(binary).args(args).env("PATH", path).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "{} {} failed ({}): {}{}",
            binary.to_string_lossy(),
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
