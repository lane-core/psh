# psh: Theoretical Foundation

## What this document is

The specification of psh's type theory, execution model, and design
rationale. psh descends from rc (Duff 1990), not from the Bourne
shell. The analysis begins there.

This document is the output of a systematic design process:
interrogation of rc's design philosophy, ksh93's implicit type
theory (discovered via the sfio-analysis and SPEC.md sequent
calculus mapping), the duploid semantics of Mangel/Melliès/
Munch-Maccagnoni, Curien-Herbelin's λμμ̃-calculus, and the
profunctor optics framework of Clarke et al. Every design
decision references its lineage.


## Design position

psh is an excellent standalone shell first. It must be usable as
a login shell on Linux, macOS, FreeBSD, and other Unix-likes
without any external infrastructure. The theoretical foundations
— sequent calculus structure, duploid polarity, profunctor
redirections, typed values — serve the standalone shell. They
make pipelines compose correctly, catch errors at binding time,
and give the interactive experience richer context for completion
and highlighting. The theory earns its keep by making psh a
better shell, not by enabling a specific platform.


## Foundational commitment: every variable is a list

Every psh variable holds a list. There is no separate "scalar"
type distinct from "list of length 1." This is the uniform
abstraction inherited from rc and reinforced by the virtual
double category framework (see `docs/vdc-framework.md`), in
which sequences are the primitive structure on cell boundaries.

Concrete consequences:

- `let count : Int = 0` is shorthand for `let count : Int = (0)`.
  Both denote a list of one int. `$#count` is 1.
- Type annotations refer to **element types**. `: Int` means
  "list whose elements are Int." Length is runtime data, not
  part of the type.
- Substitution always splices a list. A "scalar" binding splices
  one element; a list binding splices its elements. rc's
  structural substitution discipline, unchanged.
- Tuples, sums, and structs are distinct types at the **element**
  level — they can appear inside the list. `let pos : Tuple = (10,
  20)` holds a list of one tuple. `$#pos` is 1. `${pos .0}` is
  `10`.
- No scalar/list distinction means no `"$var"` quoting ceremony
  is ever needed. Variables always splice structurally.

This is Duff's principle extended across the type system: the
list structure is carried as data, never destroyed and
reconstructed.


## rc's execution model as sequent calculus

Duff's rc [1] departed from the Bourne shell for structural
reasons, not aesthetic ones. The critical moves:

**List-valued variables.** Bourne conflated "list of strings"
with "string containing separators" — every expansion re-scanned
through IFS. rc made lists first-class: `path=(. /bin)` is two
strings, never rescanned. Duff: "Input is never scanned more
than once" [1, §Design Principles]. This was the foundational
move. Everything else follows from treating the shell's data
type honestly.

**Syntax matching the semantics.** Bourne's syntax was
accidental — decades of accretion on the Mashey shell. rc
started fresh with consistent rules: `{` for grouping, `'` for
quoting (not three incompatible mechanisms), `()` for lists. The
syntax made the semantics visible.

**Plan 9 informed rc through the namespace.** `/env` was a
per-process-group directory where variables lived as files.
`fn name` stored the function body in `/env/fn#name`. This
meant `rfork e` gave you a new environment by kernel semantics,
not shell magic. The shell was a client of the namespace, not
its own micro-OS.

rc has the three-sorted structure of the λμμ̃-calculus [5],
unnamed and unenforced:

| rc construct | Sort | Evidence |
|---|---|---|
| Words: literals, `$x`, `` `{cmd} ``, `a^b` | Producers | Eager evaluation. "Input is never scanned more than once" [1, §Design]. |
| Pipe readers, redirect targets, continuations | Consumers | Implicit — waiting to receive a value. |
| Simple commands, pipelines, `if`, `for` | Cuts ⟨t \| e⟩ | `echo hello`: producer `hello` meets consumer stdout. |

The shifts exist in rc but are unnamed:

| rc mechanism | Shift type | Direction |
|---|---|---|
| `` `{cmd} `` command substitution | Force then return (↓→↑) | computation → value |
| `<{cmd}` process substitution | Namespace extension (bind) | computation → name |
| `x=val; rest` | μ̃-binding (let) | bind value, continue |

psh adds one shift that rc did not have:

| psh mechanism | Shift type | Direction |
|---|---|---|
| `$((...))` arithmetic | In-process eval (↓→↑) | computation → value (Int) |

`$((...))` is the same ↓→↑ shift as command substitution, but
the computation is arithmetic evaluated in-process rather than
a subprocess. ksh93/POSIX heritage.

psh makes two shifts explicit that rc left implicit:

1. **Command substitution without IFS.** psh splits on newlines,
   not on an arbitrary `$ifs`. The return operation (bytes → list)
   is fixed. Duff kept `$ifs` only because "indispensable" [1,
   §Design Principles]; psh removes it, closing the last re-scanning hole.

