//! Corpus loading, measured proof checks, and dependency-free benchmark reports.

use crate::checker::{CheckResult, ProofChecker};
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// A theorem declaration and its supplied reference tactic.
#[derive(Debug, Clone)]
pub struct Theorem {
    pub name: String,
    pub path: PathBuf,
    pub statement: String,
    pub expected_tactic: String,
}

impl Theorem {
    /// Render a candidate without importing any theorem libraries.
    pub fn source(&self, tactic: &str) -> String {
        let body = tactic
            .lines()
            .map(|line| format!("  {line}\n"))
            .collect::<String>();
        format!("{} := by\n{body}", self.statement)
    }
}

/// Load `.lean` files in deterministic filename order; reject malformed inputs.
///
/// # Errors
/// Returns an I/O or invalid-data error for unreadable, empty, or malformed corpora.
pub fn load_corpus(directory: &Path) -> io::Result<Vec<Theorem>> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "lean")
    });
    paths.sort();
    if paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "corpus has no .lean files",
        ));
    }
    paths
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            parse_theorem(path, &source)
        })
        .collect()
}

fn parse_theorem(path: PathBuf, source: &str) -> io::Result<Theorem> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: expected one theorem with a matching -- tactic: hint",
                path.display()
            ),
        )
    };
    let (hint, declaration) = source.split_once('\n').ok_or_else(invalid)?;
    let tactic = hint
        .strip_prefix("-- tactic: ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(invalid)?;
    let (statement, body) = declaration.split_once(":= by").ok_or_else(invalid)?;
    let statement = statement.trim();
    let name = statement
        .strip_prefix("theorem ")
        .and_then(|s| s.split_whitespace().next())
        .ok_or_else(invalid)?;
    let file_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(invalid)?;
    if body.trim() != tactic || name != file_name {
        return Err(invalid());
    }
    Ok(Theorem {
        name: name.to_owned(),
        path,
        statement: statement.to_owned(),
        expected_tactic: tactic.to_owned(),
    })
}

/// Raw Lean output, retained verbatim including any emitted goal states.
#[derive(Debug)]
pub struct Diagnostic {
    pub stdout: String,
    pub stderr: String,
    pub millis: u64,
}

impl From<CheckResult> for Diagnostic {
    fn from(result: CheckResult) -> Self {
        Self {
            stdout: result.stdout,
            stderr: result.stderr,
            millis: result.millis,
        }
    }
}

/// One executed candidate or search, with no inferred node counts.
#[derive(Debug)]
pub struct Attempt {
    pub proved: bool,
    pub millis: u64,
    pub nodes_visited: Option<usize>,
    pub tactic_used: Option<String>,
    pub proof_source: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub failures: Vec<String>,
}

/// A measured attempt and, on failure, a separately timed initial-goal probe.
#[derive(Debug)]
pub struct TheoremResult {
    pub theorem: Theorem,
    pub attempt: Attempt,
    pub initial_goal: Option<Diagnostic>,
}

/// Run a caller-supplied backend and retain exact diagnostics for failures.
///
/// The callback must return only actual executed work. Initial-goal probes are
/// separate Lean checks and are never included in the proof count or node count.
pub fn run_with(
    checker: &dyn ProofChecker,
    corpus: &[Theorem],
    mut run: impl FnMut(&Theorem) -> Attempt,
) -> Vec<TheoremResult> {
    corpus
        .iter()
        .map(|theorem| {
            let attempt = run(theorem);
            let initial_goal = (!attempt.proved).then(|| {
                checker
                    .check(&theorem.source("trace_state\nfail \"benchmark initial goal probe\""))
                    .into()
            });
            TheoremResult {
                theorem: theorem.clone(),
                attempt,
                initial_goal,
            }
        })
        .collect()
}

