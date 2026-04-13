# Polarity discipline

## Polarity discipline

### CBV/CBN split

The CBV/CBN split follows the duploid's two subcategories [MMM,
§2.1]. Word expansion is Kleisli composition: each stage
(`$x` lookup, concatenation, command substitution) takes a
partial value and produces an expanded value with possible
effects. `eval_term` recurses through `Term` nodes before the
command that consumes them has started.

Pipeline execution is **demand-driven** (operationally analogous
to co-Kleisli — not a VDC classification, see note below): `run_pipeline` forks all stages
concurrently, and data flows on demand through `pipe(2)`
endpoints. `yes | head -1` does not evaluate `yes` to
completion — the pipe's blocking read is the demand. Note:
the VDC framework §8.1 classifies pipeline *composition
structure* as Kleisli/monadic (the data on the pipe is the
positive intermediary). The execution strategy (demand-driven)
and the composition structure (data-on-pipe) are different
readings of the same pipeline — one describes how it runs,
the other how it types.

**fd-targeted pipes** (rc heritage, rc.ms lines 881-903):
`cmd |[2] cmd2` pipes fd 2 (stderr) of the left command to
stdin of the right. The general form `cmd |[n=m] cmd2`
connects fd n of the left to fd m of the right. Standard `|`
is sugar for `|[1=0]`. This selects which horizontal arrow of
the left cell connects to the right cell's input — the same
cell composition structure, different source arrow. Without
fd-targeted pipes, piping stderr while keeping stdout separate
requires process substitution gymnastics.

Cross-polarity composition — a pipeline stage that expands a
variable (CBV) and writes to a pipe (CBN) — is non-associative
in the duploid sense. Specifically: among the four cases
`(ε, ε') ∈ {⊕,⊖}²` enumerated in [MMM, §"Emergence of non-
associativity"], three associate cleanly and the fourth —
`(⊕,⊖)` — fails. Writing `•` for Kleisli (monadic) composition
and `○` for co-Kleisli (comonadic) composition, the failing
bracketing is:

    (h ○ g) • f   ≠   h ○ (g • f)

— a comonadic step `○` wrapping a monadic step `•`. This is the
operational shape of the `sh.prefix` bugs documented in [SPEC,
§"Non-associativity made concrete"]: a computation-mode
operation (DEBUG trap, `○`) intruding into a value-mode context
(compound assignment, `•`) with no mediator. sfio had the
mediator (Dccache); the ksh93 shell proper did not.

psh's sequential evaluation within each process prevents both
bracketings from being simultaneously available. Word expansion
completes before `execvp` runs; the fork boundary separates the
two polarities; the polarity frame discipline (see §Polarity
frames) mediates the remaining `↓→↑` crossings. This is
operational focalization — the same deterministic reduction
order that Curien and Munch-Maccagnoni's focused calculus [CMM10]
achieves syntactically, psh achieves operationally. See
`docs/vdc-framework.md` §8.4 for the full statement of the
non-associativity failure and its decision-procedure
classification in §8.5.

### Classical control

psh is classical. The sequent Γ ⊢ A | Δ has multiple
conclusions — Δ is populated by `trap` bindings (μ-binders
that name signal continuations). `@{ cmds }` duplicates the
continuation (classical contraction — each copy evolves
independently). The fork boundary is the shift between the
local context and the classical exterior.

psh avoids ksh93's continuation-stack corruption bugs [SPEC,
§"Continuations and classical control"] by making the μ-binder
lexically scoped. ksh93's `sigjmp_buf`/`checkpt` mechanism used dynamic
traps with global mutation (`sh.prefix`, `sh_getscope`). psh's
lexical `trap` binds a continuation in Δ for the duration of
a block — no global state, no stack corruption, no longjmp.
The classical control is tamed by lexical scope, not eliminated.

### Three operations, three roles

