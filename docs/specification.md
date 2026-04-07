# psh: Theoretical Foundation

## What this document is

The counterpart to SPEC.md for ksh26. That document maps sequent
calculus onto ksh93's C code, discovering structure that was already
there. This document starts from the other direction: psh's AST was
designed with the three-sorted structure in mind, and Rust's ownership
enforces resource discipline that ksh93 maintained by convention.

psh descends from rc, not from the Bourne shell. The analysis starts
there.

## Design position

psh is an excellent standalone shell first. It must be usable as a
login shell on Linux, macOS, and other Unix-likes without pane or
any other infrastructure deployed. The pane namespace integration
is a superpower that activates when available, not a prerequisite
for using the shell.

The theoretical foundations — sequent calculus structure, duploid
polarity, profunctor redirections, typed values — serve the
standalone shell. They make pipelines compose correctly, catch
errors at binding time, and give the interactive experience richer
context for completion and highlighting. The theory earns its keep
by making psh a better shell, not by enabling a specific platform.


## rc's execution model as sequent calculus

Duff's rc [1] has the three-sorted structure, unnamed and unenforced:

| rc construct | Sort | Evidence |
|---|---|---|
| Words: literals, `$x`, `` `{cmd} ``, `a^b` | Producers | Eager evaluation. "Input is never scanned more than once" [1, §Design]. |
| Pipe readers, redirect targets, continuations | Consumers | Implicit — waiting to receive a value. |
| Simple commands, pipelines, `if`, `for` | Cuts ⟨t \| e⟩ | `echo hello`: producer `hello` meets consumer stdout. |

The shifts exist in rc but are unnamed:

| rc mechanism | Shift type | Direction |
|---|---|---|
| `` `{cmd} `` command substitution | Force then return (↓→↑) | computation → value |
| `<{cmd}` process substitution | Thunk/future (↓N) | computation → storable value |
| `eval "$string"` | Force (elim ↓) | value → computation |
| `x=val; rest` | μ̃-binding (let) | bind value, continue |
| `fn name { body }` / return | μ-abstraction / return | capture continuation, compute |

rc's command substitution is the clearest shift. Duff: "The command
is executed and its standard output captured. The characters stored
in the variable `ifs` are used to split the output into arguments"
[1, §Command substitution]. Force-then-return: a command (negative)
is forced, output captured as a string list (positive), `$ifs`
mediates the crossing.

psh makes two shifts explicit that rc left implicit:

1. **Command substitution without IFS.** psh splits on newlines, not
   on an arbitrary `$ifs`. The return operation (bytes → list) is
   fixed. Duff kept `$ifs` only because "indispensable" [1, §Design];
   psh removes it, closing the last re-scanning hole.

2. **`@{ cmds }` as explicit namespace fork.** rc's `rfork n` called
   the kernel to copy the namespace. psh's `@{ cmds }` is syntactic
   shorthand for fork-with-isolated-scope — classical contraction
   (continuation duplicated, each copy independent). rc had `rfork`
   with flags [1, §Built-in commands]; psh makes the common case
   syntactic.


## The namespace as context (Γ)

In rc, the context Γ was two stores: shell variables (in-process hash
table) and the environment (Plan 9's `/env`, in-core per process group
[1, §Environment]). psh extends to three tiers:

| Tier | Resolution | Structural rules |
|---|---|---|
| Shell variables | `$x` — scope chain lookup, innermost first | Weakening (unset vars = empty list), contraction (multiple reads), exchange (no ordering). Scope push/pop on function call/return. |
| Process environment | `env.PATH` — flat key-value store | Weakening, contraction, exchange. Inherited by child processes (fork copies). |
| Pane namespace | `/pane/editor/attrs/cursor` — session-typed query to pane server | Weakening (server unavailable = concrete error, not empty value). **No contraction** — each `get` is a fresh query, results may differ. **No exchange** — ordering matters when mutations interleave with reads. |

The first two tiers admit all three structural rules, making them
classical contexts. The pane namespace tier restricts contraction —
reading a remote attribute twice may yield different values, and
read/write ordering is significant. Exchange holds (filter views
commute). This is an **affine** resource discipline: weakening and
exchange, no contraction.

```
get x                          # local: structural, admits contraction
get /pane/editor/attrs/cursor  # remote: affine, no contraction guarantee
```