2. **Process substitution as namespace extension.** rc's `<{cmd}`
   returned an fd path while the child ran concurrently. This is
   not a fork (synchronous shift) but a bind — it extends the fd
   namespace with a name pointing to a concurrent computation.
   The name is positive (CBV — it's a string `/dev/fd/N`); the
   computation behind it is negative (CBN, demand-driven). This
   matches Plan 9's mount model: `mount` returns immediately, the
   server behind the mount point is concurrent. Nobody considers
   `mount` a violation of sequential execution. The concurrency
   is behind the name, accessed only when something reads the fd.


## The sfio insight

ksh93's sfio library [SFIO] was the shell's implicit type
theory. The sfio-analysis suite [SFIO-1 through SFIO-12]
revealed:

**Buffer polarity.** sfio's five-pointer buffer system [SFIO-3]
encodes polarity: `_endr` active = read mode (negative,
consuming), `_endw` active = write mode (positive, producing).
Mode switching (`_sfmode()`) is a polarity shift with
reconciliation cost — seek-back for read→write, flush for
write→read. This is a shift operator with a cost, not a free
operation.

**Discipline stacks as morphism chains.** Each `Sfdisc_t` in
the stack [SFIO-7] composes like an endomorphism between the
buffer (value) and the kernel (computation). The stack as a
whole mediates the value/computation boundary.

**Dccache as non-associativity witness.** When a discipline
is pushed onto a stream with buffered data [SFIO-7], the two
possible bracketings yield different results because data
already in value mode (buffered) cannot be re-processed through
a new computation discipline. This is structurally analogous to
the duploid's failed fourth equation (Mangel/Melliès/
Munch-Maccagnoni [2], the non-associative composition of
call-by-value and call-by-name). The pattern matches; the full
duploid composition laws have not been formally verified for
sfio's discipline stack.

**The lesson for psh:** ksh93's authors built correct polarity
discipline in sfio and then failed to propagate it to the shell
proper. The `sh.prefix` bugs (SPEC.md [SPEC] bugs 001–003b)
are exactly the same non-associativity that Dccache handles
correctly — a computation (DEBUG trap) intruding into a value
context (compound assignment) with no mediator. sfio had the
mediator; the shell didn't.

psh makes polarity explicit:

- **Typed fd roles** (`Pipe`, `File`, `Tty`, `Coproc`,
  `Session`) — not sfio's universal `Sfio_t` with runtime mode
  bits. Explicit types over runtime flags.
- **Wrapped redirections** that make evaluation order structural
  — the AST nesting determines the only legal evaluation order.
  No Dccache problem possible because the profunctor composition
  prevents non-associative bracketing by construction.
- **Save/restore as lens roundtrips** — PutGet (restore after
  redirect gives saved state), GetPut (save without redirect is
  no-op), PutPut (consecutive redirects, only last matters).
  This is ksh93's `filemap[]` / `sh.topfd` pattern [SPEC,
  sfio-analysis/10-ksh-integration.md] translated to typed Rust.


## Theoretical framework

### The calculus

**Curien and Herbelin** [5] introduced the λμμ̃-calculus as a
term assignment for classical sequent calculus. Three syntactic
categories: terms (μ-binder captures the current context),
coterms (μ̃-binder captures the current value), and commands
(a cut ⟨t | e⟩ connecting them). This is the foundation.

**Spiwack** [SPW] dissects this into a polarized variant:
positive types (values, introduced eagerly) vs negative types
(computations, introduced lazily). Shift connectives (↓N for
thunking, ↑A for returning) mediate between polarities.

### The semantics

**Mangel, Melliès, and Munch-Maccagnoni** [2] define duploids —
non-associative categories integrating call-by-value (Kleisli/
monadic) and call-by-name (co-Kleisli/comonadic) computation.
Three of four associativity equations hold; the fourth's failure
captures the CBV/CBN distinction. Maps restoring full
associativity are thunkable (pure, value-like). In a dialogue
duploid (with involutive negation), thunkable = central: purity
and commutativity coincide (the Führmann-Thielecke theorem).

**Munch-Maccagnoni's thesis** [3] is where duploids originate.
The companion paper [9] gives the clearest self-contained
definition. Table 1 maps abstract structure to concrete PL
concepts: thunk, return, Kleisli, co-Kleisli, and oblique maps.

### The practice

**Binder, Tzschentke, Müller, and Ostermann** [7] present
λμμ̃ as a compiler intermediate language. Key insights:
evaluation contexts are first-class (the μ̃-binder reifies
"what happens next"); let-bindings (μ̃) are dual to control
operators (μ); ⊕ vs ⅋ error handling are dual.

**Levy** [4] defines Call-by-Push-Value, the practical
framework for the value/computation distinction. psh's
`def`/`let` + lambda split is CBPV's `U`/`F` adjunction
surfaced as syntax.


## The three sorts, made explicit

In Curien-Herbelin's λμμ̃ [5], the three syntactic categories
are:

- **Terms** (producers): values that have been computed or are
  ready to compute. They live on the left of the cut.
- **Coterms** (consumers): contexts that are waiting to receive
  a value. They live on the right of the cut.
- **Commands** (cuts): a term meeting a coterm — ⟨t | e⟩ — the
  moment of interaction where computation happens.

A cut is not a command definition. A cut is the *statement*
that connects a producer to a consumer. `echo hello` is a cut:
the producer `hello` meets the consumer (echo's I/O context —
stdout, the continuation of the script). The `def` keyword
defines a computation; the cut happens when the computation is
invoked with arguments.

### Terms (producers) — Γ

Terms are values: literals, variable references, command
substitution results, lists, lambdas, concatenations. They are
evaluated eagerly (CBV) by `eval_word` before the command that
consumes them runs. Terms inhabit the context Γ.

