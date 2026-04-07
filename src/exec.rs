//! Evaluator for psh.
//!
//! Walks the AST and executes it. fork/exec/wait for external
//! commands, direct dispatch for builtins. Pipeline wiring via
//! pipe(2). Variable expansion is CBV (eager). Pipeline stages
//! are CBN (concurrent, demand-driven).
//!
//! Uses rustix for pipe, open, waitpid, setpgid, and write.
//! Uses libc for fork, dup2, and execvp — these require raw fd
//! manipulation that rustix's owned-fd API does not support
//! (shells must redirect stdio fds that are not OwnedFd).

use std::{
    collections::HashSet,
    ffi::CString,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
    process,
};

use crate::{
    ast::*,
    env::Env,
    job::{Job, JobStatus, JobTable},
    value::Val,
};

/// Exit status — a string in rc tradition.
///
/// Plan 9: "On Plan 9 status is a character string describing
/// an error condition. On normal termination it is empty."
/// (Duff 1990, §Exit status)
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

// ── Safe wrappers around libc fd/process operations ─────────
//
// These thin wrappers centralize the unsafe blocks. Each wraps
// exactly one syscall and handles errors via Result/Option.

fn sys_fork() -> Result<libc::pid_t, String> {
    let pid = unsafe { libc::fork() };
    if pid == -1 {
        Err(format!("fork: {}", std::io::Error::last_os_error()))
    } else {
        Ok(pid)
    }
}

fn sys_dup(fd: i32) -> Result<i32, String> {
    let r = unsafe { libc::dup(fd) };
    if r == -1 {
        Err(format!("dup: {}", std::io::Error::last_os_error()))
    } else {
        Ok(r)
    }
}

fn sys_dup2(src: i32, dst: i32) {
    unsafe { libc::dup2(src, dst) };
}

fn sys_close(fd: i32) {
    unsafe { libc::close(fd) };
}

fn sys_setpgid(pid: libc::pid_t, pgid: libc::pid_t) {
    unsafe { libc::setpgid(pid, pgid) };
}