/// Verify supplied corpus hints directly; this does not measure proof discovery.
pub fn run_direct(checker: &dyn ProofChecker, corpus: &[Theorem]) -> Vec<TheoremResult> {
    run_with(checker, corpus, |theorem| {
        let source = theorem.source(&theorem.expected_tactic);
        let result = checker.check(&source);
        Attempt {
            proved: result.passed,
            millis: result.millis,
            nodes_visited: None,
            tactic_used: Some(theorem.expected_tactic.clone()),
            proof_source: result.passed.then_some(source),
            stdout: result.stdout,
            stderr: result.stderr,
            failures: Vec::new(),
        }
    })
}

/// Provenance and actual measurements from a single completed benchmark run.
#[derive(Debug)]
pub struct Report {
    pub started_unix_ms: u128,
    pub finished_unix_ms: u128,
    pub execution_path: String,
    pub source_revision: String,
    pub checker_timeout_ms: u128,
    pub toolchains: Vec<(String, String)>,
    pub minif2f_decision: String,
    pub results: Vec<TheoremResult>,
}

impl Report {
    /// Number of executed theorem attempts accepted by the checker.
    pub fn proved(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.attempt.proved)
            .count()
    }

    /// Encode valid JSON without third-party serialization libraries.
    pub fn json(&self) -> String {
        let versions = self
            .toolchains
            .iter()
            .map(|(tool, version)| format!("{}:{}", json_string(tool), json_string(version)))
            .collect::<Vec<_>>()
            .join(",");
        let records = self
            .results
            .iter()
            .map(record_json)
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            "{{\n\"schema_version\":1,\n\"started_unix_ms\":{},\n\"finished_unix_ms\":{},\n\"execution_path\":{},\n\"source_revision\":{},\n\"checker_timeout_ms\":{},\n\"toolchains\":{{{versions}}},\n\"minif2f\":{{\"status\":\"NOT RUN\",\"executed\":0,\"decision\":{}}},\n\"proved\":{},\n\"total\":{},\n\"results\":[\n{records}\n]\n}}\n",
            self.started_unix_ms,
            self.finished_unix_ms,
            json_string(&self.execution_path),
            json_string(&self.source_revision),
            self.checker_timeout_ms,
            json_string(&self.minif2f_decision),
            self.proved(),
            self.results.len(),
        )
    }

    /// Render the complete corpus and timings, plus raw failure evidence.
    pub fn markdown(&self) -> String {
        let mut output = format!(
            "# Generation 0 benchmark\n\nExecution path: `{}`.\n\nProved: **{} / {}** executed local theorems.\n\nStarted/finished (Unix ms): {} / {}.\nSource revision: `{}`. Checker timeout: {} ms.\n\nDirect LeanChecker runs verify supplied hints; they are not search scores.\nMissing node counts mean search was not used. Failure goal probes have separate timings.\n\n## Toolchains\n\n",
            self.execution_path,
            self.proved(),
            self.results.len(),
            self.started_unix_ms,
            self.finished_unix_ms,
            self.source_revision,
            self.checker_timeout_ms,
        );
        for (tool, version) in &self.toolchains {
            writeln!(output, "- {tool}: `{}`", version.replace('`', "\\`"))
                .expect("writing a String cannot fail");
        }
        output.push_str("\n## Corpus results\n\n| Theorem | Proved | Time (ms) | Nodes visited | Tactic used |\n| --- | --- | ---: | ---: | --- |\n");
        for result in &self.results {
            let attempt = &result.attempt;
            let nodes = attempt
                .nodes_visited
                .map_or_else(|| "—".to_owned(), |n| n.to_string());
            let tactic = attempt
                .tactic_used
                .as_deref()
                .unwrap_or("—")
                .replace('|', "\\|")
                .replace('\n', "<br>");
            writeln!(
                output,
                "| {} | {} | {} | {} | {} |",
                result.theorem.name, attempt.proved, attempt.millis, nodes, tactic
            )
            .expect("writing a String cannot fail");
        }
        output.push_str("\n## miniF2F — NOT RUN\n\n");
        output.push_str(&self.minif2f_decision);
        output.push('\n');
        for result in self.results.iter().filter(|result| !result.attempt.proved) {
            writeln!(output, "\n## Failure: {}\n", result.theorem.name)
                .expect("writing a String cannot fail");
            quote_diagnostic(&mut output, "Candidate stdout", &result.attempt.stdout);
            quote_diagnostic(&mut output, "Candidate stderr", &result.attempt.stderr);
            for failure in &result.attempt.failures {
                quote_diagnostic(&mut output, "Search failure", failure);
            }
            if let Some(goal) = &result.initial_goal {
                writeln!(
                    output,
                    "Initial-goal probe ({} ms; separate from candidate time):\n",
                    goal.millis
                )
                .expect("writing a String cannot fail");
                quote_diagnostic(&mut output, "Initial-goal stdout", &goal.stdout);
                quote_diagnostic(&mut output, "Initial-goal stderr", &goal.stderr);
            }
        }
        output.truncate(output.trim_end_matches('\n').len());
        output.push('\n');
        output
    }

    /// Write a unique timestamped JSON/Markdown pair without replacing old runs.
    ///
    /// # Errors
    /// Returns file errors, including collisions. A partial pair is not hidden.
    pub fn write(&self, directory: &Path) -> io::Result<(PathBuf, PathBuf)> {
        fs::create_dir_all(directory)?;
        let stem = format!("gen0-{}-{}", self.started_unix_ms, std::process::id());
        let json = directory.join(format!("{stem}.json"));
        let markdown = directory.join(format!("{stem}.md"));
        write_new(&json, self.json().as_bytes())?;
        write_new(&markdown, self.markdown().as_bytes())?;
        Ok((json, markdown))
    }
}

