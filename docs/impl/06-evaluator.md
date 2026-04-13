# Evaluator — three-function core

The evaluator is three mutually recursive functions, one per
AST sort. This is the operational realization of the λμμ̃ cut
structure: `eval_term` handles producers (Γ), `run_cmd`
handles consumers (Δ), `run_expr` handles cuts (⟨t | e⟩).

Spec correspondence: 02-calculus.md §The three sorts defines
the categorical structure. 03-polarity.md §Polarity frames
defines the save/restore mechanism for shift boundaries.
09-redirections.md defines the profunctor composition model
for redirections.

### The Shell struct

All mutable state lives on one struct. No globals.

```rust
struct Shell {
    sigma: Sigma,                   // frozen after startup
    env: Environment,               // Γ + Θ (Layer 7)
    fds: FdTable,                   // open fd table
    jobs: JobTable,                 // background job tracking
    traps: TrapTable,               // signal → handler mapping
    options: OptionSet,             // set -o state
    status: Status,                 // $status — last command's result
    loop_depth: u32,                // for break/continue validation
    try_depth: u32,                 // for catch dispatch
}
```

### eval_term: producers → Val

CBV evaluation. Every term is evaluated eagerly to a `Val`
before the command that consumes it runs. This is the Kleisli
composition through the positive subcategory.

```rust
impl Shell {
    fn eval_term(&mut self, term: &Term) -> Result<Val, ShellError> {
        match term {
            Term::Literal(_, val) => Ok(val.clone()),
            Term::Var(_, ident) => self.env.get(ident.name),
            Term::Index(_, base, idx) => {
                let b = self.eval_term(base)?;
                let i = self.eval_term(idx)?;
                self.index_val(&b, &i)
            }
            Term::Access(_, base, name) => {
                let b = self.eval_term(base)?;
                self.access_val(&b, name.name)
            }
            Term::CmdSub(_, expr) => {
                self.polarity_frame(|sh| {
                    // fork, capture stdout, return as list
                    sh.capture_output(expr)
                })
            }
            Term::List(_, elements) => {
                let vals: Vec<Val> = elements.iter()
                    .map(|e| self.eval_term(e))
                    .collect::<Result<_, _>>()?;
                Ok(Val::List(vals))
            }
            Term::Tagged(_, _, variant, payload) => {
                let p = payload.as_ref()
                    .map(|t| self.eval_term(t))
                    .transpose()?;
                let type_id = term.ann().ty.as_ref()
                    .expect("checker filled in type")
                    .type_id();
                Ok(Val::Tagged(type_id, variant.name, p.map(Box::new)))
            }
            Term::Concat(_, parts) => {
                // stringification + concatenation
                let strs: Vec<SmolStr> = parts.iter()
                    .map(|p| Ok(self.eval_term(p)?.to_str()))
                    .collect::<Result<_, ShellError>>()?;
                Ok(Val::Str(strs.concat().into()))
            }
            Term::Lambda(_, params, body) => {
                // capture current scope, store as closure in Θ
                self.make_closure(params, body)
            }
            Term::Arith(_, expr) => {
                // pure in-process computation
                // polarity frame is trivially no-op
                let n = self.eval_arith(expr)?;
                Ok(Val::Int(n))
            }
            Term::Coalesce(_, lhs, rhs) => {
                // M ?? N — sugar for match on Option
                match self.eval_term(lhs)? {
                    Val::Tagged(_, name, Some(inner))
                        if name == self.sigma.intern("some") => Ok(*inner),
                    _ => self.eval_term(rhs),
                }
            }
            // ... remaining arms
        }
    }
}
```

### run_cmd: consumers → dispatch

Commands extend the environment (bindings), control flow
(if/for/while/match), or produce Status (try/catch, trap).

