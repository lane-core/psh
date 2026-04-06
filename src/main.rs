//! psh — the pane system shell.
//!
//! rc heritage, ksh93 discipline functions, session-typed internals.
//! A command language for composing a pane system.

pub mod ast;
pub mod env;
pub mod exec;
pub mod parse;
pub mod value;

use std::io::{self, BufRead, Write};

use bpaf::Bpaf;

use crate::{exec::Shell, parse::Parser};

/// The pane system shell.
#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
struct Opts {
    /// Execute a command string
    #[bpaf(short('c'), argument("COMMAND"))]
    command: Option<String>,
    /// Script file to execute
    #[bpaf(positional("FILE"))]
    file: Option<String>,
}

fn main() {
    let opts = opts().run();

    let mut shell = Shell::new();

    if let Some(cmd) = opts.command {
        // psh -c 'command'
        match Parser::parse(&cmd) {
            Ok(prog) => {
                let status = shell.run(&prog);
                if !status.is_success() {
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("psh: {e}");
                std::process::exit(2);
            }
        }
    } else if let Some(file) = opts.file {
        // psh file.psh
        match std::fs::read_to_string(&file) {
            Ok(content) => match Parser::parse(&content) {
                Ok(prog) => {
                    let status = shell.run(&prog);
                    if !status.is_success() {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("psh: {file}: {e}");
                    std::process::exit(2);
                }
            },
            Err(e) => {
                eprintln!("psh: {file}: {e}");
                std::process::exit(1);
            }
        }
    } else {
        // Interactive mode (stub — reads lines and executes them)
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        loop {
            eprint!("psh% ");
            let _ = io::stderr().flush();
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    match Parser::parse(&line) {
                        Ok(prog) => {
                            shell.run(&prog);
                        }
                        Err(e) => {
                            eprintln!("psh: {e}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("psh: read error: {e}");
                    break;
                }
            }
        }
    }
}
