//! Integration test harness for psh.
//!
//! Runs psh as a subprocess with `-c` and compares stdout, stderr,
//! and exit code. Modeled on ksh93's shtests/_common pattern, but
//! in Rust so it integrates with cargo test.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// Path to the psh binary (built by cargo).
fn psh_bin() -> PathBuf {
    // cargo test sets this for integration tests
    let mut path = PathBuf::from(env!("CARGO_BIN_EXE_psh"));
    // Sanity: if the env var is empty, fall back to target/debug
    if !path.exists() {
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/debug/psh");
    }
    path
}

/// Result of running a psh command.
pub struct PshResult {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

impl PshResult {
    pub fn success(&self) -> bool {
        self.code == 0
    }
}

/// Run `psh -c 'script'` and capture output.
pub fn psh(script: &str) -> PshResult {
    let output: Output = Command::new(psh_bin())
        .arg("-c")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to execute psh");

    PshResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(-1),
    }
}

/// Run psh with stdin piped in.
pub fn psh_stdin(script: &str, input: &str) -> PshResult {
    use std::io::Write;

    let mut child = Command::new(psh_bin())
        .arg("-c")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to execute psh");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).ok();
    }

    let output = child.wait_with_output().expect("failed to wait on psh");

    PshResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(-1),
    }
}

/// Run a psh script file.
pub fn psh_file(path: &std::path::Path) -> PshResult {
    let output: Output = Command::new(psh_bin())
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to execute psh");

    PshResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(-1),
    }
}

/// Assert stdout matches exactly (trimming trailing newline).
#[macro_export]
macro_rules! assert_psh {
    ($script:expr, $expected:expr) => {{
        let r = $crate::harness::psh($script);
        let got = r.stdout.trim_end_matches('\n');
        assert_eq!(
            got, $expected,
            "\npsh -c {:?}\nstdout: {:?}\nstderr: {:?}\nexit: {}",
            $script, got, r.stderr, r.code
        );
    }};
}

/// Assert psh exits with success (code 0).
#[macro_export]
macro_rules! assert_psh_ok {
    ($script:expr) => {{
        let r = $crate::harness::psh($script);
        assert!(
            r.success(),
            "\npsh -c {:?} failed (exit {})\nstderr: {:?}",
            $script, r.code, r.stderr
        );
    }};
}

/// Assert psh exits with failure (code != 0).
#[macro_export]
macro_rules! assert_psh_fail {
    ($script:expr) => {{
        let r = $crate::harness::psh($script);
        assert!(
            !r.success(),
            "\npsh -c {:?} should have failed but exited 0\nstdout: {:?}",
            $script, r.stdout
        );
    }};
}
