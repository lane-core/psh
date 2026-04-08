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
`def`/`let`+`\` split is CBPV's `U`/`F` adjunction surfaced
as syntax.


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
| `\(x) => body` | Lambda | Thunked computation as value (`U` in CBPV) |

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
| `Word`/`Value` | Term (producer) | CBV — evaluated eagerly | Literal, Var, CommandSub, Concat, List |
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
- **`let` + `\` for functions.** Values in the value sort,
  first-class, with capture semantics. See §Two kinds of
  callable.
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

| | `def` | `let` + `\` (lambda) |
|---|---|---|
| Sort | Command (cut template) | Value (term) |
| Arguments | Variadic, positional ($1, $2, $*) | Fixed arity, named |
| First-class | No — named computation in Θ | Yes — value in Γ, storable |
| Scope | Dynamic (reads current scope) | Captures at definition |
| Effects | May have effects (oblique map) | Purity inferred (thunkable when pure) |
| CBPV type | `F(Status)` | `U(A → B)` or `U(A → F(B))` |
| rc analog | `fn name { body }` [1, §Functions] | (no rc analog — extension) |
| Invocation | `name arg1 arg2` | `$f(arg1, arg2)` |

The `def` keyword replaces rc's `fn`. Duff chose `fn`
deliberately, but psh renames it because psh draws a distinction
between named computations and first-class functions that rc
did not make. `def` is neutral — it defines a named computation
without claiming its role in a cut, which only happens at the
invocation site.


## Discipline functions

### .get — pure notification hook

`def x.get` is not legal. `.get` disciplines are pure — they
cannot perform I/O, cannot mutate the variable they observe,
cannot call external commands.

A `.get` discipline is defined as a lambda:

    let x.get = \() => { ... pure computation ... }

The body fires on every `$x` access as a pure notification —
side-effect-free observation (logging, tracing). The returned
value is always the stored value, not the body's output. The
body's return value is discarded.

Because `.get` does not compute the value (it merely observes
access), the variable read itself is the Getter: `view(s) = s`,
identity on the stored slot. `.get` is a hook attached to
the Getter, not the Getter itself. The purity constraint
ensures the hook cannot disturb the Getter's idempotence.

**Why pure .get is enforced.** An impure `.get` that queries
external resources creates the Dccache bug class at the
discipline level [SFIO-7]: if `.get` on variable X queries
coprocess C1, whose handler queries C2, which modifies state
that X's next `.get` sees, two reads of `$x` in a single
expression yield different values. Enforcing purity on `.get`
eliminates this by construction — the polarity boundary prevents
circular dependencies from forming.

The optics argument: the variable read is a Getter (S → A,
idempotent, composable, cacheable) in the base category W. An
impure `.get` hook would degrade the read to a Kleisli arrow
S → ΨA, which loses idempotence and breaks compositional
soundness across the discipline system.

**Live variable refresh** happens through a separate mechanism:
an explicit `def` that writes to the variable's stored slot.

    let mut cursor = 0
    def cursor.refresh { cursor = `{ cat /srv/window/cursor } }

The refresh is a command (computation sort, effectful). The
`.get` is pure (value sort, Getter). Clean polarity separation.

### .set — effectful mutation (Kleisli)

    def x.set { ... }
    def x.set(val) { ... }

`.set` fires on assignment to `x`, with `$1` bound to the new
value. The body may have effects. Reentrancy guard prevents
infinite recursion. ksh93 heritage [SPEC, §Discipline functions].

### MonadicLens structure

A variable with `.get` and `.set` disciplines is a MonadicLens
[Clarke, def:monadiclens]:

    MndLens_Ψ((A,B),(S,T)) = W(S,A) × W(S×B, ΨT)

The view (`.get`) is a morphism in the base category W — pure.
The update (`.set`) lives in Kl(Ψ) — effectful. This is a
mixed optic: Optic_{×, ⋊} where the right action ⋊ threads
through the shell's effect monad.

MonadicLens laws hold for tiers 1-2 (local variables,
environment) where the store is process-local and stable. For
remote resources accessed through namespace paths, PutGet
degrades — the lens laws become an affine contract.


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
2. **Var** → discipline-checked lookup (fire `.get`, read value)
3. **Count** → lookup then measure
4. **CommandSub** → polarity shift (↓→↑: fork, capture, return)
5. **Concat** → rc's `^` (pairwise or broadcast join)

Each stage is a function `Word → Val` with possible effects.
They compose by structural recursion over the `Word` enum.


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
   the same protocol. Degenerate for same-binary coprocesses
   ("psh protocol v1"). Extensible for cross-machine (future).
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

### PendingReply handles

`print -p` returns a `PendingReply` — a `#[must_use]` handle.
`read -p` consumes a `PendingReply` to get the response.
Dropping a `PendingReply` without reading sends a cancel (the
Tflush equivalent — affine gap compensation). The tag cannot
be reused until the handle is consumed or dropped.

PendingReply is affine in Rust's type system (Drop exists) but
linear by intent (must be consumed). Drop-as-cancel is the
compensation that bridges the gap. This is session type
discipline materialized as a Rust value. The user never sees
session types, protocol annotations, or state machines. They
see `print -p` returning something they must eventually
`read -p`.

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

### Star topology

Multiple named coprocesses (future: `HashMap<String, Coproc>`
instead of `Option<Coproc>`) each have independent tag spaces
and independent binary sessions. The shell is the hub. No
coprocess-to-coprocess communication — star topology. Deadlock
freedom by asymmetric initiator/responder topology (shell
always initiates, coprocess always responds).

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


## Extension path

Each extension adds connectives to the μμ̃ framework, not new
sorts. The sorts remain producers/consumers/commands.

### Tuples (products, ×)

Introduction: `(a, b, c)` constructs a product.
Elimination: `$t.0`, `$t.1` (projection — Lens).

    Γ ⊢ t₁ : A₁ | Δ    Γ ⊢ t₂ : A₂ | Δ
    ─────────────────────────────────────────
    Γ ⊢ (t₁, t₂) : A₁ × A₂ | Δ

Profunctor constraint: Cartesian. Accessor `$t.0` is `first`
from the Cartesian class. Composition: `$nested.0.1` = `first
. second`.

### Sums (coproducts, +)

Introduction: `tag payload` constructs a tagged value.
Elimination: `match` with structural arms.

    Γ ⊢ t : Aᵢ | Δ
    ──────────────────────
    Γ ⊢ injᵢ(t) : A₁ + A₂ | Δ

Profunctor constraint: Cocartesian. Accessor `$result.ok` is
a Prism preview. Composition across products and coproducts
yields AffineTraversal (Cartesian + Cocartesian).

### Prenex polymorphism

Type abbreviations with parameters:
`Result[T] = ok T | err ExitCode`. Not System F — no lambda
over types, no impredicativity. Decidable, no inference
complications.

### Incremental optics activation

| Phase | Types added | Optics gained | Profunctor constraint |
|---|---|---|---|
| Base (rc) | List of strings | Adapter (redirections), Lens (fd table) | Profunctor (Adapter), Cartesian (Lens) |
| +Tuples | Products | Lens on user data | Cartesian |
| +Sums | Coproducts | Prism | Cocartesian |
| +Both | Products × Coproducts | AffineTraversal | Cartesian + Cocartesian |
| +Traversal | List-aware iteration | Traversal | + Monoidal |

No rethinking at each stage. Each phase adds a constraint
class; composition across phases falls out from the type-class
union. The accessor syntax `$x.field` is stable — what changes
is whether the accessor is a Lens (product), Prism (coproduct),
or AffineTraversal (mixed).


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
