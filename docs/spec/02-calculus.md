# Calculus and framework

## Theoretical framework

### The calculus

**Curien and Herbelin** [CH00] introduced the λμμ̃-calculus as a
term assignment for classical sequent calculus. Three syntactic
categories: terms (μ-binder captures the current context),
coterms (μ̃-binder captures the current value), and commands
(a cut ⟨t | e⟩ connecting them). This is the foundation.

**Spiwack** [Spi14] dissects this into a polarized variant:
positive types (values, introduced eagerly) vs negative types
(computations, introduced lazily). Shift connectives (↓N for
thunking, ↑A for returning) mediate between polarities.

psh adopts Grokking's [BTMO23] **two-sided** reading of λμμ̃ — three
syntactic categories (producers, consumers, statements) with
μ distinct from μ̃ — rather than Spiwack's **one-sided**
reading, where there are two categories (terms and commands)
and polarity is handled at the type level via dualisation
`A⁻`. The two presentations are logically equivalent. psh
chooses the two-sided reading because a shell has an
observable operational asymmetry between producers and
consumers that the one-sided reading obscures: producers are
values in hand (literals, captured command output, variable
slots), consumers are running processes reading from pipes
and file descriptors waiting to be written. The process
boundary is the asymmetry — converting a consumer to a
producer requires forking or thunking, which costs real
resources. Spiwack's dualisation is faithful to this at the
type level, but psh names both sides at the sort level so
the evaluator dispatches differently on each.

### The semantics

**Mangel, Melliès, and Munch-Maccagnoni** [MMM] define duploids —
non-associative categories integrating call-by-value (Kleisli/
monadic) and call-by-name (co-Kleisli/comonadic) computation.
Three of four associativity equations hold; the fourth — the
`(⊕,⊖)` equation — fails, and that failure captures the CBV/CBN
distinction. Maps restoring full associativity on the left are
thunkable (pure, value-like).

psh's duploid is a **dialogue duploid** [MMM, Definition 9.4]: a
symmetric monoidal duploid equipped with positive and negative
monoidal structures `(D, ⊗, 1)` and `(D, ⅋, ⊥)` related by a
strong monoidal duality functor `¬(−)` (involutive negation).
The correspondence to psh constructs:

