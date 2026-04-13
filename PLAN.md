# Plan

Current roadmap. Living document — update when tasks complete,
priorities change, or new work is identified.

## Current state

**Design: comprehensive.** The specification lives in
`docs/spec/` (16 chapters + bibliography). Grammar in
`docs/spec/04-syntax.md`. All typing rules use classical
sequent notation. Dialogue duploid commitment, linear resources,
typed pipes, coprocesses, and 25 builtins are specified. The
spec has been through multiple multi-agent review passes.

**Implementation: stub.** The source tree:

- `src/main.rs` — binary stub, exits with 2
- `src/parse.rs` — combine boilerplate (character predicates,
  trivia, keyword/name primitives)
- `src/signal.rs` — self-pipe signal handling (preserved,
  type-system neutral)

Dependencies per `docs/implementation.md`: `anyhow`, `bpaf`,
`combine`, `fnmatch-regex`, `libc`, `rustix`, `smallvec`,
`signal_receipts`.

## Intermediate State Principle

Each implementation phase must be a shell someone would use.
The litmus test: "If we stopped here permanently, would this
be a reasonable design?" If not, merge the phases. A smaller
diff is not inherently safer; an incoherent intermediate is
strictly harder to reason about.

See `docs/agent-workflow.md` §Intermediate state principle and
`STYLEGUIDE.md` for the full statement.

## Implementation roadmap

Five phases. Each produces a coherent, usable shell. Each
phase's deliverable description says what the shell IS at that
point, not just what changed.

### Phase 1: rc minus types

**What it is:** a working shell for simple commands. Someone
would use this as a login shell for basic scripting. Equivalent
to a minimal rc — everything is strings, no type annotations,
no checker.

**What EXISTS after this phase:**

- Parser for: simple commands, pipes (`|`, `|[n]`, `|[n=m]`),
  redirections (`>`, `>>`, `<`, `<>`, `>[n=m]`, `<[n=m]`),
  single-quoted and double-quoted strings (with `$var`
  interpolation in double quotes), backslash escapes, comments,
  line continuation, lists `(a b c)`, here documents (`<<`,
  `<<'`, `<<-`), here-strings `<<<`
- Variables: assignment (`x = val`), reference (`$x`), list
  operations (`$#x`, `$x[i]`), `$*`, `$status`, `$pid`
- Control flow: `if`/`else`, `while`, `for x in list`,
  `switch`/`case` (simple pattern matching on strings)
- Three-sort AST: Term, Command, Expr — but only the basic
  nodes. No enum/struct/match/lambda nodes yet.
- Evaluator: `eval_term` (word expansion — tilde, parameter,
  command sub, glob), `run_cmd` (dispatch), `run_expr`
  (pipeline, redirect, fork/exec)
- Pipeline forking with proper fd plumbing
- Fd save/restore for redirections
- `$PATH` search, command execution via `execvp`
- Subshell `@{ }`
- Background `&` and `$apid`
- `&&` / `||` short-circuit
- Essential builtins: `.` (source), `cd`, `echo`, `read`,
  `true`, `false`, `exec`, `exit`, `set` (basic option
  handling: `-x`, `-v`, `-C`)
- Startup file sourcing (profile, rc, per §14-invocation.md)
- Signal handling (SIGINT, SIGCHLD, SIGHUP) via self-pipe
- CLOEXEC by default on all shell-created fds
- No global mutable state — `Shell` struct owns everything

**What it is NOT:** No type checker. No type annotations. No
enums, structs, tuples, maps. No `match`. No `try`/`catch`.
No discipline functions. No coprocesses. No typed pipes. No
job control (background yes, fg/bg no). No line editing. Values
are strings and lists of strings, period.

**Why this is coherent:** It is rc on Unix. A shell that parses
commands, expands words, forks processes, plumbs fds, and runs
scripts. Every Unix shell user would recognize it. The three-sort
AST is there from the start because it structures the evaluator —
it is not dead weight.

### Phase 2: the type system

**What it is:** Phase 1 plus psh's type theory. A typed rc.
The checker catches errors the Phase 1 shell would have silently
accepted. New value types enable structured programming.

**What EXISTS after this phase:**

Everything from Phase 1, plus:

- Type annotations on `let`, `def`, function signatures
- Bidirectional type checker (synth/check modes)
- Ground types: `Str`, `Int`, `Bool`, `Path`, `ExitCode`
- Compound types: `Tuple(A, B)`, `List(T)`, `Map(V)`
- User-declared `struct` and `enum` with parametric type
  constructors
- `match` expression with tagged patterns, guards, alternation
- `let-else` and `if let` for refutable patterns
- `??` nil-coalescing (desugars to match on Option)
- Lambda syntax `|x| => body` and `|x| { block }`
- `def` with return type annotation
- `$status : ExitCode` (no longer bare Int)
- Status as `Result((), ExitCode)` — `try`/`catch` as scoped
  ErrorT
- Unified `trap` (lexical / global / deletion)
- Pattern exhaustiveness checking
- Type error messages with source locations
- `Path` as component list, not string
- Val enum: `Str`, `Int`, `Bool`, `Path`, `ExitCode`, `List`,
  `Tuple`, `Struct`, `Map`, `Sum`, `Lambda`, `Status`

**What it is NOT:** No discipline functions (variables are
plain). No coprocesses. No typed pipes. No linear resources.
No polarity frames beyond the implicit ones in command
substitution. No job control.

**Why this is coherent:** It is a typed shell. You can declare
structs and enums, match on variants, catch errors with
try/catch, and the type checker tells you when you got it wrong.
This is useful independently of everything that comes after —
the type system catches bugs in shell scripts.

