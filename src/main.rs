//! psh — the pane system shell.
//!
//! rc heritage, ksh93 discipline functions, VDC-grounded internals.
//!
//! The prior implementation was retired during the VDC reframing
//! (see docs/spec/ and docs/vdc-framework.md). This
//! binary is currently a stub that reports the status and exits;
//! the parser infrastructure lives in parse.rs and the signal
//! self-pipe lives in signal.rs, ready for the next implementation
//! round to build on.

pub mod parse;
pub mod signal;

use bpaf::Bpaf;

/// The pane system shell.
#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
#[allow(dead_code)]
struct Opts {
    /// Execute a command string
    #[bpaf(short('c'), argument("COMMAND"))]
    command: Option<String>,
    /// Script file to execute
    #[bpaf(positional("FILE"))]
    file: Option<String>,
}

fn main() {
    let _opts = opts().run();
    eprintln!(
        "psh: not yet implemented — the prior grammar, AST, and \
         evaluator were retired during the VDC reframing. See \
         docs/spec/ for the current design."
    );
    std::process::exit(2);
}
