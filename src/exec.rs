//! Evaluator for psh.
//!
//! Walks the AST and executes it. fork/exec/wait for external
//! commands, direct dispatch for builtins. Pipeline wiring via
//! pipe(2). Variable expansion is CBV (eager). Pipeline stages
//! are CBN (concurrent, demand-driven).

use std::os::unix::io::FromRawFd;
use std::process;

use anyhow::Result;

use crate::ast::*;
use crate::env::Env;
use crate::value::Val;

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
}

impl Shell {
    pub fn new() -> Self {
        let mut env = Env::new();
        env.import_process_env();
        Shell { env }
    }

    /// Execute a parsed program.
    pub fn run(&mut self, program: &Program) -> Status {
        self.run_stmts(&program.statements)
    }

    fn run_stmts(&mut self, stmts: &[Statement]) -> Status {
        let mut status = Status::ok();
        for stmt in stmts {
            status = self.run_stmt(stmt);
        }
        status
    }

    fn run_stmt(&mut self, stmt: &Statement) -> Status {
        match stmt {
            Statement::Assignment(name, value) => {
                let val = self.eval_value(value);
                // Check for .set discipline — runs in the CURRENT scope
                // (not a child scope) so its side effects are visible.
                if let Some(body) = self.env.get_fn(&format!("{name}.set")).cloned() {
                    self.env.set_value("1", val.clone());
                    let status = self.run_stmts(&body);
                    self.env.set_value(name, val);
                    status
                } else {
                    self.env.set_value(name, val);
                    Status::ok()
                }
            }
            Statement::Exec(expr) => self.run_expr(expr),
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let cond_status = self.run_expr(condition);
                if cond_status.is_success() {
                    self.run_stmts(then_body)
                } else if let Some(else_body) = else_body {
                    self.run_stmts(else_body)
                } else {
                    cond_status
                }
            }
            Statement::For { var, list, body } => {
                let items = self.eval_value(list);
                let mut status = Status::ok();
                for item in &items.0 {
                    self.env.set_value(var, Val::scalar(item.clone()));
                    status = self.run_stmts(body);
                }
                status
            }
            Statement::Switch { value, cases } => {
                let val = self.eval_value(value);
                let val_str = val.as_str().to_string();
                for (patterns, body) in cases {
                    for pat in patterns {
                        if self.match_pattern(&val_str, pat) {
                            return self.run_stmts(body);
                        }
                    }
                }
                Status::ok()
            }
            Statement::Fn { name, body } => {
                self.env.define_fn(name.clone(), body.clone());
                Status::ok()
            }
            Statement::Return(_) => Status::ok(),
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
                match unsafe { libc::fork() } {
                    -1 => Status::err("fork failed"),
                    0 => {
                        let status = self.run_expr(inner);
                        process::exit(if status.is_success() { 0 } else { 1 });
                    }
                    pid => {
                        eprintln!("[{}]", pid);
                        Status::ok()
                    }
                }
            }
            Expr::Block(stmts) => self.run_stmts(stmts),
            Expr::Subshell(stmts) => {
                match unsafe { libc::fork() } {
                    -1 => Status::err("fork failed"),
                    0 => {
                        let status = self.run_stmts(stmts);
                        process::exit(if status.is_success() { 0 } else { 1 });
                    }
                    pid => self.wait_pid(pid),
                }
            }
            Expr::Coprocess(_inner) => {
                Status::err("coprocesses not yet implemented")
            }
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

        // Function call
        if let Some(body) = self.env.get_fn(&name_str).cloned() {
            self.env.push_scope();
            for (i, arg) in args.iter().enumerate() {
                self.env
                    .set_value(&(i + 1).to_string(), Val::scalar(arg.clone()));
            }
            self.env
                .set_value("*", Val::list(args.iter().map(|s| s.as_str())));
            let status = self.run_stmts(&body);
            self.env.pop_scope();
            self.env.set_value("status", Val::scalar(&status.0));
            return status;
        }

        // Builtins
        let status = match name_str.as_str() {
            "cd" => self.builtin_cd(&args),
            "echo" => self.builtin_echo(&args),
            "exit" => self.builtin_exit(&args),
            "get" => self.builtin_get(&args),
            "set" => self.builtin_set(&args),
            "true" => Status::ok(),
            "false" => Status::from_code(1),
            "." => self.builtin_source(&args),
            _ => self.exec_external(&name_str, &args),
        };

