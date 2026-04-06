//! Quoting tests.
//!
//! Derived from ksh93 tests/quoting.sh. Tests single quotes, double
//! quotes (not in rc — but psh might have them), and backslash
//! escaping. psh follows rc's quoting model: single quotes protect
//! everything, '' inside quotes is a literal quote.

use crate::harness::psh;

// ── Single quotes ──────────────────────────────────────────

#[test]
fn single_quote_literal() {
    assert_psh!("echo 'hello world'", "hello world");
}

#[test]
fn single_quote_preserves_special() {
    assert_psh!("echo '$x'", "$x");
}

#[test]
fn single_quote_preserves_pipe() {
    assert_psh!("echo 'a|b'", "a|b");
}

#[test]
fn single_quote_preserves_semicolon() {
    assert_psh!("echo 'a;b'", "a;b");
}

#[test]
fn single_quote_preserves_ampersand() {
    assert_psh!("echo 'a&b'", "a&b");
}

#[test]
fn single_quote_preserves_parens() {
    assert_psh!("echo '(a b)'", "(a b)");
}

#[test]
fn single_quote_preserves_newline() {
    assert_psh!("echo 'line1\nline2'", "line1\nline2");
}

// ── Quote inside quote (rc convention: '' = literal ') ─────

#[test]
fn rc_quote_escape() {
    // In rc, to get a literal single quote inside single quotes,
    // you close, insert a quoted quote, reopen: 'it''s'
    assert_psh!("echo 'it''s'", "it's");
}

// ── Variable expansion inside quotes ───────────────────────

#[test]
fn quoted_no_expansion() {
    // Single quotes prevent variable expansion
    assert_psh!("x = hello\necho '$x'", "$x");
}

#[test]
fn unquoted_expands() {
    assert_psh!("x = hello\necho $x", "hello");
}

// ── Backslash ──────────────────────────────────────────────

#[test]
fn backslash_escapes_special() {
    assert_psh!("echo hello\\;world", "hello;world");
}

// ── Empty string ───────────────────────────────────────────

#[test]
fn empty_single_quote() {
    assert_psh!("echo ''", "");
}