Same builtin, different structural properties. The user must decide
whether to cache a remote value locally (explicitly contracting it)
or re-query (preserving linearity). rc had `/env` — kernel-maintained,
per-process-group, coherent. psh extends the scope chain into the
network, honestly, without coherence guarantees.

`Env::scopes` (env.rs) is a `Vec<Scope>` — ksh93's `dtview()` chain.
Push on call, pop on return; `Vec::pop` drops the `HashMap`. Inner
bindings cannot escape. rc stored functions in `/env/fn#name` [1,
§Environment]; psh uses a global `HashMap` — same semantics, no `/env`
dependency.


## Pipes as cuts, fds as linear resources

A pipeline is a sequence of cuts. Each `|` creates a pipe — a pair
of fds — connecting stdout-left to stdin-right. The endpoints are
linear resources: one writer, one reader, each used exactly once.

Duff: "Pipeline topologies more general than trees can require
arbitrarily large pipe buffers, or worse, can cause deadlock" [1,
§Pipeline branching]. rc restricted itself to linear pipelines plus
tree-shaped process substitution. psh follows the same restriction.

`Expr::Pipeline(Vec<Expr>)` in ast.rs represents the linear chain.
Each stage forks a process (exec.rs `run_pipeline`). The pipe wiring
is the cut realization — `pipe(fds)` creates the resource pair,
`dup2` connects each end to the appropriate process, and `close`
on the unused ends enforces linearity. Failing to close causes
deadlock (reader never sees EOF). The implementation is correct.

### Redirections as profunctor maps

`Expr::Redirect(Box<Expr>, RedirectOp)` nests an expression inside a
redirection — transformations, not properties. The profunctor
structure:

- `Output` = rmap (post-compose on output continuation)
- `Input` = lmap (pre-compose on input source)
- `Dup` = contraction (two fds alias one resource)
- `Close` = weakening (discard a resource)

Duff: "Redirections are evaluated from left to right" [1, §Advanced
I/O Redirection]. `vc junk.c >junk.out >[2=1]` redirects fd 1 to
file, then dups fd 2 to the now-redirected fd 1. psh's parser
collects redirections left-to-right and wraps in reverse so the
leftmost is outermost — recursion inward produces left-to-right
evaluation.

`run_redirect` (exec.rs) uses save/restore: `dup(fd)` to save,
`dup2` to redirect, execute inner, `dup2` to restore. This is the
same shift pattern as ksh93's polarity frames [2, §The save/restore
pattern IS the shift] — save context, enter redirected context,
restore on exit.

### fd tracking

Two layers, static and dynamic:

**Parse-time (static).** Each `Expr` carries a bitset of live fds,
initialized to `{0, 1, 2}`. `Close` makes an fd dead;
use-after-close is a parse-time error. This is the affine resource
discipline. The profunctor nesting makes the analysis
straightforward — walk the `Redirect` chain outer-to-inner,
maintaining the live set.

**Runtime (dynamic).** A `FdTable` tracks per-fd metadata that the
kernel cannot tell you — the semantic role of each fd:

```rust
enum FdRole { Pipe, File, Tty, Coproc, Session }
struct FdEntry { role: FdRole, saved: Vec<RawFd> }
fd_table: BTreeMap<RawFd, FdEntry>
```

The save stack replaces the ad-hoc dup/restore in `run_redirect`.
Each redirect pushes a saved fd; restore pops it. The lens
invariant: save-redirect-restore is a roundtrip (PutGet: restoring
after redirect gives back the saved state; GetPut: saving without
redirecting is a no-op). This is ksh93's `filemap[]` / `sh.topfd`
pattern [2, sfio-analysis/10-ksh-integration.md] translated to a
typed Rust structure.

The fd table does NOT mirror kernel state — it records only what
the kernel can't tell you (semantic role). Plan 9 would store this
in a per-process directory under `/proc`; on commodity kernels, a
small `BTreeMap` is the honest translation. Target: under 100 lines.


## What psh prevents by construction that rc enforced by convention

### No re-scanning

Duff: "it's not a macro processor. Input is never scanned more than
once" [1, §Design Principles]. rc achieved this with list-valued
variables and brace command substitution, but kept `$ifs` for
splitting command output — the one remaining re-scan point. Duff
acknowledged it was kept only because "indispensable" [1, ibid.].

psh eliminates `$ifs`. Command substitution splits on newlines,
fixed in `Shell::eval_word` (exec.rs). No variable controls
splitting. The re-scanning hole is closed.

