//! Evaluator for psh.
//!
//! Implements the full spec: RunOutcome, try/return, capture_subprocess,
//! thunk forcing, match semantics, discipline functions, job control,
//! and profunctor redirections.
//!
//! rc heritage: status strings, list-valued variables, newline splitting.
//! ksh93 heritage: discipline functions, coprocesses, namerefs.
//! Theoretical basis: duploid-structured CBV/CBN split, ⊕ error convention.

use std::{collections::HashSet, ffi::CString, os::unix::io::RawFd};

use crate::{
    ast::*,
    env::Env,
    job::{Job, JobStatus, JobTable},
    signal,
    value::Val,
};

// ── Status ─────────────────────────────────────────────────────

/// Exit status — a string in rc tradition.
/// Empty string = success; any other string = failure.
/// Plan 9 heritage: "On Plan 9 status is a character string
/// describing an error condition" (Duff 1990).
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

    pub fn code(&self) -> i32 {
        if self.is_success() {
            0
        } else {
            self.0.parse().unwrap_or(1)
        }
    }
}

// ── RunOutcome ─────────────────────────────────────────────────

/// The return type of run_cmd / run_cmds.
///
/// CBPV's distinction made explicit: Status is a normal command
/// completion (negative sort), Value is a return-injected value
/// (positive sort, CBPV's return : A → F(A)).
#[derive(Debug, Clone)]
pub enum RunOutcome {
    /// Normal command completion.
    Status(Status),
    /// A `return` injected a value from command sort into value sort.
    Value(Val),
}

impl RunOutcome {
    pub fn ok() -> Self {
        RunOutcome::Status(Status::ok())
    }

    pub fn status(&self) -> Status {
        match self {
            RunOutcome::Status(s) => s.clone(),
            RunOutcome::Value(_) => Status::ok(),
        }
    }
}

// ── TryOutcome ─────────────────────────────────────────────────

/// Internal outcome for try block execution.
enum TryOutcome {
    /// Body completed normally.
    Completed(RunOutcome),
    /// A command failed with nonzero status — abort to else.
    Aborted(Status),
}

// ── Coprocess ──────────────────────────────────────────────────

/// A coprocess — bidirectional pipe to a child process.
pub struct Coproc {
    /// The shell's read end (reads from child's stdout).
    pub read_fd: RawFd,
    /// The shell's write end (writes to child's stdin).
    pub write_fd: RawFd,
    /// The child's pid.
    pub pid: libc::pid_t,
}

// ── CaptureResult ──────────────────────────────────────────────

/// Result of capture_subprocess — shared by `{cmd} and try-in-value.
struct CaptureResult {
    stdout: String,
    exit_code: i32,
}

// ── Shell ──────────────────────────────────────────────────────