fn sys_tcsetpgrp(fd: i32, pgid: libc::pid_t) -> Result<(), String> {
    let r = unsafe { libc::tcsetpgrp(fd, pgid) };
    if r == -1 {
        Err(format!("tcsetpgrp: {}", std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

fn sys_killpg(pgid: libc::pid_t, sig: i32) -> Result<(), String> {
    let r = unsafe { libc::killpg(pgid, sig) };
    if r == -1 {
        Err(format!("killpg: {}", std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

fn sys_getpid() -> libc::pid_t {
    unsafe { libc::getpid() }
}

/// The shell interpreter state.
///
/// Kept decomposed — job table, environment, and discipline state
/// are separate concerns. (STYLEGUIDE.md: "Shell struct: keep
/// decomposed.")
/// Active coprocess state.
///
/// ksh93 heritage (io.c coprocess). Plan 9 heritage: bidirectional
/// socketpair rather than two unidirectional pipes. The shell holds
/// one end; the child holds the other. Both sides can read and write.
struct Coproc {
    /// Shell's end of the socketpair. Bidirectional.
    fd: OwnedFd,
    /// Child PID for wait semantics.
    pid: libc::pid_t,
    /// Job table index.
    job_id: usize,
}

pub struct Shell {
    pub env: Env,
    /// ksh93 heritage: reentrancy guard for discipline functions.
    /// Prevents `fn x.set { x = $1 }` from recursing infinitely.
    /// (src/cmd/ksh93/sh/nvdisc.c — nv_disc uses SH_VARNOD flag)
    active_disciplines: HashSet<String>,
    /// Job table for background and stopped processes.
    pub jobs: JobTable,
    /// Whether we're an interactive shell (controls terminal handling).
    interactive: bool,
    /// The shell's own process group id.
    shell_pgid: libc::pid_t,
    /// Active coprocess (ksh93: only one at a time).
    coproc: Option<Coproc>,
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
            active_disciplines: HashSet::new(),
            jobs: JobTable::new(),
            interactive: false,
            shell_pgid: 0,
            coproc: None,
        }
    }

    /// Set up the shell for interactive use.
    ///
    /// Takes control of the terminal: puts the shell in its own
    /// process group and makes it the foreground group. Ignores
    /// SIGTSTP so Ctrl-Z only affects the foreground job, not the
    /// shell itself. Also ignores SIGTTIN/SIGTTOU to prevent the
    /// shell from stopping on background terminal access.
    pub fn setup_interactive(&mut self) {
        self.interactive = true;
        let pid = sys_getpid();
        sys_setpgid(pid, pid);
        self.shell_pgid = pid;

        // Take the terminal — the shell must be the foreground group.
        let _ = sys_tcsetpgrp(libc::STDIN_FILENO, pid);

        // The shell ignores job control signals directed at it.
        // Only the foreground process group should respond to these.
        crate::signal::ignore_signal(libc::SIGTSTP);
        crate::signal::ignore_signal(libc::SIGTTIN);
        crate::signal::ignore_signal(libc::SIGTTOU);
    }

    /// Check for pending signals and dispatch to rc-style handler
    /// functions. Called after each command and in the interactive loop.
    ///
    /// rc heritage: signal handlers are functions named `fn sigint { }`,
    /// `fn sighup { }`, etc. If no function is defined, the default
    /// behavior applies.
    pub fn check_signals(&mut self) {
        let pending = crate::signal::take_pending();
        for (name, _count) in pending {
            if name == "sigchld" {
                self.reap_children();
            }

            // Look up rc-style handler function
            if let Some(body) = self.env.get_fn(name).cloned() {
                self.run_cmds(&body);
            }
        }
    }

    /// Fire the `sigexit` artificial signal — runs `fn sigexit { }`
    /// if defined. Called when the shell is about to exit.
    ///
    /// rc heritage: sigexit is not a real signal. It fires on shell
    /// exit, providing cleanup hooks.
    pub fn fire_sigexit(&mut self) {
        if let Some(body) = self.env.get_fn("sigexit").cloned() {
            self.run_cmds(&body);
        }
    }

    /// Print "[n] Done command" for completed background jobs.
    /// Called before each interactive prompt.
    pub fn notify_done_jobs(&mut self) {
        // Reap any children that exited since last check.
        self.reap_children();
        let done = self.jobs.collect_done();
        for (num, cmd) in done {
            eprintln!("[{num}] Done\t{cmd}");
        }
    }

    /// Reap all finished/stopped children via waitpid(-1, WNOHANG).
    /// Updates job table entries.
    fn reap_children(&mut self) {
        loop {
            let mut wstatus: i32 = 0;
            let pid = unsafe { libc::waitpid(-1, &mut wstatus, libc::WNOHANG | libc::WUNTRACED) };
            if pid <= 0 {
                break;
            }
            self.jobs.reap(pid, wstatus);
        }
    }

    /// Execute a parsed program.
    pub fn run(&mut self, program: &Program) -> Status {
        self.run_cmds(&program.commands)
    }

    fn run_cmds(&mut self, cmds: &[Command]) -> Status {
        let mut status = Status::ok();
        for cmd in cmds {
            status = self.run_cmd(cmd);
        }
        status
    }

    fn run_cmd(&mut self, cmd: &Command) -> Status {
        match cmd {
            Command::Bind(Binding::Assignment(name, value)) => {
                let val = self.eval_value(value);
                let disc_name = format!("{name}.set");
                // Fire .set discipline if defined and not already active
                // for this variable (reentrancy guard prevents infinite
                // recursion from `fn x.set { x = $1 }`).
                if let Some(body) = self.env.get_fn(&disc_name).cloned() {
                    if self.active_disciplines.insert(disc_name.clone()) {
                        let _ = self.env.set_value("1", val.clone());
                        let status = self.run_cmds(&body);
                        self.active_disciplines.remove(&disc_name);
                        if let Err(e) = self.env.set_value(name, val) {
                            return Status::err(format!("{name}: {e}"));
                        }
                        status
                    } else {
                        if let Err(e) = self.env.set_value(name, val) {
                            return Status::err(format!("{name}: {e}"));
                        }
                        Status::ok()
                    }
                } else {
                    if let Err(e) = self.env.set_value(name, val) {
                        self.fd_write_err(&format!("psh: {name}: {e}"));
                        return Status::err(format!("{name}: {e}"));
                    }
                    Status::ok()
                }
            }
            Command::Bind(Binding::Let {
                name,
                value,
                mutable,
                export,
                type_ann,
            }) => {
                let val = self.eval_value_for_let(value);
                if let Err(e) = self
                    .env
                    .let_value(name, val, *mutable, *export, type_ann.clone())
                {
                    self.fd_write_err(&format!("psh: {name}: {e}"));
                    return Status::err(format!("{name}: {e}"));
                }
                Status::ok()
            }
            Command::Bind(Binding::Fn { name, body }) => {
                self.env.define_fn(name.clone(), body.clone());
                Status::ok()
            }
            Command::Bind(Binding::Ref { name, target }) => {
                self.env.set_nameref(name, target.clone());
                Status::ok()
            }
            Command::Exec(expr) => self.run_expr(expr),
            Command::If {
                condition,
                then_body,
                else_body,
            } => {
                let cond_status = self.run_expr(condition);
                if cond_status.is_success() {
                    self.run_cmds(then_body)
                } else if let Some(else_body) = else_body {
                    self.run_cmds(else_body)
                } else {
                    cond_status
                }
            }
            Command::For { var, list, body } => {
                let items = self.eval_value(list);
                let mut status = Status::ok();
                for item in items.iter_elements() {
                    let _ = self.env.set_value(var, item.clone());
                    status = self.run_cmds(body);
                }
                status
            }
            Command::While { condition, body } => {
                let mut status = Status::ok();
                loop {
                    let cond_status = self.run_expr(condition);
                    if !cond_status.is_success() {
                        break;
                    }
                    status = self.run_cmds(body);
                }
                status
            }
            Command::Match { value, arms } => {
                let val = self.eval_value(value);
                let val_str = val.to_string();
                for (patterns, body) in arms {
                    for pat in patterns {
                        if self.match_pattern(&val, &val_str, pat) {
                            return self.run_cmds(body);
                        }
                    }
                }
                Status::ok()
            }
            Command::Return(val) => {
                if let Some(v) = val {
                    let evaluated = self.eval_value(v);
                    let s = evaluated.to_string();
                    let _ = self.env.set_value("status", Val::scalar(&s));
                    if s.is_empty() {
                        Status::ok()
                    } else {
                        Status(s)
                    }
                } else {
                    let current = self.env.get_value("status");
                    Status(current.to_string())
                }
            }
        }
    }

    fn run_expr(&mut self, expr: &Expr) -> Status {
        match expr {
            Expr::Simple(cmd) => self.run_simple(cmd),
            Expr::Pipeline(stages) => self.run_pipeline(stages),
            Expr::Redirect(inner, op) => self.run_redirect(inner, op),
            Expr::And(left, right) => {
                let status = self.run_expr(left);
                if status.is_success() {
                    self.run_expr(right)
                } else {
                    status
                }
            }
            Expr::Or(left, right) => {
                let status = self.run_expr(left);
                if !status.is_success() {
                    self.run_expr(right)
                } else {
                    status
                }
            }
            Expr::Not(inner) => {
                let status = self.run_expr(inner);
                if status.is_success() {
                    Status::from_code(1)
                } else {
                    Status::ok()
                }
            }
            Expr::Background(inner) => {
                let cmd_str = format!("{inner:?}");
                match sys_fork() {
                    Err(e) => Status::err(e),
                    Ok(0) => {
                        // Child: put in its own process group so
                        // it doesn't receive the terminal's signals.
                        sys_setpgid(0, 0);
                        let status = self.run_expr(inner);
                        // _exit in forked children: don't run atexit handlers or
                        // Rust destructors. The kernel closes all fds on process exit.
                        unsafe { libc::_exit(if status.is_success() { 0 } else { 1 }) };
                    }
                    Ok(pid) => {
                        sys_setpgid(pid, pid);
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
            Expr::Block(stmts) => self.run_cmds(stmts),
            Expr::Subshell(stmts) => match sys_fork() {
                Err(e) => Status::err(e),
                Ok(0) => {
                    let status = self.run_cmds(stmts);
                    // _exit in forked children: don't run atexit handlers or
                    // Rust destructors. The kernel closes all fds on process exit.
                    unsafe { libc::_exit(if status.is_success() { 0 } else { 1 }) };
                }
                Ok(pid) => self.wait_pid(pid),
            },
            Expr::Coprocess(inner) => self.run_coprocess(inner),
        }
    }

    fn run_simple(&mut self, cmd: &SimpleCommand) -> Status {
        // Resolve the command name. For literals, use the raw string
        // for builtin/function dispatch (avoids tilde expansion
        // turning `~` into $home before we can match the builtin).
        // Tilde expansion only matters for external command paths.
        let raw_name = match &cmd.name {
            Word::Literal(s) => Some(s.as_str()),
            _ => None,
        };
        let name_str = raw_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.eval_word(&cmd.name).as_str().to_string());

        // rc heritage: ~ takes its patterns literally — glob metacharacters
        // are match patterns, not filesystem globs. Skip glob expansion
        // for ~ arguments.
        let skip_glob = name_str == "~";

        let mut args: Vec<String> = Vec::new();
        for arg in &cmd.args {
            let val = self.eval_word(arg);
            for s in val.to_args() {
                if !skip_glob && has_glob_meta(&s) {
                    // Expand glob against filesystem.
                    // If no matches, pass the pattern through literally (rc convention).
                    let expanded = glob_expand(&s);
                    if expanded.is_empty() {
                        args.push(s);
                    } else {
                        args.extend(expanded);
                    }
                } else {
                    args.push(s);
                }
            }
        }

        // `builtin` bypasses function lookup — allows patterns like
        // `fn cd { builtin cd $* && update_prompt }` without recursion.
        if name_str == "builtin" {
            if args.is_empty() {
                let status = Status::err("builtin: usage: builtin command [args...]");
                let _ = self.env.set_value("status", Val::scalar(&status.0));
                return status;
            }
            let builtin_name = &args[0];
            let builtin_args = &args[1..];
            let status = self.dispatch_builtin(builtin_name, builtin_args);
            let _ = self.env.set_value("status", Val::scalar(&status.0));
            return status;
        }

        // Function call
        if let Some(body) = self.env.get_fn(&name_str).cloned() {
            self.env.push_scope();
            for (i, arg) in args.iter().enumerate() {
                let _ = self
                    .env
                    .set_value(&(i + 1).to_string(), Val::scalar(arg.clone()));
            }
            let _ = self
                .env
                .set_value("*", Val::list(args.iter().map(|s| s.as_str())));
            let status = self.run_cmds(&body);
            self.env.pop_scope();
            let _ = self.env.set_value("status", Val::scalar(&status.0));
            return status;
        }

        // Builtins
        let status = self.dispatch_builtin(&name_str, &args);

        let _ = self.env.set_value("status", Val::scalar(&status.0));
        status
    }

    fn run_pipeline(&mut self, stages: &[Expr]) -> Status {
        if stages.len() == 1 {
            return self.run_expr(&stages[0]);
        }

        let mut prev_read_fd: Option<OwnedFd> = None;
        let mut children: Vec<libc::pid_t> = Vec::new();
        let mut pgid: libc::pid_t = 0;

        for (i, stage) in stages.iter().enumerate() {
            let is_last = i == stages.len() - 1;

            let pipe_fds = if !is_last {
                match rustix::pipe::pipe() {
                    Ok(fds) => Some(fds),
                    Err(e) => return Status::err(format!("pipe: {e}")),
                }
            } else {
                None
            };

            match sys_fork() {
                Err(e) => return Status::err(e),
                Ok(0) => {
                    // Child: join the pipeline's process group
                    sys_setpgid(0, pgid);

                    if let Some(read_fd) = prev_read_fd {
                        sys_dup2(read_fd.as_raw_fd(), 0);
                        // OwnedFd drops and closes automatically
                    }
                    if let Some((_, ref write_end)) = pipe_fds {
                        sys_dup2(write_end.as_raw_fd(), 1);
                    }
                    // Drop pipe fds — child doesn't need the read end
                    // of its own pipe or the write end after dup2
                    drop(pipe_fds);

                    let status = self.run_expr(stage);
                    // _exit in forked children: don't run atexit handlers or
                    // Rust destructors. The kernel closes all fds on process exit.
                    unsafe { libc::_exit(if status.is_success() { 0 } else { 1 }) };
                }
                Ok(pid) => {
                    if pgid == 0 {
                        pgid = pid;
                    }
                    // Parent also calls setpgid to handle the race
                    sys_setpgid(pid, pgid);

                    children.push(pid);
                    // Close write end in parent, keep read end for next stage
                    if let Some((read_end, _write_end)) = pipe_fds {
                        // _write_end drops and closes automatically
                        drop(prev_read_fd);
                        prev_read_fd = Some(read_end);
                    } else {
                        drop(prev_read_fd);
                        prev_read_fd = None;
                    }
                }
            }
        }
        drop(prev_read_fd);

        let mut statuses = Vec::new();
        for pid in &children {
            statuses.push(self.wait_pid(*pid));
        }

        let _ = self.env.set_value(
            "pipestatus",
            Val::list(statuses.iter().map(|s| s.0.as_str())),
        );

        statuses.pop().unwrap_or_else(Status::ok)
    }

    /// Start a coprocess: `cmd |&`
    ///
    /// Creates a socketpair (Plan 9-style bidirectional pipe).
    /// The child gets one end on stdin/stdout; the shell holds
    /// the other for read -p / print -p.
    fn run_coprocess(&mut self, inner: &Expr) -> Status {
        if self.coproc.is_some() {
            return Status::err("coprocess: a coprocess is already running");
        }

        // socketpair gives two bidirectional endpoints
        let (shell_end, child_end) = match rustix::net::socketpair(
            rustix::net::AddressFamily::UNIX,
            rustix::net::SocketType::STREAM,
            rustix::net::SocketFlags::empty(),
            None,
        ) {
            Ok(pair) => pair,
            Err(e) => return Status::err(format!("coprocess: socketpair: {e}")),
        };

        match sys_fork() {
            Err(e) => Status::err(e),
            Ok(0) => {
                // Child: wire child_end to stdin and stdout
                let child_raw = child_end.as_raw_fd();
                sys_dup2(child_raw, 0);
                sys_dup2(child_raw, 1);
                drop(child_end);
                drop(shell_end);

                let status = self.run_expr(inner);
                unsafe { libc::_exit(if status.is_success() { 0 } else { 1 }) };
            }
            Ok(pid) => {
                drop(child_end);

                // Track in job table
                let cmd_str = format!("coprocess (pid {})", pid);
                let job_id = self.jobs.insert(Job {
                    pgid: pid,
                    pids: vec![pid],
                    command: cmd_str,
                    status: JobStatus::Running,
                });

                self.coproc = Some(Coproc {
                    fd: shell_end,
                    pid,
                    job_id,
                });

                eprintln!("[{}] {}", job_id, pid);
                Status::ok()
            }
        }
    }

    /// Create a pipe with content written to the write end, returning
    /// the read end. Used for here-documents and here-strings.
    fn make_heredoc_fd(&self, content: &str) -> Result<OwnedFd, String> {
        let (read_end, write_end) = rustix::pipe::pipe().map_err(|e| format!("pipe: {e}"))?;
        // Write content and close the write end so the reader sees EOF.
        let _ = rustix::io::write(&write_end, content.as_bytes());
        drop(write_end);
        Ok(read_end)
    }

    fn run_redirect(&mut self, inner: &Expr, op: &RedirectOp) -> Status {
        match op {
            RedirectOp::Output { fd, target, append } => {
                let path = match target {
                    RedirectTarget::File(word) => self.eval_word(word).as_str().to_string(),
                    _ => return Status::err("unsupported redirect target"),
                };

                let mut flags = rustix::fs::OFlags::WRONLY | rustix::fs::OFlags::CREATE;
                if *append {
                    flags |= rustix::fs::OFlags::APPEND;
                } else {
                    flags |= rustix::fs::OFlags::TRUNC;
                }

                let file_fd =
                    match rustix::fs::open(&*path, flags, rustix::fs::Mode::from_raw_mode(0o666)) {
                        Ok(fd) => fd,
                        Err(e) => return Status::err(format!("cannot open {path}: {e}")),
                    };

                let target_raw = *fd as i32;
                let saved = match sys_dup(target_raw) {
                    Ok(fd) => fd,
                    Err(e) => return Status::err(e),
                };
                sys_dup2(file_fd.as_raw_fd(), target_raw);
                drop(file_fd); // close the original fd

                let status = self.run_expr(inner);

                sys_dup2(saved, target_raw);
                sys_close(saved);
                status
            }
            RedirectOp::Input { fd, target } => {
                let target_raw = *fd as i32;

                let read_fd: OwnedFd = match target {
                    RedirectTarget::File(word) => {
                        let path = self.eval_word(word).as_str().to_string();
                        match rustix::fs::open(
                            &*path,
                            rustix::fs::OFlags::RDONLY,
                            rustix::fs::Mode::empty(),
                        ) {
                            Ok(fd) => fd,
                            Err(e) => return Status::err(format!("cannot open {path}: {e}")),
                        }
                    }
                    RedirectTarget::HereDoc(content) => match self.make_heredoc_fd(content) {
                        Ok(fd) => fd,
                        Err(e) => return Status::err(e),
                    },
                    RedirectTarget::HereString(word) => {
                        let mut content = self.eval_word(word).to_string();
                        content.push('\n');
                        match self.make_heredoc_fd(&content) {
                            Ok(fd) => fd,
                            Err(e) => return Status::err(e),
                        }
                    }
                };

                let saved = match sys_dup(target_raw) {
                    Ok(fd) => fd,
                    Err(e) => return Status::err(e),
                };
                sys_dup2(read_fd.as_raw_fd(), target_raw);
                drop(read_fd);

                let status = self.run_expr(inner);

                sys_dup2(saved, target_raw);
                sys_close(saved);
                status
            }
            RedirectOp::Dup { dst, src } => {
                let dst_raw = *dst as i32;
                let src_raw = *src as i32;
                let saved = match sys_dup(dst_raw) {
                    Ok(fd) => fd,
                    Err(e) => return Status::err(e),
                };
                sys_dup2(src_raw, dst_raw);

                let status = self.run_expr(inner);

                sys_dup2(saved, dst_raw);
                sys_close(saved);
                status
            }
            RedirectOp::Close { fd } => {
                let fd_raw = *fd as i32;
                let saved = match sys_dup(fd_raw) {
                    Ok(fd) => fd,
                    Err(e) => return Status::err(e),
                };
                sys_close(fd_raw);

                let status = self.run_expr(inner);

                sys_dup2(saved, fd_raw);
                sys_close(saved);
                status
            }
        }
    }

    /// Dispatch a command name directly to the builtin table,
    /// falling through to exec if not a builtin.
    fn dispatch_builtin(&mut self, name: &str, args: &[String]) -> Status {
        match name {
            "cd" => self.builtin_cd(args),
            "echo" => self.builtin_echo(args),
            "exit" => self.builtin_exit(args),
            "get" => self.builtin_get(args),
            "set" => self.builtin_set(args),
            "wait" => self.builtin_wait(args),
            "jobs" => self.builtin_jobs(args),
            "fg" => self.builtin_fg(args),
            "bg" => self.builtin_bg(args),
            "read" => self.builtin_read(args),
            "print" => self.builtin_print(args),
            "true" => Status::ok(),
            "false" => Status::from_code(1),
            "~" => self.builtin_match(args),
            "." => self.builtin_source(args),
            "whatis" => self.builtin_whatis(args),
            "builtin" => {
                if args.is_empty() {
                    Status::err("builtin: usage: builtin command [args...]")
                } else {
                    self.dispatch_builtin(&args[0], &args[1..])
                }
            }
            _ => self.exec_external(name, args),
        }
    }

    // ── Word evaluation (CBV — eager) ───────────────────────

    fn eval_word(&mut self, word: &Word) -> Val {
        match word {
            Word::Literal(s) => {
                // Tilde expansion: ~/path → $home/path, ~ alone → $home
                if s == "~" {
                    return self.env.get_value("home");
                }
                if let Some(rest) = s.strip_prefix("~/") {
                    let home = self.env.get_value("home");
                    return Val::scalar(format!("{}/{rest}", home.as_str()));
                }
                Val::scalar(s.clone())
            }
            // Quoted strings always produce Str — no inference
            Word::Quoted(s) => Val::Str(s.clone()),
            Word::Var(name) => {
                let disc_name = format!("{name}.get");
                if let Some(body) = self.env.get_fn(&disc_name).cloned() {
                    if self.active_disciplines.insert(disc_name.clone()) {
                        // .get discipline runs in a readonly scope to enforce
                        // purity — mutations inside .get are discarded.
                        self.env.push_readonly_scope();
                        self.run_cmds(&body);
                        self.env.pop_scope();
                        self.active_disciplines.remove(&disc_name);
                    }
                }
                self.env.get_value(name)
            }
            Word::Index(name, idx) => {
                let val = self.env.get_value(name);
                let idx_val = self.eval_word(idx);
                let idx: usize = idx_val.as_str().parse().unwrap_or(0);
                val.index(idx)
            }
            Word::Count(name) => {
                let val = self.env.get_value(name);
                Val::Int(val.count() as i64)
            }
            Word::CommandSub(stmts) => {
                let (read_end, write_end) = match rustix::pipe::pipe() {
                    Ok(fds) => fds,
                    Err(_) => return Val::empty(),
                };

                match sys_fork() {
                    Err(_) => Val::empty(),
                    Ok(0) => {
                        drop(read_end);
                        sys_dup2(write_end.as_raw_fd(), 1);
                        drop(write_end);

                        let status = self.run_cmds(stmts);
                        // Flush buffered Rust stdout before exit
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                        // _exit in forked children: don't run atexit handlers or
                        // Rust destructors. The kernel closes all fds on process exit.
                        unsafe { libc::_exit(if status.is_success() { 0 } else { 1 }) };
                    }
                    Ok(pid) => {
                        drop(write_end);
                        let mut output = String::new();
                        let file = unsafe { std::fs::File::from_raw_fd(read_end.into_raw_fd()) };
                        use std::io::Read;
                        let mut reader = std::io::BufReader::new(file);
                        let _ = reader.read_to_string(&mut output);

                        self.wait_pid(pid);

                        if output.ends_with('\n') {
                            output.pop();
                        }
                        if output.contains('\n') {
                            Val::list(output.lines())
                        } else {
                            Val::scalar(output)
                        }
                    }
                }
            }
            Word::ProcessSub(stmts) => {
                // rc heritage: <{cmd} evaluates to /dev/fd/N where N is
                // the read end of a pipe connected to the command's stdout.
                let (read_end, write_end) = match rustix::pipe::pipe() {
                    Ok(fds) => fds,
                    Err(_) => return Val::empty(),
                };

                match sys_fork() {
                    Err(_) => Val::empty(),
                    Ok(0) => {
                        drop(read_end);
                        sys_dup2(write_end.as_raw_fd(), 1);
                        drop(write_end);

                        let status = self.run_cmds(stmts);
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                        unsafe { libc::_exit(if status.is_success() { 0 } else { 1 }) };
                    }
                    Ok(_pid) => {
                        drop(write_end);
                        let raw_fd = read_end.into_raw_fd();
                        // Clear CLOEXEC so child processes (e.g., cat, diff)
                        // can access the fd.
                        unsafe {
                            let flags = libc::fcntl(raw_fd, libc::F_GETFD);
                            libc::fcntl(raw_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
                        }
                        Val::scalar(format!("/dev/fd/{raw_fd}"))
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
            // rc heritage: $"x joins list elements with spaces
            Word::Stringify(name) => {
                let val = self.env.get_value(name);
                Val::scalar(val.to_string())
            }
        }
    }

    fn eval_value(&mut self, value: &Value) -> Val {
        match value {
            Value::Word(word) => self.eval_word(word),
            Value::List(words) => {
                if words.is_empty() {
                    return Val::Unit;
                }
                let mut items = Vec::new();
                for word in words {
                    let val = self.eval_word(word);
                    match val {
                        Val::List(inner) => items.extend(inner),
                        Val::Unit => {}
                        other => items.push(other),
                    }
                }
                if items.is_empty() {
                    Val::Unit
                } else {
                    Val::List(items)
                }
            }
        }
    }

    /// Evaluate a word in let context — runs type inference on
    /// Literal values but preserves Quoted values as Str.
    fn eval_word_for_let(&mut self, word: &Word) -> Val {
        match word {
            // Quoted strings always stay Str — no inference
            Word::Quoted(s) => Val::Str(s.clone()),
            // Literals get type inference in let context
            Word::Literal(s) => {
                if s == "~" {
                    return self.env.get_value("home");
                }
                if let Some(rest) = s.strip_prefix("~/") {
                    let home = self.env.get_value("home");
                    return Val::infer(&format!("{}/{rest}", home.as_str()));
                }
                Val::infer(s)
            }
            // Count produces Int directly
            Word::Count(name) => {
                let val = self.env.get_value(name);
                Val::Int(val.count() as i64)
            }
            // Everything else evaluates normally, then infer if Str
            _ => {
                let val = self.eval_word(word);
                match val {
                    Val::Str(s) => Val::infer(&s),
                    Val::List(items) => Val::List(
                        items
                            .into_iter()
                            .map(|v| match v {
                                Val::Str(s) => Val::infer(&s),
                                other => other,
                            })
                            .collect(),
                    ),
                    other => other,
                }
            }
        }
    }

    /// Evaluate a Value in let context — runs type inference
    /// on Literal words but not Quoted words.
    fn eval_value_for_let(&mut self, value: &Value) -> Val {
        match value {
            Value::Word(word) => self.eval_word_for_let(word),
            Value::List(words) => {
                if words.is_empty() {
                    return Val::Unit;
                }
                let mut items = Vec::new();
                for word in words {
                    let val = self.eval_word_for_let(word);
                    match val {
                        Val::List(inner) => items.extend(inner),
                        Val::Unit => {}
                        other => items.push(other),
                    }
                }
                if items.is_empty() {
                    Val::Unit
                } else {
                    Val::List(items)
                }
            }
        }
    }

    // ── Builtins ────────────────────────────────────────────

    fn builtin_cd(&mut self, args: &[String]) -> Status {
        let dir = if args.is_empty() {
            self.env.get_value("home").as_str().to_string()
        } else {
            args[0].clone()
        };
        match std::env::set_current_dir(&dir) {
            Ok(()) => Status::ok(),
            Err(e) => Status::err(format!("cd: {dir}: {e}")),
        }
    }

    /// Write output for builtins using fd 1 directly via rustix::io::write.
    /// This fixes the command substitution flush bug: println! goes
    /// through Rust's buffered stdout, which may not flush before
    /// _exit in a forked child.
    fn fd_write_line(&self, s: &str) {
        use rustix::fd::BorrowedFd;
        let stdout = unsafe { BorrowedFd::borrow_raw(1) };
        let mut buf = s.to_string();
        buf.push('\n');
        let _ = rustix::io::write(stdout, buf.as_bytes());
    }

    /// Write error messages to fd 2 (stderr) directly.
    fn fd_write_err(&self, s: &str) {
        use rustix::fd::BorrowedFd;
        let stderr = unsafe { BorrowedFd::borrow_raw(2) };
        let mut buf = s.to_string();
        buf.push('\n');
        let _ = rustix::io::write(stderr, buf.as_bytes());
    }

    fn builtin_echo(&self, args: &[String]) -> Status {
        self.fd_write_line(&args.join(" "));
        Status::ok()
    }

    fn builtin_exit(&self, args: &[String]) -> Status {
        let code = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        process::exit(code);
    }

    /// `get name` — prints the value of a variable.
    /// Routes through word evaluation to fire .get discipline.
    fn builtin_get(&mut self, args: &[String]) -> Status {
        if args.is_empty() {
            return Status::err("get: usage: get name");
        }
        let name = &args[0];

        if name.starts_with("/pane/") {
            #[cfg(feature = "pane")]
            {
                return Status::err("get: pane queries not yet implemented");
            }
            #[cfg(not(feature = "pane"))]
            {
                return Status::err(format!(
                    "get: {name}: pane support not compiled (build with --features pane)"
                ));
            }
        }

        // Route through eval_word to fire .get discipline
        let val = self.eval_word(&Word::Var(name.clone()));
        if val.is_true() {
            self.fd_write_line(&val.to_string());
            Status::ok()
        } else {
            Status::err(format!("get: {name}: not set"))
        }
    }

    fn builtin_set(&mut self, args: &[String]) -> Status {
        if args.len() < 2 {
            return Status::err("set: usage: set name value...");
        }
        let name = &args[0];
        let value = if args.len() == 2 {
            Val::scalar(args[1].clone())
        } else {
            Val::list(args[1..].iter().map(|s| s.as_str()))
        };

        if name.starts_with("/pane/") {
            #[cfg(feature = "pane")]
            {
                return Status::err("set: pane writes not yet implemented");
            }
            #[cfg(not(feature = "pane"))]
            {
                return Status::err(format!(
                    "set: {name}: pane support not compiled (build with --features pane)"
                ));
            }
        }

        if let Err(e) = self.env.set_value(name, value) {
            return Status::err(format!("set: {name}: {e}"));
        }
        Status::ok()
    }

    fn builtin_source(&mut self, args: &[String]) -> Status {
        if args.is_empty() {
            return Status::err(".: usage: . file");
        }
        let content = match std::fs::read_to_string(&args[0]) {
            Ok(s) => s,
            Err(e) => return Status::err(format!(".: {}: {e}", args[0])),
        };
        match crate::parse::Parser::parse(&content) {
            Ok(prog) => self.run(&prog),
            Err(e) => Status::err(format!(".: {}: {e}", args[0])),
        }
    }

    /// rc heritage: `whatis name` tells you what `name` is.
    /// (Duff 1990, §Builtins)
    fn builtin_whatis(&mut self, args: &[String]) -> Status {
        if args.is_empty() {
            return Status::err("whatis: usage: whatis name");
        }
        let mut status = Status::ok();
        for name in args {
            if !self.whatis_one(name) {
                status = Status::from_code(1);
            }
        }
        status
    }

    /// Print identification for a single name. Returns true if found.
    fn whatis_one(&self, name: &str) -> bool {
        let mut found = false;

        // Check functions (including discipline functions)
        if self.env.get_fn(name).is_some() {
            self.fd_write_line(&format!("fn {name} {{...}}"));
            found = true;
        }

        // Check builtins
        if is_builtin(name) {
            self.fd_write_line(&format!("builtin {name}"));
            found = true;
        }

        // Check $path for external commands
        if let Some(path) = self.find_in_path(name) {
            self.fd_write_line(&path);
            found = true;
        }

        // Check variables
        let val = self.env.get_value(name);
        if val.is_true() {
            if val.count() == 1 {
                self.fd_write_line(&format!("{name} = {val}"));
            } else {
                let items: Vec<String> = val.to_args();
                self.fd_write_line(&format!("{name} = ({})", items.join(" ")));
            }
            found = true;
        }

        if !found {
            use rustix::fd::BorrowedFd;
            let stderr = unsafe { BorrowedFd::borrow_raw(2) };
            let msg = format!("whatis: {name}: not found\n");
            let _ = rustix::io::write(stderr, msg.as_bytes());
        }
        found
    }

    /// Search $path for an executable named `name`.
    fn find_in_path(&self, name: &str) -> Option<String> {
        // If name contains '/', it's already a path
        if name.contains('/') {
            return if std::fs::metadata(name).is_ok_and(|m| !m.is_dir()) {
                Some(name.to_string())
            } else {
                None
            };
        }
        let path_val = self.env.get_value("path");
        for dir in path_val.to_args() {
            let full = format!("{dir}/{name}");
            if std::fs::metadata(&full).is_ok_and(|m| !m.is_dir()) {
                return Some(full);
            }
        }
        None
    }

    /// `~ value pattern [pattern...]` — rc match operator.
    ///
    /// Returns success if value matches any pattern, failure otherwise.
    /// Patterns use fnmatch glob syntax (*, ?, [chars]).
    /// (Duff 1990, §Simple commands — ~ is a builtin)
    fn builtin_match(&self, args: &[String]) -> Status {
        if args.len() < 2 {
            return Status::err("~: usage: ~ value pattern...");
        }
        let value = &args[0];
        for pattern in &args[1..] {
            if pattern == value {
                return Status::ok();
            }
            if has_glob_meta(pattern) {
                if let Ok(re) = fnmatch_regex::glob_to_regex(pattern) {
                    if re.is_match(value) {
                        return Status::ok();
                    }
                }
            }
        }
        Status::from_code(1)
    }

    // ── Coprocess builtins ────────────────────────────────────

    /// `read [-p] var` — read a line from stdin (or coprocess with -p).
    ///
    /// Stores the result in $var. Returns success if a line was read,
    /// failure on EOF.
    fn builtin_read(&mut self, args: &[String]) -> Status {
        let (from_coproc, var_args) = if args.first().is_some_and(|a| a == "-p") {
            (true, &args[1..])
        } else {
            (false, &args[..])
        };

        let var_name = match var_args.first() {
            Some(name) => name.clone(),
            None => return Status::err("read: usage: read [-p] var"),
        };

        let mut line = String::new();

        if from_coproc {
            // Read from coprocess
            let coproc_fd = match &self.coproc {
                Some(c) => c.fd.as_raw_fd(),
                None => return Status::err("read: no coprocess"),
            };
            // Read one byte at a time until newline or EOF
            let mut buf = [0u8; 1];
            loop {
                let fd = unsafe { rustix::fd::BorrowedFd::borrow_raw(coproc_fd) };
                match rustix::io::read(fd, &mut buf) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if buf[0] == b'\n' {
                            break;
                        }
                        line.push(buf[0] as char);
                    }
                    Err(_) => break,
                }
            }
        } else {
            // Read from stdin
            use std::io::BufRead;
            let stdin = std::io::stdin();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => return Status::from_code(1), // EOF
                Ok(_) => {
                    if line.ends_with('\n') {
                        line.pop();
                    }
                }
                Err(_) => return Status::from_code(1),
            }
        }

        let _ = self.env.set_value(&var_name, Val::scalar(line));
        Status::ok()
    }

    /// `print [-p] args...` — print to stdout (or coprocess with -p).
    ///
    /// ksh93 heritage. Like echo but with -p for coprocess output.
    fn builtin_print(&mut self, args: &[String]) -> Status {
        let (to_coproc, print_args) = if args.first().is_some_and(|a| a == "-p") {
            (true, &args[1..])
        } else {
            (false, &args[..])
        };

        let output = format!("{}\n", print_args.join(" "));

        if to_coproc {
            let coproc_fd = match &self.coproc {
                Some(c) => c.fd.as_raw_fd(),
                None => return Status::err("print: no coprocess"),
            };
            let fd = unsafe { rustix::fd::BorrowedFd::borrow_raw(coproc_fd) };
            match rustix::io::write(fd, output.as_bytes()) {
                Ok(_) => Status::ok(),
                Err(e) => Status::err(format!("print: write: {e}")),
            }
        } else {
            self.fd_write_line(&print_args.join(" "));
            Status::ok()
        }
    }

    // ── Job control builtins ─────────────────────────────────

    /// Parse a job spec like "%1" or "%2". Returns the 1-based job number.
    fn parse_jobspec(arg: &str) -> Option<usize> {
        arg.strip_prefix('%').and_then(|n| n.parse().ok())
    }

    /// `wait` — wait for background jobs.
    ///
    /// `wait` with no arguments waits for all background jobs.
    /// `wait %n` waits for a specific job.
    fn builtin_wait(&mut self, args: &[String]) -> Status {
        if let Some(arg) = args.first() {
            let Some(num) = Self::parse_jobspec(arg) else {
                return Status::err(format!("wait: {arg}: not a valid job spec"));
            };
            let Some(job) = self.jobs.get(num) else {
                return Status::err(format!("wait: %{num}: no such job"));
            };
            if matches!(job.status, JobStatus::Done(_)) {
                let status = match &job.status {
                    JobStatus::Done(s) => s.clone(),
                    _ => Status::ok(),
                };
                self.jobs.remove(num);
                return status;
            }
            let pgid = job.pgid;
            // Wait for all pids in the job
            loop {
                let mut wstatus: i32 = 0;
                let r = unsafe { libc::waitpid(-pgid, &mut wstatus, libc::WUNTRACED) };
                if r <= 0 {
                    break;
                }
                self.jobs.reap(r, wstatus);
                // Check if job is done
                if let Some(j) = self.jobs.get(num) {
                    if matches!(j.status, JobStatus::Done(_) | JobStatus::Stopped) {
                        break;
                    }
                } else {
                    break;
                }
            }
            let status = self
                .jobs
                .get(num)
                .map(|j| match &j.status {
                    JobStatus::Done(s) => s.clone(),
                    JobStatus::Stopped => Status::err("stopped"),
                    JobStatus::Running => Status::ok(),
                })
                .unwrap_or_else(Status::ok);
            // Clean up done jobs
            if self
                .jobs
                .get(num)
                .is_some_and(|j| matches!(j.status, JobStatus::Done(_)))
            {
                self.jobs.remove(num);
            }
            status
        } else {
            // Wait for all background jobs
            loop {
                let mut wstatus: i32 = 0;
                let r = unsafe { libc::waitpid(-1, &mut wstatus, libc::WUNTRACED) };
                if r <= 0 {
                    break;
                }
                self.jobs.reap(r, wstatus);
            }
            self.jobs.collect_done();
            Status::ok()
        }
    }

    /// `jobs` — list background and stopped jobs.
    fn builtin_jobs(&mut self, _args: &[String]) -> Status {
        // Reap first so status is current.
        self.reap_children();
        for (num, job) in self.jobs.iter() {
            let state = match &job.status {
                JobStatus::Running => "Running",
                JobStatus::Stopped => "Stopped",
                JobStatus::Done(s) if s.is_success() => "Done",
                JobStatus::Done(_) => "Done (error)",
            };
            self.fd_write_line(&format!("[{num}] {state}\t{}", job.command));
        }
        Status::ok()
    }

    /// `fg %n` — bring a stopped or background job to the foreground.
    ///
    /// Gives the job's process group the terminal, sends SIGCONT,
    /// and waits for completion or stop.
    fn builtin_fg(&mut self, args: &[String]) -> Status {
        let num = if let Some(arg) = args.first() {
            let Some(n) = Self::parse_jobspec(arg) else {
                return Status::err(format!("fg: {arg}: not a valid job spec"));
            };
            n
        } else {
            // Default: most recent job
            let Some(n) = self.jobs.current_job() else {
                return Status::err("fg: no current job");
            };
            n
        };

        let Some(job) = self.jobs.get(num) else {
            return Status::err(format!("fg: %{num}: no such job"));
        };
        let pgid = job.pgid;
        let pids = job.pids.clone();
        let cmd = job.command.clone();

        eprintln!("{cmd}");

        // Send SIGCONT to resume stopped jobs.
        let _ = sys_killpg(pgid, libc::SIGCONT);

        // Update status to Running.
        if let Some(j) = self.jobs.get_mut(num) {
            j.status = JobStatus::Running;
        }

        // Wait in foreground with terminal control.
        let status = self.wait_fg_pgid(pgid, &pids);

        // If job stopped, update the table. Otherwise, remove it.
        if let Some(j) = self.jobs.get(num) {
            if matches!(j.status, JobStatus::Done(_)) {
                self.jobs.remove(num);
            }
        }

        status
    }

    /// `bg %n` — continue a stopped job in the background.
    ///
    /// Sends SIGCONT to the job's process group without giving
    /// it the terminal.
    fn builtin_bg(&mut self, args: &[String]) -> Status {
        let num = if let Some(arg) = args.first() {
            let Some(n) = Self::parse_jobspec(arg) else {
                return Status::err(format!("bg: {arg}: not a valid job spec"));
            };
            n
        } else {
            let Some(n) = self.jobs.current_job() else {
                return Status::err("bg: no current job");
            };
            n
        };

        let Some(job) = self.jobs.get_mut(num) else {
            return Status::err(format!("bg: %{num}: no such job"));
        };

        if !matches!(job.status, JobStatus::Stopped) {
            return Status::err(format!("bg: %{num}: not stopped"));
        }

        let pgid = job.pgid;
        job.status = JobStatus::Running;
        let cmd = job.command.clone();
        eprintln!("[{num}] {cmd} &");

        let _ = sys_killpg(pgid, libc::SIGCONT);
        Status::ok()
    }

    // ── External commands ───────────────────────────────────

    fn exec_external(&mut self, name: &str, args: &[String]) -> Status {
        // Tilde expansion for command paths (~/bin/foo → /home/user/bin/foo).
        // Builtin/function dispatch already matched the raw name, so this
        // only affects the execvp path.
        let expanded;
        let cmd_name = if name == "~" {
            expanded = self.env.get_value("home").as_str().to_string();
            &expanded
        } else if let Some(rest) = name.strip_prefix("~/") {
            expanded = format!("{}/{rest}", self.env.get_value("home").as_str());
            &expanded
        } else {
            name
        };

        match sys_fork() {
            Err(e) => Status::err(e),
            Ok(0) => {
                let mut full_args = vec![cmd_name.to_string()];
                full_args.extend(args.iter().cloned());
                let err = exec_command(cmd_name, &full_args, &self.env.to_process_env());
                eprintln!("psh: {name}: {err}");
                unsafe { libc::_exit(127) };
            }
            Ok(pid) => self.wait_pid(pid),
        }
    }

    fn wait_pid(&mut self, pid: libc::pid_t) -> Status {
        let mut wstatus: i32 = 0;
        loop {
            let r = unsafe { libc::waitpid(pid, &mut wstatus, libc::WUNTRACED) };
            if r == -1 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::EINTR) {
                    // Interrupted by signal — reap background children, then retry
                    self.reap_children();
                    continue;
                }
                return Status::err(format!("waitpid: {err}"));
            }
            break;
        }
        if libc::WIFEXITED(wstatus) {
            Status::from_code(libc::WEXITSTATUS(wstatus))
        } else if libc::WIFSIGNALED(wstatus) {
            Status::err(format!("signal {}", libc::WTERMSIG(wstatus)))
        } else if libc::WIFSTOPPED(wstatus) {
            // Foreground job was stopped (Ctrl-Z). Add to job table.
            let job = Job {
                pgid: pid,
                pids: vec![pid],
                command: String::from("(stopped)"),
                status: JobStatus::Stopped,
            };
            let num = self.jobs.insert(job);

            // Reclaim the terminal for the shell.
            if self.interactive && self.shell_pgid > 0 {
                let _ = sys_tcsetpgrp(libc::STDIN_FILENO, self.shell_pgid);
            }
            eprintln!("\n[{num}] Stopped");
            Status::err(format!("signal {}", libc::WSTOPSIG(wstatus)))
        } else {
            Status::err("unknown exit")
        }
    }

    /// Wait for a foreground job with terminal control.
    ///
    /// Gives the terminal to the job's process group, waits for
    /// completion or stop, then reclaims the terminal.
    fn wait_fg_pgid(&mut self, pgid: libc::pid_t, pids: &[libc::pid_t]) -> Status {
        // Give the terminal to the foreground job's process group.
        if self.interactive && pgid > 0 {
            let _ = sys_tcsetpgrp(libc::STDIN_FILENO, pgid);
        }

        let mut last_status = Status::ok();
        for &pid in pids {
            loop {
                let mut wstatus: i32 = 0;
                let r = unsafe { libc::waitpid(pid, &mut wstatus, libc::WUNTRACED) };
                if r == -1 {
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() == Some(libc::EINTR) {
                        self.reap_children();
                        continue;
                    }
                    last_status = Status::err(format!("waitpid: {err}"));
                    break;
                }
                if libc::WIFEXITED(wstatus) {
                    last_status = Status::from_code(libc::WEXITSTATUS(wstatus));
                } else if libc::WIFSIGNALED(wstatus) {
                    last_status = Status::err(format!("signal {}", libc::WTERMSIG(wstatus)));
                } else if libc::WIFSTOPPED(wstatus) {
                    // Job stopped — it enters the job table as stopped.
                    // The job table update is handled by the caller.
                    last_status = Status::err(format!("signal {}", libc::WSTOPSIG(wstatus)));
                }
                break;
            }
        }

        // Reclaim the terminal.
        if self.interactive && self.shell_pgid > 0 {
            let _ = sys_tcsetpgrp(libc::STDIN_FILENO, self.shell_pgid);
        }

        last_status
    }

    fn match_pattern(&self, val: &Val, val_str: &str, pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Literal(s) => val_str == s,
            Pattern::Star => true,
            Pattern::Glob(pat) => match fnmatch_regex::glob_to_regex(pat) {
                Ok(re) => re.is_match(val_str),
                Err(_) => val_str == pat,
            },
            // Structural: tag must match Sum's tag. Binding is set
            // in the arm body by the caller (not yet implemented —
            // requires scope push with binding variable). For now,
            // match on tag only.
            Pattern::Structural { tag, binding: _ } => matches!(val, Val::Sum(t, _) if t == tag),
        }
    }
}

/// Execute a command via execvp. Only returns on error.
fn exec_command(name: &str, args: &[String], env: &[(String, String)]) -> String {
    let c_name = match CString::new(name) {
        Ok(s) => s,
        Err(e) => return format!("{e}"),
    };
    let c_args: Vec<CString> = args
        .iter()
        .filter_map(|s| CString::new(s.as_str()).ok())
        .collect();
    let c_arg_ptrs: Vec<*const libc::c_char> = c_args
        .iter()
        .map(|s| s.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect();

    for (key, val) in env {
        std::env::set_var(key, val);
    }

    unsafe {
        libc::execvp(c_name.as_ptr(), c_arg_ptrs.as_ptr());
    }
    std::io::Error::last_os_error().to_string()
}

/// Check if a name is a builtin command.
fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "cd" | "echo"
            | "exit"
            | "get"
            | "set"
            | "wait"
            | "jobs"
            | "fg"
            | "bg"
            | "read"
            | "print"
            | "true"
            | "false"
            | "~"
            | "."
            | "whatis"
            | "builtin"
    )
}

/// Check if a string contains glob metacharacters.
fn has_glob_meta(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

/// Expand a glob pattern against the filesystem.
///
/// Handles patterns like `*.rs`, `src/*.rs`, `src/*/mod.rs`.
/// Splits on `/` to handle directory components. Returns sorted
/// results (rc convention: glob results are alphabetical).
fn glob_expand(pattern: &str) -> Vec<String> {
    // Split into directory prefix and glob component.
    // For simple cases like *.rs, dir is "." and glob is "*.rs".
    // For src/*.rs, dir is "src" and glob is "*.rs".
    // For deeply nested globs, we handle one level at a time.
    let (dir, file_pat) = match pattern.rsplit_once('/') {
        Some((d, f)) => {
            if has_glob_meta(d) {
                // Directory part has globs — expand dir first,
                // then expand file pattern in each matched dir.
                let dirs = glob_expand(d);
                let mut results = Vec::new();
                for matched_dir in dirs {
                    let sub_pattern = format!("{matched_dir}/{f}");
                    results.extend(glob_expand(&sub_pattern));
                }
                results.sort();
                return results;
            }
            (d.to_string(), f)
        }
        None => (".".to_string(), pattern),
    };

    let re = match fnmatch_regex::glob_to_regex(file_pat) {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };

    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut results: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            // Don't match dotfiles unless the pattern starts with .
            if name.starts_with('.') && !file_pat.starts_with('.') {
                return None;
            }
            if re.is_match(&name) {
                if dir == "." {
                    Some(name)
                } else {
                    Some(format!("{dir}/{name}"))
                }
            } else {
                None
            }
        })
        .collect();

    results.sort();
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Parser;

    fn run(input: &str) -> Status {
        let prog = Parser::parse(input).expect("parse failed");
        let mut shell = Shell::new();
        shell.run(&prog)
    }

    #[test]
    fn true_false() {
        assert!(run("true").is_success());
        assert!(!run("false").is_success());
    }

    #[test]
    fn assignment_and_get() {
        let prog = Parser::parse("x = hello\nget x").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
        assert_eq!(shell.env.get_value("x"), Val::scalar("hello"));
    }

    #[test]
    fn list_assignment() {
        let prog = Parser::parse("x = (a b c)").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::list(["a", "b", "c"]));
    }

    #[test]
    fn if_true() {
        let prog = Parser::parse("if true { x = yes }").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("yes"));
    }

    #[test]
    fn if_false_else() {
        let prog = Parser::parse("if false { x = yes } else { x = no }").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("no"));
    }

    #[test]
    fn for_loop() {
        let prog =
            Parser::parse("result = ()\nfor x in (a b c) { result = ($result $x) }").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("result"), Val::list(["a", "b", "c"]));
    }

    #[test]
    fn function_definition_and_call() {
        let prog = Parser::parse("fn greet { echo hello }\ngreet").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
    }

    #[test]
    fn and_or() {
        assert!(run("true && true").is_success());
        assert!(!run("true && false").is_success());
        assert!(run("false || true").is_success());
        assert!(!run("false || false").is_success());
    }

    #[test]
    fn not() {
        assert!(!run("! true").is_success());
        assert!(run("! false").is_success());
    }

    #[test]
    fn echo_runs() {
        assert!(run("echo hello world").is_success());
    }

    #[test]
    fn external_command() {
        assert!(run("true").is_success());
        assert!(!run("false").is_success());
    }

    #[test]
    fn pipeline_status() {
        assert!(run("echo hello | cat").is_success());
    }

    #[test]
    fn discipline_set() {
        let prog = Parser::parse("fn x.set { validated = yes }\nx = hello").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("validated"), Val::scalar("yes"));
        assert_eq!(shell.env.get_value("x"), Val::scalar("hello"));
    }

    #[test]
    fn command_substitution() {
        let prog = Parser::parse("x = `{ echo hello }").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("hello"));
    }

    #[test]
    fn match_stmt() {
        let prog = Parser::parse(
            "x = foo\nmatch $x { case foo { result = matched } case * { result = default } }",
        )
        .unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("result"), Val::scalar("matched"));
    }

    #[test]
    fn discipline_set_reentrancy_guard() {
        let prog = Parser::parse("fn x.set { x = $1 }\nx = hello").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
        assert_eq!(shell.env.get_value("x"), Val::scalar("hello"));
    }

    #[test]
    fn discipline_get_reentrancy_guard() {
        // The .get discipline runs in a readonly scope. It fires but
        // cannot mutate global state. The inner $x read does not
        // recurse because of the reentrancy guard.
        let prog = Parser::parse("x = original\nfn x.get { echo got }\necho $x").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
        assert_eq!(shell.env.get_value("x"), Val::scalar("original"));
    }

    #[test]
    fn discipline_guard_clears_after_execution() {
        let prog = Parser::parse("fn x.set { count = fired }\nx = a\nx = b").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("count"), Val::scalar("fired"));
    }

    #[test]
    fn builtin_bypasses_function() {
        let prog = Parser::parse("fn echo { x = shadowed }\nbuiltin echo hello").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
        assert_eq!(shell.env.get_value("x"), Val::empty());
    }

    #[test]
    fn builtin_cd_via_function() {
        let prog = Parser::parse(
            "before = no\nafter = no\nfn cd { before = yes\nbuiltin cd /tmp\nafter = yes }\ncd",
        )
        .unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("before"), Val::scalar("yes"));
        assert_eq!(shell.env.get_value("after"), Val::scalar("yes"));
    }

    #[test]
    fn builtin_no_args_is_error() {
        let status = run("builtin");
        assert!(!status.is_success());
    }

    // ── New tests ───────────────────────────────────────────

    #[test]
    fn get_purity_enforcement() {
        // .get discipline body runs in a readonly scope.
        // Mutations are rejected — side_effect stays unchanged.
        let prog = Parser::parse(
            "side_effect = before\nx = value\nfn x.get { side_effect = mutated }\necho $x",
        )
        .unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("side_effect"), Val::scalar("before"));
    }

    #[test]
    fn while_loop_false_condition() {
        // false condition — body never executes
        let prog = Parser::parse("x = untouched\nwhile false { x = touched }").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("untouched"));
    }

    #[test]
    fn builtin_get_fires_discipline() {
        // `get x` routes through eval_word, firing .get discipline.
        // Since .get runs in readonly scope, it can't mutate, but
        // the stored value is still returned correctly.
        let prog = Parser::parse("x = original\nget x").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
        assert_eq!(shell.env.get_value("x"), Val::scalar("original"));
    }

    #[test]
    fn command_sub_with_builtin_echo() {
        // Builtins write to fd 1 directly — command substitution
        // captures their output correctly.
        let prog = Parser::parse("x = `{ echo captured }").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("captured"));
    }

    #[test]
    fn readonly_var_assignment_returns_error() {
        let mut shell = Shell::new();
        let _ = shell.env.set_value("ro", Val::scalar("frozen"));
        if let Some(var) = shell.env.get_mut_var("ro") {
            var.readonly = true;
        }
        let prog = Parser::parse("ro = changed").unwrap();
        let status = shell.run(&prog);
        assert!(!status.is_success());
        assert_eq!(shell.env.get_value("ro"), Val::scalar("frozen"));
    }

    #[test]
    fn return_propagates_value() {
        let mut shell = Shell::new();
        let prog = Program {
            commands: vec![Command::Return(Some(Value::Word(Word::Literal(
                "error".into(),
            ))))],
        };
        let status = shell.run(&prog);
        assert!(!status.is_success());
        assert_eq!(status.0, "error");
    }

    // ── Job control tests ──────────────────────────────────

    #[test]
    fn job_table_tracks_background() {
        // Verify the Background handler creates a job entry.
        let mut shell = Shell::new();
        let prog = Parser::parse("sleep 0 &").unwrap();
        shell.run(&prog);

        // Give the child a moment to exit, then reap.
        std::thread::sleep(std::time::Duration::from_millis(100));
        shell.reap_children();

        // The job was created (slot 1 exists or was collected).
        // Either it's still in the table as Done, or was reaped.
        // In either case, the table was used.
        let done = shell.jobs.collect_done();
        // Job 1 should have completed.
        assert!(
            done.iter().any(|(n, _)| *n == 1) || shell.jobs.get(1).is_none(),
            "background job should have been tracked"
        );
    }

    #[test]
    fn wait_builtin_waits_for_job() {
        // Run a fast background job, then `wait %1`.
        let mut shell = Shell::new();
        let prog = Parser::parse("sleep 0 &").unwrap();
        shell.run(&prog);
        assert!(shell.jobs.get(1).is_some(), "job 1 should exist");

        let prog = Parser::parse("wait %1").unwrap();
        let status = shell.run(&prog);
        // The job completed successfully.
        assert!(status.is_success());
    }

    #[test]
    fn jobs_builtin_lists() {
        let mut shell = Shell::new();
        // Insert a synthetic job to test `jobs` output.
        use crate::job::{Job, JobStatus};
        shell.jobs.insert(Job {
            pgid: 99999,
            pids: vec![99999],
            command: "test-job".into(),
            status: JobStatus::Running,
        });

        let prog = Parser::parse("jobs").unwrap();
        let status = shell.run(&prog);
        assert!(status.is_success());
    }

    #[test]
    fn sigexit_fires() {
        let mut shell = Shell::new();
        let prog = Parser::parse("fn sigexit { exited = yes }").unwrap();
        shell.run(&prog);
        shell.fire_sigexit();
        assert_eq!(shell.env.get_value("exited"), Val::scalar("yes"));
    }

    #[test]
    fn signal_handler_as_function() {
        // rc heritage: fn sigint { } defines a signal handler.
        // We test that check_signals dispatches to it.
        let mut shell = Shell::new();
        let prog = Parser::parse("fn sigint { handled = yes }").unwrap();
        shell.run(&prog);

        // Simulate a pending SIGINT by sending ourselves one,
        // but that's tricky in tests. Instead, verify the function
        // is in the table and would be dispatched.
        assert!(shell.env.get_fn("sigint").is_some());
    }

    #[test]
    fn parse_jobspec_valid() {
        assert_eq!(Shell::parse_jobspec("%1"), Some(1));
        assert_eq!(Shell::parse_jobspec("%42"), Some(42));
    }

    #[test]
    fn parse_jobspec_invalid() {
        assert_eq!(Shell::parse_jobspec("1"), None);
        assert_eq!(Shell::parse_jobspec("%abc"), None);
        assert_eq!(Shell::parse_jobspec(""), None);
    }

    #[test]
    fn wait_no_job_errors() {
        let mut shell = Shell::new();
        let prog = Parser::parse("wait %1").unwrap();
        let status = shell.run(&prog);
        assert!(!status.is_success());
    }

    #[test]
    fn fg_no_job_errors() {
        let mut shell = Shell::new();
        let prog = Parser::parse("fg %1").unwrap();
        let status = shell.run(&prog);
        assert!(!status.is_success());
    }

    #[test]
    fn bg_no_job_errors() {
        let mut shell = Shell::new();
        let prog = Parser::parse("bg %1").unwrap();
        let status = shell.run(&prog);
        assert!(!status.is_success());
    }

    // ── Here-document / here-string tests ────────────────────

    #[test]
    fn heredoc_feeds_stdin() {
        let prog = Parser::parse("x = `{ cat <<EOF\nhello\nworld\nEOF\n}").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::list(["hello", "world"]));
    }

    #[test]
    fn herestring_feeds_stdin() {
        let prog = Parser::parse("x = `{ cat <<<hello }").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("hello"));
    }

    // ── Process substitution tests ───────────────────────────

    #[test]
    fn process_sub_as_argument() {
        let prog = Parser::parse("x = `{ cat <{echo hello} }").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("hello"));
    }

    // ── ~ match operator tests ───────────────────────────────

    #[test]
    fn match_literal_succeeds() {
        assert!(run("~ foo foo").is_success());
    }

    #[test]
    fn match_literal_fails() {
        assert!(!run("~ foo bar").is_success());
    }

    #[test]
    fn match_glob_succeeds() {
        assert!(run("~ foo f*").is_success());
    }

    #[test]
    fn match_glob_fails() {
        assert!(!run("~ foo b*").is_success());
    }

    #[test]
    fn match_multiple_patterns() {
        assert!(run("~ foo f* b*").is_success());
        assert!(run("~ bar f* b*").is_success());
        assert!(!run("~ baz f* q*").is_success());
    }

    #[test]
    fn match_no_args_errors() {
        assert!(!run("~ foo").is_success());
    }

    // ── Stringify ($") tests ──────────────────────────────────

    #[test]
    fn stringify_joins_list() {
        let prog = Parser::parse("x = (a b c)\nresult = $\"x").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        let val = shell.env.get_value("result");
        assert_eq!(val, Val::scalar("a b c"));
    }

    #[test]
    fn stringify_scalar_identity() {
        let prog = Parser::parse("x = hello\nresult = $\"x").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("result"), Val::scalar("hello"));
    }

    #[test]
    fn stringify_empty_is_empty_string() {
        let prog = Parser::parse("x = ()\nresult = $\"x").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("result"), Val::scalar(""));
    }

    // ── whatis builtin tests ─────────────────────────────────

    #[test]
    fn whatis_builtin() {
        let prog = Parser::parse("whatis echo").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
    }

    #[test]
    fn whatis_function() {
        let prog = Parser::parse("fn greet { echo hi }\nwhatis greet").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
    }

    #[test]
    fn whatis_variable() {
        let prog = Parser::parse("x = hello\nwhatis x").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
    }

    #[test]
    fn whatis_not_found() {
        let prog = Parser::parse("whatis nonexistent_name_xyz").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(!status.is_success());
    }

    #[test]
    fn whatis_no_args() {
        assert!(!run("whatis").is_success());
    }

    // ── Nameref tests ────────────────────────────────────────

    #[test]
    fn nameref_read() {
        let prog = Parser::parse("x = hello\nref y = x\nresult = $y").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("result"), Val::scalar("hello"));
    }

    #[test]
    fn nameref_write() {
        let prog = Parser::parse("x = hello\nref y = x\ny = world").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("world"));
    }

    #[test]
    fn nameref_chain() {
        let prog = Parser::parse("x = val\nref y = x\nref z = y\nresult = $z").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("result"), Val::scalar("val"));
    }

    #[test]
    fn nameref_write_through_chain() {
        let prog = Parser::parse("x = old\nref y = x\nref z = y\nz = new").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::scalar("new"));
    }

    #[test]
    fn nameref_path_stores_string() {
        let prog = Parser::parse("ref cursor = /pane/editor/attrs/cursor").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        // The nameref target is the path string itself
        assert_eq!(
            shell.env.get_nameref_target("cursor"),
            Some("/pane/editor/attrs/cursor")
        );
    }

    // ── Let binding tests ──────────────────────────────────────

    #[test]
    fn let_basic_inference() {
        let prog = Parser::parse("let x = 42").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::Int(42));
    }

    #[test]
    fn let_str_inference() {
        let prog = Parser::parse("let x = hello").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::Str("hello".into()));
    }

    #[test]
    fn let_bool_inference() {
        let prog = Parser::parse("let x = true").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::Bool(true));
    }

    #[test]
    fn let_path_inference() {
        let prog = Parser::parse("let x = /tmp").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(
            shell.env.get_value("x"),
            Val::Path(std::path::PathBuf::from("/tmp"))
        );
    }

    #[test]
    fn let_immutable_rejects_reassign() {
        let prog = Parser::parse("let x = 42\nx = 99").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(!status.is_success());
        // x should still be 42
        assert_eq!(shell.env.get_value("x"), Val::Int(42));
    }

    #[test]
    fn let_mut_allows_reassign() {
        let prog = Parser::parse("let mut x = 42\nx = 99").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
    }

    #[test]
    fn let_typed_accepts_matching() {
        let prog = Parser::parse("let x : Int = 42").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
        assert_eq!(shell.env.get_value("x"), Val::Int(42));
    }

    #[test]
    fn let_typed_rejects_mismatch() {
        let prog = Parser::parse("let x : Int = hello").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(!status.is_success());
    }

    #[test]
    fn let_list_typed() {
        let prog = Parser::parse("let x : List[Int] = (1 2 3)").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
        assert_eq!(
            shell.env.get_value("x"),
            Val::List(vec![Val::Int(1), Val::Int(2), Val::Int(3)])
        );
    }

    #[test]
    fn let_list_typed_rejects_mixed() {
        let prog = Parser::parse("let x : List[Int] = (1 hello)").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(!status.is_success());
    }

    #[test]
    fn let_bracket_sugar() {
        // [Int] is sugar for List[Int]
        let prog = Parser::parse("let x : [Int] = (1 2)").unwrap();
        let mut shell = Shell::new();
        let status = shell.run(&prog);
        assert!(status.is_success());
        assert_eq!(
            shell.env.get_value("x"),
            Val::List(vec![Val::Int(1), Val::Int(2)])
        );
    }

    #[test]
    fn let_quoted_stays_str() {
        // '42' stays Str even in let context
        let prog = Parser::parse("let x = '42'").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::Str("42".into()));
    }

    #[test]
    fn let_export() {
        let prog = Parser::parse("let export x = hello").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        let var = shell.env.get("x").unwrap();
        assert!(var.exported);
    }

    #[test]
    fn let_mut_export() {
        let prog = Parser::parse("let mut export x = hello").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        let var = shell.env.get("x").unwrap();
        assert!(var.exported);
        assert!(var.mutable);
    }

    #[test]
    fn bare_assignment_stays_str() {
        // rc heritage: bare x = 42 stays Str (no inference)
        let prog = Parser::parse("x = 42").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::Str("42".into()));
    }

    #[test]
    fn count_returns_int() {
        let prog = Parser::parse("x = (a b c)\nlet n = $#x").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("n"), Val::Int(3));
    }

    #[test]
    fn let_leading_zero_stays_str() {
        let prog = Parser::parse("let x = 042").unwrap();
        let mut shell = Shell::new();
        shell.run(&prog);
        assert_eq!(shell.env.get_value("x"), Val::Str("042".into()));
    }
}
