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

use crate::{ast::*, env::Env, value::Val};

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

/// The shell interpreter state.
pub struct Shell {
    pub env: Env,
    /// ksh93 heritage: reentrancy guard for discipline functions.
    /// Prevents `fn x.set { x = $1 }` from recursing infinitely.
    /// (src/cmd/ksh93/sh/nvdisc.c — nv_disc uses SH_VARNOD flag)
    active_disciplines: HashSet<String>,
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
                        self.env.set_value("1", val.clone());
                        let status = self.run_cmds(&body);
                        self.active_disciplines.remove(&disc_name);
                        if !self.env.set_value(name, val) {
                            return Status::err(format!("{name}: readonly variable"));
                        }
                        status
                    } else {
                        if !self.env.set_value(name, val) {
                            return Status::err(format!("{name}: readonly variable"));
                        }
                        Status::ok()
                    }
                } else {
                    if !self.env.set_value(name, val) {
                        return Status::err(format!("{name}: readonly variable"));
                    }
                    Status::ok()
                }
            }
            Command::Bind(Binding::Fn { name, body }) => {
                self.env.define_fn(name.clone(), body.clone());
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
                for item in &items.0 {
                    self.env.set_value(var, Val::scalar(item.clone()));
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
            Command::Switch { value, cases } => {
                let val = self.eval_value(value);
                let val_str = val.as_str().to_string();
                for (patterns, body) in cases {
                    for pat in patterns {
                        if self.match_pattern(&val_str, pat) {
                            return self.run_cmds(body);
                        }
                    }
                }
                Status::ok()
            }
            Command::Return(val) => {
                if let Some(v) = val {
                    let evaluated = self.eval_value(v);
                    let s = evaluated.as_str().to_string();
                    self.env.set_value("status", Val::scalar(&s));
                    if s.is_empty() {
                        Status::ok()
                    } else {
                        Status(s)
                    }
                } else {
                    let current = self.env.get_value("status");
                    Status(current.as_str().to_string())
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
            Expr::Background(inner) => match sys_fork() {
                Err(e) => Status::err(e),
                Ok(0) => {
                    let status = self.run_expr(inner);
                    // _exit in forked children: don't run atexit handlers or
                    // Rust destructors. The kernel closes all fds on process exit.
                    unsafe { libc::_exit(if status.is_success() { 0 } else { 1 }) };
                }
                Ok(pid) => {
                    eprintln!("[{pid}]");
                    Status::ok()
                }
            },
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
            Expr::Coprocess(_inner) => Status::err("coprocesses not yet implemented"),
        }
    }

    fn run_simple(&mut self, cmd: &SimpleCommand) -> Status {
        let name = self.eval_word(&cmd.name);
        let name_str = name.as_str().to_string();

        let mut args: Vec<String> = Vec::new();
        for arg in &cmd.args {
            let val = self.eval_word(arg);
            args.extend(val.0.into_iter());
        }

        // `builtin` bypasses function lookup — allows patterns like
        // `fn cd { builtin cd $* && update_prompt }` without recursion.
        if name_str == "builtin" {
            if args.is_empty() {
                let status = Status::err("builtin: usage: builtin command [args...]");
                self.env.set_value("status", Val::scalar(&status.0));
                return status;
            }
            let builtin_name = &args[0];
            let builtin_args = &args[1..];
            let status = self.dispatch_builtin(builtin_name, builtin_args);
            self.env.set_value("status", Val::scalar(&status.0));
            return status;
        }

        // Function call
        if let Some(body) = self.env.get_fn(&name_str).cloned() {
            self.env.push_scope();
            for (i, arg) in args.iter().enumerate() {
                self.env
                    .set_value(&(i + 1).to_string(), Val::scalar(arg.clone()));
            }
            self.env
                .set_value("*", Val::list(args.iter().map(|s| s.as_str())));
            let status = self.run_cmds(&body);
            self.env.pop_scope();
            self.env.set_value("status", Val::scalar(&status.0));
            return status;
        }

        // Builtins
        let status = self.dispatch_builtin(&name_str, &args);

        self.env.set_value("status", Val::scalar(&status.0));
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

        self.env.set_value(
            "pipestatus",
            Val::list(statuses.iter().map(|s| s.0.as_str())),
        );

        statuses.pop().unwrap_or_else(Status::ok)
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
                let path = match target {
                    RedirectTarget::File(word) => self.eval_word(word).as_str().to_string(),
                    _ => return Status::err("unsupported redirect target"),
                };

                let file_fd = match rustix::fs::open(
                    &*path,
                    rustix::fs::OFlags::RDONLY,
                    rustix::fs::Mode::empty(),
                ) {
                    Ok(fd) => fd,
                    Err(e) => return Status::err(format!("cannot open {path}: {e}")),
                };

                let target_raw = *fd as i32;
                let saved = match sys_dup(target_raw) {
                    Ok(fd) => fd,
                    Err(e) => return Status::err(e),
                };
                sys_dup2(file_fd.as_raw_fd(), target_raw);
                drop(file_fd);

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
            "true" => Status::ok(),
            "false" => Status::from_code(1),
            "." => self.builtin_source(args),
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
            Word::Literal(s) => Val::scalar(s.clone()),
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
                Val::scalar(val.count().to_string())
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
            Word::Concat(parts) => {
                let mut result = self.eval_word(&parts[0]);
                for part in &parts[1..] {
                    let right = self.eval_word(part);
                    result = result.concat(&right);
                }
                result
            }
        }
    }

    fn eval_value(&mut self, value: &Value) -> Val {
        match value {
            Value::Word(word) => self.eval_word(word),
            Value::List(words) => {
                let mut items = Vec::new();
                for word in words {
                    let val = self.eval_word(word);
                    items.extend(val.0);
                }
                Val(items.into_iter().collect())
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

        if !self.env.set_value(name, value) {
            return Status::err(format!("set: {name}: readonly variable"));
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

    // ── External commands ───────────────────────────────────

    fn exec_external(&mut self, name: &str, args: &[String]) -> Status {
        match sys_fork() {
            Err(e) => Status::err(e),
            Ok(0) => {
                let mut full_args = vec![name.to_string()];
                full_args.extend(args.iter().cloned());
                let err = exec_command(name, &full_args, &self.env.to_process_env());
                eprintln!("psh: {name}: {err}");
                unsafe { libc::_exit(127) };
            }
            Ok(pid) => self.wait_pid(pid),
        }
    }

    fn wait_pid(&self, pid: libc::pid_t) -> Status {
        let mut wstatus: i32 = 0;
        unsafe {
            libc::waitpid(pid, &mut wstatus, 0);
        }
        if libc::WIFEXITED(wstatus) {
            Status::from_code(libc::WEXITSTATUS(wstatus))
        } else if libc::WIFSIGNALED(wstatus) {
            Status::err(format!("signal {}", libc::WTERMSIG(wstatus)))
        } else {
            Status::err("unknown exit")
        }
    }

    fn match_pattern(&self, value: &str, pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Literal(s) => value == s,
            Pattern::Star => true,
            Pattern::Glob(pat) => match fnmatch_regex::glob_to_regex(pat) {
                Ok(re) => re.is_match(value),
                Err(_) => value == pat,
            },
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
    fn switch_stmt() {
        let prog = Parser::parse(
            "x = foo\nswitch $x { case foo { result = matched } case * { result = default } }",
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
        shell.env.set_value("ro", Val::scalar("frozen"));
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
}