/// The shell interpreter state.
pub struct Shell {
    pub env: Env,
    pub jobs: JobTable,
    /// Reentrancy guard for discipline functions.
    active_disciplines: HashSet<String>,
    /// Whether the shell is in interactive mode.
    pub interactive: bool,
    /// The shell's process group id (for job control).
    shell_pgid: libc::pid_t,
    /// Active coprocess, if any.
    coproc: Option<Coproc>,
    /// Nesting depth of try blocks.
    try_depth: u32,
    /// Whether we're in a boolean context (if/while condition, &&/|| LHS).
    /// Boolean contexts are exempt from try-abort.
    in_boolean_context: bool,
    /// Accumulator for `take` in for-in-value-position.
    /// None when not inside a for-in-value context.
    take_acc: Option<Vec<Val>>,
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
        let _ = env.set_value("0", Val::Str("psh".into()));
        Shell {
            env,
            jobs: JobTable::new(),
            active_disciplines: HashSet::new(),
            interactive: false,
            shell_pgid: 0,
            coproc: None,
            try_depth: 0,
            in_boolean_context: false,
            take_acc: None,
        }
    }

    /// Set $0 (script name / shell name).
    pub fn set_argv0(&mut self, name: &str) {
        let _ = self.env.set_value("0", Val::Str(name.into()));
    }

    /// Set up the shell for interactive use.
    pub fn setup_interactive(&mut self) {
        self.interactive = true;
        self.shell_pgid = unsafe { libc::getpgrp() };
        // The shell ignores SIGTSTP — only foreground jobs receive it
        signal::ignore_signal(libc::SIGTSTP);
    }

    // ── Word evaluation (CBV) ──────────────────────────────

    /// Evaluate a Word to a Val.
    ///
    /// CBV: words are positive, evaluated eagerly before the command
    /// that consumes them runs.
    pub fn eval_word(&mut self, word: &Word) -> Val {
        match word {
            Word::Literal(s) => Val::Str(s.clone()),
            Word::Quoted(s) => Val::Str(s.clone()),
            Word::Var(name) => self.resolve_var(name),
            Word::VarAccess(name, accessors) => {
                let val = self.resolve_var(name);
                self.apply_accessors(val, accessors)
            }
            Word::Index(name, idx_word) => {
                let val = self.resolve_var(name);
                let idx_val = self.eval_word(idx_word);
                let idx_str = idx_val.to_string();
                if let Ok(i) = idx_str.parse::<usize>() {
                    val.index(i)
                } else {
                    Val::Unit
                }
            }
            Word::Count(name) => {
                let val = self.resolve_var(name);
                Val::Int(val.count() as i64)
            }
            Word::Stringify(name) => {
                let val = self.resolve_var(name);
                Val::Str(val.to_string())
            }
            Word::BraceVar(name) => self.env.get_value(name),
            Word::CommandSub(cmds) => {
                let result = self.capture_subprocess(cmds);
                // Command sub projects stdout only (π₁). Status discarded.
                let text = result.stdout.trim_end_matches('\n').to_string();
                if text.is_empty() {
                    Val::Unit
                } else if text.contains('\n') {
                    // Split on newlines into a list
                    Val::List(text.split('\n').map(|s| Val::Str(s.to_string())).collect())
                } else {
                    Val::Str(text)
                }
            }
            Word::ProcessSub(cmds) => {
                // Fork, create pipe, run cmds with stdout connected.
                // Return /dev/fd/N where N is the read end.
                let mut fds = [0i32; 2];
                if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
                    return Val::Str("/dev/null".to_string());
                }
                let read_fd = fds[0];
                let write_fd = fds[1];

                let pid = unsafe { libc::fork() };
                match pid {
                    -1 => Val::Str("/dev/null".to_string()),
                    0 => {
                        // Child: redirect stdout to write end
                        unsafe {
                            libc::close(read_fd);
                            libc::dup2(write_fd, 1);
                            libc::close(write_fd);
                        }
                        let outcome = self.run_cmds(cmds);
                        let code = outcome.status().code();
                        unsafe { libc::_exit(code) }
                    }
                    _ => {
                        // Parent: close write end, return read fd path
                        unsafe { libc::close(write_fd) };
                        Val::Str(format!("/dev/fd/{read_fd}"))
                    }
                }
            }
            Word::OutputProcessSub(cmds) => {
                // Fork, create pipe, run cmds with stdin connected.
                // Return /dev/fd/N where N is the write end.
                let mut fds = [0i32; 2];
                if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
                    return Val::Str("/dev/null".to_string());
                }
                let read_fd = fds[0];
                let write_fd = fds[1];

                let pid = unsafe { libc::fork() };
                match pid {
                    -1 => Val::Str("/dev/null".to_string()),
                    0 => {
                        // Child: redirect stdin from read end
                        unsafe {
                            libc::close(write_fd);
                            libc::dup2(read_fd, 0);
                            libc::close(read_fd);
                        }
                        let outcome = self.run_cmds(cmds);
                        let code = outcome.status().code();
                        unsafe { libc::_exit(code) }
                    }
                    _ => {
                        // Parent: close read end, return write fd path
                        unsafe { libc::close(read_fd) };
                        Val::Str(format!("/dev/fd/{write_fd}"))
                    }
                }
            }
            Word::Concat(parts) => {
                let mut result = self.eval_word(&parts[0]);
                for part in &parts[1..] {
                    let right = self.eval_word(part);
                    result = result.concat(&right);
                }
                result
            }
            Word::Tilde => {
                // Expand to $home
                self.env.get_value("home")
            }
            Word::TildePath(path) => {
                let home = self.env.get_value("home").to_string();
                Val::Str(format!("{home}/{path}"))
            }
        }
    }

    /// Resolve a variable, firing .get discipline if registered.
    fn resolve_var(&mut self, name: &str) -> Val {
        // Fire .get discipline if present and not already active
        let disc_name = format!("{name}.get");
        if self.env.has_discipline(name, "get") && !self.active_disciplines.contains(&disc_name) {
            self.active_disciplines.insert(disc_name.clone());
            if let Some((_, body)) = self.env.get_fn(&disc_name).cloned() {
                // .get runs in a readonly scope — mutations rejected
                self.env.push_readonly_scope();
                self.run_cmds(&body);
                self.env.pop_scope();
            }
            self.active_disciplines.remove(&disc_name);
        }
        self.env.get_value(name)
    }

    /// Apply an accessor chain left-to-right. Prism miss → Unit
    /// (absorbing element — once Unit, stays Unit through remaining
    /// accessors). See syntax.md §Accessors.
    fn apply_accessors(&self, mut val: Val, accessors: &[Accessor]) -> Val {
        for acc in accessors {
            val = match acc {
                Accessor::Index(i) => val.tuple_index(*i),
                Accessor::Tag(tag) => match val {
                    Val::Sum(ref t, ref payload) if t == tag => *payload.clone(),
                    _ => Val::Unit,
                },
                Accessor::Code => match val {
                    Val::ExitCode(code) => Val::Int(code as i64),
                    _ => Val::Unit,
                },
            };
        }
        val
    }

    /// Evaluate a Value (word | list | lambda | tagged).
    pub fn eval_value(&mut self, value: &Value) -> Val {
        match value {
            Value::Word(w) => self.eval_word(w),
            Value::List(words) => {
                let vals: Vec<Val> = words.iter().map(|w| self.eval_word(w)).collect();
                if vals.is_empty() {
                    Val::Unit
                } else {
                    Val::List(vals)
                }
            }
            Value::Lambda { params, body } => {
                // Capture-by-value: snapshot free variables at construction
                // time. Walk the body AST, collect $var references, subtract
                // the lambda's own params, and snapshot bound values from
                // the current scope. Enables currying — inner lambdas close
                // over outer params. Named functions (fn) do NOT capture;
                // capture is lambda-only (spec §Thunks).
                let mut referenced = free_vars_in_commands(body);
                for p in params {
                    referenced.remove(p);
                }
                let mut captures: Vec<(String, Val)> = Vec::new();
                for name in &referenced {
                    let val = self.env.get_value(name);
                    if val != Val::Unit {
                        captures.push((name.clone(), val));
                    }
                }
                // Sort for deterministic PartialEq on thunks
                captures.sort_by(|a, b| a.0.cmp(&b.0));
                Val::Thunk {
                    params: params.clone(),
                    body: body.clone(),
                    captures,
                }
            }
            Value::Try(cmds) => {
                let result = self.capture_subprocess(cmds);
                if result.exit_code == 0 {
                    // Success: wrap captured stdout as ok payload
                    let text = result.stdout.trim_end_matches('\n').to_string();
                    let val = if text.is_empty() {
                        Val::Unit
                    } else {
                        Val::Str(text)
                    };
                    Val::Sum("ok".into(), Box::new(val))
                } else {
                    Val::Sum("err".into(), Box::new(Val::ExitCode(result.exit_code)))
                }
            }
            Value::Compute(cmds) => {
                // Set up take accumulator for for-in-value blocks.
                // If any `take` commands fire, they push to this vec.
                let prev_acc = self.take_acc.take();
                self.take_acc = Some(Vec::new());
                let outcome = self.run_cmds(cmds);
                let acc = self.take_acc.take().unwrap_or_default();
                self.take_acc = prev_acc;
                // If return was issued, that takes priority
                if let RunOutcome::Value(val) = outcome {
                    return val;
                }
                // If take accumulated values, produce a List
                if !acc.is_empty() {
                    Val::List(acc)
                } else {
                    Val::Unit
                }
            }
            Value::Tagged(tag, payload) => {
                let val = self.eval_value(payload);
                Val::Sum(tag.clone(), Box::new(val))
            }
        }
    }

    // ── Shared capture primitive ─────────────────────────────

    /// Fork, run commands, capture stdout + exit code.
    ///
    /// Shared by `{cmd} (projects stdout) and try-in-value (projects both).
    /// Neither desugars into the other — they are siblings consuming
    /// different projections of this product.
    fn capture_subprocess(&mut self, cmds: &[Command]) -> CaptureResult {
        let mut fds = [0i32; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return CaptureResult {
                stdout: String::new(),
                exit_code: 1,
            };
        }
        let read_fd = fds[0];
        let write_fd = fds[1];

        let pid = unsafe { libc::fork() };
        match pid {
            -1 => CaptureResult {
                stdout: String::new(),
                exit_code: 1,
            },
            0 => {
                // Child: redirect stdout to pipe
                unsafe {
                    libc::close(read_fd);
                    libc::dup2(write_fd, 1);
                    libc::close(write_fd);
                }
                let outcome = self.run_cmds(cmds);
                let code = outcome.status().code();
                unsafe { libc::_exit(code) }
            }
            _ => {
                // Parent: read from pipe, waitpid
                unsafe { libc::close(write_fd) };
                let mut output = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = unsafe {
                        libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
                    };
                    if n <= 0 {
                        break;
                    }
                    output.extend_from_slice(&buf[..n as usize]);
                }
                unsafe { libc::close(read_fd) };

                let mut wstatus = 0i32;
                unsafe { libc::waitpid(pid, &mut wstatus, 0) };
                let exit_code = if libc::WIFEXITED(wstatus) {
                    libc::WEXITSTATUS(wstatus)
                } else {
                    1
                };

                CaptureResult {
                    stdout: String::from_utf8_lossy(&output).to_string(),
                    exit_code,
                }
            }
        }
    }

    // ── Expression evaluation ───────────────────────────────

    /// Evaluate an expression, returning Status.
    pub fn run_expr(&mut self, expr: &Expr) -> Status {
        match expr {
            Expr::Simple(sc) => self.run_simple(sc),
            Expr::Pipeline(stages) => self.run_pipeline(stages),
            Expr::And(left, right) => {
                let saved = self.in_boolean_context;
                self.in_boolean_context = true;
                let ls = self.run_expr(left);
                self.in_boolean_context = saved;
                if ls.is_success() {
                    self.run_expr(right)
                } else {
                    ls
                }
            }
            Expr::Or(left, right) => {
                let saved = self.in_boolean_context;
                self.in_boolean_context = true;
                let ls = self.run_expr(left);
                self.in_boolean_context = saved;
                if ls.is_success() {
                    ls
                } else {
                    self.run_expr(right)
                }
            }
            Expr::Not(inner) => {
                let saved = self.in_boolean_context;
                self.in_boolean_context = true;
                let s = self.run_expr(inner);
                self.in_boolean_context = saved;
                if s.is_success() {
                    Status::err("false")
                } else {
                    Status::ok()
                }
            }
            Expr::Background(inner) => {
                let pid = unsafe { libc::fork() };
                match pid {
                    -1 => Status::err("fork failed"),
                    0 => {
                        // Child: run the command, then exit
                        let s = self.run_expr(inner);
                        unsafe { libc::_exit(s.code()) }
                    }
                    _ => {
                        // Parent: add to job table
                        let cmd_str = format!("{inner:?}");
                        let job = Job {
                            pgid: pid,
                            pids: vec![pid],
                            command: cmd_str,
                            status: JobStatus::Running,
                        };
                        let num = self.jobs.insert(job);
                        eprintln!("[{num}] {pid}");
                        Status::ok()
                    }
                }
            }
            Expr::Block(cmds) => self.run_cmds(cmds).status(),
            Expr::Subshell(cmds) => {
                let pid = unsafe { libc::fork() };
                match pid {
                    -1 => Status::err("fork failed"),
                    0 => {
                        let outcome = self.run_cmds(cmds);
                        unsafe { libc::_exit(outcome.status().code()) }
                    }
                    _ => {
                        let mut wstatus = 0i32;
                        unsafe { libc::waitpid(pid, &mut wstatus, 0) };
                        if libc::WIFEXITED(wstatus) {
                            Status::from_code(libc::WEXITSTATUS(wstatus))
                        } else {
                            Status::err("subshell failed")
                        }
                    }
                }
            }
            Expr::Redirect(inner, op) => self.run_redirect(inner, op),
            Expr::Coprocess(inner) => self.run_coprocess(inner),
        }
    }

    /// Run a simple command: builtin or external.
    fn run_simple(&mut self, sc: &SimpleCommand) -> Status {
        // Special case: ~ in command position is the match operator (rc heritage).
        // The parser produces Word::Tilde for bare ~. In command-name position,
        // this is the match builtin, not tilde expansion.
        if matches!(sc.name, Word::Tilde) {
            let argv: Vec<String> = sc
                .args
                .iter()
                .map(|a| self.eval_word(a).to_string())
                .collect();
            return self.builtin_tilde(&argv);
        }

        let cmd_val = self.eval_word(&sc.name);
        let cmd_name = cmd_val.to_string();

        // Thunk forcing: if command name evaluates to Val::Thunk, force it
        if let Val::Thunk {
            params,
            body,
            captures,
        } = cmd_val
        {
            return self.force_thunk(&params, &body, &captures, &sc.args);
        }

        // Evaluate arguments
        let mut argv: Vec<String> = vec![cmd_name.clone()];
        for arg in &sc.args {
            let val = self.eval_word(arg);
            argv.extend(val.to_args());
        }

        // Try builtin first
        if let Some(status) = self.run_builtin(&cmd_name, &argv[1..]) {
            return status;
        }

        // Try function table (fn definitions)
        if let Some((fn_params, body)) = self.env.get_fn(&cmd_name).cloned() {
            self.env.push_scope();
            let arg_vals: Vec<Val> = sc.args.iter().map(|a| self.eval_word(a)).collect();
            // Bind named parameters if declared
            for (i, pname) in fn_params.iter().enumerate() {
                let val = arg_vals.get(i).cloned().unwrap_or(Val::Unit);
                let _ = self.env.set_value(pname, val);
            }
            // Always bind positional parameters ($1, $2, ..., $*)
            for (i, val) in arg_vals.iter().enumerate() {
                let _ = self.env.set_value(&(i + 1).to_string(), val.clone());
            }
            let _ = self.env.set_value(
                "*",
                if arg_vals.is_empty() {
                    Val::Unit
                } else {
                    Val::List(arg_vals)
                },
            );
            let outcome = self.run_cmds(&body);
            self.env.pop_scope();
            return outcome.status();
        }

        // External command
        self.run_external(&argv)
    }

    /// Force a thunk: push scope, restore captures, bind params, run body, pop scope.
    ///
    /// Captures are restored first so that named params can shadow them —
    /// a param with the same name as a capture wins, which is correct for
    /// curried lambdas where the inner lambda's own param overrides the
    /// outer's captured variable.
    fn force_thunk(
        &mut self,
        params: &[String],
        body: &[Command],
        captures: &[(String, Val)],
        args: &[Word],
    ) -> Status {
        self.env.push_scope();

        // Restore captured variables into the thunk's scope. Uses let_value
        // (current-scope-only) rather than set_value (walks chain) so captures
        // shadow outer bindings of the same name rather than mutating them.
        for (name, val) in captures {
            let _ = self.env.let_value(name, val.clone(), true, false, None);
        }

        // Bind named parameters (may shadow captures — correct behavior).
        // Also uses let_value to stay in the thunk scope.
        let arg_vals: Vec<Val> = args.iter().map(|a| self.eval_word(a)).collect();
        for (i, param) in params.iter().enumerate() {
            let val = arg_vals.get(i).cloned().unwrap_or(Val::Unit);
            let _ = self.env.let_value(param, val, true, false, None);
        }

        // Bind positional parameters ($1, $2, ..., $*)
        for (i, val) in arg_vals.iter().enumerate() {
            let _ = self
                .env
                .let_value(&(i + 1).to_string(), val.clone(), true, false, None);
        }
        let _ = self.env.let_value(
            "*",
            if arg_vals.is_empty() {
                Val::Unit
            } else {
                Val::List(arg_vals)
            },
            true,
            false,
            None,
        );

        let outcome = self.run_cmds(body);
        self.env.pop_scope();
        outcome.status()
    }

    /// Run an external command via fork/exec.
    fn run_external(&mut self, argv: &[String]) -> Status {
        if argv.is_empty() {
            return Status::ok();
        }

        // Resolve command in $path
        let cmd = &argv[0];
        let full_path = self.resolve_command(cmd);

        let pid = unsafe { libc::fork() };
        match pid {
            -1 => Status::err(format!("{cmd}: fork failed")),
            0 => {
                // Child process
                // Reset signal handlers
                signal::uninstall_handlers();

                let c_path = match CString::new(full_path.as_str()) {
                    Ok(p) => p,
                    Err(_) => unsafe { libc::_exit(127) },
                };
                let c_args: Vec<CString> = argv
                    .iter()
                    .map(|a| CString::new(a.as_str()).unwrap_or_default())
                    .collect();
                let c_env: Vec<CString> = self
                    .env
                    .to_process_env()
                    .iter()
                    .map(|(k, v)| CString::new(format!("{k}={v}")).unwrap_or_default())
                    .collect();

                let c_args_ptrs: Vec<*const libc::c_char> = c_args
                    .iter()
                    .map(|a| a.as_ptr())
                    .chain(std::iter::once(std::ptr::null()))
                    .collect();
                let c_env_ptrs: Vec<*const libc::c_char> = c_env
                    .iter()
                    .map(|e| e.as_ptr())
                    .chain(std::iter::once(std::ptr::null()))
                    .collect();

                unsafe {
                    libc::execve(c_path.as_ptr(), c_args_ptrs.as_ptr(), c_env_ptrs.as_ptr());
                    // If we get here, execve failed
                    let _ = libc::write(
                        2,
                        format!("psh: {cmd}: not found\n").as_ptr() as *const libc::c_void,
                        format!("psh: {cmd}: not found\n").len(),
                    );
                    libc::_exit(127);
                }
            }
            _ => {
                // Parent: wait for child
                self.wait_for_child(pid)
            }
        }
    }

    /// Wait for a foreground child process.
    fn wait_for_child(&mut self, pid: libc::pid_t) -> Status {
        loop {
            let mut wstatus = 0i32;
            let result = unsafe { libc::waitpid(pid, &mut wstatus, libc::WUNTRACED) };
            if result == -1 {
                return Status::err("waitpid failed");
            }
            if libc::WIFEXITED(wstatus) {
                let code = libc::WEXITSTATUS(wstatus);
                let status = Status::from_code(code);
                let _ = self.env.set_value("status", Val::Str(status.0.clone()));
                return status;
            }
            if libc::WIFSIGNALED(wstatus) {
                let sig = libc::WTERMSIG(wstatus);
                let status = Status::err(format!("signal {sig}"));
                let _ = self.env.set_value("status", Val::Str(status.0.clone()));
                return status;
            }
            if libc::WIFSTOPPED(wstatus) {
                // Job suspended — add to job table
                let job = Job {
                    pgid: pid,
                    pids: vec![pid],
                    command: String::new(),
                    status: JobStatus::Stopped,
                };
                let num = self.jobs.insert(job);
                eprintln!("\n[{num}] Stopped");
                return Status::err("stopped");
            }
        }
    }

    /// Resolve a command name to a full path using $path.
    fn resolve_command(&self, cmd: &str) -> String {
        // If it contains /, it's already a path
        if cmd.contains('/') {
            return cmd.to_string();
        }
        // Search $path
        let path_val = self.env.get_value("path");
        for dir in path_val.iter_elements() {
            let dir_str = dir.to_string();
            let full = format!("{dir_str}/{cmd}");
            if std::path::Path::new(&full).exists() {
                return full;
            }
        }
        // Fall back to PATH
        let path_env = self.env.get_value("PATH");
        for dir_str in path_env.to_string().split(':') {
            let full = format!("{dir_str}/{cmd}");
            if std::path::Path::new(&full).exists() {
                return full;
            }
        }
        cmd.to_string()
    }

    /// Run a pipeline: fork each stage, connect with pipes.
    fn run_pipeline(&mut self, stages: &[Expr]) -> Status {
        if stages.len() == 1 {
            return self.run_expr(&stages[0]);
        }

        let mut pids = Vec::new();
        let mut prev_read: Option<RawFd> = None;

        for (i, stage) in stages.iter().enumerate() {
            let is_last = i == stages.len() - 1;
            let mut fds = [0i32; 2];
            if !is_last && unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
                return Status::err("pipe failed");
            }

            let pid = unsafe { libc::fork() };
            match pid {
                -1 => return Status::err("fork failed"),
                0 => {
                    // Child
                    if let Some(read_fd) = prev_read {
                        unsafe {
                            libc::dup2(read_fd, 0);
                            libc::close(read_fd);
                        }
                    }
                    if !is_last {
                        unsafe {
                            libc::close(fds[0]);
                            libc::dup2(fds[1], 1);
                            libc::close(fds[1]);
                        }
                    }
                    let s = self.run_expr(stage);
                    unsafe { libc::_exit(s.code()) }
                }
                _ => {
                    // Parent
                    if let Some(read_fd) = prev_read {
                        unsafe { libc::close(read_fd) };
                    }
                    if !is_last {
                        unsafe { libc::close(fds[1]) };
                        prev_read = Some(fds[0]);
                    }
                    pids.push(pid);
                }
            }
        }

        // Wait for all children
        let mut last_status = Status::ok();
        for pid in &pids {
            last_status = self.wait_for_child(*pid);
        }
        last_status
    }

    /// Run a redirect: save fd, apply operation, evaluate inner, restore.
    fn run_redirect(&mut self, inner: &Expr, op: &RedirectOp) -> Status {
        match op {
            RedirectOp::Output { fd, target, append } => {
                let target_path = match target {
                    RedirectTarget::File(w) => self.eval_word(w).to_string(),
                    _ => return Status::err("invalid redirect target"),
                };
                let flags = libc::O_WRONLY
                    | libc::O_CREAT
                    | if *append {
                        libc::O_APPEND
                    } else {
                        libc::O_TRUNC
                    };
                let mode = 0o666;
                let c_path = CString::new(target_path.as_str()).unwrap_or_default();
                let new_fd = unsafe { libc::open(c_path.as_ptr(), flags, mode) };
                if new_fd == -1 {
                    return Status::err(format!("cannot open {target_path}"));
                }
                let saved = unsafe { libc::dup(*fd as i32) };
                if saved == -1 {
                    unsafe { libc::close(new_fd) };
                    return Status::err(format!("dup: {}", std::io::Error::last_os_error()));
                }
                unsafe { libc::dup2(new_fd, *fd as i32) };
                unsafe { libc::close(new_fd) };
                let result = self.run_expr(inner);
                unsafe { libc::dup2(saved, *fd as i32) };
                unsafe { libc::close(saved) };
                result
            }
            RedirectOp::Input { fd, target } => match target {
                RedirectTarget::File(w) => {
                    let target_path = self.eval_word(w).to_string();
                    let c_path = CString::new(target_path.as_str()).unwrap_or_default();
                    let new_fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY, 0) };
                    if new_fd == -1 {
                        return Status::err(format!("cannot open {target_path}"));
                    }
                    let saved = unsafe { libc::dup(*fd as i32) };
                    if saved == -1 {
                        unsafe { libc::close(new_fd) };
                        return Status::err(format!("dup: {}", std::io::Error::last_os_error()));
                    }
                    unsafe { libc::dup2(new_fd, *fd as i32) };
                    unsafe { libc::close(new_fd) };
                    let result = self.run_expr(inner);
                    unsafe { libc::dup2(saved, *fd as i32) };
                    unsafe { libc::close(saved) };
                    result
                }
                RedirectTarget::HereDoc { body, expand } => {
                    let content = if *expand {
                        self.expand_heredoc(body)
                    } else {
                        body.clone()
                    };
                    self.run_with_stdin_string(inner, &content)
                }
                RedirectTarget::HereString(w) => {
                    let content = self.eval_word(w).to_string();
                    self.run_with_stdin_string(inner, &content)
                }
            },
            RedirectOp::ReadWrite { fd, target } => {
                let target_path = match target {
                    RedirectTarget::File(w) => self.eval_word(w).to_string(),
                    _ => return Status::err("invalid redirect target"),
                };
                let c_path = CString::new(target_path.as_str()).unwrap_or_default();
                let new_fd =
                    unsafe { libc::open(c_path.as_ptr(), libc::O_RDWR | libc::O_CREAT, 0o666) };
                if new_fd == -1 {
                    return Status::err(format!("cannot open {target_path}"));
                }
                let saved = unsafe { libc::dup(*fd as i32) };
                if saved == -1 {
                    unsafe { libc::close(new_fd) };
                    return Status::err(format!("dup: {}", std::io::Error::last_os_error()));
                }
                unsafe { libc::dup2(new_fd, *fd as i32) };
                unsafe { libc::close(new_fd) };
                let result = self.run_expr(inner);
                unsafe { libc::dup2(saved, *fd as i32) };
                unsafe { libc::close(saved) };
                result
            }
            RedirectOp::Dup { dst, src } => {
                let saved = unsafe { libc::dup(*dst as i32) };
                if saved == -1 {
                    return Status::err(format!("dup: {}", std::io::Error::last_os_error()));
                }
                unsafe { libc::dup2(*src as i32, *dst as i32) };
                let result = self.run_expr(inner);
                unsafe { libc::dup2(saved, *dst as i32) };
                unsafe { libc::close(saved) };
                result
            }
            RedirectOp::Close { fd } => {
                let saved = unsafe { libc::dup(*fd as i32) };
                if saved == -1 {
                    return Status::err(format!("dup: {}", std::io::Error::last_os_error()));
                }
                unsafe { libc::close(*fd as i32) };
                let result = self.run_expr(inner);
                unsafe { libc::dup2(saved, *fd as i32) };
                unsafe { libc::close(saved) };
                result
            }
        }
    }

    /// Run a command with stdin fed from a string (heredoc/herestring).
    fn run_with_stdin_string(&mut self, inner: &Expr, content: &str) -> Status {
        let mut fds = [0i32; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Status::err("pipe failed");
        }
        // Write content to pipe write end
        let bytes = content.as_bytes();
        unsafe {
            libc::write(fds[1], bytes.as_ptr() as *const libc::c_void, bytes.len());
            libc::close(fds[1]);
        }
        // Redirect stdin from pipe read end
        let saved = unsafe { libc::dup(0) };
        if saved == -1 {
            unsafe { libc::close(fds[0]) };
            return Status::err(format!("dup: {}", std::io::Error::last_os_error()));
        }
        unsafe { libc::dup2(fds[0], 0) };
        unsafe { libc::close(fds[0]) };
        let result = self.run_expr(inner);
        unsafe { libc::dup2(saved, 0) };
        unsafe { libc::close(saved) };
        result
    }

    /// Expand $var references in a heredoc body.
    fn expand_heredoc(&mut self, body: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = body.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() {
                i += 1;
                let mut name = String::new();
                while i < chars.len() && crate::parse::is_var_char(chars[i]) {
                    name.push(chars[i]);
                    i += 1;
                }
                if !name.is_empty() {
                    result.push_str(&self.env.get_value(&name).to_string());
                } else {
                    result.push('$');
                }
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
        result
    }

    /// Start a coprocess: bidirectional socketpair.
    fn run_coprocess(&mut self, inner: &Expr) -> Status {
        let mut fds = [0i32; 2];
        if unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) } != 0 {
            return Status::err("socketpair failed");
        }

        let pid = unsafe { libc::fork() };
        match pid {
            -1 => {
                unsafe {
                    libc::close(fds[0]);
                    libc::close(fds[1]);
                }
                Status::err("fork failed")
            }
            0 => {
                // Child: use fds[1] for stdin/stdout
                unsafe {
                    libc::close(fds[0]);
                    libc::dup2(fds[1], 0);
                    libc::dup2(fds[1], 1);
                    libc::close(fds[1]);
                }
                let s = self.run_expr(inner);
                unsafe { libc::_exit(s.code()) }
            }
            _ => {
                // Parent: keep fds[0] for read/write to coprocess
                unsafe { libc::close(fds[1]) };
                self.coproc = Some(Coproc {
                    read_fd: fds[0],
                    write_fd: fds[0], // socketpair is bidirectional
                    pid,
                });
                Status::ok()
            }
        }
    }

    // ── Command execution ────────────────────────────────────

    /// Execute a single command.
    pub fn run_cmd(&mut self, cmd: &Command) -> RunOutcome {
        match cmd {
            Command::Exec(expr) => {
                let status = self.run_expr(expr);
                let _ = self.env.set_value("status", Val::Str(status.0.clone()));
                RunOutcome::Status(status)
            }
            Command::Bind(binding) => {
                let status = self.run_binding(binding);
                RunOutcome::Status(status)
            }
            Command::If {
                condition,
                then_body,
                else_body,
            } => {
                let saved = self.in_boolean_context;
                self.in_boolean_context = true;
                let cond_status = self.run_expr(condition);
                self.in_boolean_context = saved;

                if cond_status.is_success() {
                    self.run_cmds(then_body)
                } else if let Some(else_cmds) = else_body {
                    self.run_cmds(else_cmds)
                } else {
                    RunOutcome::Status(cond_status)
                }
            }
            Command::For { var, list, body } => {
                let list_val = self.eval_value(list);
                let mut last = RunOutcome::ok();
                for elem in list_val.iter_elements() {
                    let _ = self.env.set_value(var, elem.clone());
                    last = self.run_cmds(body);
                }
                last
            }
            Command::While { condition, body } => {
                let mut last = RunOutcome::ok();
                let mut ran_body = false;
                loop {
                    let saved = self.in_boolean_context;
                    self.in_boolean_context = true;
                    let cond_status = self.run_expr(condition);
                    self.in_boolean_context = saved;

                    if !cond_status.is_success() {
                        if !ran_body {
                            // Never entered the loop — propagate condition status
                            last = RunOutcome::Status(cond_status);
                        }
                        break;
                    }
                    ran_body = true;
                    last = self.run_cmds(body);
                }
                last
            }
            Command::Match { value, arms } => {
                let match_val = self.eval_value(value);
                let match_str = match_val.to_string();

                for (patterns, arm_body) in arms {
                    for pat in patterns {
                        let matched = match pat {
                            Pattern::Star => true,
                            Pattern::Literal(s) => match_str == *s,
                            Pattern::Glob(g) => fnmatch_match(g, &match_str),
                            Pattern::Structural { tag, binding } => {
                                if let Val::Sum(val_tag, payload) = &match_val {
                                    if val_tag == tag {
                                        // Push scope, bind payload
                                        self.env.push_scope();
                                        let _ = self.env.set_value(binding, *payload.clone());
                                        let result = self.run_cmds(arm_body);
                                        self.env.pop_scope();
                                        return result;
                                    }
                                }
                                false
                            }
                        };
                        if matched {
                            return self.run_cmds(arm_body);
                        }
                    }
                }
                // No match — ⊕ convention: Unit + nonzero status
                let _ = self
                    .env
                    .set_value("status", Val::Str("no match".to_string()));
                RunOutcome::Status(Status::err("no match"))
            }
            Command::Try {
                body: try_body,
                else_var,
                else_body,
            } => {
                self.try_depth += 1;
                let outcome = self.run_cmds_try(try_body);
                self.try_depth -= 1;

                match outcome {
                    TryOutcome::Completed(run_outcome) => run_outcome,
                    TryOutcome::Aborted(status) => {
                        if let Some(else_cmds) = else_body {
                            // Bind the error status to the else variable
                            if let Some(var) = else_var {
                                let _ = self.env.set_value(var, Val::Str(status.0.clone()));
                            }
                            self.run_cmds(else_cmds)
                        } else {
                            RunOutcome::Status(status)
                        }
                    }
                }
            }
            Command::Return(opt_value) => {
                let val = match opt_value {
                    Some(v) => self.eval_value(v),
                    None => Val::Unit,
                };
                RunOutcome::Value(val)
            }
            Command::Take(value) => {
                let val = self.eval_value(value);
                if let Some(ref mut acc) = self.take_acc {
                    acc.push(val);
                    RunOutcome::ok()
                } else {
                    eprintln!("take: not inside a for-in-value block");
                    RunOutcome::Status(Status::err("take outside for-in-value"))
                }
            }
        }
    }

    /// Run a sequence of commands.
    pub fn run_cmds(&mut self, cmds: &[Command]) -> RunOutcome {
        let mut last = RunOutcome::ok();
        for cmd in cmds {
            last = self.run_cmd(cmd);
            // If a return was issued, propagate immediately
            if matches!(last, RunOutcome::Value(_)) {
                return last;
            }
            // In a try block, check for abort after each command
            if self.try_depth > 0 && !self.in_boolean_context {
                if let RunOutcome::Status(ref s) = last {
                    if !s.is_success() {
                        return last;
                    }
                }
            }
        }
        last
    }

    /// Run commands inside a try block — returns TryOutcome.
    fn run_cmds_try(&mut self, cmds: &[Command]) -> TryOutcome {
        for cmd in cmds {
            let outcome = self.run_cmd(cmd);
            match &outcome {
                RunOutcome::Value(_) => return TryOutcome::Completed(outcome),
                RunOutcome::Status(s) => {
                    if !s.is_success() && !self.in_boolean_context {
                        return TryOutcome::Aborted(s.clone());
                    }
                }
            }
        }
        TryOutcome::Completed(RunOutcome::ok())
    }

    /// Execute a binding.
    fn run_binding(&mut self, binding: &Binding) -> Status {
        match binding {
            Binding::Assignment(name, value) => {
                let val = self.eval_value(value);
                // Assignment always stores as Str (rc heritage) unless
                // the variable already has a type annotation
                let store_val = if self.env.get(name).is_some_and(|v| v.type_ann.is_some()) {
                    val
                } else {
                    Val::Str(val.to_string())
                };

                // Fire .set discipline if present
                self.fire_set_discipline(name, &store_val);

                match self.env.set_value(name, store_val) {
                    Ok(()) => Status::ok(),
                    Err(e) => {
                        eprintln!("psh: {e}");
                        Status::err(e)
                    }
                }
            }
            Binding::Let {
                name,
                value,
                mutable,
                export,
                type_ann,
            } => {
                let raw_val = self.eval_value(value);
                // In let context, run type inference on unquoted literals
                let val = if matches!(value, Value::Word(Word::Quoted(_))) {
                    // Quoted — stays Str
                    raw_val
                } else if type_ann.is_some() {
                    // Type annotation — validation happens in let_value
                    raw_val
                } else {
                    // Infer type from string representation
                    match &raw_val {
                        Val::Str(s) => Val::infer(s),
                        _ => raw_val,
                    }
                };

                match self
                    .env
                    .let_value(name, val, *mutable, *export, type_ann.clone())
                {
                    Ok(()) => Status::ok(),
                    Err(e) => {
                        eprintln!("psh: {e}");
                        Status::err(e)
                    }
                }
            }
            Binding::Fn { name, params, body } => {
                self.env
                    .define_fn(name.clone(), params.clone(), body.clone());
                // Register discipline if it's a x.get or x.set function
                if let Some(dot_pos) = name.rfind('.') {
                    let var_name = &name[..dot_pos];
                    // Ensure the variable exists (disciplines need a target)
                    if self.env.get(var_name).is_none() {
                        let _ = self.env.set_value(var_name, Val::Unit);
                    }
                }
                Status::ok()
            }
            Binding::Ref { name, target } => {
                self.env.set_nameref(name, target.clone());
                Status::ok()
            }
        }
    }

    /// Fire .set discipline for a variable, if registered.
    fn fire_set_discipline(&mut self, name: &str, value: &Val) {
        let disc_name = format!("{name}.set");
        if self.env.has_discipline(name, "set") && !self.active_disciplines.contains(&disc_name) {
            self.active_disciplines.insert(disc_name.clone());
            if let Some((fn_params, body)) = self.env.get_fn(&disc_name).cloned() {
                self.env.push_scope();
                // Bind named params (e.g., fn x.set(val) { })
                if let Some(pname) = fn_params.first() {
                    let _ = self.env.set_value(pname, value.clone());
                }
                // $1 = the new value being assigned (positional compat)
                let _ = self.env.set_value("1", value.clone());
                self.run_cmds(&body);
                self.env.pop_scope();
            }
            self.active_disciplines.remove(&disc_name);
        }
    }

    // ── Builtins ────────────────────────────────────────────

    /// Try to run a builtin. Returns None if not a builtin.
    fn run_builtin(&mut self, name: &str, args: &[String]) -> Option<Status> {
        match name {
            "echo" => Some(self.builtin_echo(args)),
            "cd" => Some(self.builtin_cd(args)),
            "exit" => Some(self.builtin_exit(args)),
            "true" => Some(Status::ok()),
            "false" => Some(Status::err("false")),
            "get" => Some(self.builtin_get(args)),
            "set" => Some(self.builtin_set(args)),
            "read" => Some(self.builtin_read(args)),
            "print" => Some(self.builtin_print(args)),
            "wait" => Some(self.builtin_wait(args)),
            "jobs" => Some(self.builtin_jobs()),
            "fg" => Some(self.builtin_fg(args)),
            "bg" => Some(self.builtin_bg(args)),
            "whatis" => Some(self.builtin_whatis(args)),
            "." => Some(self.builtin_dot(args)),
            "builtin" => Some(self.builtin_builtin(args)),
            "~" => Some(self.builtin_tilde(args)),
            "shift" => Some(self.builtin_shift(args)),
            _ => None,
        }
    }

    fn builtin_echo(&self, args: &[String]) -> Status {
        let text = format!("{}\n", args.join(" "));
        let bytes = text.as_bytes();
        // Write directly to fd 1 to respect redirections.
        // Rust's println! uses a buffered stdout that may not
        // see dup2-level fd redirections.
        unsafe {
            libc::write(1, bytes.as_ptr() as *const libc::c_void, bytes.len());
        }
        Status::ok()
    }

    fn builtin_cd(&mut self, args: &[String]) -> Status {
        let dir = if args.is_empty() {
            self.env.get_value("home").to_string()
        } else {
            args[0].clone()
        };
        match std::env::set_current_dir(&dir) {
            Ok(()) => {
                if let Ok(cwd) = std::env::current_dir() {
                    let _ = self
                        .env
                        .set_value("PWD", Val::Str(cwd.display().to_string()));
                }
                Status::ok()
            }
            Err(e) => {
                eprintln!("psh: cd: {dir}: {e}");
                Status::err(format!("{dir}: {e}"))
            }
        }
    }

    fn builtin_exit(&mut self, args: &[String]) -> Status {
        let code = args
            .first()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        self.fire_sigexit();
        std::process::exit(code);
    }

    fn builtin_get(&self, args: &[String]) -> Status {
        if args.is_empty() {
            eprintln!("psh: get: no variable name");
            return Status::err("no variable name");
        }
        let val = self.env.get_value(&args[0]);
        println!("{val}");
        if val == Val::Unit {
            Status::err("not found")
        } else {
            Status::ok()
        }
    }

    fn builtin_set(&mut self, args: &[String]) -> Status {
        if args.len() < 2 {
            eprintln!("psh: set: usage: set name value");
            return Status::err("usage");
        }
        let name = &args[0];
        let val = Val::Str(args[1..].join(" "));
        match self.env.set_value(name, val) {
            Ok(()) => Status::ok(),
            Err(e) => {
                eprintln!("psh: set: {e}");
                Status::err(e)
            }
        }
    }

    fn builtin_read(&mut self, args: &[String]) -> Status {
        use std::io::BufRead;

        // -p flag: read from coprocess
        if args.first().is_some_and(|a| a == "-p") {
            if let Some(ref coproc) = self.coproc {
                let mut buf = [0u8; 4096];
                let n = unsafe {
                    libc::read(
                        coproc.read_fd,
                        buf.as_mut_ptr() as *mut libc::c_void,
                        buf.len(),
                    )
                };
                if n > 0 {
                    let text = String::from_utf8_lossy(&buf[..n as usize])
                        .trim_end_matches('\n')
                        .to_string();
                    let var_name = args.get(1).map(|s| s.as_str()).unwrap_or("line");
                    let _ = self.env.set_value(var_name, Val::Str(text));
                    return Status::ok();
                }
                return Status::err("read failed");
            }
            return Status::err("no coprocess");
        }

        let var_name = args.first().map(|s| s.as_str()).unwrap_or("line");
        let stdin = std::io::stdin();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => Status::err("eof"),
            Ok(_) => {
                let text = line.trim_end_matches('\n').to_string();
                let _ = self.env.set_value(var_name, Val::Str(text));
                Status::ok()
            }
            Err(e) => Status::err(format!("read: {e}")),
        }
    }

    fn builtin_print(&self, args: &[String]) -> Status {
        // -p flag: write to coprocess
        if args.first().is_some_and(|a| a == "-p") {
            if let Some(ref coproc) = self.coproc {
                let text = format!("{}\n", args[1..].join(" "));
                let bytes = text.as_bytes();
                unsafe {
                    libc::write(
                        coproc.write_fd,
                        bytes.as_ptr() as *const libc::c_void,
                        bytes.len(),
                    );
                }
                return Status::ok();
            }
            return Status::err("no coprocess");
        }
        println!("{}", args.join(" "));
        Status::ok()
    }

    fn builtin_wait(&mut self, _args: &[String]) -> Status {
        // Wait for all background jobs
        loop {
            let mut wstatus = 0i32;
            let pid = unsafe { libc::waitpid(-1, &mut wstatus, 0) };
            if pid <= 0 {
                break;
            }
            self.jobs.reap(pid, wstatus);
        }
        Status::ok()
    }

    fn builtin_jobs(&self) -> Status {
        for (num, job) in self.jobs.iter() {
            let status_str = match &job.status {
                JobStatus::Running => "Running",
                JobStatus::Stopped => "Stopped",
                JobStatus::Done(_) => "Done",
            };
            eprintln!("[{num}] {status_str}\t{}", job.command);
        }
        Status::ok()
    }

    fn builtin_fg(&mut self, args: &[String]) -> Status {
        let job_num = if let Some(arg) = args.first() {
            arg.trim_start_matches('%').parse::<usize>().unwrap_or(0)
        } else {
            self.jobs.current_job().unwrap_or(0)
        };
        if let Some(job) = self.jobs.get(job_num) {
            let pgid = job.pgid;
            unsafe {
                libc::kill(pgid, libc::SIGCONT);
            }
            self.wait_for_child(pgid)
        } else {
            Status::err("no such job")
        }
    }

    fn builtin_bg(&mut self, args: &[String]) -> Status {
        let job_num = if let Some(arg) = args.first() {
            arg.trim_start_matches('%').parse::<usize>().unwrap_or(0)
        } else {
            self.jobs.current_job().unwrap_or(0)
        };
        if let Some(job) = self.jobs.get_mut(job_num) {
            let pgid = job.pgid;
            job.status = JobStatus::Running;
            unsafe {
                libc::kill(pgid, libc::SIGCONT);
            }
            Status::ok()
        } else {
            Status::err("no such job")
        }
    }

    fn builtin_whatis(&self, args: &[String]) -> Status {
        for name in args {
            if let Some(_body) = self.env.get_fn(name) {
                eprintln!("{name}: function");
            } else {
                let val = self.env.get_value(name);
                let type_name = match &val {
                    Val::Unit => "not set",
                    Val::Bool(_) => "Bool",
                    Val::Int(_) => "Int",
                    Val::Str(_) => "Str",
                    Val::Path(_) => "Path",
                    Val::ExitCode(_) => "ExitCode",
                    Val::List(_) => "List",
                    Val::Tuple(_) => "Tuple",
                    Val::Sum(tag, _) => {
                        eprintln!("{name}: Sum({tag}, {val})");
                        continue;
                    }
                    Val::Thunk { .. } => "Thunk",
                };
                if matches!(val, Val::Unit) {
                    eprintln!("{name}: {type_name}");
                } else {
                    eprintln!("{name}: {type_name} = {val}");
                }
            }
        }
        Status::ok()
    }

    fn builtin_dot(&mut self, args: &[String]) -> Status {
        if args.is_empty() {
            return Status::err(".: no file");
        }
        match std::fs::read_to_string(&args[0]) {
            Ok(content) => match crate::parse::PshParser::parse(&content) {
                Ok(prog) => self.run(&prog),
                Err(e) => {
                    eprintln!("psh: .: {e}");
                    Status::err(e.to_string())
                }
            },
            Err(e) => {
                eprintln!("psh: .: {}: {e}", args[0]);
                Status::err(e.to_string())
            }
        }
    }

    fn builtin_builtin(&mut self, args: &[String]) -> Status {
        if args.is_empty() {
            return Status::err("builtin: no command");
        }
        // Run the named builtin, bypassing function lookup
        self.run_builtin(&args[0], &args[1..])
            .unwrap_or_else(|| Status::err(format!("{}: not a builtin", args[0])))
    }

    /// ~ match_operator value patterns...
    fn builtin_tilde(&self, args: &[String]) -> Status {
        if args.len() < 2 {
            return Status::err("~: usage: ~ value pattern...");
        }
        let value = &args[0];
        for pattern in &args[1..] {
            if fnmatch_match(pattern, value) {
                return Status::ok();
            }
        }
        Status::err("no match")
    }

    fn builtin_shift(&mut self, args: &[String]) -> Status {
        let n: usize = if args.is_empty() {
            1
        } else {
            match args[0].parse() {
                Ok(v) => v,
                Err(_) => return Status::err("shift: numeric argument required"),
            }
        };
        // Read current positional params from $*
        let star = self.env.get_value("*");
        let mut positionals: Vec<Val> = match star {
            Val::List(v) => v,
            Val::Unit => vec![],
            other => vec![other],
        };
        // Drop first n
        if n > positionals.len() {
            return Status::err("shift: count exceeds positional parameters");
        }
        positionals.drain(..n);
        // Rebind $1, $2, ... and $*
        // First clear old positionals by setting them to Unit
        for i in 1..=(positionals.len() + n) {
            let _ = self.env.set_value(&i.to_string(), Val::Unit);
        }
        for (i, val) in positionals.iter().enumerate() {
            let _ = self.env.set_value(&(i + 1).to_string(), val.clone());
        }
        let _ = self.env.set_value(
            "*",
            if positionals.is_empty() {
                Val::Unit
            } else {
                Val::List(positionals)
            },
        );
        Status::ok()
    }

    // ── Signal handling ─────────────────────────────────────

    /// Check for pending signals and dispatch to handler functions.
    pub fn check_signals(&mut self) {
        let pending = signal::take_pending();
        for (sig_name, count) in pending {
            // SIGCHLD: reap zombie children
            if sig_name == "sigchld" {
                self.reap_children();
                continue;
            }
            // Dispatch to rc-style signal handler function
            if let Some((_, body)) = self.env.get_fn(sig_name).cloned() {
                for _ in 0..count {
                    self.run_cmds(&body);
                }
            }
        }
    }

    /// Reap zombie children (SIGCHLD handler).
    fn reap_children(&mut self) {
        loop {
            let mut wstatus = 0i32;
            let pid = unsafe { libc::waitpid(-1, &mut wstatus, libc::WNOHANG) };
            if pid <= 0 {
                break;
            }
            self.jobs.reap(pid, wstatus);
        }
    }

    /// Fire the sigexit artificial signal.
    pub fn fire_sigexit(&mut self) {
        if let Some((_, body)) = self.env.get_fn("sigexit").cloned() {
            self.run_cmds(&body);
        }
    }

    /// Print done jobs before prompt (interactive mode).
    pub fn notify_done_jobs(&mut self) {
        for (num, cmd) in self.jobs.collect_done() {
            eprintln!("[{num}] Done\t{cmd}");
        }
    }

    // ── Public entry point ──────────────────────────────────

    /// Execute a parsed program.
    pub fn run(&mut self, program: &Program) -> Status {
        self.run_cmds(&program.commands).status()
    }
}