```rust
impl Shell {
    fn run_cmd(&mut self, cmd: &Command) -> Status {
        match cmd {
            Command::Assign(_, ident, term) => {
                let val = self.eval_term(term)?;
                self.env.set(ident.name, val);
                Status::ok()
            }
            Command::Let(_, flags, pat, _, term) => {
                let val = self.eval_term(term)?;
                self.env.bind_pattern(pat, val, flags);
                Status::ok()
            }
            Command::Def(_, target, params, _, body) => {
                self.env.register_def(target, params, body);
                Status::ok()
            }
            Command::If(_, cond, then_body, else_branch) => {
                let s = self.run_expr(cond)?;
                if s.is_ok() {
                    self.run_program(then_body)
                } else if let Some(branch) = else_branch {
                    match branch {
                        ElseBranch::ElseIf(cmd) => self.run_cmd(cmd),
                        ElseBranch::Else(prog) => self.run_program(prog),
                    }
                } else {
                    Status::ok()
                }
            }
            Command::For(_, ident, list, body) => {
                let vals = self.eval_term(list)?;
                self.loop_depth += 1;
                let mut last = Status::ok();
                for v in vals.as_list() {
                    self.env.push_scope();
                    self.env.bind(ident.name, v);
                    last = self.run_program(body);
                    self.env.pop_scope();
                    if self.check_break_continue() { break; }
                }
                self.loop_depth -= 1;
                last
            }
            Command::Match(_, scrutinee, arms) => {
                let val = self.eval_term(scrutinee)?;
                for arm in arms {
                    if let Some(binds) = self.match_patterns(&arm.patterns, &val) {
                        if arm.guard_satisfied(&binds, self)? {
                            self.env.push_scope();
                            self.env.bind_all(binds);
                            let result = self.run_lambda_body(&arm.body);
                            self.env.pop_scope();
                            return result;
                        }
                    }
                }
                // exhaustiveness checked statically — unreachable
                unreachable!("non-exhaustive match passed checker")
            }
            Command::Try(_, body, err_name, handler) => {
                self.try_depth += 1;
                let result = self.run_program(body);
                self.try_depth -= 1;
                match result {
                    Status::Ok => Status::ok(),
                    Status::Err(exit_code) => {
                        self.env.push_scope();
                        self.env.bind(err_name.name, exit_code.to_val());
                        let r = self.run_program(handler);
                        self.env.pop_scope();
                        r
                    }
                }
            }
            Command::Trap(_, signal, handler, body) => {
                self.install_trap(signal, handler, body)
            }
            // ... remaining arms
        }
    }
}
```

### run_expr: cuts → Status

Expressions are where producers meet consumers: pipelines,
redirections, backgrounding. The profunctor layer.

```rust
impl Shell {
    fn run_expr(&mut self, expr: &Expr) -> Status {
        match expr {
            Expr::Cmd(_, cmd) => self.run_cmd(cmd),

            Expr::Pipe(_, stages) => {
                self.run_pipeline(stages)
            }

            Expr::And(_, lhs, rhs) => {
                let s = self.run_expr(lhs)?;
                if s.is_ok() { self.run_expr(rhs) } else { s }
            }

            Expr::Or(_, lhs, rhs) => {
                let s = self.run_expr(lhs)?;
                if s.is_ok() { s } else { self.run_expr(rhs) }
            }

            Expr::Not(_, inner) => {
                let s = self.run_expr(inner)?;
                s.negate()
            }

            Expr::Redirected(_, inner, redirects) => {
                // profunctor wrapping: save fds, apply redirects,
                // run inner, restore fds
                self.with_redirects(redirects, |sh| sh.run_expr(inner))
            }

            Expr::Background(_, inner) => {
                self.spawn_background(inner, false)
            }

            Expr::BackgroundDisown(_, inner) => {
                self.spawn_background(inner, true)
            }

            Expr::Subshell(_, program) => {
                self.fork_and_run(program)
            }

            // ... remaining arms
        }
    }
}
```

### Polarity frames

The save/restore mechanism for ↓→↑ shifts. Three call sites:
command substitution, discipline `.set`/`.refresh`, and
(trivially) arithmetic expansion.

```rust
impl Shell {
    /// Execute `body` inside a polarity frame.
    /// Saves the expansion context, runs the body, restores.
    fn polarity_frame<F, T>(&mut self, body: F) -> Result<T, ShellError>
    where F: FnOnce(&mut Shell) -> Result<T, ShellError>
    {
        // Save: fd state, expansion accumulator, positional params
        let saved_fds = self.fds.snapshot();
        let saved_expansion = self.env.save_expansion_context();

        let result = body(self);

        // Restore: guaranteed even on error/signal
        self.env.restore_expansion_context(saved_expansion);
        self.fds.restore(saved_fds);

        result
    }
}
```

