//! Checks candidate Lean source in isolated temporary files with a wall-clock timeout.

use crate::checker::{CheckResult, ProofChecker};
use std::env;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Executes Lean with warnings treated as errors and captures its exact diagnostics.
///
/// This is a verifier for trusted Lean input, not an operating-system sandbox. Lean
/// source can execute elaborator code or change options, so untrusted source needs
/// an external sandbox and an independent policy on axioms and imports.
#[derive(Clone, Debug)]
pub struct LeanChecker {
    binary: PathBuf,
    timeout: Duration,
}

impl Default for LeanChecker {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("/root/.elan/bin/lean"),
            timeout: Duration::from_secs(10),
        }
    }
}

impl LeanChecker {
    /// Uses dominic's existing Lean executable with a ten-second timeout.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the wall-clock limit for one subprocess, including its startup.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Selects an existing Lean executable without installing a toolchain.
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Returns the executable used for proof checks and version reporting.
    #[must_use]
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// Returns the configured wall-clock limit for each check.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    fn run(&self, source: &str, started: Instant) -> io::Result<CheckResult> {
        let temporary = TemporaryDirectory::create()?;
        let source_path = temporary.path.join("Candidate.lean");
        let stdout_path = temporary.path.join("stdout");
        let stderr_path = temporary.path.join("stderr");
        create_file(&source_path)?.write_all(source.as_bytes())?;

        // Files prevent either stream from filling a pipe, and a descendant that
        // inherits output handles cannot keep a reader thread blocked after timeout.
        let mut command = Command::new(&self.binary);
        command
            .arg("-DwarningAsError=true")
            .arg(&source_path)
            .env("PATH", toolchain_path()?)
            .stdin(Stdio::null())
            .stdout(Stdio::from(create_file(&stdout_path)?))
            .stderr(Stdio::from(create_file(&stderr_path)?));
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }

        let mut child = command.spawn().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("could not start Lean {}: {error}", self.binary.display()),
            )
        })?;
        let outcome = wait_for_child(&mut child, started, self.timeout);
        let stdout = fs::read(&stdout_path)?;
        let stderr = fs::read(&stderr_path)?;
        let mut result = CheckResult {
            passed: false,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            millis: elapsed_millis(started),
        };
        match outcome {
            Ok(Some(status)) => result.passed = status.success(),
            Ok(None) => append_diagnostic(
                &mut result.stderr,
                &format!("Lean timed out after {} ms", self.timeout.as_millis()),
            ),
            Err(error) => append_diagnostic(&mut result.stderr, &error.to_string()),
        }
        Ok(result)
    }
}

impl ProofChecker for LeanChecker {
    fn check(&self, lean_source: &str) -> CheckResult {
        let started = Instant::now();
        self.run(lean_source, started)
            .unwrap_or_else(|error| CheckResult {
                passed: false,
                stdout: String::new(),
                stderr: format!("Lean checker I/O failure: {error}"),
                millis: elapsed_millis(started),
            })
    }
}

fn elapsed_millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn append_diagnostic(output: &mut String, message: &str) {
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(message);
    output.push('\n');
}

fn toolchain_path() -> io::Result<OsString> {
    let existing = env::var_os("PATH").unwrap_or_default();
    let paths = [
        PathBuf::from("/root/.elan/bin"),
        PathBuf::from("/root/.cargo/bin"),
    ];
    env::join_paths(paths.into_iter().chain(env::split_paths(&existing)))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn wait_for_child(
    child: &mut Child,
    started: Instant,
    timeout: Duration,
) -> io::Result<Option<ExitStatus>> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                #[cfg(unix)]
                terminate_process_group(child.id());
                return Ok(Some(status));
            }
            Ok(None) => {}
            Err(error) => {
                terminate_and_reap(child)?;
                return Err(error);
            }
        }
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            terminate_and_reap(child)?;
            return Ok(None);
        }
        thread::sleep(POLL_INTERVAL.min(timeout - elapsed));
    }
}

fn terminate_and_reap(child: &mut Child) -> io::Result<()> {
    #[cfg(unix)]
    terminate_process_group(child.id());
    // A child can exit between polling and killing. Always reap it in either case.
    let _ = child.kill();
    child.wait().map(|_| ())
}

#[cfg(unix)]
fn terminate_process_group(id: u32) {
    // A wrapper can exit while its children keep running. Clean the group on
    // normal completion as well as timeout; an already-empty group is harmless.
    // The existing system utility avoids adding a dependency or unsafe FFI.
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{id}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn create_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    fn create() -> io::Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        for _ in 0..100 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "lean-proof-check-{}-{timestamp}-{sequence}",
                std::process::id()
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique Lean temporary directory",
        ))
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