// ── Glob matching ──────────────────────────────────────────────

/// Match a glob pattern against a string using fnmatch-regex.
fn fnmatch_match(pattern: &str, value: &str) -> bool {
    match fnmatch_regex::glob_to_regex(pattern) {
        Ok(regex) => regex.is_match(value),
        Err(_) => pattern == value,
    }
}

// ── Make is_var_char accessible for heredoc expansion ──────────
// The function is in parse.rs and pub(crate)

// ── Free variable collection ───────────────────────────────────

/// Collect all variable names referenced in a command body.
///
/// Walks the AST and returns every `$var` reference — the caller
/// subtracts the lambda's own params to get true free variables.
/// Used by capture-by-value at lambda construction time.
fn free_vars_in_commands(cmds: &[Command]) -> HashSet<String> {
    let mut vars = HashSet::new();
    for cmd in cmds {
        free_vars_command(cmd, &mut vars);
    }
    vars
}

fn free_vars_command(cmd: &Command, vars: &mut HashSet<String>) {
    match cmd {
        Command::Exec(expr) => free_vars_expr(expr, vars),
        Command::Bind(binding) => free_vars_binding(binding, vars),
        Command::If {
            condition,
            then_body,
            else_body,
        } => {
            free_vars_expr(condition, vars);
            for c in then_body {
                free_vars_command(c, vars);
            }
            if let Some(eb) = else_body {
                for c in eb {
                    free_vars_command(c, vars);
                }
            }
        }
        Command::For { var: _, list, body } => {
            free_vars_value(list, vars);
            for c in body {
                free_vars_command(c, vars);
            }
        }
        Command::Match { value, arms } => {
            free_vars_value(value, vars);
            for (patterns, body) in arms {
                for pat in patterns {
                    if let Pattern::Structural { binding, .. } = pat {
                        // The binding is introduced, not referenced —
                        // but we don't subtract it here because this
                        // function collects all references, not free
                        // variables proper. The caller subtracts params.
                        let _ = binding;
                    }
                }
                for c in body {
                    free_vars_command(c, vars);
                }
            }
        }
        Command::While { condition, body } => {
            free_vars_expr(condition, vars);
            for c in body {
                free_vars_command(c, vars);
            }
        }
        Command::Try {
            body,
            else_var: _,
            else_body,
        } => {
            for c in body {
                free_vars_command(c, vars);
            }
            if let Some(eb) = else_body {
                for c in eb {
                    free_vars_command(c, vars);
                }
            }
        }
        Command::Return(val) => {
            if let Some(v) = val {
                free_vars_value(v, vars);
            }
        }
        Command::Take(val) => free_vars_value(val, vars),
    }
}