### Phase 3: discipline and resources

**What it is:** psh proper. The features that distinguish psh
from every other shell: codata discipline functions, linear
resources, coprocesses, typed pipes, polarity frames.

**What EXISTS after this phase:**

Everything from Phase 2, plus:

- Discipline functions: `.get` (pure), `.refresh` (effectful),
  `.set` (mutator) — the codata model
- Polarity frames at all ↓→↑ shift sites (command sub,
  arithmetic, `.refresh`/`.set` bodies)
- CBV focusing for `.get` (fires once per variable per
  expression)
- Three-zone linear resource model (classical / affine / linear)
- `let !x` for classical promotion, `set -o linear`
- Coprocesses: named channels, 9P protocol, per-tag binary
  sessions, wire format, negotiate/teardown
- `print -p` / `read -p` with ReplyTag tracking
- Typed pipes: `|[T]` syntax, `Stream(T)` session type,
  static type check at the cut site
- `$((...))` arithmetic with polarity frame
- String methods (`.length`, `.upper`, `.lower`, `.split`, etc.)
- Per-type accessor namespaces (`def Type.method`)

**What it is NOT:** No job control beyond background. No line
editing. No interactive niceties.

**Why this is coherent:** This is the shell the spec describes.
All the novel features are present. You can write discipline
functions, use linear resources for init scripts, talk to
coprocesses with session-typed protocols, and compose shell
functions with typed pipes. A power user writing system scripts
would choose this over bash.

### Phase 4: interactive

**What it is:** psh as a daily driver. Job control, line
editing, history, completion — the interactive layer that makes
a shell pleasant for humans.

**What EXISTS after this phase:**

Everything from Phase 3, plus:

- Job table, `fg`/`bg`/`jobs`/`wait -n`/`kill`, `%N` word
- REPL with line editing (emacs and vi modes via `set -o`)
- History (`$HISTFILE`, `$HISTSIZE`, persistence, `Ctrl-R`,
  `fc` builtin)
- Tab completion (paths, commands, variables, struct fields,
  enum variants, programmable `complete` registrations)
- Configuration layout (`~/.config/psh/`, `rc.d/` drop-ins,
  `completions/`)
- `$TMOUT` idle timeout
- Prompt customization with variable expansion
- `$COLUMNS`/`$LINES` tracking via SIGWINCH

**Why this is coherent:** It is a complete interactive shell.
Someone would set this as their `$SHELL` and use it daily.

### Phase 5: hardening

**What it is:** psh production-ready. Test suite, edge cases,
error message polish, performance. The shell is functionally
complete in Phase 4; this phase makes it reliable.

**What EXISTS after this phase:**

Everything from Phase 4, plus:

- Integration test suite from spec examples
- Property tests (no-rescan, sequence preservation, CBV
  focusing, bidirectional soundness)
- Edge case coverage (signal races, fd exhaustion, deep
  recursion, large pipelines, Unicode edge cases)
- Error message audit (every error has a source location,
  clear language, actionable suggestion where possible)
- Performance baseline (startup time, pipeline throughput,
  glob speed)
- Man pages (psh(1), psh-syntax(7), psh-types(7),
  psh-builtins(1))

## Phase 1 implementation detail

Since Phase 1 is the next step, here is the build sequence
within it. Each sub-step produces something testable.

### 1a: AST + parser for simple commands

Define the Term/Command/Expr AST nodes for:
- Simple command: `echo hello world`
- Variable reference: `$x`, `$x[i]`
- Literal strings (single and double quoted)
- Lists: `(a b c)`

Write combine parsers matching `docs/spec/04-syntax.md`.
**Test:** parse `echo hello` and print the AST.

### 1b: Evaluator for simple commands

- `eval_term` — word expansion (literal, variable reference)
- `run_cmd` — simple command dispatch (external via `execvp`,
  builtins via dispatch table)
- `run_expr` — simple command execution (fork, exec, wait)
- `Shell` struct with variable store, `$PATH` search
- Builtins: `echo`, `true`, `false`, `exit`

**Test:** `echo hello` prints `hello`. `true; echo $status`
prints 0.

### 1c: Pipes and redirections

- Parse pipes (`|`) and redirections (`>`, `<`, `>>`, `<>`)
- `run_expr` for pipelines — fork each stage, wire fds
- Fd save/restore (the Lens pattern for redirections)
- CLOEXEC on all shell-created fds

**Test:** `echo hello | cat` works. `echo hello > /tmp/x`
works. `cat < /tmp/x` works.

### 1d: Variables and control flow

- Assignment: `x = val`, `let x = val`
- Substitution: `$x` splices list elements
- Control flow: `if`/`else`, `while`, `for x in list`
- Subshell: `@{ }`
- Background: `cmd &`
- Short-circuit: `&&`, `||`
- Per-command locals: `VAR=val cmd`

**Test:** FizzBuzz in psh. For loop over a list. Conditional
branching.

### 1e: Remaining lexical + startup

- Here documents (`<<`, `<<'`, `<<-`)
- Here-strings (`<<<`)
- Glob expansion (fnmatch-regex)
- Command substitution `` `{cmd} ``
- Tilde expansion
- Startup file sourcing (`.` builtin)
- `cd`, `read`, `exec`, `set`, `.` builtins
- Signal handling (SIGINT, SIGCHLD, SIGHUP)

**Test:** a real script — something you'd put in a crontab.
The shell is usable for basic automation.

## Non-goals

See `docs/spec/16-features.md` §Non-goals for the full list
with reasoning. Key items: no parametric polymorphism on `def`
signatures, no typed pipes between external processes, no
structured serialization over pipes, no pipeline fusion as
user-visible feature.