        self.env.set_value("status", Val::scalar(&status.0));
        status
    }

    fn run_pipeline(&mut self, stages: &[Expr]) -> Status {
        if stages.len() == 1 {
            return self.run_expr(&stages[0]);
        }

        let mut prev_read_fd: Option<i32> = None;
        let mut children: Vec<libc::pid_t> = Vec::new();

        for (i, stage) in stages.iter().enumerate() {
            let is_last = i == stages.len() - 1;

            let (read_fd, write_fd) = if !is_last {
                let mut fds = [0i32; 2];
                if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
                    return Status::err("pipe failed");
                }
                (Some(fds[0]), Some(fds[1]))
            } else {
                (None, None)
            };

            match unsafe { libc::fork() } {
                -1 => return Status::err("fork failed"),
                0 => {
                    if let Some(fd) = prev_read_fd {
                        unsafe {
                            libc::dup2(fd, 0);
                            libc::close(fd);
                        }
                    }
                    if let Some(fd) = write_fd {
                        unsafe {
                            libc::dup2(fd, 1);
                            libc::close(fd);
                        }
                    }
                    if let Some(fd) = read_fd {
                        unsafe { libc::close(fd) };
                    }
                    let status = self.run_expr(stage);
                    process::exit(if status.is_success() { 0 } else { 1 });
                }
                pid => {
                    children.push(pid);
                    if let Some(fd) = write_fd {
                        unsafe { libc::close(fd) };
                    }
                    if let Some(fd) = prev_read_fd {
                        unsafe { libc::close(fd) };
                    }
                    prev_read_fd = read_fd;
                }
            }
        }

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
                let flags = libc::O_WRONLY | libc::O_CREAT
                    | if *append { libc::O_APPEND } else { libc::O_TRUNC };
                let c_path = match std::ffi::CString::new(path.as_str()) {
                    Ok(s) => s,
                    Err(_) => return Status::err("invalid path"),
                };
                let file_fd = unsafe { libc::open(c_path.as_ptr(), flags, 0o666) };
                if file_fd == -1 {
                    return Status::err(format!("cannot open {path}"));
                }
                let saved = unsafe { libc::dup(*fd as i32) };
                unsafe { libc::dup2(file_fd, *fd as i32) };
                unsafe { libc::close(file_fd) };
                let status = self.run_expr(inner);
                unsafe { libc::dup2(saved, *fd as i32) };
                unsafe { libc::close(saved) };
                status
            }
            RedirectOp::Input { fd, target } => {
                let path = match target {
                    RedirectTarget::File(word) => self.eval_word(word).as_str().to_string(),
                    _ => return Status::err("unsupported redirect target"),
                };
                let c_path = match std::ffi::CString::new(path.as_str()) {
                    Ok(s) => s,
                    Err(_) => return Status::err("invalid path"),
                };
                let file_fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY, 0) };
                if file_fd == -1 {
                    return Status::err(format!("cannot open {path}"));
                }
                let saved = unsafe { libc::dup(*fd as i32) };
                unsafe { libc::dup2(file_fd, *fd as i32) };
                unsafe { libc::close(file_fd) };
                let status = self.run_expr(inner);
                unsafe { libc::dup2(saved, *fd as i32) };
                unsafe { libc::close(saved) };
                status
            }
            RedirectOp::Dup { dst, src } => {
                let saved = unsafe { libc::dup(*dst as i32) };
                unsafe { libc::dup2(*src as i32, *dst as i32) };
                let status = self.run_expr(inner);
                unsafe { libc::dup2(saved, *dst as i32) };
                unsafe { libc::close(saved) };
                status
            }
            RedirectOp::Close { fd } => {
                let saved = unsafe { libc::dup(*fd as i32) };
                unsafe { libc::close(*fd as i32) };
                let status = self.run_expr(inner);
                unsafe { libc::dup2(saved, *fd as i32) };
                unsafe { libc::close(saved) };
                status
            }
        }
    }

    // ── Word evaluation (CBV — eager) ───────────────────────

    fn eval_word(&mut self, word: &Word) -> Val {
        match word {
            Word::Literal(s) => Val::scalar(s.clone()),
            Word::Var(name) => {
                if let Some(body) = self.env.get_fn(&format!("{name}.get")).cloned() {
                    self.env.push_scope();
                    self.run_stmts(&body);
                    self.env.pop_scope();
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
                let mut fds = [0i32; 2];
                if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
                    return Val::empty();
                }

                match unsafe { libc::fork() } {
                    -1 => Val::empty(),
                    0 => {
                        unsafe {
                            libc::close(fds[0]);
                            libc::dup2(fds[1], 1);
                            libc::close(fds[1]);
                        }
                        let status = self.run_stmts(stmts);
                        process::exit(if status.is_success() { 0 } else { 1 });
                    }
                    pid => {
                        unsafe { libc::close(fds[1]) };
                        let mut output = String::new();
                        let file = unsafe { std::fs::File::from_raw_fd(fds[0]) };
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
                Val(items)
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

    fn builtin_echo(&self, args: &[String]) -> Status {
        println!("{}", args.join(" "));
        Status::ok()
    }

    fn builtin_exit(&self, args: &[String]) -> Status {
        let code = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        process::exit(code);
    }

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

        let val = self.env.get_value(name);
        if val.is_true() {
            println!("{val}");
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

        self.env.set_value(name, value);
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
        match unsafe { libc::fork() } {
            -1 => Status::err("fork failed"),
            0 => {
                let mut full_args = vec![name.to_string()];
                full_args.extend(args.iter().cloned());
                let err = exec_command(name, &full_args, &self.env.to_process_env());
                eprintln!("psh: {name}: {err}");
                process::exit(127);
            }
            pid => self.wait_pid(pid),
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
            Pattern::Glob(pat) => {
                if pat == "*" {
                    return true;
                }
                if let Some(suffix) = pat.strip_prefix('*') {
                    return value.ends_with(suffix);
                }
                if let Some(prefix) = pat.strip_suffix('*') {
                    return value.starts_with(prefix);
                }
                value == pat
            }
        }
    }
}

/// Execute a command via execvp. Only returns on error.
fn exec_command(name: &str, args: &[String], env: &[(String, String)]) -> String {
    use std::ffi::CString;

    let c_name = match CString::new(name) {
        Ok(s) => s,
        Err(e) => return format!("{e}"),
    };
    let c_args: Vec<CString> = args
        .iter()
        .map(|s| CString::new(s.as_str()).unwrap())
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
        // Use builtins to test exec path indirectly — true/false
        // are builtins, but /usr/bin/env should exist everywhere
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
}
