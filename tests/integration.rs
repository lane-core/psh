//! Integration tests for psh.
//!
//! Each module exercises one feature area, inspired by ksh93u+m's
//! test suite but written for psh's rc heritage and typed value model.
//! Tests run psh as a subprocess — they test the shell as users see it.

#[macro_use]
mod harness;

mod basic;
mod control_flow;
mod coprocess;
mod discipline;
mod functions;
mod glob;
mod heredoc;
mod let_bindings;
mod lists;
mod nameref;
mod quoting;
mod redirection;
mod subshell;
mod typed_values;
mod variables;
