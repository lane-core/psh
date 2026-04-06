//! Variable expansion tests.
//!
//! Derived from ksh93 tests/variables.sh. Tests assignment, $x expansion,
//! scoping, export, list variables, count ($#x), indexing ($x(n)),
//! and the rc-heritage bare assignment model.

use crate::harness::psh;

// ── Basic assignment and expansion ─────────────────────────

#[test]
fn bare_assignment() {
    assert_psh!("x = hello\necho $x", "hello");
}

#[test]
fn assignment_with_spaces() {
    assert_psh!("x = hello world\necho $x", "hello");
}

#[test]
fn assignment_list() {
    assert_psh!("x = (a b c)\necho $x", "a b c");
}

#[test]
fn assignment_overwrite() {
    assert_psh!("x = first\nx = second\necho $x", "second");
}

// ── Variable expansion ─────────────────────────────────────

#[test]
fn expand_undefined_is_empty() {
    assert_psh!("echo $undefined_var_xyz", "");
}

#[test]
fn expand_in_double_context() {
    assert_psh!("x = hello\necho $x world", "hello world");
}

// ── Count ($#x) ────────────────────────────────────────────

#[test]
fn count_list() {
    assert_psh!("x = (a b c)\necho $#x", "3");
}

#[test]
fn count_scalar() {
    assert_psh!("x = hello\necho $#x", "1");
}

#[test]
fn count_empty_list() {
    assert_psh!("x = ()\necho $#x", "0");
}

// ── Indexing ($x(n)) ───────────────────────────────────────

#[test]
fn index_first() {
    assert_psh!("x = (a b c)\necho $x(1)", "a");
}

#[test]
fn index_last() {
    assert_psh!("x = (a b c)\necho $x(3)", "c");
}

#[test]
fn index_out_of_bounds() {
    assert_psh!("x = (a b c)\necho $x(5)", "");
}

// ── Concatenation ──────────────────────────────────────────

#[test]
fn concat_scalars() {
    assert_psh!("x = hello\necho $x^world", "helloworld");
}

#[test]
fn concat_list_pairwise() {
    // rc heritage: (a b)^(x y) = ax by (pairwise, not cross-product)
    assert_psh!("a = (hello good)\nb = (world bye)\necho $a^$b", "helloworld goodbye");
}

// ── Stringify ($") ─────────────────────────────────────────

#[test]
fn stringify_joins_list() {
    assert_psh!("x = (a b c)\necho $\"x", "a b c");
}

#[test]
fn stringify_scalar() {
    assert_psh!("x = hello\necho $\"x", "hello");
}

// ── rc-style lowercase builtins ────────────────────────────

#[test]
fn home_is_set() {
    let r = psh("echo $home");
    assert!(r.success());
    assert!(!r.stdout.trim().is_empty(), "expected $home to be set");
}

#[test]
fn user_is_set() {
    let r = psh("echo $user");
    assert!(r.success());
    assert!(!r.stdout.trim().is_empty(), "expected $user to be set");
}

// ── Export ──────────────────────────────────────────────────

#[test]
fn exported_var_visible_to_child() {
    // Export a variable via let, then run a subprocess that reads the env
    assert_psh!(
        "let export X_TEST_VAR = hello_from_psh\n/usr/bin/env | grep X_TEST_VAR | cat",
        "X_TEST_VAR=hello_from_psh"
    );
}
