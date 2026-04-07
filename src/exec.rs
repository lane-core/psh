//! Evaluator for psh — to be rebuilt with RunOutcome, try,
//! return, capture_subprocess, thunk forcing, and match semantics.
//!
//! Scrapped and awaiting rewrite. See docs/specification.md for
//! the theoretical foundation and docs/syntax.md for semantics.

use crate::{
    ast::*,
    env::Env,
    job::JobTable,
};

/// Exit status — a string in rc tradition.
#[derive(Debug, Clone, PartialEq)]
pub struct Status(pub String);

impl Status {
    pub fn ok() -> Self {
        Status(String::new())
    }

    pub fn from_code(code: i32) -> Self {
        if code == 0 {
            Status::ok()
        } else {
            Status(code.to_string())
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Status(msg.into())
    }

    pub fn is_success(&self) -> bool {
        self.0.is_empty()
    }
}

/// The shell interpreter state.
pub struct Shell {
    pub env: Env,
    pub jobs: JobTable,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        let mut env = Env::new();
        env.import_process_env();
        Shell {
            env,
            jobs: JobTable::new(),
        }
    }

    /// Set up the shell for interactive use.
    pub fn setup_interactive(&mut self) {
        // stub — awaiting rewrite
    }

    /// Check for pending signals.
    pub fn check_signals(&mut self) {
        // stub — awaiting rewrite
    }

    /// Fire the sigexit artificial signal.
    pub fn fire_sigexit(&mut self) {
        // stub — awaiting rewrite
    }

    /// Print done jobs before prompt.
    pub fn notify_done_jobs(&mut self) {
        // stub — awaiting rewrite
    }

    /// Execute a parsed program.
    pub fn run(&mut self, _program: &Program) -> Status {
        Status::err("evaluator not yet implemented — awaiting rewrite")
    }
}