In psh's AST, terms are the `Word`/`Value` sort.

| psh construct | Term type | Notes |
|---|---|---|
| `hello` | Literal | Positive, inert |
| `$x` | Variable reference | Projects from Γ |
| `` `{cmd} `` | Command substitution | Shift ↓→↑: computation forced, result returned as value |
| `(a b c)` | List | Product of strings |
| `$x^$y` | Concatenation | Kleisli composition of two terms |
| `|x| => body` | Lambda | Thunked computation as value (`U` in CBPV) |
| `ok(42)` | Sum (injection) | Tagged value — coproduct introduction |

### Coterms (consumers) — Δ

Coterms are contexts waiting to receive a value. They are
the part of the computation that hasn't happened yet — what
comes next after a value is produced. In rc, coterms were
entirely implicit. psh names them.

| psh construct | Coterm type | Notes |
|---|---|---|
| Pipe reader (`stdin` of next stage) | Continuation | Waiting for bytes from the producer |
| Redirect target (`>file`) | I/O context | Waiting for output to direct somewhere |
| The rest of the script after `x = val` | Continuation (μ̃) | `x = val; rest` — `rest` is the coterm |
| Signal handler in `trap` | Named continuation (μ) | Waiting for a signal to fire |
| `catch e { handler }` | Error continuation | Waiting for a nonzero status |

Coterms populate Δ. In the classical sequent Γ ⊢ A | Δ,
Δ contains the continuations — alternative futures that the
computation might jump to. In psh, Δ is populated by:

- **trap bindings**: `trap SIGINT { handler } { body }` binds
  the handler as a continuation in Δ for the duration of the
  body. The μ-binder `μα.c` in the calculus [5] — α names the
  signal continuation, c is the body that runs with α in scope.
- **catch bindings**: `try { body } catch e { handler }` binds
  the error handler as a continuation in Δ for the duration of
  the try body. Semantically similar but triggered by status
  rather than signal.

The evaluator function `run_expr` handles the coterm sort:
pipelines (co-Kleisli — demand-driven) and redirections
(profunctor transformations on the I/O context).

### Commands (cuts) — ⟨t | e⟩

A command is a cut: a term meets a coterm and computation
happens. The statement `echo $x` is the cut ⟨$x | echo-context⟩
where the producer ($x, evaluated) meets the consumer (echo's
stdout binding + the script continuation).

| psh construct | Cut structure | Notes |
|---|---|---|
| `echo hello` | ⟨hello \| stdout + continuation⟩ | Simple command |
| `cmd1 \| cmd2` | ⟨cmd1-stdout \| cmd2-stdin⟩ | Pipeline: cut via pipe |
| `x = val` | ⟨val \| μ̃x.rest⟩ | Assignment: value cut against a binder |
| `if(cond) { A } else { B }` | ⟨status \| case(A, B)⟩ | Coproduct elimination |
| `match(v) { arms }` | ⟨v \| case(arm₁, ..., armₙ)⟩ | Multi-way elimination |

In psh's AST, the `Binding` sort handles μ̃-binders (context
extension: assignment, let, def, ref) and the `Command` sort
handles cuts and control flow (exec, if, for, match, try, trap).

### The AST's four sorts

The AST has four node types — the three logical sorts plus the
profunctor layer:

| psh sort | λμμ̃ analog | Evaluation | Examples |
|---|---|---|---|
| `Word`/`Value` | Term (producer) | CBV — evaluated eagerly | Literal, Var, CommandSub, Concat, List, Tuple, Sum |
| `Expr` | Profunctor layer | CBN for pipelines, structural for redirections | Pipeline, Redirect, Background |
| `Binding` | μ̃-binder | Extends context Γ | Assignment, Cmd, Let |
| `Command` | Cut / control | Connects terms to coterms, or branches | Exec, If, For, Match, Try, Trap |

The `Expr` sort is an engineering choice, not a logical one —
it separates the profunctor transformations (redirections,
pipelines) from the cut/control layer. Logically, `Expr`
constructs are part of the coterm apparatus: pipelines build
co-Kleisli contexts, redirections transform I/O contexts via
profunctor maps. The evaluator boundary `run_expr` enforces
this: it handles the coterm machinery before `run_cmd` performs
the cut.


## Polarity discipline

### CBV/CBN split

The CBV/CBN split follows the duploid's two subcategories [2,
§2.1]. Word expansion is Kleisli composition: each stage
(`$x` lookup, concatenation, command substitution) takes a
partial value and produces an expanded value with possible
effects. `eval_word` recurses through `Word` nodes before the
command that consumes them has started.

Pipeline execution is co-Kleisli: `run_pipeline` forks all
stages concurrently, and data flows on demand through `pipe(2)`
endpoints. `yes | head -1` does not evaluate `yes` to
completion. The pipe's blocking read is the demand.

Cross-polarity composition — a pipeline stage that expands a
variable (CBV) and writes to a pipe (CBN) — is non-associative
in the duploid sense. psh's sequential evaluation within each
process prevents both bracketings from being simultaneously
available. Word expansion completes before `execvp` runs; the
fork boundary separates the two polarities. This is operational
focalization — the same deterministic reduction order that
Curien and Munch-Maccagnoni's focused calculus [8] achieves
syntactically, psh achieves operationally.

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

2. **Process substitution** (`<{cmd}`): bind a name to a
   concurrent computation. Namespace extension. The name
   `/dev/fd/N` is positive (a string); the computation behind
   it is negative (demand-driven, reads trigger it). This is
   Plan 9's mount model — synchronous bind, concurrent server.
   Focalization is not violated because the bind itself is
   instantaneous; the concurrency is behind the name.

3. **Pipeline** (`|`): concurrent cut. Co-Kleisli composition.
   Each `|` creates a pipe — a linear resource pair — connecting
   stdout-left to stdin-right. Both sides run concurrently.
   Demand flows right-to-left via blocking reads.


## Syntax

The formal grammar and all syntactic decisions are in
`docs/syntax.md`. This section summarizes the design rationale
that connects syntax to semantics.

rc's actual syntax is the baseline. Every convention from rc
is preserved unless explicitly departed from with justification.
The formal grammar in syntax.md annotates each production with
its rc heritage or extension rationale.

Key syntactic decisions with semantic grounding:

- **`def` instead of `fn`** for command definitions. rc's `fn`
  was a misnomer — it defines a cut, not a function. `def`
  names the sort honestly. See §Two kinds of callable.
- **`let` + lambda for functions.** Values in the value sort,
  first-class, with capture semantics. Lambdas use `|x| => expr`
  or `|x| { block }`. See §Two kinds of callable.
- **rc parentheses** around conditions: `if(cond)`,
  `while(cond)`, `for(x in list)`, `match(expr)`.
- **`else` instead of `if not`.** Duff acknowledged rc's
  weakness here [1, §Design Principles].
- **`match`/`=>` instead of `switch`/`case`.** rc's `case` arms
  are top-level commands in a list; psh's `match` uses structured
  `=>` arms with `;` separators. The operation is genuinely
  different. `match` names the operation honestly.
- **`try`/`catch`** — scoped ErrorT. See §Error model.
- **`trap SIGNAL { handler } { body }`** — lexical μ-binder.
  See §Error model.


## Two kinds of callable

ksh93's compound variables [SPEC, §Compound variables] were its
struct system, never named as such. `typeset -C` created
name-value trees; `${x.field}` accessed them; disciplines
mediated access. psh's `def`/lambda distinction is informed by
this: ksh93 needed both effectful procedures (functions) and
inert data accessors (compound variable fields), but conflated
them in the `Namval_t` machinery.

| | `def` | `let` + lambda (`|x|`) |
|---|---|---|
| Sort | Command (cut template) | Value (term) |
| Arguments | Variadic, positional ($1, $2, $*) | Fixed arity, named |
| First-class | No — named computation in Θ | Yes — value in Γ, storable |
| Scope | Dynamic (reads current scope) | Captures at definition |
| Effects | May have effects (oblique map) | Purity inferred (thunkable when pure) |
| CBPV type | `F(Status)` | `U(A → B)` or `U(A → F(B))` |
| rc analog | `fn name { body }` [1, §Functions] | (no rc analog — extension) |
| Invocation | `name arg1 arg2` | `name arg1 arg2` (bare word forces the lambda) |

The `def` keyword replaces rc's `fn`. Duff chose `fn`
deliberately, but psh renames it because psh draws a distinction
between named computations and first-class functions that rc
did not make. `def` is neutral — it defines a named computation
without claiming its role in a cut, which only happens at the
invocation site.


## Discipline functions

A variable with `.get` and `.set` disciplines is **codata** in
the sense of the sequent calculus: its behavior under observation
(reading) and mutation (assignment) is defined by destructor and
constructor cells, not by a naive stored slot. The discipline
cells *are* the variable's semantics.

### The codata model

In the sequent calculus, data types are defined by constructors
(how to build a value) and eliminated by pattern matching. Codata
types are defined by destructors (how to observe or transform a
value) and eliminated by copattern matching — the producer must
respond to each destructor invocation.

A variable with discipline functions is codata:

- **`.get`** is the destructor that fires on observation. Reading
  the variable invokes `.get`, which computes the value seen by
  the accessor.
- **`.set`** is the constructor that fires on mutation. Assigning
  to the variable invokes `.set`, which mediates how the new
  value is stored (or rejected, transformed, or propagated).

A variable without discipline functions is ordinary data: the
stored value is what you read, assignment replaces the stored
value. Adding disciplines moves the variable into the codata
world where its semantics become whatever the disciplines
compute.

### .get — the codata observer

`.get` disciplines are defined as `def` cells. The body computes
the value seen by the accessor:

    let mut cursor = 0
    def cursor.get {
        cursor = `{ cat /srv/input/cursor }
    }