### Scope discipline via ownership

rc's local variables: `a=local echo $a` — scoped to the command's
duration by convention [1, §Local Variables]. psh's `push_scope` /
`pop_scope` creates a `Scope` on a `Vec`; popping drops the
`HashMap`. Inner bindings cannot escape. Convention becomes type
enforcement.

### Discipline function reentrancy

`fn x.set { x = $1 }` would recurse infinitely without a guard.
ksh93 used the `SH_VARNOD` flag in `nvdisc.c`. psh uses
`Shell::active_disciplines`, a `HashSet<String>` — if the
discipline is already active, the inner operation bypasses it.
Explicit tracking instead of a hidden flag bit.

### Status as string, exit as ⊕

rc: "On Plan 9 status is a character string describing an error
condition. On normal termination it is empty" [1, §Exit status].
psh's `Status(pub String)` preserves this — the ⊕ convention [2,
§The ⊕ / ⅋ error-handling duality] where the caller inspects a
tagged value.
`Status::is_success()` checks emptiness.

The ⅋ convention (traps) is deferred. When added, the ⊕/⅋ duality
applies: `$status` is ⊕ (caller inspects), traps are ⅋ (callee
invokes continuation).


## Where psh diverges from rc and why

### Discipline functions (from ksh93, not rc)

rc had no variable-access hooks. psh adds `.get` (co-Kleisli — pure
observation, may recompute) and `.set` (Kleisli — effectful mutation)
from ksh93. This serves the namespace model: a local variable with a
`.set` discipline behaves identically to a remote pane attribute with
an `AttrWriter`. Same polarity, different location.

### Unified namespace via `get`/`set`

rc had `$x` for variables and `/env/` for the environment — distinct
syntax, distinct semantics. psh unifies through `get`/`set` builtins
resolving against three tiers. The namespace grows; the language does
not (shell.md Principle 2).

### `@{ }` subshell instead of `rfork`

rc's `rfork` [1, §Built-in commands] mapped directly to Plan 9's
`rfork(2)`. psh replaces it with `@{ cmds }` — fork with a copy of
the current scope. Equivalent to `rfork nefs`; the fine-grained
flags are lost, but so is the kernel dependency.

### Coprocesses (from ksh93, not rc)

rc had no bidirectional pipes. psh adds `cmd |&` from ksh93 — a
concurrent cut with two channels. The shell holds both a write end
(positive) and a read end (negative) to the child. Dual-channel cut,
not single-channel.


## Formal structure

### The four sorts, made explicit

ksh93's interpreter has duploid structure that SPEC.md identifies
but that the C code never names. psh names it. The AST has four
node types reflecting the sequent calculus:

| psh sort | λμμ̃ analog | Evaluation | Examples |
|---|---|---|---|
| `Word`/`Value` | Term (producer) | CBV — evaluated eagerly | `Literal`, `Var`, `CommandSub`, `Concat`, `Tuple`, `Sum` |
| `Expr` | Profunctor layer | CBN for pipelines, structural for redirections | `Pipeline`, `Redirect`, `Background` |
| `Binding` | μ̃-binder (let) | Extends context Γ | `Assignment`, `Fn`, `Let`, `LetTry`, `Ref` |
| `Command` | Cut / control | Connects producers to consumers, or branches | `Exec`, `If`, `For`, `Match`, `Try` |

In ksh93, this structure is implicit — the `Shnode_t` union carries
it via `tretyp & COMMSK` tags, but nothing in the type system
enforces the sort boundaries. psh separates them as Rust enums.
Cross-sort composition requires explicit embedding: `Command::Exec`
wraps an `Expr`; `Command::Bind` wraps a `Binding`;
`Word::CommandSub` wraps `Vec<Command>`.

The AST reflects the sort structure rather than enforcing it at the
type level — `Command` still mixes cuts (`Exec`) with control flow
(`If`, `For`). The sorts are discoverable in the AST, and the
function boundaries in the evaluator (`eval_word` vs `run_expr` vs
`run_cmd`) enforce the polarity discipline at the call-graph level.

### CBV/CBN split

