//! Here-document and here-string tests.
//!
//! Derived from ksh93 tests/heredoc.sh. Tests <<EOF here-documents
//! and <<<word here-strings.

use crate::harness::psh;

// ── Here-documents ─────────────────────────────────────────

#[test]
fn heredoc_basic() {
    assert_psh!("cat <<EOF\nhello world\nEOF", "hello world");
}

#[test]
fn heredoc_multiline() {
    assert_psh!("cat <<EOF\nline1\nline2\nline3\nEOF", "line1\nline2\nline3");
}

#[test]
fn heredoc_preserves_spaces() {
    assert_psh!("cat <<EOF\n  indented\nEOF", "  indented");
}

#[test]
fn heredoc_variable_expansion() {
    assert_psh!("x = hello\ncat <<EOF\n$x world\nEOF", "hello world");
}

#[test]
fn heredoc_quoted_no_expansion() {
    // Quoting the delimiter suppresses expansion
    assert_psh!("x = hello\ncat <<'EOF'\n$x world\nEOF", "$x world");
}

#[test]
fn heredoc_empty() {
    assert_psh!("cat <<EOF\nEOF", "");
}

// ── Here-strings ───────────────────────────────────────────

#[test]
fn herestring_basic() {
    assert_psh!("cat <<<hello", "hello");
}

#[test]
fn herestring_with_expansion() {
    assert_psh!("x = world\ncat <<<$x", "world");
}