Every `$cursor` access fires the discipline, which refreshes the
stored slot from an external source. The access then returns the
refreshed value. This is the "live variable" pattern: the
variable's value is computed on demand.

The body may have arbitrary effects: logging, tracing, metrics,
coprocess queries, filesystem reads. The polarity frame
discipline (see §Polarity discipline) protects the surrounding
expansion context from the computation-mode intrusion. A `.get`
body may issue coprocess queries; the shift structure is the
same ↓→↑ pattern as command substitution, and the polarity frame
is sufficient to make this safe.

### CBV focusing as the reentrancy semantics

Within a single expression's evaluation, `.get` fires at most
once per variable. Subsequent occurrences of the same variable
in the same expression use the already-produced value.

This is not memoization as an optimization. It is the correct
focusing behavior of the focused sequent calculus: in CBV, a
producer is evaluated (focused) to a value once, and the
resulting value is used at each consumption site. The `.get`
discipline is a shift from computation to value (↓→↑); once the
shift lands, the result is a value, and values in CBV are used
without re-evaluation.

Concretely: in the expression `echo $cursor $cursor`, the
discipline fires on the first `$cursor`, produces a value, and
the second `$cursor` uses that same value. The expression sees
one consistent reading.

Across expressions, the discipline fires again. A second `echo
$cursor` on a new line will run `.get` fresh and may see
different state. Cross-expression consistency is not guaranteed
— the backing state can change, and that is the expected
behavior of codata backed by external resources.