The CBV/CBN split follows the duploid's two subcategories [3, §2.1].
Word expansion is Kleisli composition: each stage (`$x` lookup,
concatenation, command substitution) takes a partial value and
produces an expanded value with possible side effects. `eval_word`
in exec.rs recurses through `Word` nodes before the command that
consumes them has started. Pipeline execution is co-Kleisli:
`run_pipeline` forks all stages concurrently, and data flows on
demand through `pipe(2)` endpoints. `yes | head -1` does not
evaluate `yes` to completion. The pipe's blocking read is the demand.

Cross-polarity composition — a pipeline stage that expands a variable
(CBV) and writes to a pipe (CBN) — is non-associative in the duploid
sense. psh's sequential evaluation within each process prevents both
bracketings from being simultaneously available. Word expansion
completes before `execvp` runs; the fork boundary separates the two
polarities. This is the same resolution that ksh93 achieves by
accident and that SPEC.md documents as the single-process
serialization constraint.

### Intuitionistic within process, classical across forks

psh uses the ⊕ error convention exclusively. Every operation returns
`Status`. There is no `siglongjmp`, no continuation capture, no
`checkpt` stack. This restricts psh to intuitionistic control within
a single process: the sequent Γ ⊢ A has a single conclusion. No
μ-binder exists in the surface syntax — the user cannot capture the
current continuation.

Across fork boundaries, psh is classical. `@{ cmds }` duplicates
the continuation (classical contraction — each copy evolves
independently). The fork boundary is the shift between the
intuitionistic interior and the classical exterior: `fork()` copies
the entire context, producing two independent processes each with
intuitionistic single-conclusion evaluation.

This eliminates ksh93's continuation-stack corruption bugs [2,
§"Continuations and classical control"] by absence. ksh93's
`sigjmp_buf`/`checkpt` mechanism is the ⅋ convention — classical
control with reified continuations. psh's ⊕-only discipline
means every computation returns exactly once to its immediate caller
(CBPV without control operators [8, §2.2]).

**⊕ is the v1 convention; ⅋ is anticipated.** When traps are added,
the ⊕/⅋ duality applies: `$status` is ⊕ (caller inspects), traps
are ⅋ (callee invokes continuation). The design must not close the
door on ⅋.

### The shift operators

Three concrete constructs are polarity shifts:

**The ↓→↑ shift** (computation → value) is mediated by a shared
`capture_subprocess` primitive that forks a child, pipes stdout,
runs the body, calls waitpid, and returns a `(stdout, exit_code)`
product. Two operations project from this product — siblings, not
parent-child:

**`try { cmd }`** in value position projects both components and
wraps them as `Result[T]` = `Sum("ok", captured_val) |
Sum("err", ExitCode(n))`. CBV — evaluates immediately.

**Command substitution** `` `{ cmd } `` projects stdout only
(π₁ of the product). The exit status is discarded. On failure,
returns `Unit`. CBV — evaluates immediately.

Neither desugars into the other. They share the fork+capture
mechanism but consume different projections of its output. The
fork boundary makes both shifts total: the child's environment
is discarded on exit.

**Assignment** `x = value` is μ̃-binding. `Command::Bind(Binding::
Assignment(...))` evaluates the right-hand side (CBV — `eval_value`
forces the `Value` to a `Val`), then stores the result in the
environment. Discipline function hooks fire at the binding site.

**Discipline functions** are polarity boundary crossings. A `.set`
discipline fires when assignment occurs — computation (the discipline
body) intrudes into a value-mode context. A `.get` discipline fires
when `$x` is expanded — computation intrudes into word expansion.
These are the exact sites where ksh93's `sh.prefix` bugs occur [2,
§"The critical pair"]. psh avoids the critical pair: there is no
global mutable mode marker. The reentrancy guard is the one place
where a dynamic check is required.

### The fork boundary as thunkability test

An operation is thunkable if wrapping it in `` `{ ... } `` (the
↓→↑ shift) is semantically equivalent to running it inline, modulo
the fork boundary's effect isolation.

**Thunkable:** pure word expansion (`$x`, `$#x`, literal concat),
external commands with no shell-visible side effects (`date`, `cat`).

**Non-thunkable:** assignment (`x = val` — lost in fork), discipline-
triggering reads with side effects, `cd`, `set`, any builtin that
modifies the shell's own state.

The fork provides what a type system would otherwise need to enforce:
an isolation boundary that separates pure values from effectful
computations. Thunkable ⟹ central holds in any duploid [4, Prop 6].


## Profunctor structure in the evaluator

### Wrapped redirections make evaluation order structural

