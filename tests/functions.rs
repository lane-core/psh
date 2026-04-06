//! Function definition and invocation tests.
//!
//! Derived from ksh93 tests/functions.sh. Tests fn definition,
//! arguments ($1, $2, ...), recursion, and interaction with builtins.

use crate::harness::psh;

// ── Basic function definition and call ─────────────────────

#[test]
fn fn_definition_and_call() {
    assert_psh!("fn greet { echo hello }\ngreet", "hello");
}

#[test]
fn fn_with_args() {
    assert_psh!("fn greet { echo hello $1 }\ngreet world", "hello world");
}

#[test]
fn fn_multiple_args() {
    assert_psh!("fn show { echo $1 $2 $3 }\nshow a b c", "a b c");
}

#[test]
fn fn_no_args_positionals_empty() {
    assert_psh!("fn f { echo $1 }\nf", "");
}

// ── Function overriding ────────────────────────────────────

#[test]
fn fn_redefine() {
    assert_psh!("fn f { echo first }\nfn f { echo second }\nf", "second");
}

// ── Function as pipeline element ───────────────────────────

#[test]
fn fn_in_pipeline() {
    assert_psh!("fn upper { cat }\necho hello | upper", "hello");
}

// ── Nested function calls ──────────────────────────────────

#[test]
fn fn_nested_calls() {
    assert_psh!("fn a { echo a }\nfn b { a; echo b }\nb", "a\nb");
}

// ── Builtin keyword bypasses function ──────────────────────

#[test]
fn builtin_bypasses_fn() {
    assert_psh!("fn echo { print OVERRIDE }\nbuiltin echo real", "real");
}

// ── Function exit status ───────────────────────────────────

#[test]
fn fn_returns_last_status() {
    assert_psh_ok!("fn f { true }\nf");
    assert_psh_fail!("fn f { false }\nf");
}
