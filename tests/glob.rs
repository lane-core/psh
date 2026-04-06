//! Globbing tests.
//!
//! Derived from ksh93 tests/glob.sh. Tests * ? [] glob patterns
//! and recursive globbing.

use crate::harness::psh;
use std::fs;

fn setup_glob_dir() -> String {
    let dir = std::env::temp_dir().join("psh_glob_test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("a.txt"), "").unwrap();
    fs::write(dir.join("b.txt"), "").unwrap();
    fs::write(dir.join("c.log"), "").unwrap();
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::write(dir.join("sub/d.txt"), "").unwrap();
    dir.to_string_lossy().to_string()
}

fn cleanup_glob_dir(dir: &str) {
    let _ = fs::remove_dir_all(dir);
}

// ── Star glob ──────────────────────────────────────────────

#[test]
fn glob_star_txt() {
    let dir = setup_glob_dir();
    let r = psh(&format!("cd {dir}\necho *.txt"));
    assert!(r.success(), "stderr: {}", r.stderr);
    let mut parts: Vec<&str> = r.stdout.trim().split_whitespace().collect();
    parts.sort();
    assert_eq!(parts, vec!["a.txt", "b.txt"]);
    cleanup_glob_dir(&dir);
}

#[test]
fn glob_star_all() {
    let dir = setup_glob_dir();
    let r = psh(&format!("cd {dir}\necho *"));
    assert!(r.success(), "stderr: {}", r.stderr);
    let parts: Vec<&str> = r.stdout.trim().split_whitespace().collect();
    // Should include a.txt, b.txt, c.log, sub (but not . or ..)
    assert!(parts.contains(&"a.txt"));
    assert!(parts.contains(&"c.log"));
    assert!(parts.contains(&"sub"));
    cleanup_glob_dir(&dir);
}

// ── Question mark glob ─────────────────────────────────────

#[test]
fn glob_question() {
    let dir = setup_glob_dir();
    let r = psh(&format!("cd {dir}\necho ?.txt"));
    assert!(r.success(), "stderr: {}", r.stderr);
    let mut parts: Vec<&str> = r.stdout.trim().split_whitespace().collect();
    parts.sort();
    assert_eq!(parts, vec!["a.txt", "b.txt"]);
    cleanup_glob_dir(&dir);
}

// ── Bracket glob ───────────────────────────────────────────

#[test]
fn glob_bracket() {
    let dir = setup_glob_dir();
    let r = psh(&format!("cd {dir}\necho [ab].txt"));
    assert!(r.success(), "stderr: {}", r.stderr);
    let mut parts: Vec<&str> = r.stdout.trim().split_whitespace().collect();
    parts.sort();
    assert_eq!(parts, vec!["a.txt", "b.txt"]);
    cleanup_glob_dir(&dir);
}

// ── No match returns literal ───────────────────────────────

#[test]
fn glob_no_match_literal() {
    let dir = setup_glob_dir();
    let r = psh(&format!("cd {dir}\necho *.xyz"));
    assert!(r.success());
    // rc convention: unmatched glob stays literal
    assert_eq!(r.stdout.trim(), "*.xyz");
    cleanup_glob_dir(&dir);
}