fn free_vars_expr(expr: &Expr, vars: &mut HashSet<String>) {
    match expr {
        Expr::Simple(sc) => {
            free_vars_word(&sc.name, vars);
            for arg in &sc.args {
                free_vars_word(arg, vars);
            }
            for (_, val) in &sc.assignments {
                free_vars_value(val, vars);
            }
        }
        Expr::Redirect(inner, _) => free_vars_expr(inner, vars),
        Expr::Pipeline(stages) => {
            for s in stages {
                free_vars_expr(s, vars);
            }
        }
        Expr::And(l, r) | Expr::Or(l, r) => {
            free_vars_expr(l, vars);
            free_vars_expr(r, vars);
        }
        Expr::Not(inner) | Expr::Background(inner) | Expr::Coprocess(inner) => {
            free_vars_expr(inner, vars);
        }
        Expr::Block(cmds) | Expr::Subshell(cmds) => {
            for c in cmds {
                free_vars_command(c, vars);
            }
        }
    }
}

fn free_vars_binding(binding: &Binding, vars: &mut HashSet<String>) {
    match binding {
        Binding::Assignment(_, val) | Binding::Let { value: val, .. } => {
            free_vars_value(val, vars);
        }
        Binding::Fn { body, .. } => {
            for c in body {
                free_vars_command(c, vars);
            }
        }
        Binding::Ref { .. } => {}
    }
}

