#[path = "../src/checker.rs"]
mod checker;
#[path = "../src/lean.rs"]
mod lean;

use checker::ProofChecker;
use lean::LeanChecker;
use std::time::{Duration, Instant};

#[test]
fn valid_proof_is_accepted_by_real_lean() {
    let checker = LeanChecker::new();
    assert_eq!(checker.timeout(), Duration::from_secs(10));
    assert_eq!(
        checker.binary(),
        std::path::Path::new("/root/.elan/bin/lean")
    );
    let result = checker.check("theorem identity (p : Prop) : p → p := by intro h; exact h\n");
    assert!(result.passed, "{}\n{}", result.stdout, result.stderr);
}

#[test]
fn incomplete_proof_preserves_exact_goal_diagnostics() {
    let result = LeanChecker::new().check("theorem unfinished (p : Prop) : p → p := by intro h\n");
    assert!(!result.passed, "an unfinished proof must fail");
    assert!(
        result.stdout.contains("unsolved goals"),
        "{}",
        result.stdout
    );
    assert!(result.stdout.contains("h : p"), "{}", result.stdout);
    assert!(result.stdout.contains("⊢ p"), "{}", result.stdout);
}

#[test]
fn sorry_and_admit_are_errors_from_lean() {
    for tactic in ["sorry", "admit"] {
        let result = LeanChecker::new().check(&format!("theorem unsound : False := by {tactic}\n"));
        assert!(!result.passed, "{tactic} must not count as a proof");
        assert!(result.stdout.contains("error:"), "{}", result.stdout);
        assert!(result.stdout.contains("sorry"), "{}", result.stdout);
    }
}

#[test]
fn missing_executable_is_a_failed_check() {
    let result = LeanChecker::new()
        .with_binary("/no-such-lean-checker-test-directory/lean")
        .check("theorem valid : True := by trivial\n");
    assert!(!result.passed);
    assert!(
        result.stderr.contains("could not start Lean"),
        "{}",
        result.stderr
    );
}

#[test]
fn simultaneous_checks_use_independent_sources() {
    std::thread::scope(|scope| {
        let good = scope.spawn(|| LeanChecker::new().check("theorem t : 1 = 1 := by rfl\n"));
        let bad = scope.spawn(|| LeanChecker::new().check("theorem t : 1 = 2 := by rfl\n"));
        assert!(good.join().expect("valid check thread panicked").passed);
        assert!(!bad.join().expect("invalid check thread panicked").passed);
    });
}

#[cfg(unix)]
mod subprocess {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_SCRIPT: AtomicU64 = AtomicU64::new(0);

    struct Script {
        path: PathBuf,
    }

    impl Script {
        fn new(body: &str) -> Self {
            let number = NEXT_SCRIPT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "lean-checker-test-{}-{number}.sh",
                std::process::id()
            ));
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)
                .expect("create test script");
            use std::io::Write;
            write!(file, "#!/bin/sh\n{body}\n").expect("write test script");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .expect("set test script permissions");
            Self { path }
        }

        fn checker(&self) -> LeanChecker {
            LeanChecker::new().with_binary(&self.path)
        }
    }

    impl Drop for Script {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
            let _ = fs::remove_file(self.path.with_extension("sh.marker"));
        }
    }

    #[test]
    fn nonzero_exit_cannot_be_overridden_by_success_text() {
        let script = Script::new("printf 'passed=true\\n'; printf 'failure detail\\n' >&2; exit 7");
        let result = script.checker().check("ignored by mock");
        assert!(!result.passed);
        assert_eq!(result.stdout, "passed=true\n");
        assert_eq!(result.stderr, "failure detail\n");
    }

    #[test]
    fn streams_larger_than_pipe_buffers_are_captured_and_temporary_source_is_removed() {
        let script = Script::new(
            "printf '%s\\n' \"$2\"; i=0; while [ \"$i\" -lt 10000 ]; do printf 'stdout-line\\n'; printf 'stderr-line\\n' >&2; i=$((i+1)); done",
        );
        let result = script
            .checker()
            .check("source retained until process exits");
        assert!(result.passed, "{}", result.stderr);
        assert!(result.stdout.len() > 100_000);
        assert!(result.stderr.len() > 100_000);
        let path = result.stdout.lines().next().expect("source path in output");
        assert!(
            !std::path::Path::new(path).exists(),
            "temporary source leaked: {path}"
        );
    }

    #[test]
    fn timeout_kills_process_and_preserves_output() {
        let script = Script::new("printf '%s\\n' \"$2\"; printf 'started\\n' >&2; exec sleep 20");
        let started = Instant::now();
        let result = script
            .checker()
            .with_timeout(Duration::from_millis(100))
            .check("ignored by mock");
        assert!(!result.passed);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "timeout did not bound the check"
        );
        assert!(
            result.millis >= 100,
            "reported duration: {} ms",
            result.millis
        );
        assert!(result.stderr.contains("started"), "{}", result.stderr);
        assert!(
            result.stderr.contains("timed out after 100 ms"),
            "{}",
            result.stderr
        );
        let path = result.stdout.lines().next().expect("source path in output");
        assert!(
            !std::path::Path::new(path).exists(),
            "temporary source leaked: {path}"
        );
    }

    #[test]
    fn exited_wrapper_does_not_leave_background_work_running() {
        let script = Script::new("(sleep 0.3; printf leaked > \"$0.marker\") &\nexit 0");
        let result = script.checker().check("ignored by mock");
        assert!(result.passed, "{}", result.stderr);
        std::thread::sleep(Duration::from_millis(600));
        assert!(
            !script.path.with_extension("sh.marker").exists(),
            "a descendant survived the completed check"
        );
    }
}
