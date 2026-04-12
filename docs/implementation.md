# psh: Implementation Notes

## What this document is

Crate dependencies, implementation strategy, and engineering
decisions. Updated as the implementation evolves. Companion to
`specification.md` (semantics) and `syntax.md` (grammar).


## Dependencies

Each dependency must earn its keep. A shell is a long-lived
process that starts fast and stays small. No dependency is
included for convenience alone.

### Core

**combine** (4.x) — monadic parser combinator library. The
parser (`parse.rs`) is a six-layer architecture matching the
grammar's structure: lexical primitives, word atoms, free
carets, expression precedence, commands, program. combine's
`Stream` trait and error recovery map directly onto psh's
parsing needs. Chosen over nom because the grammar is
recursive (nested command substitution, match arms, lambda
bodies) and combine's monadic style handles recursion
naturally without the macro complexity nom requires for the
same. Chosen over hand-written recursive descent because the
grammar is still evolving and combinator composition lets us
restructure productions without rewriting control flow.

**rustix** (1.x, features: pipe, process, stdio, termios, fs,
net) — safe Rust bindings to Linux/Unix syscalls, bypassing
libc where possible. Used for pipe creation, process control
(fork, exec, waitpid), fd manipulation (dup2, close), terminal
control, and filesystem operations. Chosen over raw libc
because rustix provides safe wrappers with proper error types.
Chosen over std where std's abstractions are too high-level
(std::process doesn't expose fd-level control needed for
redirections, coprocesses, and the save/restore lens pattern).
The features list is explicit — we use only what we need.

**libc** (0.2) — still required alongside rustix for signal
handling infrastructure and any syscall rustix doesn't yet
wrap. Target: minimize libc surface as rustix coverage grows.

**smallvec** (1.x, feature: union) — inline-allocated vector.
Used for the scope chain (most functions have 1-3 local
variables), argument lists (most commands take 1-5 args), and
redirect stacks (most commands have 0-2 redirections). These
are hot paths where heap allocation per invocation is
measurable. The `union` feature enables the smallest possible
inline representation. smallvec is a single file with no
transitive dependencies — it adds ~200 lines to the build,
not a framework.

### Error handling

**anyhow** (1.x) — ergonomic error handling for the shell's
internal plumbing (startup, config loading, signal setup).
Not used in the hot path (command execution uses `Status`
directly). anyhow is appropriate here because internal errors
are diagnostic ("failed to read ~/.psh/env"), not structured
data.

### Argument parsing

**bpaf** (0.9, feature: derive) — command-line argument parser
for psh's own flags (`psh -c 'cmd'`, `psh -l`, `psh file`).
Not used for parsing shell syntax (that's combine). Chosen
over clap because bpaf is smaller, has no proc-macro
dependency in the default feature set, and its derive mode
is sufficient for psh's simple flag interface. A shell's
own argument parsing should not pull in a framework heavier
than the shell itself.

### Pattern matching

**fnmatch-regex** (0.3) — converts fnmatch glob patterns to
regex. Used by the `=~` operator and `match` glob arms. rc's
pattern matching uses fnmatch semantics [1, §Simple commands];
this crate provides exactly that. Small, focused, no transitive
dependencies.

### Signals

**signal_receipts** (0.2, features: premade, channel_notify_
facility) — signal receipt tracking. Provides the self-pipe
pattern for async-signal-safe notification. psh's signal
handling uses the flag-and-self-pipe approach: signals set a
flag and write to a pipe; the main loop reads the pipe and
dispatches. This crate provides the infrastructure without
reimplementing the self-pipe. When lexical `trap` is
implemented, the self-pipe mechanism delivers signals to the
innermost active trap scope.

## Crate budget

Current: 8 required dependencies, 2 optional. Target: hold
this line. New dependencies require justification in this
document before being added. The test: would you accept this
dependency in a login shell that runs on every terminal open?


## Source structure

    src/
        main.rs     — entry point, argument parsing, REPL loop
        ast.rs      — three-sort AST (Word, Expr, Command)
        parse.rs    — combine-based parser matching syntax.md
        check.rs    — bidirectional type checker (~500-900 lines)
        exec.rs     — evaluator (eval_word, run_expr, run_cmd)
        env.rs      — scope chain, variable store, discipline dispatch
        value.rs    — Val enum, Display/FromStr
        job.rs      — job control, background processes
        signal.rs   — signal handling, self-pipe

The AST has three sorts matching the spec: `Word` (producers),
`Expr` (engineering layer — pipelines + redirections), `Command`
(statements / cuts). Consumers are synthesized implicitly from
the statement's shape, not stored as AST nodes. The evaluator
enforces the sort boundary at the call-graph level: `eval_word`
(CBV, value sort), `run_expr` (profunctor layer), `run_cmd`
(cuts, command sort).


## Implementation principles

**Match existing code patterns.** The codebase has conventions;
follow them. Read before writing.

**The parser is the grammar.** `parse.rs` should read as a
transliteration of `syntax.md`. Each production in the grammar
maps to a named parser function. When the grammar changes, the
parser changes to match.

**Val is inert.** `Val` is pure positive data — Clone, no
embedded errors, no computation-mode signals. Effects live in
the evaluator, not in the value type.

**Errors at boundaries.** Validate at the boundary between the
shell and external systems (user input, filesystem, coprocess
wire format). Internal code trusts internal invariants.

**CLOEXEC by default.** Every fd created by the shell
(pipes, redirections, coprocess socketpairs) is O_CLOEXEC
unless explicitly inherited by a child. This prevents fd
leaks across exec boundaries. rustix's pipe and socket
creation functions support CLOEXEC flags natively.

**Signal safety across fork.** Between `fork()` and
`execve()`, signal handlers are inherited. The shell must
either block signals before fork and restore after exec, or
ensure all signal handlers are async-signal-safe in the child.
The self-pipe pattern via signal_receipts handles this: signal
handlers only write a byte to the pipe fd, which is safe.

**No global mutable state.** The `Shell` struct owns all
mutable state. No `static mut`, no thread-local mutation.
The reentrancy guard for discipline functions is a field on
`Shell`, not a global flag. This is the lesson from ksh93's
`sh.prefix` / `sh_getscope` bugs [SPEC, §The critical pair].


## References

[1] Tom Duff. "Rc — The Plan 9 Shell." 1990.
[SPEC] ksh26 Theoretical Foundation. `refs/ksh93/ksh93-analysis.md`
