pub struct CheckResult {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
    pub millis: u64,
}

pub trait ProofChecker {
    fn check(&self, lean_source: &str) -> CheckResult;
}
