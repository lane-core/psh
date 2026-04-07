//! psh — the pane system shell.
//!
//! rc heritage, ksh93 discipline functions, session-typed internals.
//! A command language for composing a pane system.

pub mod ast;
pub mod env;
pub mod exec;
pub mod job;
pub mod parse;
pub mod signal;
pub mod value;

use std::io::{self, BufRead, Write};

use bpaf::Bpaf;

use crate::{exec::Shell, parse::PshParser};

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
        signal::install_handlers();
        match PshParser::parse(&cmd) {
            Ok(prog) => {
                let status = shell.run(&prog);
                shell.check_signals();
                shell.fire_sigexit();
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
        signal::install_handlers();
        match std::fs::read_to_string(&file) {
            Ok(content) => match PshParser::parse(&content) {
                Ok(prog) => {
                    let status = shell.run(&prog);
                    shell.check_signals();
                    shell.fire_sigexit();
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
        // Interactive mode
        shell.setup_interactive();
        signal::install_handlers();

        let stdin = io::stdin();
        let mut reader = stdin.lock();
        loop {
            // Report completed background jobs before prompting.
            shell.notify_done_jobs();

            eprint!("psh% ");
            let _ = io::stderr().flush();
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if line.trim().is_empty() {
                        shell.check_signals();
                        continue;
                    }
                    match PshParser::parse(&line) {
                        Ok(prog) => {
                            shell.run(&prog);
                        }
                        Err(e) => {
                            eprintln!("psh: {e}");
                        }
                    }
                    shell.check_signals();
                }
                Err(e) => {
                    // EINTR from SIGINT — cancel input line, re-prompt
                    if e.kind() == io::ErrorKind::Interrupted {
                        eprintln!();
                        continue;
                    }
                    eprintln!("psh: read error: {e}");
                    break;
                }
            }
        }
        shell.fire_sigexit();
    }
}
