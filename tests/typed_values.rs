//! Typed value tests.
//!
//! psh-specific: tests the Val enum (Unit, Bool, Int, Str, Path, List),
//! type coercion, and Prism validation at boundaries.

use crate::harness::psh;

// ── Type inference in let context ──────────────────────────

#[test]
fn int_42() {
    assert_psh!("let x = 42\nwhatis x", "x : Int = 42");
}

#[test]
fn int_negative() {
    assert_psh!("let x = -5\nwhatis x", "x : Int = -5");
}

#[test]
fn bool_true() {
    assert_psh!("let x = true\nwhatis x", "x : Bool = true");
}

#[test]
fn bool_false() {
    assert_psh!("let x = false\nwhatis x", "x : Bool = false");
}

#[test]
fn path_absolute() {
    assert_psh!("let x = /usr/bin\nwhatis x", "x : Path = /usr/bin");
}

#[test]
fn path_relative_dot() {
    assert_psh!("let x = ./foo\nwhatis x", "x : Path = ./foo");
}

#[test]
fn str_plain() {
    assert_psh!("let x = hello\nwhatis x", "x : Str = hello");
}

// ── Count returns Int ──────────────────────────────────────

#[test]
fn count_returns_int() {
    assert_psh!("x = (a b c)\nlet n = $#x\nwhatis n", "n : Int = 3");
}

// ── List value ─────────────────────────────────────────────

#[test]
fn list_basic() {
    assert_psh!("let x = (a b c)\necho $x", "a b c");
}

#[test]
fn list_of_ints() {
    assert_psh!("let x : [Int] = (1 2 3)\necho $x", "1 2 3");
}

// ── Empty list ─────────────────────────────────────────────

#[test]
fn empty_list() {
    assert_psh!("x = ()\necho $#x", "0");
}