### .set — the codata constructor

`.set` disciplines are defined as `def` cells. The body receives
the incoming value as `$1` and mediates the assignment:

    def x.set {
        # $1 is the new value being assigned
        # the body may validate, transform, reject, or propagate
    }

`.set` fires on every assignment to `x`. Typical patterns:

- **Validation.** Reject assignments that don't meet a
  constraint, by calling `return` with a nonzero status.
- **Transformation.** Normalize or clamp the value before storing
  (e.g., clamp a percentage to 0-100).
- **Propagation.** Write the value to an external resource
  (coprocess, filesystem) as a side effect of the assignment.
- **Notification.** Log the change, emit metrics, trigger
  dependent updates.

CBV focusing applies symmetrically: within one assignment
expression, `.set` fires once. The incoming value is focused to
a positive form, the discipline runs, the assignment completes.

### Reentrancy and the polarity frame

Because discipline bodies can issue the same operation they are
mediating (a `.get` that reads other variables, a `.set` that
triggers further assignments), reentrancy is a real concern.
Each discipline invocation runs inside a polarity frame that
prevents the discipline from firing recursively on the variable
it is attached to. Within the body of `x.get`, a reference to
`$x` returns the current stored value directly, bypassing the
discipline. Similarly, within `x.set`, an assignment to `x`
writes to the stored slot directly.

This is the same polarity frame mechanism that protects the
surrounding expansion context from computation-mode intrusions.
The frame saves context, runs the discipline, restores context
on exit. Reentrancy within the frame is resolved by a flag on
the variable's discipline state.

### MonadicLens structure

A variable with `.get` and `.set` disciplines is a MonadicLens
[Clarke, def:monadiclens]:

    MndLens_Ψ((A,B),(S,T)) = W(S, ΨA) × W(S × B, ΨT)

Under the codata model, both view and update live in Kl(Ψ) —
the shell's effect monad. The view is the `.get` computation
(which may have effects); the update is the `.set` computation
(same). This is a proper monadic lens, not a mixed optic.

The MonadicLens laws hold modulo the effects:

- **PutGet:** assigning a value and then reading it returns the
  assigned value, *if* `.set` stores it faithfully and `.get`
  reads the stored slot. A `.set` that transforms or a `.get`
  that recomputes may break PutGet — this is the price of
  codata.
- **GetPut:** reading and then assigning back is a no-op, *if*
  the disciplines are inverse to each other.
- **PutPut:** the second assignment overrides the first, *if*
  `.set` is idempotent under repeated assignment.

For ordinary variables without discipline functions, the view is
identity in W (trivially pure) and the laws hold unconditionally.
Adding disciplines moves the variable into Kl(Ψ), where the laws
become contracts the user must maintain, not automatic
consequences.

### Known caveat: cross-variable consistency

A `.get` discipline that queries mutable external state may
produce inconsistent reads across expressions. If `.get` on X
queries a coprocess whose response depends on state modified by
a concurrent process, two expressions involving `$x` may see
different underlying stored values. Within one expression, CBV
focusing gives consistency (the value is computed once and
reused). Across expressions, the discipline fires fresh each
time, and the state it observes may have changed.

This is documented behavior, not a bug. The codata model makes
the value's computation explicit — if that computation depends
on mutable external state, its results depend on when it runs.
Users who need strict cross-expression consistency should either
avoid discipline-backed variables for the relevant reads or cache
values into discipline-free variables.


## Profunctor structure

### Redirections as profunctor maps

A traditional shell AST bolts redirections onto commands as a
flat list. This representation is silent about evaluation
order. psh encodes redirections as wrapping:

    Redirect(
        Redirect(
            Simple(cmd),
            Output { fd: 1, target: File("file") }
        ),
        Dup { dst: 2, src: 1 }
    )

The profunctor structure:

- `Output` = rmap (post-compose on output continuation)
- `Input` = lmap (pre-compose on input source)

Dup and Close are structural rules on the fd context, not
profunctor maps:

- `Dup` = contraction (two fds alias one resource)
- `Close` = weakening (discard a resource)

Duff: "Redirections are evaluated from left to right" [1,
§Advanced I/O Redirection]. The wrapped representation makes
the only legal evaluation order the correct one. The profunctor
laws hold by construction.

This is the minimal system — two genuine optics survive in the
rc-compatible base:

1. **Redirections** — Adapter (Profunctor constraint only)
2. **fd save/restore** — Lens (Cartesian constraint)

