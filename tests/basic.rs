//! Basic shell operations.
//!
//! Derived from ksh93 tests/basic.sh. Tests fundamental operations:
//! simple commands, exit codes, pipelines, semicolons, &&/||, negation,
//! background jobs, and the `echo` builtin.

use crate::harness::psh;

// ── Simple commands and exit codes ─────────────────────────

#[test]
fn true_exits_zero() {
    assert_psh_ok!("true");
}

#[test]
fn false_exits_nonzero() {
    assert_psh_fail!("false");
}

#[test]
fn echo_hello() {
    assert_psh!("echo hello", "hello");
}

#[test]
fn echo_multiple_args() {
    assert_psh!("echo hello world", "hello world");
}

#[test]
fn echo_no_args() {
    assert_psh!("echo", "");
}

#[test]
fn echo_n_suppresses_newline() {
    let r = psh("echo -n hello");
    assert_eq!(r.stdout, "hello");
}

// ── Semicolons and sequencing ──────────────────────────────

#[test]
fn semicolons_sequence() {
    assert_psh!("echo a; echo b", "a\nb");
}

#[test]
fn newline_sequences() {
    assert_psh!("echo a\necho b", "a\nb");
}

// ── Pipelines ──────────────────────────────────────────────

#[test]
fn pipe_echo_to_cat() {
    assert_psh!("echo hello | cat", "hello");
}

#[test]
fn pipe_chain() {
    assert_psh!("echo hello world | cat | cat", "hello world");
}

#[test]
fn pipe_exit_code_is_last() {
    assert_psh_ok!("false | true");
    assert_psh_fail!("true | false");
}

// ── Short-circuit operators ────────────────────────────────

#[test]
fn and_both_true() {
    assert_psh_ok!("true && true");
}

#[test]
fn and_first_false() {
    assert_psh_fail!("false && true");
}

#[test]
fn and_second_false() {
    assert_psh_fail!("true && false");
}

#[test]
fn or_both_false() {
    assert_psh_fail!("false || false");
}

#[test]
fn or_first_true() {
    assert_psh_ok!("true || false");
}

#[test]
fn or_second_true() {
    assert_psh_ok!("false || true");
}

#[test]
fn and_short_circuits() {
    // If && short-circuits, the echo should not run
    assert_psh!("false && echo nope", "");
}

#[test]
fn or_short_circuits() {
    // If || short-circuits, the second echo should not run
    assert_psh!("true || echo nope", "");
}

// ── Negation ───────────────────────────────────────────────

#[test]
fn not_true() {
    assert_psh_fail!("! true");
}

#[test]
fn not_false() {
    assert_psh_ok!("! false");
}

// ── Blocks ─────────────────────────────────────────────────

#[test]
fn block_executes() {
    assert_psh!("{ echo hello }", "hello");
}

#[test]
fn block_sequences() {
    assert_psh!("{ echo a; echo b }", "a\nb");
}

// ── Exit builtin ───────────────────────────────────────────

#[test]
fn exit_zero() {
    let r = psh("exit 0");
    assert_eq!(r.code, 0);
}

#[test]
fn exit_nonzero() {
    let r = psh("exit 1");
    assert_eq!(r.code, 1);
}

#[test]
fn exit_arbitrary_code() {
    let r = psh("exit 42");
    assert_eq!(r.code, 42);
}

// ── Print builtin ──────────────────────────────────────────

#[test]
fn print_basic() {
    assert_psh!("print hello", "hello");
}

#[test]
fn print_multiple() {
    assert_psh!("print hello world", "hello world");
}

// ── Builtin command ────────────────────────────────────────

#[test]
fn builtin_bypasses_function() {
    assert_psh!("fn echo { print override }\nbuiltin echo real", "real");
}
