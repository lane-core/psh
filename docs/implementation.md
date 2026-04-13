# psh: Implementation Notes

## What this document is

Crate dependencies, implementation strategy, and engineering
decisions. Updated as the implementation evolves. Companion to
`docs/spec/` (semantics) and `docs/spec/04-syntax.md` (grammar).


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
are diagnostic ("failed to read config"), not structured
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
        ast.rs      — three-sort AST (Term, Expr, Command)
        parse.rs    — combine-based parser matching docs/spec/04-syntax.md
        check.rs    — bidirectional type checker (~500-900 lines)
        exec.rs     — evaluator (eval_term, run_expr, run_cmd)
        env.rs      — scope chain, variable store, discipline dispatch
        value.rs    — Val enum, Display/FromStr
        job.rs      — job control, background processes
        signal.rs   — signal handling, self-pipe

The AST has three sorts matching the λμμ̃ categories: `Term`
(producers — values, Γ side), `Command` (consumers — command
shapes that expect to receive values, Δ side), `Expr` (cuts
— where producers meet consumers: pipelines, redirections,
fork/exec). The evaluator enforces the sort boundary:
`eval_term` (CBV, produces Val), `run_cmd` (dispatches on
consumer shape), `run_expr` (executes the cut — fd wiring,
pipeline setup, redirect composition).


## Implementation principles

**Match existing code patterns.** The codebase has conventions;
follow them. Read before writing.

**The parser is the grammar.** `parse.rs` should read as a
transliteration of `docs/spec/04-syntax.md`. Each production in the grammar
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


## Type representation architecture

The typechecker operates uniformly over all types — ground,
compound, and newtype — via a trait hierarchy in the Rust
implementation. This avoids special-casing and makes newtypes
zero-cost extensions.

### Trait hierarchy

```rust
trait Type {
    fn sort(&self) -> Sort;           // positive / negative
    fn zone(&self) -> Zone;           // classical / affine / linear
    fn backing(&self) -> Option<&dyn Type>;  // newtype: Some(underlying)
    fn eq_nominal(&self, other: &dyn Type) -> bool;
    fn eq_structural(&self, other: &dyn Type) -> bool;
}

trait Number: Type { }               // arithmetic-capable
trait Iterable: Type { }             // supports for/map/filter/each/fold
trait Accessible: Type { }           // supports dot/bracket accessors
```

Ground types (`Str`, `Int`, `Bool`) implement the relevant
subtraits directly. Compound types (`List(T)`, `Map(V)`,
`Tuple`, `Struct`, `Enum`) implement based on their structure.
Newtypes **inherit subtraits from their backing type** — a
`type Meters = Int` implements `Number` because `Int` does.

### Newtype representation

A newtype entry in Σ is:

```rust
struct NewtypeEntry {
    name: TypeName,
    params: Vec<TypeParam>,
    backing: TypeRef,
    renaming: HashMap<VariantName, VariantName>,  // new => old
    // reverse map computed once at registration
    reverse: HashMap<VariantName, VariantName>,   // old => new
}
```

The renaming table is static, resolved at parse/elaborate time.
No runtime cost, no dynamic dispatch. The typechecker consults
`backing` to verify payloads and `renaming` to map constructor
names. Trait inheritance is computed once at registration:
`Meters` checks `Int`'s trait set and inherits it.

### Syntax particle traits

The parser mirrors the type trait hierarchy with its own
trait layer for syntactic objects:

```rust
trait SyntaxParticle {
    fn sort(&self) -> SyntaxSort;        // term / command / expr
}

trait Constructor: SyntaxParticle {
    fn payload(&self) -> &dyn Type;
    fn parent_type(&self) -> TypeName;
    fn is_nullary(&self) -> bool {       // Unit payload → no parens
        self.payload().is_unit()
    }
}

trait PatternForm: SyntaxParticle {
    fn bindings(&self) -> &[Binding];
    fn is_refutable(&self) -> bool;
}
```

Parser decisions — "does this constructor need parens?", "is
this pattern refutable?", "what delimiter separates children?"
— are trait queries on syntax objects, not name-matching. The
Unit-nullary rule (e.g., `none` in `Option`) is just
`Constructor::is_nullary` checking `payload().is_unit()`.

Both layers are populated by Σ registration: a `type` or `enum`
declaration creates type entries for the checker and constructor
entries for the parser in one pass.

### Design principle

The `Type` trait exposes what the typechecker needs uniformly:
equality, sort, zone, backing representation. Subtraits capture
capability families. The typechecker never asks "is this a
newtype?" — it asks "does this type implement Number?" and gets
the right answer regardless of whether the type is direct or
newtype. This is the implementation-level analogue of the optics
result: `Adapter ∘ Prism = Prism` — newtypes are transparent
to capability queries.

### Upper/lower bounds on elaborator complexity

If type inference becomes an albatross, the escape hatch is
weakening inference by leaning on the namespace system to
disambiguate. Weaken inference, not the type system.

Features naturally suggested by the type system should stay in
the design. `set -o` can disable non-essential features for
performance, but this is not an excuse to make core safety
features slow and then justify it with an off-switch. Safety
features achievable within the design goals must be implemented
well. The default is safe; opting out is conscious and rare.


## References

All citation keys resolve to `docs/spec/references.md`.

- `[Duf90]` — Duff, "Rc — The Plan 9 Shell." 1990.
- ksh26 Theoretical Foundation: `refs/ksh93/ksh93-analysis.md`