| Dialogue structure | psh construct | Surface syntax |
|---|---|---|
| Product `×` (= `!A ⊗ !B` classically) | Tuples (§Tuples) | `(a, b)` |
| Tensor `⊗` (linear product) | Tuples inside `linear` blocks | `linear { let pair = (fd1, fd2) }` |
| Negative par `⅋` | Trap continuations (§Error model) | `trap SIG { h } { body }` |
| Involutive negation `¬(−)` | Type-level polarity swap | `¬A` = continuation expecting `A` |
| Downshift `↓` | Thunk behind a name | `<{cmd}`, `.refresh` |
| Upshift `↑` | Force / return | `` `{cmd} `` = ↓→↑, `let` binding |
| Exponential `!` | Classical zone | `let !x = b`, `!T` in type annotations |

The internal type theory is the **linear classical L-calculus**
[MMM, §9.3], which extends λμμ̃ with involutive negation and the
negative symmetric monoidal structure `(⅋, ⊥)` dual to positive
`(⊗, 1)`. The exponentials `!`/`?` mark the boundary between the
linear and classical zones — shell variables live under `!` by
default (freely duplicable, discardable); resource types
(coprocess tags, scoped fds) live in the linear zone (§Linear
resources). psh's unit-free fragment (no unit types) drops `1`
and `⊥`; structural results from [MMM] hold in the unit-free
setting because the relevant proofs do not depend on the units.

**Theorem 9.5** [MMM] establishes the equivalence: every duploid
from a dialogue chirality `L ⊣ R` carries dialogue duploid
structure, and conversely. psh's `L ⊣ R` is the adjunction
every duploid admits [MMM], realized operationally as the polarity
frame (§Polarity frames). The dialogue commitment upgrades the
adjunction to a full dialogue chirality by adding the duality
functor (process substitution's polarity swap) and the negative
monoidal structure (trap's ⅋).

The **Hasegawa-Thielecke theorem** [MMM, §9.6] holds in full:
**a map is thunkable if and only if it is central** in any
dialogue duploid. The forward direction (thunkable ⇒ central,
Proposition 8550) holds in any symmetric monoidal duploid. The
dialogue commitment licenses the reverse: **central ⇒
thunkable** — any map that commutes with all others under `⊗`
is necessarily pure. The `(⊕,⊖)` non-associativity is
preserved — dialogue does not restore it. Oblique maps remain
neither thunkable nor central; polarity frames remain mandatory
at every boundary crossing. An associative dialogue duploid
would be a `*`-autonomous category [MMM, Definition 9.4]; psh's
duploid is non-associative, so psh is not `*`-autonomous — the
non-associativity is load-bearing, not an artifact.

**Munch-Maccagnoni's thesis** [Mun13] is where duploids originate.
The companion paper [Mun14] gives the clearest self-contained
definition. Table 1 maps abstract structure to concrete PL
concepts: thunk, return, Kleisli, co-Kleisli, and oblique maps.

### The practice

**Binder, Tzschentke, Müller, and Ostermann** [BTMO23] present
λμμ̃ as a compiler intermediate language. Key insights:
evaluation contexts are first-class (the μ̃-binder reifies
"what happens next"); let-bindings (μ̃) are dual to control
operators (μ); ⊕ vs ⅋ error handling are dual.

**Levy** [Lev04] defines Call-by-Push-Value, the practical
framework for the value/computation distinction. psh's
`def`/`let` + lambda split is CBPV's `U`/`F` adjunction
surfaced as syntax. The F/U adjunction bridges value types
and computation types on the positive (Γ) side — it does
not bridge Γ and Δ; the ⊕/⅋ duality (§"Error model") is
the Γ/Δ split.


## The three sorts, made explicit

In Curien-Herbelin's λμμ̃ [CH00], the three syntactic categories
are:

- **Terms** (producers): values that have been computed or are
  ready to compute. They live on the left of the cut.
- **Coterms** (consumers): contexts that are waiting to receive
  a value. They live on the right of the cut.
- **Commands** (consumers): contexts waiting to receive a value
  — the coterm side. `echo` consumes arguments. `if` consumes
  a status. `match` consumes a value and eliminates by tag.
- **Expressions** (cuts): a term meeting a command — ⟨t | e⟩ —
  the moment of interaction where computation happens. The
  `Expr` layer wires up pipelines, redirections, and fork/exec.

`echo hello` is a cut: the producer `hello` meets the consumer
`echo`. The `def` keyword defines a computation; the cut
happens when the computation is invoked with arguments.

### Terms (producers) — Γ

Terms are values: literals, variable references, command
substitution results, lists, lambdas, concatenations. They are
evaluated eagerly (CBV) by `eval_term` before the command that
consumes them runs. Terms inhabit the context Γ.

In psh's AST, terms are the `Term` sort.

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
| `catch (e) { handler }` | Error continuation | Waiting for a nonzero status |

Coterms populate Δ. In the classical sequent Γ ⊢ A | Δ,
Δ contains the continuations — alternative futures that the
computation might jump to. In psh, Δ is populated by:

- **trap bindings**: `trap SIGINT { handler } { body }` binds
  the handler as a continuation in Δ for the duration of the
  body. The μ-binder `μα.c` in the calculus [CH00] — α names the
  signal continuation, c is the body that runs with α in scope.
- **catch bindings**: `try { body } catch (e) { handler }` binds
  the error handler as a continuation in Δ for the duration of
  the try body. Semantically similar but triggered by status
  rather than signal.

The evaluator function `run_expr` handles the cut sort:
pipelines (demand-driven) and redirections (profunctor
transformations on the I/O context). `run_cmd` handles the
consumer sort: dispatching on the command's consumer shape.

### Commands (consumers) — Δ

A command is a consumer: it describes what expects to receive
values and what it does with them. `echo` is a consumer —
it takes arguments, writes them to stdout, and continues.
`if` is a consumer — it takes a status and dispatches.
`match` is a consumer — it takes a value and eliminates by
tag. Assignment `x = _;` is a consumer — the μ̃-binder
`μ̃x.rest` waits for a value to bind.

| psh construct | Consumer shape | Notes |
|---|---|---|
| `echo` | stdout writer + continuation | Simple command consumer |
| `x = _;` | μ̃x.⟨rest \| α⟩ | μ̃-binder: waits for a value |
| `if(_) { A } else { B }` | case(A, B) | Coproduct elimination on status |
| `match(_) { arms }` | case(arm₁, ..., armₙ) | Multi-way elimination |
| `trap SIG { h } { _ }` | μα.⟨body \| α⟩ | μ-binder: names signal continuation |

In psh's AST, commands are the consumer sort. `let`,
assignment, `def`, `if`, `for`, `while`, `match`, `try`/`catch`,
and `trap` are all `Command` nodes — each with a specific
consumer shape (μ̃-binder, case dispatch, μ-binder). The
evaluator dispatches on the shape via `run_cmd`.

### Exprs (cuts) — ⟨t | e⟩

An expression is a cut: the moment a term meets a command and
computation happens. `echo hello` is the cut ⟨hello | echo⟩.
`cmd1 | cmd2` composes two cuts via a pipe. The `Expr` layer
is where producers meet consumers — pipeline wiring,
redirection composition, fork/exec.

| psh construct | Cut structure | Notes |
|---|---|---|
| `echo hello` | ⟨hello \| echo-consumer⟩ | Simple: term meets command |
| `cmd1 \| cmd2` | ⟨cmd1-out \| cmd2-in⟩ | Pipeline: cut via pipe fd |
| `cmd > file` | redirect ∘ ⟨args \| cmd⟩ | Redirect wraps the cut |
| `x = val; rest` | ⟨val \| μ̃x.⟨rest \| α⟩⟩ | Assignment: cut against binder |

The evaluator function `run_expr` handles the cut sort:
pipeline setup (fd wiring, fork), redirect application
(profunctor wrapping), and the connection of terms to
commands.

### The AST's three sorts

psh's AST has three sorts matching the λμμ̃ categories [CH00],
[BTMO23, §2]:

| psh node | λμμ̃ category | Evaluator | Role |
|---|---|---|---|
| `Term` | Producer (term) | `eval_term` → Val | Values, literals, accessors, lambdas — Γ |
| `Command` | Consumer (coterm) | `run_cmd` → dispatch | Command shapes: what expects to consume values — Δ |
| `Expr` | Cut (statement) | `run_expr` → Status | Where producer meets consumer: pipelines, redirects, fork/exec |

The `Expr` sort separates the profunctor transformations (redirections,
pipelines) from the cut/control layer. Logically, `Expr`
constructs are part of the consumer apparatus: pipelines build
co-Kleisli contexts, redirections transform I/O contexts via
profunctor maps. The evaluator boundary `run_expr` enforces
this: it handles the consumer machinery before `run_cmd`
performs the cut.


