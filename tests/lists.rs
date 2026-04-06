//! List operation tests.
//!
//! rc heritage: first-class lists, pairwise concatenation, count,
//! indexing, flattening. Tests the list value model.

use crate::harness::psh;

// ── List creation ──────────────────────────────────────────

#[test]
fn list_literal() {
    assert_psh!("echo (a b c)", "a b c");
}

#[test]
fn list_nested_flattens() {
    // rc: nested lists flatten: ((a b) c) = (a b c)
    assert_psh!("x = (a b)\ny = ($x c)\necho $y", "a b c");
}

// ── List operations ────────────────────────────────────────

#[test]
fn list_count() {
    assert_psh!("x = (a b c d e)\necho $#x", "5");
}

#[test]
fn list_index_1() {
    assert_psh!("x = (a b c)\necho $x(1)", "a");
}

#[test]
fn list_index_2() {
    assert_psh!("x = (a b c)\necho $x(2)", "b");
}

#[test]
fn list_index_3() {
    assert_psh!("x = (a b c)\necho $x(3)", "c");
}

// ── Pairwise concat ────────────────────────────────────────

#[test]
fn concat_pairwise() {
    assert_psh!(
        "a = (hello good)\nb = (world bye)\necho $a^$b",
        "helloworld goodbye"
    );
}

#[test]
fn concat_scalar_with_list() {
    // Scalar broadcasts to each element
    assert_psh!("x = (a b c)\necho pre-^$x", "pre-a pre-b pre-c");
}

// ── Stringify ($") ─────────────────────────────────────────

#[test]
fn stringify_list() {
    assert_psh!("x = (hello world)\necho $\"x", "hello world");
}

// ── Iteration ──────────────────────────────────────────────

#[test]
fn for_over_list() {
    assert_psh!("x = (a b c)\nfor(i in $x) { echo item=$i }", "item=a\nitem=b\nitem=c");
}