Word expansion has Kleisli structure — each stage is a function
`Word → Val` with possible effects, composing sequentially.
This is a composition pattern in the shell's effect monad,
not an optic. It provides the
view morphism that the discipline system's MonadicLens uses.

The full optic hierarchy (Prism, AffineTraversal, Traversal)
activates when products and coproducts are added.

### Word expansion as Kleisli pipeline

ksh93's `macro.c` expansion pipeline (tilde → parameter →
command sub → arithmetic → field split → glob) is Kleisli
composition [SPEC, §"The monadic side"]. psh's `eval_word` has
a simpler pipeline:

1. **Literal** → identity (pure, no effects)
2. **Var** → codata access: if the variable has a `.get`
   discipline, invoke it (polarity shift, runs in a polarity
   frame, result memoized within the expression by CBV
   focusing); otherwise read the stored value directly
3. **Count** → lookup then measure
4. **CommandSub** → polarity shift (↓→↑: fork, capture, return)
5. **Concat** → rc's `^` (pairwise or broadcast join)

Each stage is a function `Word → Val` with possible effects.
They compose by structural recursion over the `Word` AST.


## Coprocesses (9P-shaped discipline)

### Design lineage

ksh93 introduced coprocesses (`cmd |&`) — bidirectional channels
between the shell and a child process. These were untyped byte
streams with no protocol discipline. Bash extended them with
named coprocesses. Neither had a conversation discipline.

Plan 9's 9P protocol [9P] is the design inspiration: a
session protocol imposed on a byte stream. The sequence
Tversion/Rversion, Tattach/Rattach, Twalk, Topen, Tread/Twrite,
Tclunk is a state machine — what Honda [Honda98] would later
formalize as session types. 9P session-typed its IPC informally,
enforced by runtime checks rather than compile-time types.

psh extracts 9P's conversation shape (not its wire protocol):

1. **Negotiate** — one round-trip confirming both sides speak
   the same protocol. For same-binary coprocesses this is a
   trivial handshake ("psh protocol v1"). The negotiate step
   exists so that the protocol is self-describing from the
   first byte — no out-of-band assumptions about the peer.
2. **Request-response pairs** — every request gets a response.
   No fire-and-forget. No ambiguity about whose turn it is.
3. **Error at any step** — failure is always a valid response,
   not a special case.
4. **Orderly teardown** — explicit close, not just EOF/SIGPIPE.
   EOF is the fallback for crashes; explicit close is for
   graceful shutdown with a reason.

### Per-tag binary sessions

Tags multiplex independent binary sessions over one channel.
Each tag has the session type `Send<Req, Recv<Resp, End>>` —
exactly one legal action at each step. The tag is a session
identifier, not a reason to abandon session discipline.

This mirrors 9P's multiplexing: tags are transaction
identifiers (one per outstanding request, like 9P's uint16
tags), and each tag identifies an independent request-response
exchange. In 9P, fids are the stateful session-like entities
(with lifecycles: walk → open → read/write → clunk); tags
correlate requests to responses across concurrent fids. psh's
coprocess tags serve the same correlation role.

The tag space is uint16 (65535). The practical limit comes
from backpressure (socketpair buffer full = sender blocks),
not from an artificial constant. Design for the ceiling,
operate at the floor.

### The user-visible protocol

`print -p name 'request'` sends a request to the named coprocess
and returns an `Int` tag identifying the outstanding request.
`let` binds the tag directly, per the CBPV rule that `let`
accepts effectful computations:

    let tag = print -p myserver 'query'
    # tag is an Int — a list of one element, $#tag is 1

`read -p name reply` reads the oldest outstanding response
(FIFO order) into `reply`. `read -p name -t $tag reply` reads
the response for a specific tag. The user holds plain Int tags
— there is no linear handle type at the shell level.

Simple FIFO pattern (no tag capture):

    print -p myserver 'query1'
    print -p myserver 'query2'
    read -p myserver reply1     # response to query1
    read -p myserver reply2     # response to query2

Pipelined pattern with out-of-order reads:

    let t1 = print -p db 'slow_query'
    let t2 = print -p db 'fast_query'
    read -p db -t $t2 fast      # read fast response first
    read -p db -t $t1 slow      # then the slow one

Error responses (the coprocess returns an error frame) produce
a nonzero status on `read -p`, with the error message bound to
the reply variable. Standard ⊕ error handling applies: check
status, use `try`/`catch`, etc.

### Shell-internal tracking

The shell maintains, per coprocess, a set of outstanding tags
(tags that have been sent but not yet read). `print -p`
allocates the lowest available tag, records it as outstanding,
and returns it. `read -p` (without `-t`) pops the oldest
outstanding tag when its response arrives. `read -p -t N`
removes tag N specifically when its response is read. Stale or
invalid tags produce a nonzero status with a descriptive error.

Internally, the shell tracks each outstanding tag with an
affine obligation handle. When a handle is dropped without
being consumed (the tag's response is never read), the shell
sends a cancel frame (Tflush equivalent) on the channel,
telling the coprocess to discard any pending work for that
tag. This prevents stale responses from being delivered after
the tag has been reused. The handle discipline is implementation
detail — users see only the tag integers.

### Implementation

~40 lines of phantom session types:

    trait Session: Send + 'static {
        type Dual: Session<Dual = Self>;
    }
    impl Session for () { type Dual = (); }
    struct Send<T, S: Session = ()>(PhantomData<(T, S)>);
    struct Recv<T, S: Session = ()>(PhantomData<(T, S)>);
    // HasDual derived from Session::Dual

