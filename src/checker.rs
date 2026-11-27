//! The shared contract between proof search and a proof-checking implementation.
//!
//! Implementations must only set `passed` when the supplied source is accepted.
//! Preserve diagnostics verbatim, including Lean's unsolved goals. Implementations
//! that launch subprocesses must enforce their own timeout: this synchronous
//! interface cannot interrupt a check that is already running.

pub struct CheckResult { pub passed: bool, pub stdout: String, pub stderr: String, pub millis: u64 }
pub trait ProofChecker { fn check(&self, lean_source: &str) -> CheckResult; }