fn free_vars_value(val: &Value, vars: &mut HashSet<String>) {
    match val {
        Value::Word(w) => free_vars_word(w, vars),
        Value::List(words) => {
            for w in words {
                free_vars_word(w, vars);
            }
        }
        Value::Lambda { params: _, body } => {
            // Recurse into nested lambda bodies — their free vars
            // are our free vars too (the nested lambda will capture
            // them at its own construction time, but we still need
            // to identify them as referenced here).
            for c in body {
                free_vars_command(c, vars);
            }
        }
        Value::Try(cmds) => {
            for c in cmds {
                free_vars_command(c, vars);
            }
        }
        Value::Compute(cmds) => {
            for c in cmds {
                free_vars_command(c, vars);
            }
        }
        Value::Tagged(_, payload) => free_vars_value(payload, vars),
    }
}

fn free_vars_word(word: &Word, vars: &mut HashSet<String>) {
    match word {
        Word::Var(name) | Word::Count(name) | Word::Stringify(name) | Word::BraceVar(name) => {
            vars.insert(name.clone());
        }
        Word::VarAccess(name, _) => {
            vars.insert(name.clone());
        }
        Word::Index(name, idx) => {
            vars.insert(name.clone());
            free_vars_word(idx, vars);
        }
        Word::Concat(parts) => {
            for p in parts {
                free_vars_word(p, vars);
            }
        }
        Word::CommandSub(cmds) | Word::ProcessSub(cmds) | Word::OutputProcessSub(cmds) => {
            for c in cmds {
                free_vars_command(c, vars);
            }
        }
        // Literals, quoted strings, tilde — no variable references
        Word::Literal(_) | Word::Quoted(_) | Word::Tilde | Word::TildePath(_) => {}
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::PshParser;

    fn run(input: &str) -> (Shell, RunOutcome) {
        let prog = PshParser::parse(input).unwrap();
        let mut shell = Shell::new();
        let outcome = shell.run_cmds(&prog.commands);
        (shell, outcome)
    }

    fn run_status(input: &str) -> Status {
        let (_, outcome) = run(input);
        outcome.status()
    }

    fn run_with_shell(shell: &mut Shell, input: &str) -> RunOutcome {
        let prog = PshParser::parse(input).unwrap();
        shell.run_cmds(&prog.commands)
    }

    // ── Status ─────────────────────────────────────────────

    #[test]
    fn status_ok() {
        assert!(Status::ok().is_success());
        assert_eq!(Status::ok().code(), 0);
    }

    #[test]
    fn status_err() {
        let s = Status::err("fail");
        assert!(!s.is_success());
        assert_eq!(s.0, "fail");
    }

    #[test]
    fn status_from_code() {
        assert!(Status::from_code(0).is_success());
        assert!(!Status::from_code(1).is_success());
        assert_eq!(Status::from_code(42).code(), 42);
    }

    // ── RunOutcome ─────────────────────────────────────────

    #[test]
    fn run_outcome_status() {
        let o = RunOutcome::ok();
        assert!(o.status().is_success());
    }

    #[test]
    fn run_outcome_value() {
        let o = RunOutcome::Value(Val::Int(42));
        assert!(o.status().is_success()); // Value outcome has ok status
    }

    // ── Simple commands ────────────────────────────────────

    #[test]
    fn echo_hello() {
        let s = run_status("echo hello");
        assert!(s.is_success());
    }

    #[test]
    fn true_builtin() {
        let s = run_status("true");
        assert!(s.is_success());
    }

    #[test]
    fn false_builtin() {
        let s = run_status("false");
        assert!(!s.is_success());
    }

    // ── Variables ──────────────────────────────────────────

    #[test]
    fn assignment_and_expansion() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = hello");
        assert_eq!(shell.env.get_value("x"), Val::Str("hello".into()));
    }

    #[test]
    fn let_type_inference() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let x = 42");
        assert_eq!(shell.env.get_value("x"), Val::Int(42));
    }

    #[test]
    fn let_bool_inference() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let x = true");
        assert_eq!(shell.env.get_value("x"), Val::Bool(true));
    }

    #[test]
    fn let_quoted_stays_str() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let x = '42'");
        assert_eq!(shell.env.get_value("x"), Val::Str("42".into()));
    }

    #[test]
    fn assignment_walks_scope() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = outer");
        shell.env.push_scope();
        run_with_shell(&mut shell, "x = inner");
        // rc behavior: assignment walks scope chain
        assert_eq!(shell.env.get_value("x"), Val::Str("inner".into()));
        shell.env.pop_scope();
        assert_eq!(shell.env.get_value("x"), Val::Str("inner".into()));
    }

    // ── Control flow ───────────────────────────────────────

    #[test]
    fn if_true_branch() {
        let s = run_status("if true { echo yes }");
        assert!(s.is_success());
    }

    #[test]
    fn if_false_else() {
        let s = run_status("if false { echo yes } else { true }");
        assert!(s.is_success());
    }

    #[test]
    fn for_loop_runs() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let mut count = 0");
        // For loop with list
        let s = run_status("for x in (a b c) { echo $x }");
        assert!(s.is_success());
    }

    #[test]
    fn while_false_no_loop() {
        let s = run_status("while false { echo never }");
        // While condition is false — the status from the false condition
        assert!(!s.is_success());
    }

    // ── Functions ──────────────────────────────────────────

    #[test]
    fn fn_definition_and_call() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "fn greet { echo hello }");
        let s = run_with_shell(&mut shell, "greet").status();
        assert!(s.is_success());
    }

    #[test]
    fn fn_with_args() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "fn say { echo $1 }");
        let s = run_with_shell(&mut shell, "say world").status();
        assert!(s.is_success());
    }

    // ── Match ──────────────────────────────────────────────

    #[test]
    fn match_literal() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = hello");
        let s = run_with_shell(
            &mut shell,
            "match $x { hello => echo matched; * => echo nope }",
        )
        .status();
        assert!(s.is_success());
    }

    #[test]
    fn match_no_match() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = other");
        let s = run_with_shell(&mut shell, "match $x { hello => echo matched }").status();
        assert!(!s.is_success()); // ⊕ convention: no match → nonzero
    }

    #[test]
    fn match_glob_pattern() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = hello.txt");
        let s = run_with_shell(
            &mut shell,
            "match $x { *.txt => echo text; * => echo other }",
        )
        .status();
        assert!(s.is_success());
    }

    // ── Try ────────────────────────────────────────────────

    #[test]
    fn try_success() {
        let s = run_status("try { true }");
        assert!(s.is_success());
    }

    #[test]
    fn try_abort_to_else() {
        let s = run_status("try { false } else e { echo $e }");
        assert!(s.is_success());
    }

    // ── Return ─────────────────────────────────────────────

    #[test]
    fn return_produces_value() {
        let (_, outcome) = run("return 42");
        assert!(matches!(outcome, RunOutcome::Value(_)));
    }

    #[test]
    fn return_bare_produces_unit() {
        let (_, outcome) = run("return");
        match outcome {
            RunOutcome::Value(Val::Unit) => {}
            other => panic!("expected Value(Unit), got {other:?}"),
        }
    }

    // ── Lambda / Thunk ─────────────────────────────────────

    #[test]
    fn lambda_creates_thunk() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let f = \\x => echo $x");
        assert!(matches!(shell.env.get_value("f"), Val::Thunk { .. }));
    }

    #[test]
    fn lambda_captures_by_value() {
        // Captured at construction time — later mutation doesn't affect thunk
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let x = 42");
        run_with_shell(&mut shell, "let f = \\() => echo $x");
        run_with_shell(&mut shell, "x = 99");
        // The thunk should have captured x=42
        if let Val::Thunk { captures, .. } = shell.env.get_value("f") {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0], ("x".to_string(), Val::Int(42)));
        } else {
            panic!("expected Thunk");
        }
    }

    #[test]
    fn lambda_capture_currying() {
        // Currying: outer lambda returns inner lambda that captures $x.
        // The braced body form is needed because => after a lambda
        // value is not single-command parseable without braces.
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let x = outer");
        run_with_shell(
            &mut shell,
            "let f = \\() => { let g = \\() => echo $x; $g }",
        );
        // f captures x=outer at construction time
        if let Val::Thunk { captures, .. } = shell.env.get_value("f") {
            assert!(
                captures
                    .iter()
                    .any(|(n, v)| n == "x" && *v == Val::Str("outer".into())),
                "expected x=outer in captures, got {captures:?}"
            );
        } else {
            panic!("expected Thunk");
        }
    }

    #[test]
    fn lambda_no_capture_of_own_params() {
        // A lambda's own params are NOT captured — they are bound at force time
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let f = \\x => echo $x");
        if let Val::Thunk { captures, .. } = shell.env.get_value("f") {
            assert!(
                captures.is_empty(),
                "params should not appear in captures: {captures:?}"
            );
        } else {
            panic!("expected Thunk");
        }
    }

    #[test]
    fn lambda_captures_multiple_vars() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let a = hello");
        run_with_shell(&mut shell, "let b = world");
        run_with_shell(&mut shell, "let f = \\() => echo $a $b");
        if let Val::Thunk { captures, .. } = shell.env.get_value("f") {
            assert_eq!(captures.len(), 2);
            // Sorted alphabetically for deterministic order
            assert!(captures.iter().any(|(n, _)| n == "a"));
            assert!(captures.iter().any(|(n, _)| n == "b"));
        } else {
            panic!("expected Thunk");
        }
    }

    #[test]
    fn lambda_no_free_vars_empty_captures() {
        // A lambda with no free variables has empty captures
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let f = \\x => echo $x");
        if let Val::Thunk { captures, .. } = shell.env.get_value("f") {
            assert!(captures.is_empty());
        } else {
            panic!("expected Thunk");
        }
    }

    #[test]
    fn lambda_unbound_vars_not_captured() {
        // Variables not in scope at construction time are not captured
        // (they resolve to Unit, which we skip)
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let f = \\() => echo $unbound_var");
        if let Val::Thunk { captures, .. } = shell.env.get_value("f") {
            assert!(
                !captures.iter().any(|(n, _)| n == "unbound_var"),
                "unbound vars should not be captured: {captures:?}"
            );
        } else {
            panic!("expected Thunk");
        }
    }

    #[test]
    fn lambda_captures_restored_at_force_time() {
        // Verify captured values are visible in the thunk body at force time.
        // Use `let mut` so the variable can be reassigned, and pre-declare
        // `result` so the thunk body's assignment walks the scope chain.
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let mut x = captured_value");
        run_with_shell(&mut shell, "result = none");
        run_with_shell(&mut shell, "let f = \\() => { result = $x }");
        // Mutate x after capture
        run_with_shell(&mut shell, "x = mutated");
        // Force the thunk — it should see x=captured_value, not mutated
        run_with_shell(&mut shell, "$f");
        assert_eq!(
            shell.env.get_value("result"),
            Val::Str("captured_value".into()),
            "thunk body should see captured value, not current scope"
        );
    }

    #[test]
    fn lambda_param_shadows_capture() {
        // If a lambda param has the same name as a captured var, the param wins.
        // Pre-declare `result` so the thunk body's assignment walks the chain.
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let x = from_outer");
        run_with_shell(&mut shell, "result = none");
        // f has `x` as a param — x is NOT captured (subtracted from free vars)
        run_with_shell(&mut shell, "let f = \\x => { result = $x }");
        run_with_shell(&mut shell, "$f from_param");
        assert_eq!(
            shell.env.get_value("result"),
            Val::Str("from_param".into()),
            "param should shadow capture"
        );
    }

    // ── Tilde match operator ───────────────────────────────

    #[test]
    fn tilde_match_success() {
        let s = run_status("~ hello he*");
        assert!(s.is_success());
    }

    #[test]
    fn tilde_match_failure() {
        let s = run_status("~ hello wo*");
        assert!(!s.is_success());
    }

    // ── Discipline functions ───────────────────────────────

    #[test]
    fn set_discipline_fires() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = initial");
        run_with_shell(&mut shell, "let mut log = none");
        run_with_shell(&mut shell, "fn x.set { log = fired }");
        run_with_shell(&mut shell, "x = changed");
        assert_eq!(shell.env.get_value("log"), Val::Str("fired".into()));
    }

    // ── Redirections ───────────────────────────────────────

    #[test]
    fn redirect_output_creates_file() {
        let path = format!("/tmp/psh_test_redir_{}.txt", std::process::id());
        let input = format!("echo hello >{path}");
        run_status(&input);
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(content.trim(), "hello");
        let _ = std::fs::remove_file(&path);
    }

    // ── Nameref ────────────────────────────────────────────

    #[test]
    fn nameref_resolves() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "target = data");
        run_with_shell(&mut shell, "ref alias = target");
        assert_eq!(shell.env.get_value("alias"), Val::Str("data".into()));
    }

    // ── Arrow body ─────────────────────────────────────────

    #[test]
    fn if_arrow_body_works() {
        let s = run_status("if true => echo yes");
        assert!(s.is_success());
    }

    #[test]
    fn fn_arrow_body_works() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "fn greet => echo hello");
        let s = run_with_shell(&mut shell, "greet").status();
        assert!(s.is_success());
    }

    // ── And/Or/Not ─────────────────────────────────────────

    #[test]
    fn and_short_circuit() {
        let s = run_status("true && true");
        assert!(s.is_success());
        let s = run_status("false && true");
        assert!(!s.is_success());
    }

    #[test]
    fn or_short_circuit() {
        let s = run_status("true || false");
        assert!(s.is_success());
        let s = run_status("false || true");
        assert!(s.is_success());
    }

    #[test]
    fn not_inverts() {
        let s = run_status("! true");
        assert!(!s.is_success());
        let s = run_status("! false");
        assert!(s.is_success());
    }

    // ── Pipeline ───────────────────────────────────────────

    #[test]
    fn pipeline_works() {
        let s = run_status("echo hello | cat");
        assert!(s.is_success());
    }

    // ── Whatis ─────────────────────────────────────────────

    #[test]
    fn whatis_builtin() {
        let s = run_status("whatis echo");
        assert!(s.is_success());
    }

    // ── List operations ────────────────────────────────────

    #[test]
    fn list_assignment() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = (a b c)");
        assert_eq!(shell.env.get_value("x"), Val::Str("a b c".into()));
    }

    #[test]
    fn count_variable() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let x = (a b c)");
        // $#x should be 3
        let val = shell.eval_word(&Word::Count("x".into()));
        assert_eq!(val, Val::Int(3));
    }

    // ── Free variable collection ──────────────────────────

    #[test]
    fn free_vars_simple_var() {
        let prog = PshParser::parse("echo $x $y").unwrap();
        let vars = free_vars_in_commands(&prog.commands);
        assert!(vars.contains("x"));
        assert!(vars.contains("y"));
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn free_vars_no_vars() {
        let prog = PshParser::parse("echo hello world").unwrap();
        let vars = free_vars_in_commands(&prog.commands);
        assert!(vars.is_empty());
    }

    #[test]
    fn free_vars_count_and_stringify() {
        let prog = PshParser::parse("echo $#x $\"y").unwrap();
        let vars = free_vars_in_commands(&prog.commands);
        assert!(vars.contains("x"));
        assert!(vars.contains("y"));
    }

    #[test]
    fn free_vars_nested_in_if() {
        let prog = PshParser::parse("if true { echo $a } else { echo $b }").unwrap();
        let vars = free_vars_in_commands(&prog.commands);
        assert!(vars.contains("a"));
        assert!(vars.contains("b"));
    }

    // ── F5: Structural match arm ──────────────────────────────

    #[test]
    fn match_structural_arm() {
        // Construct a Sum value directly (tagged values not yet in parser)
        // and verify structural pattern match decomposes it correctly.
        let mut shell = Shell::new();
        // Inject a Sum("ok", Str("42")) directly into the environment
        let sum_val = Val::Sum("ok".into(), Box::new(Val::Str("42".into())));
        let _ = shell.env.set_value("x", sum_val);
        // Pre-declare result so the match arm's assignment walks scope
        let _ = shell.env.set_value("result", Val::Str("none".into()));
        // match $x { ok v => result = $v; err e => result = error }
        let outcome = run_with_shell(
            &mut shell,
            "match $x { ok v => result = $v; err e => result = error }",
        );
        assert!(outcome.status().is_success());
        assert_eq!(
            shell.env.get_value("result"),
            Val::Str("42".into()),
            "structural arm should bind payload to $v"
        );
    }

    // ── F6: Try abort prevents subsequent commands ────────────

    #[test]
    fn try_abort_prevents_subsequent() {
        let mut shell = Shell::new();
        let _ = shell.env.set_value("x", Val::Str("unset".into()));
        run_with_shell(&mut shell, "try { false; x = should_not_run }");
        assert_eq!(
            shell.env.get_value("x"),
            Val::Str("unset".into()),
            "false should abort try before x assignment"
        );
    }

    // ── F7: Try boolean-context exemption ─────────────────────

    #[test]
    fn try_boolean_context_exempt() {
        // Boolean contexts (&&/|| LHS) inside try do NOT trigger abort.
        // `false || true` evaluates false on the LHS (boolean context,
        // exempt) then true on the RHS — overall success.
        let mut shell = Shell::new();
        let _ = shell.env.set_value("x", Val::Str("unset".into()));
        run_with_shell(&mut shell, "try { false || true; x = should_run }");
        assert_eq!(
            shell.env.get_value("x"),
            Val::Str("should_run".into()),
            "boolean context (|| LHS) should not trigger try abort"
        );
    }

    // ── F8: .get discipline fires on access ───────────────────

    #[test]
    fn get_discipline_fires_on_access() {
        // .get runs in a readonly scope — it cannot mutate other variables.
        // Verify that accessing $x fires the .get body (notification-only)
        // and that the stored value is returned unchanged.
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = original");
        run_with_shell(&mut shell, "fn x.get { echo discipline_fired }");
        // Access $x — .get fires (output goes to stdout, we can't capture
        // it here but the important thing is it doesn't crash and the
        // stored value is returned)
        let val = shell.resolve_var("x");
        assert_eq!(val, Val::Str("original".into()));
    }

    // ── F9: Reentrancy guard for .set discipline ──────────────

    #[test]
    fn discipline_set_reentrancy_guard() {
        // fn x.set { x = $1 } would recurse infinitely without the guard.
        // The reentrancy guard in fire_set_discipline prevents this.
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "fn x.set { x = $1 }");
        run_with_shell(&mut shell, "x = hello");
        // Should not panic/overflow, and x should be "hello"
        assert_eq!(shell.env.get_value("x"), Val::Str("hello".into()));
    }

    // ── F10: Redirect save/restore roundtrip ──────────────────

    #[test]
    fn redirect_restores_fd() {
        let path = format!("/tmp/psh_test_redir_{}", std::process::id());
        let input = format!("echo hello >{path}");
        run_status(&input);
        // After redirect completes, stdout should be restored (not the file).
        // Verify the file contains only "hello".
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(content.trim(), "hello");
        // A second echo without redirect should NOT append to the file
        run_status("echo world");
        let content2 = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            content2.trim(),
            "hello",
            "stdout should be restored after redirect"
        );
        let _ = std::fs::remove_file(&path);
    }

    // ── F12: For loop body runs correct number of times ───────

    #[test]
    fn for_loop_runs_body() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "result = ''");
        // Accumulate loop variable values into result
        run_with_shell(
            &mut shell,
            "for x in (a b c) { result = `{ echo $result$x } }",
        );
        // result should contain "abc" (accumulated across 3 iterations)
        let val = shell.env.get_value("result").to_string();
        assert_eq!(val.trim(), "abc", "for loop should run body 3 times");
    }

    // ── F13: Redirect output with unique temp file ────────────

    #[test]
    fn redirect_output_unique_tempfile() {
        let path = format!("/tmp/psh_test_redir_unique_{}", std::process::id());
        let input = format!("echo hello >{path}");
        run_status(&input);
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(content.trim(), "hello");
        let _ = std::fs::remove_file(&path);
    }

    // ── F14: Thunk forcing with arguments ─────────────────────

    #[test]
    fn thunk_forcing_with_args() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let greet = \\name => echo hello $name");
        let s = run_with_shell(&mut shell, "$greet world").status();
        assert!(s.is_success(), "thunk forcing with args should succeed");
    }

    // ── F15: Let immutability enforcement ─────────────────────

    #[test]
    fn let_immutable_rejects_reassign() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let x = 42");
        let outcome = run_with_shell(&mut shell, "x = 99");
        // Reassignment of immutable let binding should fail
        assert!(
            !outcome.status().is_success(),
            "reassignment of immutable variable should fail"
        );
        // x should still be its original value
        assert_eq!(shell.env.get_value("x"), Val::Int(42));
    }

    // ── Accessor evaluation ───────────────────────────────────

    #[test]
    fn accessor_tuple_projection() {
        let mut shell = Shell::new();
        let _ = shell.env.set_value(
            "t",
            Val::Tuple(vec![Val::Int(42), Val::Str("hello".into())]),
        );
        // $t.0 → 42, $t.1 → "hello"
        run_with_shell(&mut shell, "x = $t.0");
        assert_eq!(shell.env.get_value("x"), Val::Str("42".into()));
        run_with_shell(&mut shell, "x = $t.1");
        assert_eq!(shell.env.get_value("x"), Val::Str("hello".into()));
    }

    #[test]
    fn accessor_tuple_out_of_bounds() {
        let mut shell = Shell::new();
        let _ = shell
            .env
            .set_value("t", Val::Tuple(vec![Val::Int(1), Val::Int(2)]));
        // .5 out of bounds → Unit → empty string via assignment
        run_with_shell(&mut shell, "x = $t.5");
        assert_eq!(shell.env.get_value("x"), Val::Str(String::new()));
    }

    #[test]
    fn accessor_sum_ok() {
        let mut shell = Shell::new();
        let _ = shell
            .env
            .set_value("r", Val::Sum("ok".into(), Box::new(Val::Int(42))));
        run_with_shell(&mut shell, "x = $r.ok");
        assert_eq!(shell.env.get_value("x"), Val::Str("42".into()));
    }

    #[test]
    fn accessor_sum_miss_is_unit() {
        let mut shell = Shell::new();
        let _ = shell
            .env
            .set_value("r", Val::Sum("ok".into(), Box::new(Val::Int(42))));
        // .err on an ok Sum → Unit (Prism miss)
        run_with_shell(&mut shell, "x = $r.err");
        assert_eq!(shell.env.get_value("x"), Val::Str(String::new()));
    }

    #[test]
    fn accessor_code_extracts_exit_code() {
        let mut shell = Shell::new();
        let _ = shell.env.set_value("e", Val::ExitCode(127));
        run_with_shell(&mut shell, "x = $e.code");
        assert_eq!(shell.env.get_value("x"), Val::Str("127".into()));
    }

    #[test]
    fn accessor_code_on_non_exitcode_is_unit() {
        let mut shell = Shell::new();
        let _ = shell.env.set_value("x", Val::Int(42));
        run_with_shell(&mut shell, "y = $x.code");
        assert_eq!(shell.env.get_value("y"), Val::Str(String::new()));
    }

    #[test]
    fn accessor_chain_prism_then_lens() {
        let mut shell = Shell::new();
        let _ = shell.env.set_value(
            "r",
            Val::Sum(
                "ok".into(),
                Box::new(Val::Tuple(vec![Val::Int(10), Val::Int(20)])),
            ),
        );
        // $r.ok.1 — Prism into ok, then tuple projection .1
        run_with_shell(&mut shell, "x = $r.ok.1");
        assert_eq!(shell.env.get_value("x"), Val::Str("20".into()));
    }

    #[test]
    fn accessor_chain_miss_absorbs() {
        let mut shell = Shell::new();
        let _ = shell
            .env
            .set_value("r", Val::Sum("ok".into(), Box::new(Val::Int(42))));
        // .err misses → Unit, then .0 on Unit → still Unit
        run_with_shell(&mut shell, "x = $r.err.0");
        assert_eq!(shell.env.get_value("x"), Val::Str(String::new()));
    }

    #[test]
    fn accessor_brace_var_escape_hatch() {
        let mut shell = Shell::new();
        let _ = shell.env.set_value("stem", Val::Str("main".into()));
        // ${stem}.c is free caret concat, not accessor
        run_with_shell(&mut shell, "x = ${stem}.c");
        assert_eq!(shell.env.get_value("x"), Val::Str("main.c".into()));
    }

    // ── Try in value position (Phase D2) ──────────────────────

    #[test]
    fn try_value_success() {
        let (_, outcome) = run("let result = try { echo 42 }");
        assert!(outcome.status().is_success());
    }

    #[test]
    fn try_value_success_produces_ok_sum() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let mut result = try { echo 42 }");
        let val = shell.env.get_value("result");
        assert_eq!(val, Val::Sum("ok".into(), Box::new(Val::Str("42".into()))));
    }

    #[test]
    fn try_value_failure_produces_err_sum() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let mut result = try { false }");
        let val = shell.env.get_value("result");
        // false exits 1 with no stdout
        match val {
            Val::Sum(tag, payload) => {
                assert_eq!(tag, "err");
                match *payload {
                    Val::ExitCode(code) => assert_ne!(code, 0),
                    other => panic!("expected ExitCode, got {other:?}"),
                }
            }
            other => panic!("expected Sum, got {other:?}"),
        }
    }

    #[test]
    fn try_value_ok_accessor() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let mut r = try { echo hello }");
        // Access .ok to get the payload
        run_with_shell(&mut shell, "x = $r.ok");
        assert_eq!(shell.env.get_value("x"), Val::Str("hello".into()));
    }

    #[test]
    fn try_value_err_accessor() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let mut r = try { false }");
        // .ok on err Sum → Unit (Prism miss)
        run_with_shell(&mut shell, "x = $r.ok");
        assert_eq!(shell.env.get_value("x"), Val::Str(String::new()));
        // .err gives ExitCode, .code extracts it
        run_with_shell(&mut shell, "y = $r.err.code");
        assert_ne!(shell.env.get_value("y"), Val::Str(String::new()));
    }

    // ── Named function parameters ─────────────────────────────

    #[test]
    fn fn_named_params_bind() {
        let mut shell = Shell::new();
        // Pre-declare x so assignment inside fn walks scope chain
        run_with_shell(&mut shell, "x = unset");
        run_with_shell(&mut shell, "fn greet(name) { x = $name }");
        run_with_shell(&mut shell, "greet world");
        assert_eq!(shell.env.get_value("x"), Val::Str("world".into()));
    }

    #[test]
    fn fn_named_params_and_positional() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = unset\ny = unset");
        run_with_shell(&mut shell, "fn f(a b) { x = $a; y = $1 }");
        run_with_shell(&mut shell, "f hello world");
        // Named param
        assert_eq!(shell.env.get_value("x"), Val::Str("hello".into()));
        // Positional $1 also works
        assert_eq!(shell.env.get_value("y"), Val::Str("hello".into()));
    }

    #[test]
    fn fn_named_params_missing_arg_is_unit() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = unset");
        run_with_shell(&mut shell, "fn f(a b) { x = $b }");
        run_with_shell(&mut shell, "f only_one");
        // $b was not provided → Unit → empty string
        assert_eq!(shell.env.get_value("x"), Val::Str(String::new()));
    }

    #[test]
    fn fn_discipline_set_named_param() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = unset");
        run_with_shell(&mut shell, "let mut v = 0");
        run_with_shell(&mut shell, "fn v.set(val) { x = $val }");
        run_with_shell(&mut shell, "v = 42");
        assert_eq!(shell.env.get_value("x"), Val::Str("42".into()));
    }

    // ── $0 and shift ──────────────────────────────────────────

    #[test]
    fn dollar_zero_default() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = $0");
        assert_eq!(shell.env.get_value("x"), Val::Str("psh".into()));
    }

    #[test]
    fn dollar_zero_set_argv0() {
        let mut shell = Shell::new();
        shell.set_argv0("myscript.psh");
        run_with_shell(&mut shell, "x = $0");
        assert_eq!(shell.env.get_value("x"), Val::Str("myscript.psh".into()));
    }

    #[test]
    fn shift_positional_params() {
        let mut shell = Shell::new();
        // Simulate a function call that sets positional params
        let _ = shell.env.set_value("1", Val::Str("a".into()));
        let _ = shell.env.set_value("2", Val::Str("b".into()));
        let _ = shell.env.set_value("3", Val::Str("c".into()));
        let _ = shell.env.set_value(
            "*",
            Val::List(vec![
                Val::Str("a".into()),
                Val::Str("b".into()),
                Val::Str("c".into()),
            ]),
        );
        run_with_shell(&mut shell, "shift");
        // $1 is now "b", $2 is "c"
        assert_eq!(shell.env.get_value("1"), Val::Str("b".into()));
        assert_eq!(shell.env.get_value("2"), Val::Str("c".into()));
    }

    #[test]
    fn shift_by_n() {
        let mut shell = Shell::new();
        let _ = shell.env.set_value("1", Val::Str("a".into()));
        let _ = shell.env.set_value("2", Val::Str("b".into()));
        let _ = shell.env.set_value("3", Val::Str("c".into()));
        let _ = shell.env.set_value(
            "*",
            Val::List(vec![
                Val::Str("a".into()),
                Val::Str("b".into()),
                Val::Str("c".into()),
            ]),
        );
        run_with_shell(&mut shell, "shift 2");
        assert_eq!(shell.env.get_value("1"), Val::Str("c".into()));
    }

    // ── Value-producing blocks (Phase 9) ──────────────────────

    #[test]
    fn compute_match_return() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = hello");
        run_with_shell(
            &mut shell,
            "let icon = match $x { hello => return yes; * => return no }",
        );
        assert_eq!(shell.env.get_value("icon"), Val::Str("yes".into()));
    }

    #[test]
    fn compute_if_return() {
        let mut shell = Shell::new();
        run_with_shell(
            &mut shell,
            "let v = if true { return ok } else { return fail }",
        );
        assert_eq!(shell.env.get_value("v"), Val::Str("ok".into()));
    }

    #[test]
    fn compute_if_else_return() {
        let mut shell = Shell::new();
        run_with_shell(
            &mut shell,
            "let v = if false { return ok } else { return fail }",
        );
        assert_eq!(shell.env.get_value("v"), Val::Str("fail".into()));
    }

    #[test]
    fn compute_block_return() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let v = { return 42 }");
        assert_eq!(shell.env.get_value("v"), Val::Int(42));
    }

    #[test]
    fn compute_block_effects_before_return() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "x = before");
        run_with_shell(&mut shell, "let v = { x = after; return done }");
        assert_eq!(shell.env.get_value("v"), Val::Str("done".into()));
        assert_eq!(shell.env.get_value("x"), Val::Str("after".into()));
    }

    #[test]
    fn compute_no_return_is_unit() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let v = { true }");
        assert_eq!(shell.env.get_value("v"), Val::Unit);
    }

    #[test]
    fn take_for_collect() {
        let mut shell = Shell::new();
        run_with_shell(&mut shell, "let result = for x in (a b c) { take $x }");
        assert_eq!(
            shell.env.get_value("result"),
            Val::List(vec![
                Val::Str("a".into()),
                Val::Str("b".into()),
                Val::Str("c".into()),
            ])
        );
    }

    #[test]
    fn take_for_filter() {
        let mut shell = Shell::new();
        run_with_shell(
            &mut shell,
            "let result = for x in (aa ab ba bb) { if ~ $x a* { take $x } }",
        );
        assert_eq!(
            shell.env.get_value("result"),
            Val::List(vec![Val::Str("aa".into()), Val::Str("ab".into())])
        );
    }

    #[test]
    fn take_nothing_is_unit() {
        let mut shell = Shell::new();
        run_with_shell(
            &mut shell,
            "let result = for x in (a b c) { if false { take $x } }",
        );
        assert_eq!(shell.env.get_value("result"), Val::Unit);
    }

    #[test]
    fn take_outside_for_is_error() {
        let (_, outcome) = run("take hello");
        assert!(!outcome.status().is_success());
    }
}
