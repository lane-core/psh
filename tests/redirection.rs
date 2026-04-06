//! Redirection tests.
//!
//! Derived from ksh93 tests/io.sh. Tests output/input redirection,
//! append, fd duplication, and close. Uses temp files.

use crate::harness::psh;
use std::fs;

fn tmp_path(name: &str) -> String {
    let dir = std::env::temp_dir().join("psh_test");
    fs::create_dir_all(&dir).ok();
    dir.join(name).to_string_lossy().to_string()
}

// ── Output redirection ─────────────────────────────────────

#[test]
fn redirect_stdout_to_file() {
    let f = tmp_path("redir_out");
    let script = format!("echo hello > {f}");
    let r = psh(&script);
    assert!(r.success(), "stderr: {}", r.stderr);
    assert_eq!(fs::read_to_string(&f).unwrap().trim(), "hello");
    fs::remove_file(&f).ok();
}

#[test]
fn redirect_append() {
    let f = tmp_path("redir_append");
    let script = format!("echo first > {f}\necho second >> {f}");
    let r = psh(&script);
    assert!(r.success(), "stderr: {}", r.stderr);
    let contents = fs::read_to_string(&f).unwrap();
    assert_eq!(contents.trim(), "first\nsecond");
    fs::remove_file(&f).ok();
}

// ── Input redirection ──────────────────────────────────────

#[test]
fn redirect_stdin_from_file() {
    let f = tmp_path("redir_in");
    fs::write(&f, "hello from file\n").unwrap();
    let script = format!("cat < {f}");
    assert_psh!(&script, "hello from file");
    fs::remove_file(&f).ok();
}

// ── Stderr redirection ─────────────────────────────────────

#[test]
fn redirect_stderr_to_file() {
    let f = tmp_path("redir_err");
    // echo to stderr via fd 2 redirection: print -u2 or redirect
    let script = format!("echo error >[2] {f}");
    let r = psh(&script);
    assert!(r.success(), "stderr: {}", r.stderr);
    assert_eq!(fs::read_to_string(&f).unwrap().trim(), "error");
    fs::remove_file(&f).ok();
}

// ── Fd duplication ─────────────────────────────────────────

#[test]
fn dup_stderr_to_stdout() {
    // Redirect stderr to stdout: >[2=1]
    // This should merge stderr into stdout
    let r = psh("echo hello >[2=1]");
    assert!(r.success());
    // hello should appear on stdout (or both)
    assert!(r.stdout.contains("hello"));
}

// ── Pipeline with redirection ──────────────────────────────

#[test]
fn pipe_and_redirect() {
    let f = tmp_path("redir_pipe");
    let script = format!("echo hello | cat > {f}");
    let r = psh(&script);
    assert!(r.success(), "stderr: {}", r.stderr);
    assert_eq!(fs::read_to_string(&f).unwrap().trim(), "hello");
    fs::remove_file(&f).ok();
}
