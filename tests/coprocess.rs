//! Coprocess tests.
//!
//! Derived from ksh93 tests/coprocess.sh. Tests the |& coprocess
//! operator, read -p, and print -p for bidirectional communication.

use crate::harness::psh;

// ── Basic coprocess ────────────────────────────────────────

#[test]
fn coproc_cat() {
    assert_psh!(
        "cat |&\nprint -p hello\nread -p line\necho $line",
        "hello"
    );
}

#[test]
fn coproc_echo() {
    assert_psh!(
        "echo hello |&\nread -p line\necho $line",
        "hello"
    );
}