No par dependency. The session types live in the Rust
implementation's type signatures — verified by the compiler
when the builtins are written.

### Wire format

Length-prefixed frames (the 9P approach [9P]):

    frame = length[4 bytes, LE u32] tag[2 bytes, LE u16] payload[length - 2 bytes]
    error = length[4 bytes, LE u32] tag[2 bytes, LE u16] '!' error_message

Length-prefixed rather than newline-delimited because payloads
may contain newlines (multi-line strings, command output,
heredocs). The tag is binary u16 for efficiency; the payload
is text (Display/FromStr).

### Named coprocesses

Coprocesses are named. The shell holds a `HashMap<String,
Coproc>` — each coprocess has a name, its own socketpair, its
own independent tag space, and its own binary sessions.

    server |& myserver           # start named coprocess
    print -p myserver 'query'    # write to myserver
    read -p myserver reply       # read from myserver

    worker |& bg                 # another coprocess
    print -p bg 'task'           # independent channel

Anonymous `cmd |&` (no name) targets a default coprocess.
`print -p` / `read -p` without a name target the default.
This preserves ksh93 compatibility for simple cases while
enabling multiple simultaneous coprocesses.

**Lifecycle.** Named coprocesses are reaped on scope exit
(subshell close, function return) or explicit close. A dead
coprocess's name becomes available again — no zombie entries.
Rust's `Drop` on `Coproc` handles cleanup.

**Topology.** The shell is the hub. No coprocess-to-coprocess
communication — star topology. Each coprocess talks only to
the shell. Deadlock freedom by asymmetric initiator/responder
discipline (shell always initiates, coprocess always responds).
N independent binary sessions, same topology class as one.

## Namespace (three tiers)

| Tier | Resolution | Structural rules |
|---|---|---|
| Shell variables | `$x` — scope chain lookup | Weakening, contraction, exchange (classical) |
| Process environment | `env.PATH` — flat key-value | Weakening, contraction, exchange (classical) |
| Filesystem namespace | `/srv/window/cursor` — read from filesystem | Weakening, exchange. **No contraction** — each read is a fresh operation. |

The first two tiers admit all three structural rules (classical
contexts). The filesystem tier restricts contraction — reading
a file twice may yield different results if the underlying state
changed. This is honest: the shell does not guarantee coherence
for filesystem reads.

`get`/`set` builtins resolve against all three tiers uniformly.
The namespace grows; the language does not. This is Plan 9's
principle: `/env` was a filesystem [1, §Environment]; psh
extends the scope chain into the filesystem honestly.


## Error model

### ⊕ and ⅋

Every operation returns `Status(pub String)`. rc: "On Plan 9
status is a character string describing an error condition. On
normal termination it is empty" [1, §Exit status]. psh preserves
this. `Status::is_success()` checks emptiness.

The ⊕/⅋ duality: `$status` is ⊕ (positive — caller inspects a
tagged value). Traps are ⅋ (negative — callee invokes a
continuation). Both are present.

### try/catch — scoped ErrorT (⊕ discipline)

`try { body } catch e { handler }` changes the sequencing
combinator within `body` from unconditional `;` to monadic `;ₜ`
that checks Status after each command. On nonzero status,
execution aborts to the handler. The handler binding `e` is a
μ̃-binder on the error case.

Equivalent to lexically-scoped `set -e` without POSIX `set -e`'s
composability defects. Boolean contexts (if/while conditions,
&&/|| LHS, `!` commands) are exempt.

### trap — lexical μ-binder (⅋ discipline)

`trap SIGNAL { handler } { body }` installs a signal handler
for the duration of the body. The handler is the μ-binder of
Curien-Herbelin [5, §2.1] — it captures the continuation and
names it. Lexically scoped: inner shadows outer, uninstalled
on body exit.


## Tuples (products, ×)

Tuples are products — fixed-size heterogeneous containers.
They are the first connective psh adds beyond rc's list-of-
strings base type.

**Syntax.** Comma-separated values in parentheses.
Space-separated values in parentheses remain lists (rc
heritage). The comma is the disambiguator.

    (a b c)         # list — rc heritage, space-separated
    (10, 20)        # tuple — comma-separated
    ('lane', '/home/lane', 1000)

**Typing rule** (product introduction, classical):

    Γ ⊢ t₁ : A₁ | Δ    Γ ⊢ t₂ : A₂ | Δ
    ─────────────────────────────────────────
    Γ ⊢ (t₁, t₂) : A₁ × A₂ | Δ

**Accessor syntax** (product elimination — Lens projection):

    let pos = (10, 20)
    echo ${pos.0}          # 10
    echo ${pos.1}          # 20

    let record = ('lane', '/home/lane', 1000)
    echo ${record.0}       # lane
    echo ${record.2}       # 1000

Accessors `.0`, `.1`, `.2` etc. are Lens projections — the
`first`, `second` etc. of the Cartesian profunctor class.
Braces are required for accessor syntax, matching ksh93's
`${x.field}` convention. Without braces, `.` is always a free
caret boundary: `$x.c` = `$x ^ .c` (rc heritage). The brace
is the opt-in for structured access.

Composition: `${nested.0.1}` = `first . second` — ordinary
function composition of profunctor optics.

