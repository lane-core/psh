//! Control flow tests.
//!
//! Tests if/else, for, while, switch (rc's case). Derived from
//! ksh93 tests/case.sh and tests/loop.sh, adapted for rc grammar.

use crate::harness::psh;

// ── if/else ────────────────────────────────────────────────

#[test]
fn if_true() {
    assert_psh!("if(true) { echo yes }", "yes");
}

#[test]
fn if_false() {
    assert_psh!("if(false) { echo yes }", "");
}

#[test]
fn if_else() {
    assert_psh!("if(false) { echo yes } else { echo no }", "no");
}

#[test]
fn if_pipeline_condition() {
    assert_psh!("if(echo test | cat) { echo ok }", "ok");
}

#[test]
fn if_nested() {
    assert_psh!(
        "if(true) { if(false) { echo inner } else { echo outer } }",
        "outer"
    );
}

// ── for ────────────────────────────────────────────────────

#[test]
fn for_list() {
    assert_psh!("for(x in a b c) { echo $x }", "a\nb\nc");
}

#[test]
fn for_empty_list() {
    assert_psh!("for(x in) { echo $x }", "");
}

#[test]
fn for_single() {
    assert_psh!("for(x in hello) { echo $x }", "hello");
}

#[test]
fn for_variable_list() {
    assert_psh!("items = (x y z)\nfor(i in $items) { echo $i }", "x\ny\nz");
}

// ── while ──────────────────────────────────────────────────

#[test]
fn while_countdown() {
    // Use a mutable variable and decrement
    assert_psh!(
        "let mut n = 3\nwhile(test $n -gt 0) { echo $n; n = `{ expr $n - 1 } }",
        "3\n2\n1"
    );
}

#[test]
fn while_false_never_runs() {
    assert_psh!("while(false) { echo nope }", "");
}

// ── switch ─────────────────────────────────────────────────

#[test]
fn switch_exact_match() {
    assert_psh!(
        "x = hello\nswitch($x) { case hello { echo matched } }",
        "matched"
    );
}

#[test]
fn switch_no_match() {
    assert_psh!(
        "x = hello\nswitch($x) { case world { echo nope } }",
        ""
    );
}

#[test]
fn switch_glob_pattern() {
    assert_psh!(
        "x = hello\nswitch($x) { case h* { echo globbed } }",
        "globbed"
    );
}

#[test]
fn switch_multiple_cases() {
    assert_psh!(
        "x = b\nswitch($x) { case a { echo first } case b { echo second } case c { echo third } }",
        "second"
    );
}

#[test]
fn switch_star_default() {
    assert_psh!(
        "x = zzz\nswitch($x) { case a { echo nope } case * { echo default } }",
        "default"
    );
}

#[test]
fn switch_first_match_wins() {
    assert_psh!(
        "x = hello\nswitch($x) { case h* { echo first } case hello { echo second } }",
        "first"
    );
}