A traditional shell AST bolts redirections onto commands as a flat
list. This representation is silent about evaluation order — the
evaluator must impose a convention. Get it wrong and `cmd >file
>[2=1]` redirects stderr to the terminal instead of the file.

psh encodes redirections as wrapping:

```
Redirect(
    Redirect(
        Simple(cmd),
        Output { fd: 1, target: File("file") }
    ),
    Dup { dst: 2, src: 1 }
)
```

Inner-to-outer nesting IS left-to-right evaluation. The tree
structure makes the only legal evaluation order the correct one.
`run_redirect` recurses inward: save fd, apply operation, evaluate
inner, restore. Each layer is a self-contained scope. The profunctor
laws hold concretely: nesting composes redirections the same way
regardless of how you group them.

A flat-list representation has no parenthesization — the evaluator
must choose one — which is where bugs enter. The wrapped
representation eliminates this class of bug by construction, not
by testing.

### Discipline functions as MonadicLens

A variable with `.get` and `.set` disciplines is a MonadicLens:

- `fn x.get { ... }` is a **notification hook** (co-Kleisli
  position). It fires on `$x` access. The body runs in a readonly
  scope — mutations are rejected. The returned value is always the
  stored value, not the body's output. The `.get` body cannot
  influence what `$x` evaluates to. It can observe, log, trigger
  side effects (to stderr), but the value flows through unchanged.

- `fn x.set { ... }` is the **update** (Kleisli). It takes a value
  (`$1`) and may produce effects. The interpreter stores the value
  afterward regardless. Reentrancy guard prevents infinite recursion.