The frame prevents the `sh.prefix` corruption pattern from
ksh93 — computation-mode operations inside value-mode contexts
cannot corrupt positive-mode state because the frame isolates
them.

### Pipeline execution

Pipelines are concurrent cuts. Each `|` creates a pipe pair
and forks both sides.

```rust
impl Shell {
    fn run_pipeline(&mut self, stages: &[PipeStage]) -> Status {
        if stages.len() == 1 {
            return self.run_expr(&stages[0].expr);
        }

        let mut prev_read: Option<RawFd> = None;
        let mut children: Vec<Pid> = Vec::new();

        for (i, stage) in stages.iter().enumerate() {
            let last = i == stages.len() - 1;

            // Create pipe for non-last stages
            let (read_fd, write_fd) = if !last {
                let (r, w) = rustix::pipe::pipe_with(PipeFlags::CLOEXEC)?;
                (Some(r), Some(w))
            } else {
                (None, None)
            };

            let pid = self.fork_stage(
                &stage.expr,
                prev_read,    // stdin from previous stage
                write_fd,     // stdout to next stage
            )?;
            children.push(pid);

            // Close our copies
            if let Some(w) = write_fd { rustix::io::close(w); }
            if let Some(r) = prev_read { rustix::io::close(r); }

            prev_read = read_fd;
        }

        // Wait for all children, return last stage's status
        // (or first failure if pipefail is set)
        self.wait_pipeline(&children)
    }
}
```

### Redirect composition (profunctor wrapping)

Redirections are the Lens save/restore pattern: save the
current fd mapping, apply the redirect, run the command,
restore.

```rust
impl Shell {
    fn with_redirects<F>(&mut self, redirects: &[Redirect], body: F)
        -> Status
    where F: FnOnce(&mut Shell) -> Status
    {
        // Save fd state (the Lens "get")
        let saved = self.fds.save();

        // Apply redirects left-to-right (profunctor composition)
        for redir in redirects {
            self.apply_redirect(redir)?;
        }

        // Run body with modified fds
        let result = body(self);

        // Restore fd state (the Lens "put")
        self.fds.restore(saved);

        result
    }
}
```

### Discipline dispatch

When `$x` is accessed and `x` has a `.get` discipline, the
evaluator calls the discipline body. When `x = val` is
assigned and `x` has a `.set` discipline, the evaluator wraps
the assignment in a polarity frame and calls `.set`.

```rust
impl Shell {
    fn access_val(&mut self, base: &Val, name: Name) -> Result<Val, ShellError> {
        let type_id = base.type_id();

        // Check per-type method registry first
        if let Some(method) = self.sigma.methods.get(&(type_id, name)) {
            return self.call_method(method, base);
        }

        // Check struct field access
        if let Some(entry) = self.sigma.structs.get(&type_id) {
            if let Some(idx) = entry.field_index(name) {
                return Ok(base.struct_field(idx));
            }
        }

        Err(ShellError::NoSuchAccessor(type_id, name))
    }

    fn assign_with_discipline(&mut self, name: Name, val: Val) {
        if let Some(set_def) = self.env.get_discipline(name, "set") {
            // Polarity frame around .set body
            self.polarity_frame(|sh| {
                sh.env.set_reentrancy_guard(name, true);
                let result = sh.call_def(set_def, &[val]);
                sh.env.set_reentrancy_guard(name, false);
                result
            });
        } else {
            // No discipline — direct slot write
            self.env.set(name, val);
        }
    }
}
```

The reentrancy guard prevents infinite recursion: inside a
`.set` body, `x = val` is the primitive slot write (bypasses
the cocase). This is the polarity frame discipline from
08-discipline.md.

### What the evaluator does NOT do

- **No type checking.** Types are resolved by the checker.
  The evaluator trusts `ann.ty` annotations.
- **No parsing.** The AST is fully constructed before
  evaluation begins.
- **No Σ mutation.** Σ is frozen. The evaluator reads it for
  method dispatch and constructor info.
- **No global state.** All state lives on `Shell`. The
  evaluator is reentrant (modulo the reentrancy guard on
  discipline functions, which is a per-variable flag on
  `Shell`, not a global).