fn write_new(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

fn record_json(result: &TheoremResult) -> String {
    let theorem = &result.theorem;
    let attempt = &result.attempt;
    let proof = attempt
        .proof_source
        .as_deref()
        .map_or_else(|| "null".to_owned(), json_string);
    let nodes = attempt
        .nodes_visited
        .map_or_else(|| "null".to_owned(), |n| n.to_string());
    let tactic = attempt
        .tactic_used
        .as_deref()
        .map_or_else(|| "null".to_owned(), json_string);
    let failures = attempt
        .failures
        .iter()
        .map(|s| json_string(s))
        .collect::<Vec<_>>()
        .join(",");
    let goal = result.initial_goal.as_ref().map_or_else(
        || "null".to_owned(),
        |goal| {
            format!(
                "{{\"stdout\":{},\"stderr\":{},\"millis\":{}}}",
                json_string(&goal.stdout),
                json_string(&goal.stderr),
                goal.millis
            )
        },
    );
    format!(
        "{{\"name\":{},\"path\":{},\"statement\":{},\"expected_tactic\":{},\"proved\":{},\"millis\":{},\"nodes_visited\":{nodes},\"tactic_used\":{tactic},\"proof_source\":{proof},\"stdout\":{},\"stderr\":{},\"failures\":[{failures}],\"initial_goal\":{goal}}}",
        json_string(&theorem.name),
        json_string(&theorem.path.to_string_lossy()),
        json_string(&theorem.statement),
        json_string(&theorem.expected_tactic),
        attempt.proved,
        attempt.millis,
        json_string(&attempt.stdout),
        json_string(&attempt.stderr)
    )
}

fn quote_diagnostic(output: &mut String, label: &str, diagnostic: &str) {
    writeln!(output, "{label}:\n").expect("writing a String cannot fail");
    for line in diagnostic.lines() {
        if !line.is_empty() {
            output.push_str("    ");
            output.push_str(line);
        }
        output.push('\n');
    }
    output.push('\n');
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                write!(output, "\\u{:04x}", ch as u32).expect("writing a String cannot fail")
            }
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}