1. **Command substitution** (`` `{cmd} ``): fork + wait +
   capture. Synchronous shift ↓→↑. Pure focalization. The
   mechanism forks, pipes stdout, runs the body, calls waitpid,
   returns `(stdout, exit_code)`. CBV — evaluates immediately.

2. **Process substitution** (`<{cmd}`): downshift `↓` into the
   fd namespace. The negative CBN pipeline is thunked behind a
   `/dev/fd/N` name. The name is positive (a string), the
   computation behind it is negative (demand-driven, reads
   trigger it). This is Plan 9's mount model — synchronous
   bind, concurrent server. Focalization is not violated because
   the bind itself is instantaneous; the concurrency is behind
   the name. Distinct from command substitution: command
   substitution forces the computation to produce a value
   (`↓→↑`), while process substitution thunks it behind a name
   (`↓` only).

3. **Pipeline** (`|`): concurrent cut. Co-Kleisli composition.
   Each `|` creates a pipe — a linear resource pair — connecting
   stdout-left to stdin-right. Both sides run concurrently.
   Demand flows right-to-left via blocking reads.

### Polarity frames

A **polarity frame** is the operational mechanism for a `↓→↑`
shift — forcing a negative (computation-mode) term to produce a
positive (value-mode) result. Structurally, a frame is a
restriction-like cell in the VDC of shell programs (cf.
fcmonads §7, virtual equipments): it saves the positive-mode
state (the expansion context — splice positions, CBV-focused
values, partial word accumulators), runs the negative computation,
and restores the positive-mode state on exit with the produced
value substituted in.

The frame is the operational analog of the shift connectives
`↓` / `↑` from the focused sequent calculus, and of the `L ⊣ R`
adjunction every duploid admits [MMM, §"Duploids," adjunctions-
duploids theorem]. Without the frame, a computation-mode
operation inside a value-mode context can silently corrupt
positive-mode state — the `sh.prefix` bug pattern documented in
`refs/ksh93/ksh93-analysis.md`, which is the operational form of the `(⊕,⊖)` non-
associativity named above.

Polarity frames are invoked in three places in psh:

- **Command substitution** `` `{cmd} `` — frame saves the word
  expansion context, forks a subprocess, captures stdout, restores
  the context with the result list substituted. Full ↓→↑ shift.
- **Arithmetic expansion** `$((…))` — frame is operationally
  trivial (pure in-process computation, no effects to guard), but
  the shift is still type-theoretic: the expression is `μα.⊙(e₁,
  e₂;α)` in [BTMO23, §2.1]'s arithmetic translation, a statement
  wrapped in a μ-binder to produce a positive value. The frame
  mechanism is the same; its save/restore steps simplify to no-ops.
- **Discipline refresh** `.refresh` and **mutation** `.set`
  bodies — frame saves the expansion context, raises a reentrancy
  flag on the variable, runs the body, restores the context and
  clears the flag on exit. See §"Reentrancy and the polarity
  frame" in §Discipline functions.

Pure `.get` bodies do *not* require a polarity frame. `.get` is
a pure map into the positive subcategory `P_t`; there is no
polarity crossing and nothing to reenter. This is the single
biggest simplification the codata model earns by separating pure
observation from effectful refresh.

Process substitution `<{cmd}` is a structurally distinct case:
the shift is a downshift (`↓`) that binds a computation behind
a name, rather than a force. The operational realization is a
fork that returns immediately with `/dev/fd/N`; the name is
positive, the computation behind the name is negative and
demand-driven. The polarity frame discipline applies only to
the bind, not to the deferred computation.

**Nested process substitution.** `cmd1 <{cmd2 <{cmd3}}` is
valid — the grammar is recursive. Evaluation order is
left-to-right, innermost-first: `<{cmd3}` forks first and
allocates `/dev/fd/N`; that name is substituted into `cmd2`'s
argument list; `<{cmd2 /dev/fd/N}` then forks and allocates
`/dev/fd/M`; that name is substituted into `cmd1`. Each fork
allocates the lowest available fd not already in use. The
inner substitution's fd is visible to the outer command only
as a string (the `/dev/fd/N` path) — there is no fd-table
sharing between the nested forks. Each process substitution
is an independent downshift with its own fd allocation.


### Linear resources and exponentials

The linear classical L-calculus (§The semantics) partitions the
typing context into a **classical zone** (variables under `!`,
with contraction and weakening) and a **linear zone** (bare
variables, no structural rules). psh extends this two-zone
partition with an **affine** middle zone (weakening with runtime
cleanup, no contraction) for resource types that have a
well-defined cancellation protocol. The three usage disciplines:

| Zone | Default for | Structural rules | Surface syntax |
|---|---|---|---|
| **Classical** (`!A`) | Value types: Str, Int, Bool, Path, ExitCode, List, Tuple, Struct | Contraction + weakening (copy, discard freely) | `let x = b` or `let !x = b` |
| **Affine** | Resource types with cleanup: ReplyTag | No contraction; weakening triggers runtime cleanup | `let tag = print -p name 'q'` |
| **Linear** (bare `A`) | Bare-annotated bindings; default under `set -o linear` | No contraction, no weakening | `let fd : Fd = open f` |

**Default behavior.** `let x = expr` infers the zone from the
type. Ground value types (Str, Int, Bool, Path, ExitCode, List,
Tuple, Struct) are implicitly `!`-promoted — classical, freely
duplicable and discardable. This is why `$x` can appear multiple
times in an expression with no ceremony. Resource types with
runtime cleanup actions (ReplyTag) are affine — use at most
once, drop triggers the cleanup (Tflush for coprocess tags).
Most shell code lives entirely in the classical zone and never
encounters linearity.

**Explicit `!` promotion.** `let !x = expr` or a `!T` type
annotation forces a binding into the classical zone regardless
of its type's default. This is the L-calculus promotion rule
(A → !A): the user asserts "I am managing this resource myself;
give me classical access." Example: `let !fd : Fd = dup $log_fd` — the
fd is freely reusable because the user has explicitly accepted
responsibility for its lifecycle.

**Linear bindings.** A bare type annotation without `!` places
the binding in the linear zone: `let fd : Fd = open 'lockfile'`.
The type checker requires that `fd` is consumed (closed,
passed to a function that consumes it, or otherwise used exactly
once) on every control-flow path. Failure to consume is a type
error, not a runtime cleanup — this is the strict discipline for
resource-critical code (init scripts, supervision trees,
long-running services).

**Zone defaults for resource types.** `Fd` defaults to linear
because there is no protocol-level cleanup for abandoned file
descriptors — a leaked fd is a silent resource leak. `ReplyTag`
defaults to affine because the Tflush/Rflush protocol provides
safe cancellation — dropping a tag is a well-defined protocol
action, not a leak. The `!` promotion (`let !fd = dup $log_fd`)
overrides both defaults when the user accepts responsibility.

**`set -o linear` — linear mode.** The `set -o linear` option
(§14-invocation.md) changes the default zone from classical to
linear for all subsequent bindings. Under linear mode,
`let x = expr` produces a linear binding even for value types.
Explicit `!` marks classical islands. The type checker verifies
that all linear bindings are consumed on every control-flow
path.

```
#!/usr/bin/env psh
set -o linear

let fd = open $notify_fd       # Fd — linear (must consume)
let name = $1                  # Str — linear (must use)
let !config = read_config()    # !List(Str) — classical island

write $fd '\n'                 # fd consumed
exec $name                     # name consumed
```

For scoped linear mode within an otherwise classical script,
use a subshell:

```
@{ set -o linear; critical_section }
```

The subshell inherits the parent's options, applies linear
mode, and the mode dies with the subshell.

**Exceptional exit under linear mode.** When linear-mode code
is exited via `try`/`catch` abort or signal handler `return N`,
unconsumed linear bindings degrade to affine cleanup: the
runtime sends Tflush for outstanding ReplyTags and closes
outstanding Fds. This is consistent with the polarity-frame
unwind semantics (§Polarity frames). The type checker warns
where exceptional paths leave bindings unconsumed, but does not
reject — the runtime cleanup guarantees protocol correctness
regardless.

**Type annotations.** `!` and `?` appear in type position with
their L-calculus meaning. `!T` is "classical T" (freely
usable). Bare `T` is "linear T" (use exactly once). `?T` is the
negative dual of `!T` — it appears in continuation/coterm
position and is not expected in typical user code. Function
signatures use these annotations to express resource contracts:

```
def supervise : (Str, !Fd) -> Status {
    # first arg: linear Str (must use the service name)
    # second arg: classical Fd (log fd, freely reusable)
}
```

**De Morgan duality.** The exponentials are dual: `?A = (!(A⊥))⊥`.
This connects to the ⊕/⅋ duality already in the spec: `try`
operates on ⊕ (data, positive, under `!` in the classical zone),
while `trap` operates on ⅋ (codata, negative, under `?` in the
continuation zone). The two compose orthogonally (§Signal
interaction with try blocks) because they live in dual zones.

**Operational cost.** The three-zone model adds no runtime
overhead for classical code. The `!` promotion is the default
and requires no tracking. Affine tracking is already implemented
(coprocess tag tracking via the outstanding-tags HashMap).
Linear checking is a compile-time verification pass — it
constrains control-flow paths at the type level, not at runtime.
`set -o linear` is a type-checker directive — it constrains
the default zone but adds no runtime overhead.


