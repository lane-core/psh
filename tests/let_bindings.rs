//! let binding tests.
//!
//! psh-specific: typed let bindings with mut, export, type annotations,
//! Prism validation. No ksh93 equivalent — this is new in psh.

use crate::harness::psh;

// ── Basic let ──────────────────────────────────────────────

#[test]
fn let_basic() {
    assert_psh!("let x = hello\necho $x", "hello");
}

#[test]
fn let_int_inference() {
    assert_psh!("let x = 42\nwhatis x", "x : Int = 42");
}

#[test]
fn let_bool_inference() {
    assert_psh!("let x = true\nwhatis x", "x : Bool = true");
}

#[test]
fn let_path_inference() {
    assert_psh!("let x = /tmp\nwhatis x", "x : Path = /tmp");
}

#[test]
fn let_str_inference() {
    assert_psh!("let x = hello\nwhatis x", "x : Str = hello");
}

// ── Immutability ───────────────────────────────────────────

#[test]
fn let_immutable_rejects_reassign() {
    assert_psh_fail!("let x = 42\nx = 99");
}

#[test]
fn let_mut_allows_reassign() {
    assert_psh_ok!("let mut x = 42\nx = 99");
}

#[test]
fn let_mut_updates_value() {
    assert_psh!("let mut x = first\nx = second\necho $x", "second");
}

// ── Type annotations ───────────────────────────────────────

#[test]
fn let_typed_int_accepts() {
    assert_psh_ok!("let x : Int = 42");
}

#[test]
fn let_typed_int_rejects() {
    assert_psh_fail!("let x : Int = hello");
}

#[test]
fn let_typed_bool_accepts() {
    assert_psh_ok!("let x : Bool = true");
}

#[test]
fn let_typed_bool_rejects() {
    assert_psh_fail!("let x : Bool = 42");
}

#[test]
fn let_typed_path_accepts() {
    assert_psh_ok!("let x : Path = /tmp");
}

#[test]
fn let_typed_path_rejects() {
    assert_psh_fail!("let x : Path = hello");
}

// ── List types ─────────────────────────────────────────────

#[test]
fn let_list_typed() {
    assert_psh_ok!("let x : List[Int] = (1 2 3)");
}

#[test]
fn let_list_typed_rejects_mixed() {
    assert_psh_fail!("let x : List[Int] = (1 hello)");
}

#[test]
fn let_bracket_sugar() {
    assert_psh_ok!("let x : [Int] = (1 2 3)");
}

// ── Export ──────────────────────────────────────────────────

#[test]
fn let_export() {
    assert_psh!(
        "let export X_PSH_TEST = hello\n/usr/bin/env | grep X_PSH_TEST | cat",
        "X_PSH_TEST=hello"
    );
}

#[test]
fn let_mut_export() {
    assert_psh!(
        "let mut export X_PSH_MUT = hello\nX_PSH_MUT = updated\n/usr/bin/env | grep X_PSH_MUT | cat",
        "X_PSH_MUT=updated"
    );
}

// ── Quoting prevents inference ─────────────────────────────

#[test]
fn let_quoted_stays_str() {
    // '42' stays Str even in let context
    assert_psh!("let x = '42'\nwhatis x", "x : Str = 42");
}

#[test]
fn let_leading_zero_stays_str() {
    // 042 has leading zero — stays Str (not octal)
    assert_psh!("let x = 042\nwhatis x", "x : Str = 042");
}

// ── Bare assignment stays Str ──────────────────────────────

#[test]
fn bare_assignment_stays_str() {
    // rc heritage: x = 42 stays Str, no inference
    assert_psh!("x = 42\nwhatis x", "x : Str = 42");
}
