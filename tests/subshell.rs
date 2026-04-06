//! Subshell and command substitution tests.
//!
//! Derived from ksh93 tests/subshell.sh. Tests @{} subshells
//! (isolated namespace) and `{} command substitution.

use crate::harness::psh;

// ── Command substitution ───────────────────────────────────

#[test]
fn comsub_basic() {
    assert_psh!("echo `{echo hello}", "hello");
}

#[test]
fn comsub_in_assignment() {
    assert_psh!("x = `{echo world}\necho $x", "world");
}

#[test]
fn comsub_strips_trailing_newline() {
    assert_psh!("x = `{echo hello}\necho $x", "hello");
}

#[test]
fn comsub_nested() {
    assert_psh!("echo `{echo `{echo nested}}", "nested");
}

#[test]
fn comsub_in_args() {
    assert_psh!("echo hello `{echo world}", "hello world");
}

// ── Subshell (@{}) ─────────────────────────────────────────

#[test]
fn subshell_isolates_vars() {
    assert_psh!(
        "x = outer\n@{ x = inner; echo $x }\necho $x",
        "inner\nouter"
    );
}

#[test]
fn subshell_exit_code() {
    assert_psh_ok!("@{ true }");
    assert_psh_fail!("@{ false }");
}

// ── Process substitution ───────────────────────────────────

#[test]
fn process_sub() {
    // <{cmd} evaluates to /dev/fd/N
    assert_psh!("cat <{echo hello}", "hello");
}