Tuples are positive (value sort), admit all structural rules
(weakening, contraction, exchange). They are inert data —
Clone, no embedded effects.

**ksh93 lineage.** ksh93's compound variables (`typeset -C`)
were its struct system. `${x.field}` accessed named fields;
disciplines mediated access. psh's tuples with positional
accessors are the typed version — same functionality, explicit
in the type system rather than implicit in the `Namval_t`
machinery.


## Sums (coproducts, +)

Sums are coproducts — tagged values representing alternatives.
They are the second connective psh adds beyond rc's base types.

**Syntax.** `tag(payload)` constructs a tagged value. `tag` is
a bare word, immediately followed by `(` (no space). The
payload is a value inside the parens.

    let result = ok(42)
    let e = err('not found')
    let opt = some('/tmp/file')
    let empty = none()

In command position, `ok 42` (with space) is a command named
`ok` with argument `42` — not sum construction. The `NAME(`
token (no space) commits the parser to sum construction.

**Typing rule** (coproduct introduction, classical):

    Γ ⊢ t : Aᵢ | Δ
    ──────────────────────
    Γ ⊢ injᵢ(t) : A₁ + A₂ | Δ

**Elimination** via `match` with structural arms:

    match($result) {
        ok(val)  => echo 'got '$val;
        err(msg) => echo 'failed: '$msg
    }

Structural arms use `tag(binding) =>` — the same parens syntax
as construction. The binding is a μ̃-binder scoped to the arm
body. The variable does not escape the arm.

**Accessor syntax** (coproduct elimination — Prism preview):

    echo ${result.ok}       # Prism preview: 42 (or empty if err)
    echo ${result.err}      # Prism preview: 'not found' (or empty if ok)

`${x.tag}` is a Prism preview — partial projection that
succeeds if the tag matches, fails (returns empty) otherwise.
Profunctor constraint: Cocartesian. Composition across products
and coproducts yields AffineTraversal (Cartesian +
Cocartesian): `${result.ok.0}` = Prism then Lens.

Sums are positive (value sort), admit all structural rules.
They are inert data — Clone, no embedded effects.


## Extension path

Extensions add connectives to the μμ̃ framework, not new
sorts. The sorts remain producers/consumers/commands.

### Polymorphism

Parametric type abbreviations — syntax and semantics undecided.

### Optics activation

| Type | Optic | Profunctor constraint |
|---|---|---|
| Lists (rc base) | Traversal (iteration) | Monoidal |
| Tuples (products) | Lens (projection) | Cartesian |
| Sums (coproducts) | Prism (preview) | Cocartesian |
| Products × Coproducts | AffineTraversal | Cartesian + Cocartesian |
| fd table (save/restore) | Lens | Cartesian |
| Redirections | Adapter | Profunctor |

The accessor syntax `${x.N}` (tuples) and `${x.tag}` (sums)
is stable. Braces are always required. What changes is whether
the accessor is a Lens (product), Prism (coproduct), or
AffineTraversal (mixed), determined by the type at the access
point.


## References

[1] Tom Duff. "Rc — The Plan 9 Shell." 1990.
    `reference/plan9/papers/rc.ms`

[2] Mangel, Melliès, Munch-Maccagnoni. "Duploids."
    `~/gist/classical-notions-of-computation-duploids.gist.txt`

[3] Munch-Maccagnoni. "Syntax and Models of a Non-Associative
    Composition of Programs and Proofs." Thesis, 2013.

[4] Levy. *Call-by-Push-Value.* Springer, 2004.

[5] Curien, Herbelin. "The Duality of Computation." ICFP, 2000.

[6] Wadler. "Call-by-Value is Dual to Call-by-Name." ICFP, 2003.

[7] Binder, Tzschentke, Müller, Ostermann. "Grokking the
    Sequent Calculus." 2023.
    `~/gist/grokking-the-sequent-calculus.gist.txt`

[8] Curien, Munch-Maccagnoni. "The Duality of Computation
    Under Focus." TCS, 2010.

[9] Munch-Maccagnoni. "Models of a Non-Associative Composition."
    FoSSaCS, 2014.

[9P] Plan 9 9P protocol. `reference/plan9/man/5/`

[Honda98] Honda, Vasconcelos, Kubo. "Language Primitives and
    Type Discipline for Structured Communication-Based
    Programming." ESOP, 1998.

[Clarke] Clarke, Boisseau, Gibbons. "Profunctor Optics, a
    Categorical Update." Compositionality, 2024.
    `~/gist/DontFearTheProfunctorOptics/`

[SPW] Spiwack. "A Dissection of L." 2014.
    `~/gist/dissection-of-l.gist.txt`

[Be] Haiku / BeOS. Application Kit, Interface Kit, I/O hierarchy.
    `reference/haiku-book/`

[SPEC] ksh26 Theoretical Foundation. `~/src/ksh/ksh/SPEC.md`

[SFIO] sfio Operational Semantics Reference.
    `~/src/ksh/ksh/notes/sfio-analysis/README.md`

[SFIO-3] sfio Buffer Model.
    `~/src/ksh/ksh/notes/sfio-analysis/03-buffer-model.md`

[SFIO-7] sfio Disciplines.
    `~/src/ksh/ksh/notes/sfio-analysis/07-disciplines.md`
