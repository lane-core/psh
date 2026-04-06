//! psh — the pane system shell.
//!
//! rc heritage, ksh93 discipline functions, session-typed internals.
//! A command language for composing a pane system.

pub mod ast;
pub mod env;
pub mod exec;
pub mod lex;
pub mod parse;
pub mod value;

fn main() {
    eprintln!("psh: not yet implemented");
    std::process::exit(1);
}
