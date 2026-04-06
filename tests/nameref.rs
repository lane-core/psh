//! Nameref tests.
//!
//! Derived from ksh93 tests/nameref.sh. Tests ref (psh's nameref):
//! ref x = target makes x an alias that resolves through target.

use crate::harness::psh;

// ── Basic nameref ──────────────────────────────────────────

#[test]
fn ref_basic() {
    assert_psh!("x = hello\nref y = x\necho $y", "hello");
}

#[test]
fn ref_follows_updates() {
    assert_psh!("x = first\nref y = x\nx = second\necho $y", "second");
}

#[test]
fn ref_write_through() {
    // Writing to the ref should update the target
    assert_psh!("let mut x = hello\nref y = x\ny = world\necho $x", "world");
}

// ── Ref to list ────────────────────────────────────────────

#[test]
fn ref_to_list() {
    assert_psh!("x = (a b c)\nref y = x\necho $y", "a b c");
}

#[test]
fn ref_count_through() {
    assert_psh!("x = (a b c)\nref y = x\necho $#y", "3");
}

// ── Ref in function ────────────────────────────────────────

#[test]
fn ref_in_function() {
    assert_psh!(
        "fn show { ref v = $1; echo $v }\nx = hello\nshow x",
        "hello"
    );
}
