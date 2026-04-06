# psh: Theoretical Foundation

## What this document is

The counterpart to SPEC.md for ksh26. That document maps sequent
calculus onto ksh93's C code, discovering structure that was already
there. This document starts from the other direction: psh's AST was
designed with the three-sorted structure in mind, and Rust's ownership
enforces resource discipline that ksh93 maintained by convention.

psh descends from rc, not from the Bourne shell. The analysis starts
there.


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
| `Word`/`Value` | Term (producer) | CBV — evaluated eagerly | `Literal`, `Var`, `CommandSub`, `Concat` |
| `Expr` | Profunctor layer | CBN for pipelines, structural for redirections | `Pipeline`, `Redirect`, `Background` |
| `Binding` | μ̃-binder (let) | Extends context Γ | `Assignment`, `Fn` |
| `Command` | Cut / control | Connects producers to consumers, or branches | `Exec`, `If`, `For`, `Switch` |

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

**Command substitution** `` `{ cmd } `` is ↓→↑: force a computation
into value position. `eval_word` handles `Word::CommandSub` by
forking a child, piping stdout back, running the commands in the
child (↓), collecting the output, returning a `Val` (↑). The fork
boundary makes the shift total: the child's entire environment is
discarded on exit. No state leaks back except the pipe's bytes.

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

- `fn x.get { ... }` is the **view** (co-Kleisli). It extracts a
  value from context. psh **enforces purity** by running `.get`
  bodies in a readonly scope — mutations inside `.get` are
  discarded. The reentrancy guard prevents recursive firing.

- `fn x.set { ... }` is the **update** (Kleisli). It takes a value
  (`$1`) and may produce effects. The interpreter stores the value
  afterward regardless. Reentrancy guard prevents infinite recursion.

The composition with pane's namespace MonadicLens follows from the
shared optic type: `get /pane/editor/attrs/cursor` fires a remote
AttrReader (co-Kleisli); `set /pane/editor/cursor 42` fires a remote
AttrWriter (Kleisli). A nameref bridges local and remote:

```
ref cursor = /pane/editor/attrs/cursor
fn cursor.set { set /pane/editor/cursor $1 }
```

**Scope of lens laws:** MonadicLens laws (PutGet, GetPut, PutPut)
hold for tiers 1-2 (local variables, environment) where the store
is process-local and stable. For tier 3 (pane namespace), PutGet
degrades — `get` after `set` is not guaranteed to return the set
value because the remote store may have changed. The lens laws
become an affine contract: the shell does not guarantee round-trip
fidelity for remote attributes. This matches the structural-rule
distinction: tiers 1-2 admit contraction (classical), tier 3 does
not (affine).

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
sequences between psh and a pane server, between coprocess endpoints,
or between job handles and the shell.

These are different concerns at different layers. The duploid
determines *when* a `par` session begins and ends (inside a cut).
`par` determines *what happens* during the session. They compose
without interference because word expansion finishes, the builtin
dispatches, and only then does the session-typed exchange begin.


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
