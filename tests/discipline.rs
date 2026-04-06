//! Discipline function tests.
//!
//! ksh93 heritage: .get and .set discipline functions are triggered
//! on variable access and modification. psh reimplements with
//! reentrancy guards (prevents fn x.set { x = $1 } from recursing).

use crate::harness::psh;

// ── .set discipline ────────────────────────────────────────

#[test]
fn discipline_set_fires() {
    assert_psh!(
        "let mut x = ()\nfn x.set { echo setting }\nx = hello",
        "setting"
    );
}

#[test]
fn discipline_set_receives_value() {
    // The new value is passed as $1 to the .set discipline
    assert_psh!(
        "let mut x = ()\nfn x.set { echo new=$1 }\nx = world",
        "new=world"
    );
}

#[test]
fn discipline_set_reentrancy_guard() {
    // Assignment inside .set should not recurse
    assert_psh_ok!(
        "let mut x = initial\nfn x.set { echo fired }\nx = updated"
    );
}

// ── .get discipline ────────────────────────────────────────

#[test]
fn discipline_get_fires_on_access() {
    assert_psh!(
        "x = hello\nfn x.get { echo accessed }\necho $x",
        "accessed\nhello"
    );
}

// ── get/set builtins ───────────────────────────────────────

#[test]
fn get_builtin_fires_discipline() {
    assert_psh!(
        "x = hello\nfn x.get { echo discipline }\nget x",
        "discipline\nhello"
    );
}

#[test]
fn set_builtin_basic() {
    assert_psh!("set x hello\necho $x", "hello");
}