**Why `.get` is notification-only, not a view transformer.** The
roundtable deliberated this extensively. A transforming `.get` (where
the body's stdout replaces the returned value) would break the
MonadicLens laws: PutGet fails because `view(set(s, b))` returns
whatever `.get` computes, not `b`. GetPut fails because storing the
transformed value back changes the state. The variable degrades from
a lawful MonadicLens to an arbitrary pair of effectful functions with
no compositional guarantees. ksh93's transforming `.get` discipline
(via `.sh.value`) was the single largest source of crash bugs in
ksh93u+m — scoping leaks, use-after-free in subshell discipline
chains, and crash in get/getn disciplines from longjmp interaction.

Computed variables (`let x = try { body }`) serve the "live query"
use case that transforming `.get` would otherwise handle. The
computed variable is explicitly tier-3 (affine, no laws) while `.get`
disciplines preserve tier-2 MonadicLens laws.

The composition with pane's namespace MonadicLens follows from the
shared optic type: `get /pane/editor/attrs/cursor` fires a remote
AttrReader (co-Kleisli); `set /pane/editor/cursor 42` fires a remote
AttrWriter (Kleisli).

**Scope of lens laws:** MonadicLens laws (PutGet, GetPut, PutPut)
hold for tiers 1-2 (local variables, environment) where the store
is process-local and stable. For tier 3 (pane namespace, computed
variables), PutGet degrades — `get` after `set` is not guaranteed
to return the set value because the remote store may have changed.
The lens laws become an affine contract: the shell does not
guarantee round-trip fidelity for remote attributes. This matches
the structural-rule distinction: tiers 1-2 admit contraction
(classical), tier 3 does not (affine).

### Products, coproducts, and the optic hierarchy

Val's type constructors map directly to the optic hierarchy:

| Type constructor | Optic | Constraint |
|---|---|---|
| Tuple (product, ×) | Lens | Cartesian |
| Sum (coproduct, +) | Prism | Cocartesian |
| Tuple × Sum | AffineTraversal | Cartesian + Cocartesian |
| List (sequence) | Traversal | Monoidal |

Products give users Lenses: `$pos.0` projects the first element
of a tuple. Coproducts give users Prisms: `match $result { case ok $v { } }` decomposes a tagged value. Composing both gives
AffineTraversals: `$result.ok.name` is Prism then Lens.

Sum values are the user's coproduct constructor — the open
counterpart to Val's fixed enum. Val's Rust enum is a closed
coproduct (the implementation's sum type). Sum is an open
coproduct (the user's sum type). Without Sum, the Cocartesian
half of the optic hierarchy would be theoretically present but
operationally unreachable for user-defined domains. Every script
needing domain-specific alternatives would encode them as lists
with tag strings — the void-pointer pattern the type system
exists to prevent.

ExitCode is a reified computation outcome. It enters the value
world through `try` (the ↑ shift). `try { body }` runs the body
and produces either `ok(captured_value)` (Sum "ok") or
`err(ExitCode(n))` (Sum "err"). The type is `Result[T] =
T | ExitCode`. The coproduct structure is explicit in the type
annotation and in the runtime Sum value.

### Error metadata on variables

For stored variables (tiers 1-2), the value is always clean data.
For computed variables (`let x = try { }`), the value IS the
Sum result — `Sum("ok", T)` on success, `Sum("err",
ExitCode(n))` on failure. The `$x.err` accessor is a Prism
preview into the err branch of that Sum result. The `$x.ok`
accessor previews the ok branch.

Each Var also carries `error: Option<String>` — the human-readable
error message from stderr of the last failed evaluation. This is
diagnostic metadata (following Plan 9's errstr model), not the
primary error signal. The ExitCode in the Sum result is the
structural error value; the error string is for user display.

Val stays inert — pure positive data, Clone, no embedded error
signals. Adding Err as a Val variant was rejected because it
breaks Val's inertness — an Err in value position is a
computation-mode signal (negative) embedded in a value-mode type
(positive), the same polarity confusion the specification
identifies in BMessage. The Sum result preserves the
separation: error is a coproduct branch (positive data with a
tag), not a mode violation.

### `try` as the ⊕→⅋ converter

The specification establishes two error conventions:
- **⊕** (positive): caller inspects `$status`. Explicit. v1.
- **⅋** (negative): callee invokes continuation. Traps. Deferred.

`try` is the ⊕→⅋ converter — it takes explicit ⊕ checking and
makes it automatic within a lexical scope. Inside `try`, every
command's Status is checked; nonzero aborts to the `else`
handler. This is `set -e` done right: lexically scoped (not
dynamic), with explicit handler (not silent termination), and
with boolean-context exemptions (if/while conditions, &&/||
LHS are not checked).

ksh26's SPEC.md identifies `set -e` as the ⊕→⅋ converter.
psh's `try` is the principled version with proper scoping. The
ErrorT monad transformer sits in the evaluator's control flow
(the cut-elimination engine), not in the value sort. Val stays
positive. The monad is at the right level.

**Distinction: scoped `try` vs value-position `try`.** `try { } else { }`
is the ⊕→⅋ converter — inside the block, errors abort to the
handler (⅋ discipline). `try { }` in value position (RHS of
`let`, argument to a command) is CBV — the body evaluates
immediately via `capture_subprocess`, returning `Result[T]` =
`Sum("ok", T) | Sum("err", ExitCode(n))`. All `let`
bindings are CBV — there is no call-by-name `let` form. Live
re-evaluation uses `.get` discipline functions, not CBN `let`.

The value-position form stays in ⊕ — the caller inspects
the Sum result via `match $x` or the `.ok`/`.err` accessors.
No automatic abort occurs. The `try` keyword marks "fallible
computation" in both cases; the syntactic context determines
whether the error discipline is ⅋ (scoped handler) or ⊕ (caller
inspects).

### Word expansion as Kleisli pipeline

ksh93's macro.c expansion pipeline (tilde → parameter → command sub
→ arithmetic → field split → glob) is Kleisli composition [2,
§"The monadic side"]. psh's `eval_word` has a simpler pipeline:

1. **Literal** → identity (pure, no effects)
2. **Var** → discipline-checked lookup (co-Kleisli: fire `.get`, read value)
3. **Index** → lookup then project
4. **Count** → lookup then measure
5. **CommandSub** → polarity shift (↓→↑: fork, capture, return)
6. **Concat** → rc's `^` (pairwise or broadcast join)

Each stage is a function `Word → Val` with possible effects. They
compose by structural recursion over the `Word` enum.


## I/O architecture

### Design position: Plan 9 directness, not sfio complexity

ksh93 built a 15,000-line I/O substrate (sfio) with discipline
stacks, pools, string streams, format engines, and mmap support.
Plan 9 had bio(2) — 300 lines: a buffer and a file descriptor.
psh follows Plan 9: `std::io::BufReader`/`BufWriter` (Rust's bio
equivalent) for line-oriented reads, raw syscalls everywhere else.

sfio's complexity came from trying to be the kernel, a file server,
and the application simultaneously. psh delegates I/O to the kernel
and Rust's `std::io`. Discipline functions operate at the variable
layer (semantic level), not the byte-stream layer. If psh's I/O
layer exceeds ~200 lines, something has gone wrong.

The sfio analysis [2, sfio-analysis/] informs three specific
designs.

### Command substitution capture

Currently every `` `{cmd} `` forks and pipes regardless of output
size. ksh93's `sftmp(PIPE_BUF)` pattern starts in-memory and
promotes to file on overflow — the `_tmpexcept` polarity shift
[2, sfio-analysis/09-string-and-temp.md].

psh adopts a two-tier capture strategy:

```rust
enum CaptureBuffer {
    Mem(Vec<u8>),         // value-mode: pure memory, no syscalls
    File(std::fs::File),  // computation-mode: fd-backed, real I/O
}
```

Accumulate in `Vec<u8>` up to a threshold; spill to an anonymous
temp file (`memfd_create` or `O_TMPFILE`) on overflow. The polarity
shift is the same as sfio's — value (memory) promotes to
computation (fd) — but through Rust's enum dispatch rather than
sfio's `memcpy` identity swap. The invariant from sfio applies:
the handle the evaluator holds must not change across the
promotion.

In optic terms [2, sfio-analysis/09-string-and-temp.md], this is
a change of monoidal action: Optic_{×, ×} (plain Lens — both
actions cartesian, Ψ = Id) becomes Optic_{×, ⋊} (MonadicLens —
update action now Kleisli for IO). The optic interface is
preserved; the algebra under it shifts.

### Coprocess I/O

ksh93 uses two unidirectional pipes for coprocesses. Plan 9 pipes
were bidirectional by default — both ends of `pipe(2)` could read
and write, because pipes were 9P connections to the `#|` device.

psh uses `socketpair(AF_UNIX, SOCK_STREAM)` for coprocesses. No
`shutdown` on either end. One fd per side — the child reads and
writes its end, the shell reads and writes its end. Fewer fds to
track, fewer close-ordering bugs than the two-pipe model.

```rust
struct Coproc {
    fd: RawFd,           // bidirectional socket endpoint
    pid: libc::pid_t,
}
```

The session type structure is two independent linear channels
(read and write) on the same fd, not a single sequenced protocol.
The shell can read and write in any order. Deadlock prevention is
by convention (half-duplex: write then read, or read then write).

### Atomic writes for structured output

sfio's `SFIO_WHOLE` flag ensures writes are atomic at the buffer
level. psh builtins that produce multi-line output (`get` printing
a list, `echo` in a pipeline) should write the complete output in
one `write(2)` call to avoid interleaving with other pipeline
stages. This matters when `echo $multiline_var | ...` sends to a
pipeline.

### What sfio teaches about non-associativity in I/O

The sfio discipline stack's `Dccache` mechanism [2, sfio-analysis/
07-disciplines.md] is the duploid's non-associativity made concrete
in I/O: data that has crossed from computation to value mode (was
already buffered by a discipline) cannot be re-processed through a
new discipline without corruption. The mediator (`Dccache_t`)
explicitly resolves the bracketing.

psh's wrapped-redirect representation prevents this at the AST
level — the nesting determines the only legal bracketing, so
the Dccache problem cannot arise. If psh ever adds runtime stream
transformations (encoding conversion, compression on redirected
output), the Dccache non-associativity will reappear and will need
explicit mediation.


## Design heritage: ksh93 and BeOS

### The let/control duality

SPEC.md [2] identifies variable assignment (μ̃) as dual to trap
binding (μ). Both bind a name in a context; they differ in which
side of the sequent they extend. In psh:

| Mechanism | Binder | Side | Polarity |
|---|---|---|---|
| `fn x.set { }` | μ̃ (let) | Value (Γ) | Kleisli: effectful mutation |
| `fn x.get { }` | μ̃ (dual) | Value (Γ) | co-Kleisli: pure observation |
| `fn sigint { }` | μ (control) | Computation (Θ) | ⅋: callee-driven |

The save/restore discipline they all require is the same. The
reentrancy guard (`active_disciplines: HashSet<String>`) prevents
re-entry during discipline execution — the shift marker.

### The SFIO discipline stack as polarity boundary

ksh93's SFIO discipline stack [2, sfio-analysis/07-disciplines.md]
mediates between the stream's buffer (value-mode, positive) and the
OS (computation-mode, negative). psh's discipline functions reproduce
this on a different substrate: the stored variable value is the
buffer; the side effects triggered by `.set` are the OS-facing
computation.

### ksh93's Namval_t vs psh's Var

ksh93's `Namval_t` carries value, type, attributes, discipline stack,
namespace linkage, and tree pointers — a monolithic state object.
psh's `Var` has four fields: `value`, `exported`, `readonly`,
`discipline`. The discipline is a pair of function names, not a
stack. Single-layer suffices because psh has no typed variables —
ksh93's stack existed for `typeset -T` inheritance.

### BMessage polarity confusion

BeOS's BMessage was a compound type: `positive_data × ↑(negative_
continuation)` that was never decomposed. The reply_port (negative,
linear) was embedded with the data (positive, copyable). This caused
aliased continuations, thread-unsafe reply paths, and coupled
lifetimes.

psh avoids this by keeping `Val` inert — pure positive data, `Clone`,
no embedded continuations. Discipline dispatch is separate. The value
never carries its own access protocol.

**Danger zones:** async discipline dispatch would reintroduce the
tangling. If a `.set` body performs an async remote write, the
discipline's computation overlaps with subsequent value operations —
the BMessage mistake in a new form. The reentrancy guard is a
synchronous `HashSet`; it becomes unsound across await points. psh
must keep discipline dispatch synchronous and serialized.

### par and the duploid: distinct layers

The duploid governs evaluation order within the interpreter — how
words become values, how pipelines compose, where polarity boundaries
fall. `par` governs communication protocols — the typed exchange
sequences between psh and a pane server.

par is NOT a direct dependency of psh. It enters through pane-session
(feature-gated). psh's internal machinery — profunctor redirections,
discipline functions, scope chains, pipeline wiring, job handles —
uses Rust's ownership, raw fds, and `oneshot` channels. These are
sufficient: the profunctor structure is in the AST nesting, the
MonadicLens is in the discipline protocol, the affine fd discipline
is in `dup`/`close` bracketing.

par enters psh at one boundary: when `get`/`set` builtins connect
to a pane server, pane-session manages the par handshake internally.
The evaluator produces fully-evaluated arguments, hands them to the
builtin, and the builtin opens a session. The session is consumed
within that synchronous builtin call and returns a `Val` or `Status`.
Word expansion finishes before the session begins; the session
finishes before the next command starts. The two layers compose
without interference because they never overlap in time.

Job handles use `oneshot::Receiver<ExitStatus>` with a `JobHandle`
wrapper providing Drop compensation (disown on drop). This is the
ReplyPort pattern from pane, without par — the obligation is
enforced by `#[must_use]` and Drop, not by session type duality.
Coprocesses use raw `socketpair`. Signals use self-pipe. These are
kernel I/O operations, not typed protocols.


## References

1. Tom Duff. "Rc — The Plan 9 Shell." Reprinted in the Plan 9
   Programmer's Manual, Volume 2. Originally internal Bell Labs
   memorandum, 1990. `reference/plan9/papers/rc.ms`

2. SPEC.md — ksh26 theoretical foundation. Maps λμμ̃-calculus
   onto ksh93's interpreter.
   `/Users/lane/src/ksh/ksh/SPEC.md`

3. Éléonore Mangel, Paul-André Melliès, and Guillaume
   Munch-Maccagnoni. "Classical notions of computation and the
   Hasegawa-Thielecke theorem." POPL, 2026.

4. Guillaume Munch-Maccagnoni. "Models of a Non-Associative
   Composition." FoSSaCS, 2014.

5. David Binder, Marco Tzschentke, Marius Müller, and Klaus
   Ostermann. "Grokking the Sequent Calculus (Functional Pearl)."
   ICFP, 2024.

6. Pierre-Louis Curien and Hugo Herbelin. "The duality of
   computation." ICFP, 2000.

7. Philip Wadler. "Call-by-Value is Dual to Call-by-Name, Reloaded."
   RTA, 2005.

8. Paul Blain Levy. *Call-by-Push-Value.* Springer, 2004.

9. ksh26 sfio analysis suite. Operational semantics of ksh93's I/O
   substrate, with polarity annotations.
   `/Users/lane/src/ksh/ksh/notes/sfio-analysis/`

10. Clarke, Elgot, Gibbons, Sherwood-Taylor, Wu. "Profunctor Optics,
    a Categorical Update." Compositionality, 2024.
