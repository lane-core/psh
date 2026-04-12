# psh: Theoretical Foundation

## What this document is

The specification of psh's type theory, execution model, and design
rationale. psh descends from rc (Duff 1990), not from the Bourne
shell. The analysis begins there.

This document is the output of a systematic design process:
interrogation of rc's design philosophy, ksh93's implicit type
theory (discovered via the sfio-analysis and ksh93-analysis.md
sequent calculus mapping), the duploid semantics of Mangel/Melliès/
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
  20)` holds a list of one tuple. `$#pos` is 1. `$pos[0]` is
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
| `` `{cmd} `` command substitution | Force then return (↓→↑), oblique map | computation → value |
| `<{cmd}` process substitution | Downshift ↓ (thunk into namespace) | computation → name |
| `x=val; rest` | μ̃-binding (let) | bind value, continue |

psh adds one statement-to-producer move that rc did not have:

| psh mechanism | Logical shape | Effect |
|---|---|---|
| `$((...))` arithmetic | `μα.⊙(e₁,e₂;α)` — μ-binding around a binop statement | In-process, no subprocess, pure central map in `P_t` |

`$((...))` is **distinct from command substitution**, not a
copy. Command substitution is a genuine oblique map in the
duploid — it packages the body as a thunk, forces it by
forking, runs a full shell statement whose effects include
subprocess creation and I/O, and captures the byte-valued
return; the inner computation straddles CBV/CBN because the
forked pipeline is itself co-Kleisli. `$((...))` has neither
polarity straddle nor subprocess. Per [7, §2.1] "Arithmetic
Expressions," arithmetic binop in λμμ̃ is a **statement**
shaped `⊙(p₁, p₂; c)` taking two producers and a consumer;
the surface form `e₁ + e₂` translates as `μα.⊙(⟦e₁⟧, ⟦e₂⟧; α)`
— a μ-binding wrapping a statement to produce a positive. Any
"shift" here is type-theoretic only: the shell does fire a
polarity frame around `$((...))` to match the uniform mechanism
described in §Polarity frames, but since the inner computation
is effect-free the frame's save and restore steps simplify to
no-ops. Operationally trivial; categorically a pure central
map. ksh93/POSIX heritage for the syntax; the categorical
reading is psh's own.

psh makes two shifts explicit that rc left implicit:

1. **Command substitution without IFS.** psh splits on newlines,
   not on an arbitrary `$ifs`. The return operation (bytes → list)
   is fixed. Duff kept `$ifs` only because "indispensable" [1,
   §Design Principles]; psh removes it, closing the last re-scanning hole.

2. **Process substitution as downshift into namespace.** rc's
   `<{cmd}` returned an fd path while the child ran concurrently.
   Categorically this is a **downshift `↓`**: the negative CBN
   pipeline is thunked behind a name (a `/dev/fd/N` string) so
   it can be passed to a CBV caller. The name is positive (CBV —
   a string); the computation behind the name is negative (CBN,
   demand-driven, reads through the fd trigger it). The downshift
   itself is synchronous (the bind is immediate), but the
   computation is only scheduled; it runs when the fd is opened.
   This is not a `↓→↑` shift — there is no upshift back, because
   the caller receives the name, not the computation's eventual
   value. This matches Plan 9's mount model: `mount` returns
   immediately with a name, the server behind the mount point is
   concurrent. Nobody considers `mount` a violation of sequential
   execution. The concurrency is behind the name, accessed only
   when something reads the fd.


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

psh adopts Grokking's [7] **two-sided** reading of λμμ̃ — three
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

**Mangel, Melliès, and Munch-Maccagnoni** [2] define duploids —
non-associative categories integrating call-by-value (Kleisli/
monadic) and call-by-name (co-Kleisli/comonadic) computation.
Three of four associativity equations hold; the fourth — the
`(⊕,⊖)` equation — fails, and that failure captures the CBV/CBN
distinction. Maps restoring full associativity on the left are
thunkable (pure, value-like); one direction of the Hasegawa-
Thielecke implication, **thunkable ⇒ central** [2, Proposition
8550], holds in every symmetric monoidal duploid — no dialogue
structure or involutive negation required. psh cites only the
forward direction; the reverse (central ⇒ thunkable) is the
full Hasegawa-Thielecke theorem and requires dialogue-duploid
structure that psh does not commit to.

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
surfaced as syntax. The F/U adjunction bridges value types
and computation types on the positive (Γ) side — it does
not bridge Γ and Δ; the ⊕/⅋ duality (§"Error model") is
the Γ/Δ split.


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
| `catch (e) { handler }` | Error continuation | Waiting for a nonzero status |

Coterms populate Δ. In the classical sequent Γ ⊢ A | Δ,
Δ contains the continuations — alternative futures that the
computation might jump to. In psh, Δ is populated by:

- **trap bindings**: `trap SIGINT { handler } { body }` binds
  the handler as a continuation in Δ for the duration of the
  body. The μ-binder `μα.c` in the calculus [5] — α names the
  signal continuation, c is the body that runs with α in scope.
- **catch bindings**: `try { body } catch (e) { handler }` binds
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

In psh's AST, assignments and let-bindings are *statements*
whose consumer is a μ̃-binder: `x = val; rest` is the cut
⟨val | μ̃x.⟨rest | α⟩⟩ where α is the outer continuation. The
μ̃-binder is a consumer, not a separate sort — psh's AST does
not have a dedicated `Binding` node type. Assignments, `let`,
and `def` are `Command` nodes whose consumer side is μ̃-shaped,
and the evaluator dispatches on the shape.

### The AST's three sorts (plus engineering layer)

psh's AST has three logical sorts matching Grokking's [7,
§"Syntax"] three syntactic categories — producers, consumers,
statements — plus one engineering layer:

| psh node | λμμ̃ category | Role |
|---|---|---|
| `Word` / `Value` | Producer (term) | Values, variable refs, command substitutions, lambdas — everything in Γ |
| `Coterm` (synthesized) | Consumer | Evaluation contexts: pipe readers, redirect targets, μ̃-binders (let / assign), trap signal handlers, catch bindings |
| `Command` | Statement (cut) | Every cut: simple commands, pipelines, assignments ⟨val \| μ̃x.rest⟩, if/for/match/try/trap |
| `Expr` | engineering boundary | Wraps pipelines + redirections — consumer machinery grouped for evaluator organization, not a logical sort |

A `let x = M; rest` or `x = val; rest` is a *statement*, not a
separate sort: desugared, it is the cut ⟨M | μ̃x.⟨rest | α⟩⟩
[7, §2.1]. The μ̃-binder is a consumer alternative in Grokking's
grammar `c ::= α | μ̃x.s | D(p̄;c̄) | case{...}`; it lives
*inside* a statement as the consumer slot, not alongside the
statement as a peer sort. psh synthesizes consumers implicitly
from the statement's shape rather than storing them as
first-class nodes — the same way rc's consumers are implicit,
made just explicit enough for sort-directed evaluation.

The `Expr` sort is an engineering choice, not a logical one —
it separates the profunctor transformations (redirections,
pipelines) from the cut/control layer. Logically, `Expr`
constructs are part of the consumer apparatus: pipelines build
co-Kleisli contexts, redirections transform I/O contexts via
profunctor maps. The evaluator boundary `run_expr` enforces
this: it handles the consumer machinery before `run_cmd`
performs the cut.


## Polarity discipline

### CBV/CBN split

The CBV/CBN split follows the duploid's two subcategories [2,
§2.1]. Word expansion is Kleisli composition: each stage
(`$x` lookup, concatenation, command substitution) takes a
partial value and produces an expanded value with possible
effects. `eval_word` recurses through `Word` nodes before the
command that consumes them has started.

Pipeline execution is **demand-driven** (co-Kleisli in the
execution strategy): `run_pipeline` forks all stages
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
`(ε, ε') ∈ {⊕,⊖}²` enumerated in [2, §"Emergence of non-
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
order that Curien and Munch-Maccagnoni's focused calculus [8]
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
adjunction every duploid admits [2, §"Duploids," adjunctions-
duploids theorem]. Without the frame, a computation-mode
operation inside a value-mode context can silently corrupt
positive-mode state — the `sh.prefix` bug pattern documented in
[SPEC], which is the operational form of the `(⊕,⊖)` non-
associativity named above.

Polarity frames are invoked in three places in psh:

- **Command substitution** `` `{cmd} `` — frame saves the word
  expansion context, forks a subprocess, captures stdout, restores
  the context with the result list substituted. Full ↓→↑ shift.
- **Arithmetic expansion** `$((…))` — frame is operationally
  trivial (pure in-process computation, no effects to guard), but
  the shift is still type-theoretic: the expression is `μα.⊙(e₁,
  e₂;α)` in [7, §2.1]'s arithmetic translation, a statement
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
  was a misnomer — it defines a cut template, not a function.
  `def` names the sort honestly. Type/variable disambiguation
  in dotted names (`def List.length { }` vs `def x.set { }`) is
  by capitalization. See §Two kinds of callable.
- **`let` + lambda for functions.** Values in the value sort,
  first-class, with capture semantics. Lambdas use `|x| => expr`
  or `|x| { block }`; nullary is `| | => expr`. `let` is CBPV's
  μ̃-binder: it accepts any computation producing a value
  (pure, builtin call, pipeline, command substitution). See
  §Two kinds of callable.
- **rc parentheses** around conditions: `if(cond)`,
  `while(cond)`, `for(x in list)`, `match(expr)`.
- **`else` instead of `if not`.** Duff acknowledged rc's
  weakness here [1, §Design Principles].
- **`match`/`=>` instead of `switch`/`case`.** rc's `case` arms
  are top-level commands in a list; psh's `match` uses structured
  `=>` arms with `;` separators. The operation is genuinely
  different. Patterns are constructor-shaped (`(h t)` for cons,
  `(a, b, c)` for tuple, `ok(v)` for sum, `Pos { x, y }` for
  struct destructuring). Pattern alternation uses `|` between patterns
  (ML/Rust convention, unambiguous inside match arms). Pure
  guards via `if(cond)` after the pattern — restricted to
  side-effect-free expressions (comparisons, arithmetic).
- **Two accessor forms: bracket and dot.** Bracket `$a[i]` is
  projection by runtime value — tuples (`$t[0]`), lists
  (`$l[n]`), maps (`$m['key']`). Returns `Option(T)`. Dot
  `$x .name` is named field/method/discipline access with
  required leading space — the space disambiguates from rc's
  free caret (`$stem.c` = `$stem ^ .c`). Bracket binds
  immediately after `$name` (no space); `$a [0]` with space
  is a separate argument. See §Accessors.
- **Uniform tagged construction.** `NAME(args)` with `NAME`
  immediately followed by `(` (no space) commits the parser to
  tagged construction. Args are space-delimited (list-style).
  Covers enum variant construction (`ok(42)`, `err('msg')`).
- **`try`/`catch`** — scoped ErrorT. Changes the sequencing
  combinator inside `try` from unconditional `;` to monadic `;ₜ`
  that checks status after each command; on nonzero status,
  aborts to the handler. See §Error model.
- **`trap` — unified signal handling.** Grammar `trap SIGNAL
  (body body?)?` gives three forms: lexical (`trap SIGNAL { h }
  { body }`, scoped μ-binder), global (`trap SIGNAL { h }`,
  registration at top-level), and deletion (`trap SIGNAL`,
  removes a global handler). Precedence: innermost lexical >
  outer lexical > global > OS default. See §Error model for the
  full operational model.
- **`$((...))` arithmetic** — in-process pure computation
  returning an `Int`. Bare names inside refer to their integer
  values (no `$` needed). Polarity shift ↓→↑ without fork.
- **Single quotes only for string literals**, with `\`-escapes
  for literal characters (`\'`, `\$`, `\n` is literal `n`, etc.)
  and `\<whitespace>` forms as trivia (including line
  continuation via `\<newline>`). See syntax.md §Backslash
  escapes.


## Bidirectional type checking

psh's typechecker is bidirectional — types flow through
expressions in two directions without unification variables
and without cross-expression inference. The algorithm is
strictly weaker than Hindley-Milner and strictly stronger
than monomorphic surface-form checking; it is the well-studied
"bottom-up synth plus top-down check" pattern [Pierce-Turner,
Dunfield].

### The two modes

Every expression is checked in one of two modes:

- **Synth mode** (`Γ ⊢ e ⇒ T`): the expression `e` is given
  and the checker computes ("synthesizes") a type `T` from
  its surface form. Literals, typed variables, and expressions
  with sufficient type information at the leaves synth their
  type bottom-up.

- **Check mode** (`Γ ⊢ e ⇐ T`): the expression `e` and an
  expected type `T` are both given, and the checker verifies
  that `e` inhabits `T`. Record literals, enum construction
  with under-determined type parameters, and expressions whose
  type is determined by context are checked top-down against
  the expected type.

Which mode an expression is in depends on where it appears:

- **Synth-site**: a position where no expected type is supplied
  by context. A `let x = e` binding without an annotation is a
  synth site for `e`.
- **Check-site**: a position where the expected type is known
  from context. `let x : T = e`, function argument positions,
  `def` return type, match arm scrutinee binding, pattern let
  binders — all are check sites for the expression they contain.

### Three structural rules

**(Check from synth) — the bridge rule.** If an expression can
synth a type and the context expects a type, the checker
verifies the two agree:

    Γ ⊢ e ⇒ T'    T' = T
    ─────────────────────
    Γ ⊢ e ⇐ T

Type equality is nominal (`Pos` and `Tuple(Int, Int)` are
distinct) and structural within a type (tuple arities and
element types must match position-by-position).

**(Annotation) — the synth-from-check rule.** If the user
provides an annotation, the annotated expression synths to
the annotated type after checking in check-mode against it:

    Γ ⊢ e ⇐ T
    ──────────────────
    Γ ⊢ (e : T) ⇒ T

This is how the user supplies type information that the
expression alone cannot provide.

**(Ambiguity is error) — the rule without deferral.** If an
expression has no synth rule applicable to its form and no
check-site context to supply a type, the binding is a type
error at the binding site. The checker does NOT defer by
leaving a type hole and waiting for a later use to pin it.
Users must supply an annotation explicitly when the
expression carries no type information:

    let r = none                         # ERROR: no synth rule for nullary enum constructor; no context
    let r : Option(Str) = none           # OK: annotation provides context

Under-determined bindings are rejected at their site rather
than carried as open obligations. The consequence is a simpler
checker and clearer errors, at the cost of requiring
annotations in the narrow cases where synth alone gives
nothing.

### Expressions by mode

All typing rules use classical sequent notation
`Γ ⊢ t : A | Δ` (matching psh's λμμ̃ foundation — Grokking
[7, §2], Curien-Herbelin [5]). Each rule carries a **mode
annotation** `[synth]` or `[check]` indicating the
implementation strategy in the bidirectional checker. The mode
is **derived** from the rule structure: [synth] when the
conclusion type is determined by premises or Σ; [check] when
it must come from outside (continuation context, annotation).

| Expression | Mode | Why |
|---|---|---|
| Literal (`42`, `'str'`) | [synth] | type fixed by literal form |
| Variable `$x` | [synth] | type from `(x : A) ∈ Γ` |
| Tuple `(a, b)` | [synth] | component types assemble the product |
| List `(a b c)` | [synth] | element types agree; empty `()` is [check] |
| Struct `Pos { x = 10; y = 20 }` | [synth] | type name `T` in syntax + `T ∈ Σ` |
| Struct `Pos { 10, 20 }` | [synth] | same — type name at construction site |
| Enum variant `ok(42)` | [synth-if-pinned] | payload pins params; [check] if unpinned |
| Nullary enum `none` | [check] | no payload, no bottom-up info |
| Map literal `{'k': v, ...}` | [synth] | value types determine V; empty `{}` is [check] |
| Bracket `$a[i]` | [synth] | result type from collection type in Γ |
| Lambda `\|x\| => body` | [synth-if-pinned] | body operations pin params; [check] if unpinned |
| Function call `f $arg` | [synth] | `f`'s signature in Θ determines result |
| Match arm body | [check] | checked against match's expected type |
| `def` body tail | [check] | checked against α's type in Δ (return type) |
| `return expr` | [check] | checked against declared return type |

Expressions not listed fall back to [synth] when their form
determines a type.

### Type parameter pinning for parametric types

Parametric type constructors (`Option(T)`, `Result(T, E)`,
`List(T)`, `Map(V)`, user-declared `MyEnum(A, B)`) have
type parameters that must be pinned at the construction site
via a combination of synth and check:

- **Pinned bottom-up (synth)** — a typed argument position
  pins the corresponding parameter. `some(42)` synthesizes
  `Option(Int)` because `42 : Int` pins `T = Int`.
- **Pinned top-down (check)** — the expected type at the
  construction site pins any parameter not pinned bottom-up.
  `let r : Option(Str) = none` pins `T = Str` from the
  annotation because `none` has no argument to synth from.
- **Both, reconciled** — if both directions pin the same
  parameter, they must agree or it is a type error.
  `let r : Option(Int) = some('x')` is rejected because synth
  gives `T = Str` and check gives `T = Int`.

If any parameter remains unpinned after both directions run,
the binding is an error per the ambiguity rule above.

### Lambda parameter pinning

Lambda parameter types use the same write-once slot discipline
as parametric type constructors, with a different information
source (body operations instead of positional arguments). When a lambda appears at a synth
site (e.g., bare `let` without annotation), the checker tries
to pin each parameter type from monomorphic operations in the
body:

    let double = |x| => $((x * 2))    # x pinned to Int by *
    let greet = |name| => 'hello '$name .upper
                                       # name pinned to Str by .upper

Each parameter gets a write-once slot. Operations with
monomorphic signatures (arithmetic operators, string methods,
`.insert` on typed maps, comparisons) write the expected type
into the slot when they encounter a slot-typed argument. After
the body returns:

- All slots filled → lambda synths to
  `(ParamTypes...) -> BodyType`
- Any slot unfilled → error: "cannot infer type of parameter
  x; add annotation"

This is NOT unification — slots are write-once cells, not
unification variables. No backtracking, no occurs check, no
constraint propagation through parametric types. A slot that
appears only in parametric position (e.g., `|x| => some(x)`)
stays unfilled because `some` doesn't constrain `T` to a
ground type.

When a lambda appears in check-mode position (function
argument, annotated `let`), the expected function type pins
all parameters top-down as before — body-pinning is the
fallback for synth sites only.

### Why not Hindley-Milner?

The bidirectional algorithm has no unification variables, no
constraint solver, no let-generalization, and no inference
across expression boundaries. It terminates in linear time
on the AST size with no backtracking. The implementation
footprint is ~300-600 lines of Rust with readable error
messages (vs ~1200+ for HM with comparable ergonomics).

HM's additional power — full rank-1 let-polymorphism on
function signatures — is deliberately out of scope (see
§Non-goals). The bidirectional algorithm covers every
psh construct that actually exists, including parametric
type constructors on user-declared types, without paying
for machinery psh will never use.


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

The `def` keyword replaces rc's `fn`. psh renames it because
psh draws a distinction between named computations and
first-class functions that rc did not make. `def` is neutral
— it defines a named computation without claiming its role
in a cut, which only happens at the invocation site.

**`return` typing.** `return` is a μ-binding: `return v` =
`μα.⟨v | α⟩` where α is the `def`'s outer continuation. In
value-returning defs (`def name : ReturnType`), `return expr`
checks `expr` against the declared return type (check-mode).
In status-returning defs (no declared return type), `return N`
checks N against `Int`. Implicit return from the final
expression in a body also checks against the declared return
type. Every `return` in a body must agree on type — multiple
return paths are checked against the same declared type.

**`for`/`while` typing.** Both are commands (cuts). `for(x in
list) { body }`: the list expression is a producer in synth-mode;
the loop variable `x` is a μ̃-binder scoped to the body, typed as
the list's element type; the body is a command sequence. The
result status is the last iteration's status (0 for empty
iteration — rc convention). `while(cond) { body }`: the
condition pipeline is a command producing a status that drives
⊕ coproduct elimination (continue on zero, stop on nonzero);
the body is a command sequence. Both are standard cut forms —
no polarity frame, no shift.


## Discipline functions

A variable may be equipped with **discipline cells** — `def`-
registered bodies that mediate observation, refresh, and mutation.
A variable so equipped is **codata** in the sense of the sequent
calculus: its behavior is defined by destructors, and the
discipline cells *are* the variable's semantics. Three disciplines
are recognized:

- **`.get`** — the **pure observer**. A body of type `W(S, A)`
  that reads the stored slot and returns a value. No effects
  allowed.
- **`.refresh`** — the **effectful updater**. A body in `Kl(Ψ)`
  that may invoke arbitrary shell machinery (subshells, coprocess
  queries, filesystem reads) and writes the updated value into
  the stored slot. Invoked as an imperative command at a step
  boundary.
- **`.set`** — the **mutator**. A body that receives an incoming
  value, mediates the assignment, and writes the slot.

The split between observation and refresh is a return to rc's
observation philosophy ([1] §Environment): observation is a read,
mutation is an imperative step, and the shell's reference model
never hides work behind a variable reference. Plan 9 realized
this via `/env` as a kernel filesystem; psh realizes the same
philosophy on contemporary unix-likes using whatever filesystem
or IPC mechanism the user chooses — the spirit is portable even
though Plan 9's specific mechanism is not.

ksh93 collapsed observation and refresh by allowing its `get`
discipline to run arbitrary shell code on every reference [SPEC,
§Discipline Functions]. psh declines to import that design: it
hides work at the reference site, conflicts with Duff's "no
hidden work" principle, and interacts unsoundly with session-
typed coprocess channels when a signal unwinds a polarity frame
holding a `PendingReply` obligation.

### The codata model

In the sequent calculus, data types are defined by constructors
(how to build a value) and eliminated by pattern matching. Codata
types are defined by destructors (how to observe or transform a
value) and eliminated by copattern matching: the producer is a
cocase that says how to respond to each destructor invocation
[7, §6.3].

A disciplined variable is the cocase

    cocase{ get(α)     ⇒ ⟨.get-body | α⟩,
            refresh(α) ⇒ ⟨.refresh-body | α⟩,
            set(v; α)  ⇒ ⟨.set-body[v] | α⟩ }

where `.get-body` is a pure producer in `W`, `.refresh-body` is
a statement in `Kl(Ψ)`, and `.set-body` is a statement taking one
producer argument (the incoming value) and mediating the slot
write. All three are **destructors** of the codata type; the
cocase is the sole constructor (the variable *is* its cocase).
Per [7, §6.3], a codata constructor is the whole cocase; `.set`
is a destructor with one producer argument, not a constructor
in its own right.

A variable without discipline cells is ordinary data: the stored
value is what you read, assignment replaces the stored value, and
there is no cocase.

### .get — the pure observer

A `.get` body is a pure computation `W(S, A)`: it reads the
stored slot and returns a value without invoking effects. No
subshells, no coprocess queries, no filesystem reads. Effects
belong in `.refresh`. By default every disciplined variable has
the trivial `.get` that reads its stored slot as a pure value;
user-defined `.get` bodies are permitted but must remain pure
(typically to compute a derived view of the slot).

The once-per-expression reuse property is a theorem, not an
operational convention: pure maps into positive values are
thunkable by construction in the symmetric monoidal duploid, and
thunkable maps are central [2, Prop 8550]. Central maps may be
reused at every consumption site inside an expression without
disturbing composition order. CBV argument expansion therefore
evaluates `.get` once and shares the result at every occurrence
of the variable in the same expression, as a consequence of
thunkability — not as an appeal to Downen-style static focusing
(which is a syntactic rewrite pass, not a runtime reuse
mechanism).

There is no polarity frame around `.get`. The input and output
both live in the positive subcategory `P_t`; there is no polarity
crossing, and nothing to reenter.

### .refresh — the effectful updater

A `.refresh` body is a statement in `Kl(Ψ)`: it may invoke any
shell machinery — subshells, coprocess queries, filesystem reads,
pipelines — and is responsible for writing the updated value into
the stored slot. It is invoked as an imperative command at a step
boundary, never implicitly by reference.

Canonical shape (portable across contemporary unix-likes; the
rc/Plan 9 "observation is a file read" philosophy [1] §Environment
realized on unix without requiring `/env` or 9P services):

    let mut cursor = 0
    def cursor.refresh {
        cursor = `{ cat $XDG_STATE_HOME/psh/cursor }
    }

    cursor.refresh
    echo $cursor

`cursor.refresh` is a command-position invocation of the
discipline cell, parsed as a single NAME head and looked up
in Θ — syntactically the same shape as invoking a `def`-named
computation, and semantically the destructor `.refresh` of the
disciplined variable's cocase (§"The codata model"). It runs
at a step boundary, produces a status, and composes with
`try`/`catch` and `trap` the same way any other command does.
The parser's NAME-head dispatch plus the capitalization
convention (`def Type.method` for per-type methods uppercase;
`def varname.discipline` for per-variable disciplines
lowercase) is enough to disambiguate `cursor.refresh` from a
per-type method invocation. Users who want the ksh93 "live
variable" ergonomics wrap the pair in their own function — the
rc `fn cd` pattern [1, §Functions] applied to discipline
invocation:

    def show_cursor { cursor.refresh; echo $cursor }

`.refresh` is the site of the ↓→↑ polarity shift. The body runs
in computation mode inside a polarity frame that saves the
surrounding expansion context, runs the computation, and restores
the context on exit (see §Polarity frames). Inside the frame,
`cursor = value` is the primitive slot write: it bypasses the
cocase (which would recurse into `.refresh`) and writes the slot
directly.

Failure propagation is rc-native: `.refresh` errors surface as
a nonzero `$status` at the invocation site, which `try`/`catch`
catches the same way it catches any command failure. Silencing
requires the user's explicit `try { cursor.refresh } catch (_) { }`.

**Race bound under frame unwind.** A `.refresh` body that
issues coprocess requests holds its `PendingReply` tag
obligations inside the polarity frame. If the frame unwinds
before the body completes — signal handler issues `return N`,
`try`/`catch` aborts — the outstanding tags enter the draining
state described in §"Shell-internal tracking" and any stale
Rresponse is discarded. The primitive slot write at the end of
the body is unreachable in this case, so the slot retains its
prior value. This bounds the drop-as-cancel race: the window
is the duration of an explicit `cursor.refresh` invocation,
not every variable reference, and the slot is always either
fully updated or fully untouched (never half-written). Users
who need transactional semantics across cancel should wrap the
refresh in `try { cursor.refresh } catch (_) { }` and test for
the prior-value case explicitly.

### .set — the mutator

A `.set` body receives the incoming value as `$1` and mediates
the assignment. Unlike `.get`, `.set` may have effects — the
assignment is already at a step boundary, and effects at that
point are user-visible and expected.

    def x.set {
        # $1 is the new value being assigned
        # the body may validate, transform, reject, or write
        # the slot via the primitive assignment x = v
    }

`.set` fires on every assignment to `x`. Typical patterns:

- **Validation.** Reject assignments that don't meet a constraint,
  by calling `return` with a nonzero status.
- **Transformation.** Normalize or clamp the value before storing
  (e.g., clamp a percentage to 0-100).
- **Propagation.** Write the value to an external resource
  (coprocess, filesystem) as a side effect of the assignment.
- **Notification.** Log the change, emit metrics, trigger
  dependent updates.

**Who writes the stored slot.** Under the cocase framing, the
`.set` body owns the write. Inside `.set`'s polarity frame,
`x = v` is the primitive slot write: it bypasses the cocase
(which would recurse into `.set`) and writes the stored slot
directly. A `.set` body that does not perform such an assignment
does not update the slot. The evaluator does not write the slot
after `.set` returns — every state transition goes through a
destructor body [7, §6.3]. This makes `.set` the sole legitimate
writer of a disciplined variable's slot from the assignment
side; `.refresh` is the legitimate writer from the observation
side.

### Reentrancy and the polarity frame

`.refresh` and `.set` bodies may reference the variable they are
mediating (a `.refresh` that reads `$x` to compute the next value,
a `.set` that reads the old value before deciding what to store).
Within such a body, a reference to `$x` fires the default pure
`.get` on the current slot — which returns whatever value is
currently stored. There is no reentrancy problem at `.get`: pure
observation has no frame to reenter.

`.refresh` and `.set` themselves are guarded by polarity frames
(see §Polarity frames). Each frame saves the expansion context,
raises a reentrancy flag on the variable, runs the body, restores
the context on exit, and clears the flag. Inside the frame,
`x = v` writes the slot directly (it bypasses the cocase).
Recursive invocation of `.refresh` within its own body — calling
`x.refresh` while the flag is raised — is caught by the flag and
reported as a runtime error; a discipline that needs to refresh
itself mid-refresh is ill-defined.

The narrowing relative to prior drafts: only `.refresh` and
`.set` need polarity frames. `.get` is pure and needs none,
which is the simplest possible reentrancy story — there is
nothing to reenter.

### Mixed-optic structure

A variable with `.get` and `.set` (with or without `.refresh`)
is a **mixed optic** in Clarke's sense — specifically a monadic
lens [Clarke, def:monadiclens]:

    MndLens_Ψ((A,B),(S,T)) = W(S, A) × W(S × B, ΨT)

The view `.get` lives in the pure base `W`, and the update
`.set` lives in `Kl(Ψ)` — two different categories of morphisms,
glued by the action `⋊ : W × Kl(Ψ) → Kl(Ψ)` [Clarke,
prop:monadiclens]. "Mixed optic" is Clarke's term for an optic
whose decomposition and reconstruction categories differ; it
is not a term for "impure optic." psh's disciplined variable is
monadic-lens-shaped precisely because its view is pure and its
update is effectful — the defining case of a mixed optic. Earlier
drafts of this spec claimed the psh construct was "a proper
monadic lens, not a mixed optic," which inverted Clarke's
terminology; `prop:monadiclens` explicitly establishes that
monadic lenses *are* mixed optics.

The monadic lens laws [Clarke, cited via AbouSaleh et al. 2016
§2.3], stated up to Kleisli equality on the update side:

- **PutGet.**  `set s b >>=_Ψ get  ≡  return b` — the view is
  pure on the right side of the equation; the left side runs one
  effect and discards it. Holds when `.set` stores the value
  faithfully.
- **GetPut.**  `get s >>=_Ψ (λa. set s a)  ≡_Ψ  return s` — the
  view produces a pure value, the update writes it back. Holds
  when `.set` is inverse to `.get` on the stored slot.
- **PutPut.**  `set s b₁ >>=_Ψ (λs'. set s' b₂)  ≡_Ψ  set s b₂`
  — consecutive writes collapse to the last. Holds when `.set`
  is idempotent under Kleisli composition.

For ordinary variables without discipline cells, the view is
identity in `W`, the update is trivial, and all three laws hold
unconditionally. Adding `.set` turns the laws into contracts the
user maintains by discipline: a `.set` that silently transforms
its input breaks PutGet; a non-idempotent `.set` breaks PutPut.
The spec does not mechanically verify the laws; they are
documented here as the expected discipline and the user's
obligation.

`.refresh` is orthogonal to the mixed-optic structure. It is an
imperative update to the view — a "write from outside" — that
the user invokes explicitly. The view remains pure between
refreshes; `.refresh` is not part of the lens, and its presence
or absence does not change the lens laws.


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
not an optic. The `.set` update side of the discipline system's
mixed monadic lens lives in the same `Kl(Ψ)` the expansion
pipeline lives in; the `.get` view is pure `W`.

The full optic hierarchy (Prism, AffineTraversal, Traversal)
activates when products and coproducts are added.

### Word expansion as Kleisli pipeline

ksh93's `macro.c` expansion pipeline (tilde → parameter →
command sub → arithmetic → field split → glob) is Kleisli
composition [SPEC, §"The monadic side"]. psh's `eval_word` has
a simpler pipeline:

1. **Literal** → identity (pure, no effects)
2. **Var** → read the stored slot, invoking the variable's
   `.get` cell (pure `W(S, A)` — default is the identity slot
   reader; a user-defined `.get` must remain pure). The result
   is thunkable and is reused within the expression by CBV
   focusing [2, Prop 8550]. No polarity frame — `.get` has no
   effects to guard. Effectful state refresh is the job of
   `.refresh`, invoked separately as an imperative command.
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
   the same protocol. Uses the standard wire format with tag 0
   (reserved for negotiate). The shell sends a request frame
   with payload `"psh/1"` on tag 0; the coprocess responds on
   tag 0 with `"psh/1"` (accept) or an error frame (reject).
   Mismatch or error kills the coprocess channel. Session type:
   `Send<Version, Recv<VersionAck, S>>` where S transitions to
   the per-tag multiplexed protocol. The negotiate step exists
   so that the protocol is self-describing from the first byte
   — no out-of-band assumptions about the peer.
2. **Request-response pairs** — every request gets a response.
   No fire-and-forget. No ambiguity about whose turn it is.
3. **Error at any step** — failure is always a valid response,
   not a special case.
4. **Orderly teardown** — explicit close via a close frame,
   not just EOF/SIGPIPE. The close frame uses tag 0 (the
   negotiate tag, repurposed after negotiate completes) with
   payload `"close"`. The coprocess acknowledges with a
   response on tag 0 and then closes its end of the
   socketpair. Outstanding per-tag sessions are cancelled
   (Tflush sent for each, responses drained) before the close
   frame is sent. EOF without a preceding close frame is the
   crash fallback — the shell treats it as an unclean death,
   fails all outstanding tags with error status, and reaps the
   coprocess. Channel state machine: negotiate → active →
   draining (outstanding tags being flushed) → close-sent →
   close-acked → closed.

### Per-tag binary sessions

Tags multiplex independent binary sessions over one channel.
Each tag has the session type `Send<Req, Recv<Resp, End>>` —
exactly one legal action at each step. The tag is a session
identifier, not a reason to abandon session discipline.

**Cancellation** extends each per-tag session type with an
internal choice (⊕) on the shell side after Send: the shell
may either await the response or cancel. Cancellation uses
a Tflush frame **on the same tag** being cancelled, and the
coprocess acknowledges with an Rflush on that tag. The
extended per-tag session type is:

    Send<Req, (Recv<Resp, End> ⊕ Cancel<Recv<Flush_Ack, End>>)>

The shell chooses one branch per tag. This mirrors 9P's
Tflush/Rflush transaction [9P, §flush]. Cancellation is
strictly shell-initiated, preserving the asymmetric initiator
discipline. Users interact only with per-tag sessions via
`print -p` / `read -p` and never see Tflush directly.

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

Internally, the shell tracks each outstanding tag with a
handle parameterised by a phantom session-state type. Rust's
type system enforces at compile time that a handle can only be
consumed in its `AwaitingReply` state and only once — the
consume method moves `self` and returns a handle in the
`Consumed` state. Compile-time use-site affinity, not a
true linear type discipline (Rust disallows specialised `Drop`
impls per `E0366`, so drop-as-cancel is a runtime invariant
rather than a type-level guarantee).

When a handle is dropped without being consumed (the tag's
response is never read), the shell sends a Tflush frame on
the admin session, telling the coprocess to discard any
pending work for that tag. The tag then enters a **draining
state**: it is still outstanding from the shell's perspective,
but no user code owns it, and it is not available for
reallocation. The tag leaves the outstanding set only when
the coprocess acknowledges with an Rflush response on the
admin session — 9P-style Tflush/Rflush pairing. Rresponse
frames for a tag in draining state are discarded silently:
they are the expected residual of a cancel race, not a
protocol violation.

Tag reuse is therefore gated on **session termination**, not
on cancel dispatch. The sequence `allocated → sent → (response
received | Tflush sent → Rflush received) → freed` is the
only state machine the shell maintains per tag, and the `End`
of the per-tag session corresponds to the free step. The
handle discipline is implementation detail — users see only
tag integers.

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

Length-prefixed frames in the 9P style [9P] — length and tag
headers, but without 9P's separate Tcode byte. Frame kind is
recovered from the first payload byte on the receiver side:

    request    = length[4 bytes, LE u32] tag[2 bytes, LE u16] payload[length - 2 bytes]
    response   = length[4 bytes, LE u32] tag[2 bytes, LE u16] payload[length - 2 bytes]
    error      = length[4 bytes, LE u32] tag[2 bytes, LE u16] '!' error_message
    tflush     = length[4 bytes, LE u32] tag[2 bytes, LE u16] '#'                         (length = 3)
    rflush     = length[4 bytes, LE u32] tag[2 bytes, LE u16] '#'                         (length = 3)

`'!'` marks an error response; `'#'` marks a flush transaction
(Tflush from shell, Rflush back from coprocess). All other
first-byte values are ordinary request/response payloads.
Length-prefixed rather than newline-delimited because payloads
may contain newlines (multi-line strings, command output,
heredocs). The tag is binary u16 for efficiency; the payload
is UTF-8 text (Display/FromStr). An error frame with an empty
`error_message` is a protocol violation; the shell tears down
the session on receipt.

**MAX_FRAME_SIZE** is 16 MiB. Any frame whose length prefix
exceeds this is a protocol violation: the channel is torn
down, outstanding tags fail with error status, and the
coprocess is killed. This is a defensive constant to bound
memory use against buggy or hostile peers — not a semantic
limit on legitimate payloads.

**Reserved first-byte values.** `'!'` (0x21) marks error
responses; `'#'` (0x23) marks flush transactions. Normal
request/response payloads must not begin with these bytes.
If a payload naturally starts with `!` or `#`, the sender
must prefix it with a NUL byte (0x00) as an escape; the
receiver strips a leading NUL. Direction (Tflush vs Rflush)
is determined by which side of the socketpair the frame
arrives on — the shell writes to its end and reads from the
coprocess's end.

**Tag 0 is reserved** for the negotiate and close protocols.
User-visible tags start at 1.

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
the shell. Deadlock freedom follows from a simple per-channel
argument: each shell-to-coprocess channel is an independent
binary session with asymmetric initiative (shell always sends
first), so deadlock freedom is immediate by duality per
channel, and cross-channel deadlock is impossible because no
coprocess blocks on another coprocess.

Carbone, Marin, and Schürmann's forwarder logic [CMS] provides
the generalization path: their **MCutF admissibility theorem**
(§5) proves that multiparty compatible compositions can be
mediated by a forwarder. The current design does not use CMS
directly — the shell initiates and consumes, it does not
forward between coprocesses — but if psh ever adds
coprocess-to-coprocess routing, CMS provides the theoretical
foundation for deadlock freedom of the mediated composition.

Within this frame, psh restricts itself further: the shell
always initiates and the coprocess always responds on each
per-tag binary session `Send<Req, Recv<Resp, End>>`. This
asymmetric discipline makes each per-tag interaction duality-
safe (no interleaved cycles, no crossed initiative), so two-
party deadlock freedom per tag is immediate and multiparty
safety reduces to the forwarder correctness of the shell
itself.

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

**Per-command local variables** (rc heritage, rc.ms lines
1045-1066): `VAR=value cmd` scopes the assignment to the
duration of a single command. The variable reverts after the
command completes. This is the terse per-command form for
environment setup — `PATH='/custom/bin' make install` — and
is distinct from `let` block scoping. Both compose: block
scoping covers compound blocks, per-command scoping covers
the common single-command case.


## Error model

### ⊕ and ⅋

Every operation returns `Status(pub String)`. rc: "On Plan 9
status is a character string describing an error condition. On
normal termination it is empty" [1, §Exit status]. psh preserves
this. `Status::is_success()` checks emptiness.

Linear logic gives two disjunction connectives — not two names
for the same thing, but two genuinely different kinds of error
handling [7, §"Linear Logic and the Duality of Exceptions"]:

- **⊕ (plus, positive / data):** a tagged return value.
  Constructors `Inl(t)` / `Inr(t)`; elimination by
  `case{Inl(x) ⇒ s₁, Inr(y) ⇒ s₂}`. The caller inspects the
  tag. Rust's `Result<T, E>` and Haskell's `Either` are this
  shape. **psh's `$status` is ⊕**: every command returns a
  tagged value, and `try { body } catch (e) { handler }`
  pattern-matches on success/failure at step boundaries.
  `$status` is data; `try` is the consuming case.

  **Pipeline status.** `$status : Int` holds the exit code of
  the last command (or the last pipeline component). For full
  pipeline diagnostics, `$pipestatus : List(Int)` holds the
  exit codes of all pipeline components in order. Two variables,
  two types — `$status` never changes type. For a simple
  command (not a pipeline), `$pipestatus` is a single-element
  list equal to `($status)`. This follows bash/zsh convention
  (`$PIPESTATUS` / `$pipestatus`) with psh's native list type.

- **⅋ (par, negative / codata):** a pair — more generally an
  N-tuple — of continuations, one per outcome. The callee
  decides which continuation to invoke. A cocase
  `cocase{Par(α,β) ⇒ s}` binds two covariables and the body
  `s` jumps to exactly one of them. **psh's `trap SIGNAL
  { handler } { body }` is ⅋**: the body runs under a cocase
  binding one continuation per installed handler, and signal
  delivery is the callee (the shell's step-boundary dispatcher)
  jumping to the chosen handler's α. `trap` is codata — the
  same codata framing psh uses for discipline cells (§"The
  codata model").

The two are De Morgan duals: `(σ ⊕ τ)⁻ = σ⁻ ⅋ τ⁻`. Both styles
are present in psh because both styles show up in shell
programs. `try`/`catch` is the data/caller-inspects form;
`trap` is the codata/callee-jumps form. They compose
orthogonally (§"Signal interaction with try blocks") because
they act on different sorts: `try` on command statuses in the
positive Γ, `trap` on signal continuations in the negative Δ.

### try/catch — scoped ErrorT (⊕ discipline)

`try { body } catch (e) { handler }` changes the sequencing
combinator within `body` from unconditional `;` to monadic `;ₜ`
that checks Status after each command. On nonzero status,
execution aborts to the handler. The handler binding `e` is a
μ̃-binder on the error case.

Equivalent to lexically-scoped `set -e` without POSIX `set -e`'s
composability defects. Boolean contexts (if/while conditions,
&&/|| LHS, `!` commands) are exempt.

### trap — unified signal handling (⅋ discipline)

Grammar: `trap SIGNAL (body body?)?`. Three forms distinguished
by block count:

**Lexical** (two blocks): `trap SIGNAL { handler } { body }`
— installs the handler for the duration of the body, the
μ-binder of Curien-Herbelin [5, §2.1]. The handler captures a
signal continuation scoped to the body. Inner lexical traps
shadow outer for the same signal. The handler may `return N`
to abort the body with status N.

**Global** (one block): `trap SIGNAL { handler }` — registers
the handler at the top-level object's signal interface.
Persists until overridden or removed. This is the vertical
arrow form: it modifies the ambient signal-channel interface.

**Deletion** (no block): `trap SIGNAL` — removes a
previously-installed global handler.

Precedence at signal delivery: innermost lexical > outer
lexical > global > OS default. Signal masking via empty
handler (`trap SIGNAL { }` in either scope).

### Signal delivery model

Signals fire at **interpreter step boundaries**, which include
(1) between-command points in a block and (2) wake-from-block
points during child waits. The shell's main loop uses `poll(2)`
on both the child-status fd and a self-pipe. When a signal
arrives during a child wait, the self-pipe wakes the poll loop;
the shell handles the signal before resuming the wait. For
SIGINT specifically, the shell forwards the signal to the
child's process group (`kill(-pgid, SIGINT)`) so that
long-running children can be interrupted.

**EINTR policy:** builtins retry on EINTR by default. External
commands handle EINTR themselves — if an external command exits
nonzero due to interruption, that status flows through `try`
normally. This avoids spurious `catch` triggers from transient
EINTR returns in builtins.

**EXIT handler:** `EXIT` is an artificial signal synthesized by
the shell when the process is about to exit (rc heritage —
Duff's `sigexit`). It fires on normal exit and on `exit` called
from within a signal handler. Each process (including
subshells created via `@{ }`) has its own EXIT handler
registration; inherited handlers are copies that evolve
independently in the child (Plan 9 `rfork` model).

### Signal interaction with try blocks

When a signal arrives during a `try { body } catch (e) { }`:

1. **Between commands, handler returns.** The signal fires at
   the next step boundary. If the handler calls `return N`,
   the try body terminates with status N. The catch clause
   fires if N is nonzero.

2. **Between commands, handler does not return.** The handler
   runs its side effects, execution resumes at the next
   command, and try's status check proceeds normally.

3. **Handler calls `exit`.** The shell (or subshell) exits.
   Neither the remaining try body nor the catch clause runs.
   EXIT handlers fire during shutdown.

4. **Builtin interrupted by signal (EINTR).** The builtin
   retries on EINTR. The signal flag is still set. The handler
   fires at the next step boundary after the builtin completes.

`trap` and `try` compose orthogonally because they operate on
different sorts: `trap` on signal continuations (⅋), `try` on
command status (⊕). A lexical `trap` inside a `try` body fires
first when a signal arrives; if the trap returns a status, try
inspects it through its normal status-check mechanism.


## Tuples (products, ×)

Tuples are products — fixed-size heterogeneous containers.
They are the first connective psh adds beyond rc's list-of-
strings base type.

**Syntax.** Parentheses in psh have three roles, distinguished
by delimiter and prefix:

| Form | Delimiter | Interpretation |
|---|---|---|
| `(a b c)` | space | List — splicable sequence, runtime arity |
| `(a, b, c)` | comma | Tuple — single product value, fixed arity |
| `NAME(a b c)` | space (after tag) | Tagged construction — enum variant |

The comma is the list/tuple disambiguator. The tag prefix
(`NAME` immediately followed by `(`, no space) commits the
parser to enum variant construction. Tuples require at least
two elements — a 1-tuple is isomorphic to its element and
adds nothing. `(42)` is a one-element list.

    (a b c)         # list — rc heritage, space-separated
    (10, 20)        # tuple — comma-separated, minimum 2
    ('lane', '/home/lane', 1000)

Under the "every variable is a list" model, both store as
lists at the outer level, but the element types differ:

- `let xy = (10 20)` stores `[Int, Int]` — a list of two ints.
  `$#xy` is 2.
- `let xy2 = (10, 20)` stores `[Tuple(Int, Int)]` — a list of
  one tuple value. `$#xy2` is 1.

**Lists splice, tuples do not.** Substitution splices lists
into argument positions; tuples remain bundled as a single
value. This is visible when constructing an enum variant:

    let args = (42 'reason')   # list — two values
    let r = err($args)         # works — list splices, 2 args

    let pair = (42, 'reason')  # tuple — one bundled value
    let r = err($pair)         # does NOT work — 1 arg (arity mismatch)
    let r = err($pair[0] $pair[1])   # explicit destructure

The friction reflects a real semantic difference: lists are for
argument sequences (spliced, iterated, consumed positionally),
tuples are for structured values (kept bundled, accessed by
position via bracket `$t[0]`, `$t[1]`).

**Typing rule** (product introduction):

    Γ ⊢ t₁ : A₁ | Δ    Γ ⊢ t₂ : A₂ | Δ              [synth]
    ─────────────────────────────────────────
    Γ ⊢ (t₁, t₂) : A₁ × A₂ | Δ

**Accessor syntax** (product elimination — Lens projection):

    let pos = (10, 20)
    echo $pos[0]           # 10
    echo $pos[1]           # 20

    let record = ('lane', '/home/lane', 1000)
    echo $record[0]        # lane
    echo $record[2]        # 1000

Tuple element access uses **bracket notation** `$t[i]` with a
literal integer index. The bracket binds immediately after
`$name` with no space — `$pos[0]` is a projection; `$pos [0]`
(with space) is a separate argument. Bracket indices are
0-based. Negative indices resolve statically (the checker knows
the tuple arity): `$t[-1]` on a 2-tuple is `$t[1]`.

Tuple bracket access requires a **literal** index and is
**statically bounds-checked** — out-of-bounds is a type error,
not a runtime `none`. The result type is the field type
directly: `$t[0]` on `(Int, Str)` has type `Int`; `$t[1]`
has type `Str`. This makes tuple bracket a true **Lens**
(total, Cartesian) — PutGet/GetPut/PutPut hold
unconditionally. A runtime variable `$t[$n]` on a
heterogeneous tuple is a type error because the return type
cannot be determined statically.

(Lists and maps return `Option(T)` because their indices are
runtime values and partiality is inherent. Tuples are different:
the index is a literal, the arity is static, and the checker
can reject out-of-bounds at compile time.)

Composition: `$nested[0][1]` = `first . second` — ordinary
function composition of profunctor optics, chained
left-to-right. Bracket and dot accessors compose freely:
`$t[0] .name` is Lens then Lens (= Lens).

Tuples are positive (value sort), admit all structural rules
(weakening, contraction, exchange). They are inert data —
Clone, no embedded effects.

**ksh93 lineage.** ksh93 used `${a[n]}` for array subscripting
(sh.1 lines 1117-1131) with braces required around subscripted
variables (sh.1 lines 1289-1291). psh's bracket notation
follows the same convention: `$a[i]` for positional access.
The distinction: ksh93 required braces (`${a[0]}`); psh uses
bare `$a[0]` with the no-space rule disambiguating from
separate arguments.


## Structs (named products, ×)

Structs are named product types — nominal records with
declared field types and named accessors. A struct declaration
does two things: registers a nominal type whose values have
the declared shape, and auto-generates named and positional
accessors on the declared type.

**Declaration:**

    struct Pos {
        x: Int;
        y: Int
    }

    struct Rgb {
        r: Int;
        g: Int;
        b: Int
    }

Fields are declared with `name: Type` form separated by `;` or
newline. The declaration is nominal — `Pos` and `Tuple(Int,
Int)` are distinct types even though they share representation,
and there is no coercion between them.

**Construction** uses the type name followed by a brace literal:
`Type { fields }`. The type name at the construction site makes
struct literals self-typing (synth-capable) and syntactically
unambiguous against blocks and map literals.

Two construction forms, distinguished by delimiter:

*Named form* — semicolons, `field = value` entries:

    let p = Pos { x = 10; y = 20 }
    let red = Rgb { r = 255; g = 0; b = 0 }

*Positional form* — commas, values in declaration order:

    let p = Pos { 10, 20 }
    let red = Rgb { 255, 0, 0 }

Positional form requires at least two fields. Single-field
structs use the named form only.

The checker verifies: for named form, all and only declared
fields are present, types match field-by-field, field order
in the literal does not matter. For positional form, arity
matches the declaration, types match position-by-position.

**Name-pun shorthand** (named form only). When the right-hand
side of a field is a variable whose name matches the field,
the `= NAME` part may be elided:

    let x = 10
    let y = 20
    let p = Pos { x; y }         # equivalent to Pos { x = x; y = y }

**Parser disambiguation.** The type name prefix resolves the
brace ambiguity:

- `NAME {` where NAME is uppercase → struct construction
- `{ expr : expr, ... }` → map literal (colon + commas)
- `{ cmd; cmd }` → block (commands + semicolons)

This is the `,` vs `;` rule that runs through the language:
commas delimit structural products (tuples, positional records,
maps), semicolons delimit sequential operations (commands,
named record fields).

**Construction in various contexts:**

    # self-typing — no annotation needed
    let p = Pos { x = 10; y = 20 }
    let p = Pos { 10, 20 }

    # return value
    def origin : Pos { Pos { x = 0; y = 0 } }
    def move : Pos -> Pos {
        |p| => Pos { x = $p .x + 1; y = $p .y }
    }

    # function argument — type explicit at the call site
    def distance : Pos -> Pos -> Int {
        |a b| => $(( abs($a .x - $b .x) + abs($a .y - $b .y) ))
    }
    distance Pos { x = 0; y = 0 } Pos { x = 3; y = 4 }
    distance Pos { 0, 0 } Pos { 3, 4 }

    # missing or extra fields are type errors
    let p = Pos { x = 10 }               # ERROR: missing field y
    let p = Pos { x = 10; y = 20; z = 5 } # ERROR: no field z on Pos

**Accessors** are auto-generated from the declaration. A
`struct Pos { x: Int; y: Int }` declaration registers:

- `.x` and `.y` — named accessors (Lens projections on the
  `Pos` type)
- `.fields` — returns `List((Str, Str))` of `(name, value)`
  pairs in declaration order, with values coerced to strings
  (serialization boundary for generic traversal)
- `.values` — returns `List(T)` of field values in declaration
  order, **only when all fields share a single type T**
  (checker-gated; heterogeneous structs do not get `.values`)

Named accessors work on struct values:

    let p = Pos { x = 10; y = 20 }
    echo $p .x               # 10
    echo $p .y               # 20

Generic traversal:

    for (name, val) in $p .fields {
        echo $name '=' $val  # 'x = 10', 'y = 20'
    }

Homogeneous typed iteration:

    let vals = $p .values    # List(Int) = (10 20)

There is no bracket positional access on structs — bracket
`[i]` is for ordered/keyed collections (tuples, lists, maps).
Struct field access is dot-only, reflecting that struct fields
are named observers, not indexed projections.

The struct declaration is a batch registration in the per-type
accessor namespace, equivalent to writing `def Pos.x { }` and
`def Pos.y { }` by hand. `.fields` and `.values` are
auto-generated `def Type.fields` and `def Type.values`
methods — computations that produce new lists, not lens
projections into the struct's storage.

**Pattern matching** uses the type name prefix, symmetric with
construction. Named and positional patterns mirror the two
construction forms:

    match ($p) {
        Pos { x = 0; y = 0 }   => echo 'origin';
        Pos { x = _; y = 0 }   => echo 'on x-axis';
        Pos { x = 0; y = _ }   => echo 'on y-axis';
        Pos { x; y }           => echo $x $y;        # name-pun
        Pos { _, y }           => echo 'y=' $y        # positional
    }

All declared fields must appear in named patterns (wildcards
`_` are fine for fields you don't care about). The name-pun
shorthand `Pos { x; y }` is equivalent to `Pos { x = x; y = y }`
and works in patterns as well as at construction sites.

**Pattern let** accepts struct patterns, binding multiple
names from a single destructuring:

    let Pos { x, y } = $p            # positional
    let Pos { x = px; y = py } = $p  # named, explicit binding names

**Mutation** requires `let mut`. Struct fields are immutable by
default; mutation takes the form of whole-struct replacement:

    let mut p = Pos { x = 10; y = 20 }
    p = Pos { x = 30; y = $p .y }

No field-level mutation syntax (`p .x = 30`). Whole-struct
replacement is consistent with the value model: structs are
positive data, Clone, and mutation means rebinding the
variable. Field-level mutation sugar can come later as the
Lens `set` operation if a clear use case emerges.

**No anonymous records.** Every record type requires a `struct`
declaration. There is no free-standing "record type" that
accepts arbitrary field names structurally; the type at
construction is always a declared struct, named at the
construction site.

**Typing rules** (named product introduction):

*Named form:*

    struct T { f₁:A₁; …; fₙ:Aₙ } ∈ Σ
    Γ ⊢ eᵢ : Aᵢ | Δ   (for each i)                   [synth]
    { f₁, …, fₙ } = { π | (fπ = _) ∈ literal }
    ──────────────────────────────────────────────
    Γ ⊢ T { f₁ = e₁; …; fₙ = eₙ } : T | Δ

*Positional form:*

    struct T { f₁:A₁; …; fₙ:Aₙ } ∈ Σ     n ≥ 2      [synth]
    Γ ⊢ eᵢ : Aᵢ | Δ   (for each i)
    ──────────────────────────────────────────────
    Γ ⊢ T { e₁, …, eₙ } : T | Δ

Both rules are [synth]: the type `T` appears in the term
syntax and in Σ, so the conclusion type is determined by
premises. Field sub-expressions are checked against Σ's
declared types (the premises constrain `eᵢ : Aᵢ`), but the
overall struct type flows bottom-up.

**In VDC terms:** a struct declaration specifies a cell with a
fixed multi-source signature. `Pos : Int, Int → Pos` says the
constructor cell has two `Int` horizontal arrows on top and
one `Pos` horizontal arrow on the bottom. The named accessors
are destructor invocations — the codata view of the struct,
dual to the constructor's data view. The `struct` keyword is
the syntactic form that batches the two views together:
registering the constructor (positive introduction, realized
as the type-prefixed brace literal) and the projections
(negative destructors, realized as the `.x` named accessors)
at once, unifying the data/codata duality in a single
declaration.


## Enums (coproducts, +)

Enums are nominal coproducts — user-declared tagged unions
with a fixed set of named variants. Each variant has an
optional payload type, and construction uses tagged form
`variant(payload)` for variants that carry a payload, bare
`variant` for nullary variants.

**Declaration:**

    enum Option(T) {
        some(T);
        none
    }

    enum Result(T, E) {
        ok(T);
        err(E)
    }

    enum CompileResult {
        success(Path);
        warning(Str);
        error(ErrorInfo)
    }

    struct ErrorInfo { message: Str; line: Int }

The form is `enum Name(TypeParams) { variant₁(Type₁);
variant₂(Type₂); …; variantₙ }` where:

- `Name` is the enum type name (uppercase, psh capitalization
  rule).
- `(TypeParams)` is an optional parameter list with comma-
  delimited type variables (uppercase, Rust convention). Omit
  the parens entirely for non-parametric enums.
- The body is a brace-delimited, `;`-separated list of variant
  declarations.
- Each variant declaration is either `name(PayloadType)` for a
  variant carrying a payload or bare `name` for a nullary
  variant. Variant names are lowercase (value-namespace,
  psh capitalization rule).
- `PayloadType` is any type in scope: a base type, a tuple, a
  struct, or another enum (including a parametric instance).

Multi-field payloads use a separate `struct` declaration
referenced from the variant. There is no inline record syntax
inside enum variants — if a variant needs named fields, declare
a struct and reference it.

**Construction** mirrors declaration: tagged form for variants
with payloads, bare name for nullary variants.

    let r : Result(Int, Str) = ok(42)
    let r : Result(Int, Str) = err('not found')
    let o : Option(Path)     = some(/tmp/file)
    let o : Option(Path)     = none
    let c : CompileResult    = success(/tmp/a.out)
    let c : CompileResult    = warning('deprecated syntax')
    let c : CompileResult    = error(ErrorInfo { message = 'syntax error'; line = 42 })

In the last case, `error(...)` is tagged construction with an
argument of type `ErrorInfo`, and the argument is a brace
record literal (struct construction). The expected type
`ErrorInfo` flows top-down through `error`'s payload into the
literal via the bidirectional check. Tagged construction and
brace record literals compose naturally.

**Bare `none` vs parenthesized `none()`.** Nullary variants
are bare — no parens at construction. `()` is reserved for
the empty list. `none()` is not valid syntax under this
rule.

**Command-position ambiguity.** In command position, `ok 42`
(with space) is a command named `ok` with argument `42`, not
enum construction. The `NAME(` token (no space before `(`)
commits the parser to enum construction. For nullary variants,
the bare form `none` in command position is a command call —
enum construction of a nullary variant requires a value
context (annotation, function argument, return, etc.) to
signal that it should be interpreted as construction rather
than as a command. In practice this is rarely ambiguous
because nullary variants are almost always bound via
annotation.

**Type-parameter determination at construction.** An enum
value's type parameters are pinned bidirectionally:

- **Bottom-up (synth)** — a construction site with a typed
  argument pins any type parameter that appears in the
  argument's position. `some(42)` synthesizes `Option(Int)`.
- **Top-down (check)** — the expected type at the construction
  site pins any type parameter not pinned by argument types.
  `none` inside `let o : Option(Str) = none` pins `T = Str`
  from the annotation.

If all type parameters are pinned by one or both directions,
construction is well-typed. If any parameter is left
unpinned, the binding is a type error at the binding site —
the user must supply an annotation. The rule has no
unification variables and no cross-expression inference.

Worked cases:

    let r = some(42)                     # OK: T=Int synthesized, r : Option(Int)
    let r : Option(Str) = none           # OK: T=Str from annotation
    let r = none                         # ERROR: T unpinned, annotation required
    let r = ok(42)                       # ERROR: T=Int synthesized but E unpinned
    let r : Result(Int, Str) = ok(42)    # OK: E=Str from annotation
    def o : Option(Path) { none }        # OK: T=Path from return type
    def o : Option(Int)  { some('x') }   # ERROR: Str vs Int mismatch

**Typing rule** (coproduct introduction):

    enum T(Ᾱ) { …; variantᵢ(Bᵢ); … } ∈ Σ
    Γ ⊢ e : Bᵢ[τ̄/Ᾱ] | Δ                     [synth-if-pinned, check otherwise]
    ──────────────────────────────────────────────────────────
    Γ ⊢ variantᵢ(e) : T(τ̄) | Δ

    enum T(Ᾱ) { …; variantᵢ; … } ∈ Σ        [check]
    ──────────────────────────────────────────────────
    Γ ⊢ variantᵢ : T(τ̄) | Δ                  (nullary — τ̄ from context)

The τ̄ are instantiation choices for the enum's type
parameters, pinned by the combination of argument synthesis
and context check as described above.

**Elimination** via `match` with tagged patterns symmetric to
construction:

    match ($r) {
        ok(val)  => echo 'got '$val;
        err(msg) => echo 'failed: '$msg
    }

    match ($c) {
        success(p)                        => echo 'built: '$p;
        warning(w)                        => echo 'warning: '$w;
        error(ErrorInfo { message = msg; line = ln }) => echo 'error: '$msg' at '$ln;
        error(ErrorInfo { message; line })            => echo 'error: '$message' at '$line
    }

    match ($o) {
        some(x) => echo $x;
        none    => echo 'nothing'
    }

Variant patterns use the same form as construction: `tag(pat)`
for payload-bearing variants, bare `tag` for nullary. The
argument pattern `pat` can be a variable binding, a wildcard,
a literal, a tuple pattern, a struct record pattern, or a
nested enum pattern. Struct record patterns use the same
brace form as struct construction with full support for the
name-pun shorthand.

**Pattern let** works on enum variants when the pattern is
irrefutable (only one variant), which is rare. For refutable
patterns use `let-else`:

    let some(path) = lookup key else {
        echo 'not found'; return 1
    }
    # path : Path, available below only if lookup succeeded

**`let-else` typing rule** (refutable μ̃-binder):

    Γ ⊢ M : A | Δ                           [synth on RHS]
    Γ ⊢ pat : A ⊣ Γ'                        (pattern binds Γ')
    Γ | else-body diverges ⊢ Δ              (else must return/exit)
    ─────────────────────────────────────────────────────
    Γ' ⊢ rest : (Γ' ⊢ Δ)                   (bindings scoped below)

The else-body is a consumer in Δ (an error continuation).
It must diverge — `return N`, `exit`, or an infinite loop.
If it does not diverge, the bindings from `pat` would be
uninitialized on the fall-through path, which is a type
error. Sort: command (cut). The pattern match + else branch
is a single focused elimination with two arms.

**Observation** uses `match` — variant names are constructors
(`ok(42)`, `err('msg')`), not postfix accessors. There is no
`$result .ok` Prism preview via dot; enum dispatch goes through
`match` arms. Profunctor constraint on the Prism structure:
Cocartesian.

**Guards** refine pattern arms with a pure condition:

    match($x) {
        ok(v) if($v > 0) => handle_positive $v;
        ok(v)            => handle_nonpositive $v;
        err(msg)         => fail $msg
    }

Syntax: `pattern if(cond) => body`. The guard expression is
restricted to **pure, side-effect-free expressions** —
comparisons, arithmetic, boolean connectives, string equality.
No commands, no command substitution, no effects. The parser
accepts the full expression grammar in guard position; the
**checker** enforces the purity restriction by rejecting guard
expressions containing command invocations, command substitution,
or assignments. This keeps guards in the positive subcategory
P_t: the pattern binds variables (positive), the guard tests them
(positive-to-positive), no polarity boundary is crossed.

Guard failure backtracks to the next arm. Desugaring to focused
core groups arms by constructor tag and nests the guard as a
`case` on `Bool` inside the arm body:

    ⟨$x | case{
        ok(v) ⇒ ⟨$v > 0 | case{true ⇒ A, false ⇒ B}⟩,
        err(msg) ⇒ C
    }⟩

Every case in the desugared form is focused — the scrutinee
is a value when the case fires (not a computation). Guards
are surface sugar that does not disturb the core calculus.

**Exhaustiveness:** guarded arms are non-exhaustive. The
checker treats them conservatively — a constructor with only
guarded arms requires a fallback (unguarded arm or wildcard).

**Why pure-only:** effectful guards would create a (+,−)
composition site inside case dispatch. A failed effectful
guard has already committed its side effects, making
"resume matching" unsound — the world has changed between
arms. Pure guards are thunkable (central per Duploids
Proposition 8550), compose freely, and backtrack safely
because no state was modified. Effectful conditions belong
in `if` inside the arm body, where the effect is explicit
and no backtracking occurs.

Enums are positive (value sort), admit all structural rules.
They are inert data — Clone, no embedded effects.


## Features and non-goals

psh is conceived as a unified totality, not a staged roadmap.
Every feature is either in the design or explicitly out of
scope with reasoning. There is no "v1/v2" split; the spec
describes the shell as a whole.

### Features beyond the rc base

The following types and features are load-bearing members of
psh's type system beyond what rc had. Each is in the design;
full spec sections will be added or refined as the design
stabilizes.

- **Map type** — associative arrays with O(1) lookup.
  Parametric type constructor `Map(V)`. Keys are always
  strings — matching the unanimous convention of bash, zsh,
  ksh93, nushell, and ysh. Integer-keyed collections use
  lists; string-keyed associative lookup uses maps.

  **Construction** has three paths:

  *Map literal* — brace syntax with colon key-value separator
  and comma delimiter, syntactically distinct from struct
  record literals (`=` and `;`):

      let m : Map(Int) = {'name': 1, 'age': 2}

  Map literals can synthesize their type from entries
  (value types must agree). Empty `{}` is [check] — the
  expected type resolves whether it is an empty map or an
  empty struct.

  **Typing rule** (map introduction):

      Γ ⊢ vᵢ : V | Δ    (for each entry 'kᵢ': vᵢ)    [synth]
      ─────────────────────────────────────────────
      Γ ⊢ {'k₁': v₁, …, 'kₙ': vₙ} : Map(V) | Δ

  *Builder chain* — `.insert` is a pure functional update
  method on Map values (distinct from the discipline `.set`
  on variables), returning a new Map:

      let m : Map(Int) = Map.empty .insert 'name' 1 .insert 'age' 2

  *Bulk constructor* — `Map.from_list : List((Str, V)) → Map(V)`
  constructs from a list of key-value tuples:

      let pairs = (('name', 1) ('age', 2))
      let m : Map(Int) = Map.from_list $pairs

  **Access** uses bracket notation — `$m['key']` returns
  `Option(V)` (`some(v)` or `none`). Inside brackets is
  expression context (never glob); the index expression must
  be of type `Str`. **Insertion** via assignment —
  `m['key'] = v` on a `let mut` map desugars to
  `m = $m .insert 'key' v` (discipline-transparent).

  **Accessors:** `.keys` returns `List(Str)`, `.values`
  returns `List(V)`. Key-indexed view (`$m['key']`) is an
  affine traversal; iterate-all-values (`.values`) is a
  traversal.

- **String methods on `Str`** — fork-free string operations
  registered as `def Str.name { }` accessor methods. `.length`,
  `.upper`, `.lower`, `.split`, `.strip_prefix`,
  `.strip_suffix`, `.replace`, `.contains`. Partial operations
  return `Option(T)`; predicates return status. Replaces
  ksh93's `${var#pat}`/`${var%pat}` parameter expansion.

- **Job control builtins** — `fg`, `bg`, `jobs`, `wait` (with
  `-n` for any-child), `kill`. Job IDs as a new word form:
  `%N` expands to the PID of job N.

- **Here documents and here-strings** — rc heritage (rc.ms
  lines 906-973). Six forms from three orthogonal toggles:

  | Form | Expansion | Tab strip | fd |
  |---|---|---|---|
  | `<<EOF` | yes | no | stdin |
  | `<<'EOF'` | no (literal) | no | stdin |
  | `<<-EOF` | yes | tabs stripped | stdin |
  | `<<-'EOF'` | no | tabs stripped | stdin |
  | `<<[n]EOF` | yes | no | fd n |
  | `<<-[n]EOF` | yes | tabs stripped | fd n |

  Quoted marker (`'EOF'`) suppresses variable expansion. `<<-`
  strips leading tabs from body and closing marker (ksh93
  heritage, sh.1 line 3867). fd-targeted form `<<[n]` directs
  to fd n instead of stdin (rc heritage, rc.ms line 959).
  Here-string `<<<` is the degenerate case: `cmd <<< 'input'`
  is a one-line here document with no closing marker.

  In VDC terms: a cell with a constant (or expansion-computed)
  horizontal arrow on the specified fd. §8.5 classification:
  monadic (positive-to-positive), same as string literals.

- **`$((...))` arithmetic** — documented in §"rc's execution
  model"; in-process pure expression evaluation returning an
  `Int`, shaped as `μα.⊙(e₁, e₂; α)` per [7, §2.1].

- **Parametric type constructors on user declarations.**
  Users may declare parametric struct and enum types with
  uppercase type parameters in the declaration header:
  `enum Result(T, E) { ok(T); err(E) }`,
  `struct Pair(A, B) { first: A; second: B }`. Type
  parameters live on type declarations only; `def` signatures
  reference fully-instantiated ground types
  (`def parse : Str -> Result(Int, Str)`), never polymorphic
  ones. This is strictly weaker than rank-1 prenex
  polymorphism on function signatures (see §Non-goals) and
  does not require Hindley-Milner machinery — only structural
  monomorphization at each use site.

### Non-goals

The following are explicitly out of scope. Adding them would
distort the type theory or bloat the implementation without
serving the focused shell psh is designed to be.

- **Parametric polymorphism on `def` signatures.** Rank-1
  prenex `∀` at function boundaries — `def map(T, U) : (T ->
  U) -> List(T) -> List(U)` or similar — is not part of the
  design. Function-level polymorphism requires either VETT's
  hyperdoctrine semantics (which abandons FVDblTT's single-
  VDC home and in-language protype-isomorphism reasoning) or
  the unpublished dependent-FVDblTT extension. The rank-1
  free-theorem benefit (`∀α. α → α` is uniquely identity) is
  too weak to justify elaborator complexity, annotation
  burden, and the categorical commitments required.
  Init-script robustness is delivered instead by rich
  ground-typed builtins and the polarity discipline that
  already carries Reynolds-style parametricity internally at
  the phase boundary [Sterling-Harper, logical-relations-as-
  types §4226-4243]. Generic combinators (`map`, `filter`,
  `fold`) are shell builtins whose types live at the Rust
  implementation layer and are never surfaced to the shell
  user. Parametric type *constructors* on type declarations
  (see Features above) are permitted because they require
  only monomorphization at use sites, which is structurally
  different from rank-1 prenex `∀` at function signatures.
  The `type` keyword remains reserved in case a concrete
  shell-level use case ever emerges that monomorphic ground
  types cannot serve.

- **Typed session channels on pipes.** Pipes are byte
  streams. The typed-IPC case is served entirely by
  coprocesses (§Coprocesses), which have session-type
  discipline per tag. Typing pipes would force every pipeline
  stage to commit to a protocol — breaking rc's text-stream
  pipeline ergonomics and conflating two different IPC
  mechanisms. There is no binding site at `|` to pin session
  types, because pipe producers and consumers are separately
  compiled processes with no compile-time link. Pipes are
  byte streams, and typed IPC goes through coprocesses.

- **Refinement session types on coprocess payloads.** Das et
  al. ("Practical Refinement Session Type Inference") prove
  that refinement session type checking is undecidable even
  for simple linear arithmetic refinements; practical
  implementations require SMT-solver integration (Rast ships
  Z3). The phantom-session-type substrate psh uses for
  coprocess protocols gets its guarantees from Rust's own
  type checker and does not compose with solver-based
  refinement checking. Base-type refinements on payload
  values (`NonEmpty(Str)`, `Positive(Int)`) are a value-layer
  question that could ride on a future refinement-types
  addition if psh ever acquires one; that is orthogonal to
  session types.

- **Pipeline fusion as a user-visible feature.** The Segal
  condition (fcmonads §5) gives the categorical account of
  when a sequence of pipeline stages has a composite. This is
  an implementation-level optimization opportunity — the
  evaluator may fuse adjacent stages for performance when
  their composite exists — not a user-facing construct.
  Exposing fusion as a user feature would require committing
  to the pseudo-double-category equations that a VDC
  instance may not satisfy in general, and would force users
  to reason about composite existence at sites where they
  should just be writing shell. Fusion stays in the
  implementation notes.

- **Parametric polymorphism on pipes.** See above. Both the
  "typed pipes" and the "parametric polymorphism" non-goals
  are self-reinforcing — the former requires type variables
  at pipeline stages, the latter rules out user-visible type
  variables in function signatures, and together they pin
  pipes at byte streams.

### Reserved keywords

`type` is reserved for possible future use if parametric
polymorphism on function signatures is ever reconsidered. It
is not currently in the grammar.

`enum` is active — user-declared enums land under the
"Features beyond the rc base" section above.

### Optics activation

The accessor system uses two syntactic forms: **bracket**
`$a[i]` for projection by runtime value, and **dot** `$a .name`
for named field/method/discipline access. This split reflects
an optic selection boundary: bracket selects an optic by a
runtime-valued index (the index is itself a producer), while
dot selects by a static symbol resolved at parse/check time.
Both sides produce standard profunctor optics; the syntax
determines how the optic is selected, not which class it is.

| Type | Access form | Optic | Profunctor constraint |
|---|---|---|---|
| Lists (rc base) | `for x in $l` | Traversal (iteration) | Traversing (Applicative) |
| Lists | `$l[n]` | Affine traversal (index) | Cartesian + Cocartesian |
| Tuples (products) | `$t[i]` (literal index) | Lens (projection) | Cartesian |
| Structs (named products) | `$s .field` | Lens (named) | Cartesian |
| Sums (coproducts) | `match` | Prism (case analysis) | Cocartesian |
| Map(V), key lookup | `$m['key']` | Affine traversal (partial) | Cartesian + Cocartesian |
| Map(V), all values | `.values` | Getter (read-only) | — |
| List slice | `$l[a..b]` | Affine fold (read-only) | (read-only restriction of AffineTraversal) |
| fd table (save/restore) | (internal) | Lens | Cartesian |
| Redirections | (wrapping) | Adapter | Profunctor |

Discipline-equipped variables are not listed here — they are
mixed monadic lenses per `def:monadiclens`, orthogonal to the
type-shape classification above. See §"Mixed-optic structure"
in §Discipline functions.

**Bracket/dot partition.** The table partitions cleanly — no
row needs both accessor forms. Bracket covers indexed/keyed
access (tuples, lists, maps); dot covers named observers
(struct fields, type methods, discipline functions). Optic
composition across the boundary follows standard Tambara
module composition: `$t[0] .name` (Lens ∘ Lens = Lens),
`$m['key'] .name` (AffineTraversal ∘ Lens = AffineTraversal).

**Traversing / Applicative** is the Tambara-module class
corresponding to Clarke's power-series action [Clarke,
`def:traversal`]. It is the class for van-Laarhoven-style
traversals `forall f. Applicative f => (a -> f b) -> (s -> f t)`,
and matches the Haskell `profunctors` library convention.

**Affine traversal** requires a cartesian-closed base category
and symmetric-monoidal-closed cocartesian structure [Clarke,
`def:affine`]. For psh's pure value category W this is
satisfied; the "Cartesian + Cocartesian" profunctor constraint
is sufficient for user-facing classification.

**Map type** gets two rows because the two views are
structurally different: a single-key bracket lookup
(`$m['key']`) is a partial projection — affine traversal, may
or may not hit. Iteration over all values (`.values`) is an
unconditional fold — a proper Traversal in the Applicative
sense.

**List element access** is an affine traversal, not a Lens,
because the index may be out of bounds. `$l[n]` returns
`Option(T)` — `some(value)` or `none`. This matches map key
lookup, which also returns `Option(V)`. The partiality is
inherent: `Int` does not encode bounds, and psh has no
dependent types to prove `0 ≤ n < len(l)` statically.

**`.fields` and `.values` on structs** are Getters (read-only,
always succeed, no update path) — not Lenses, because they
produce a new list, not a focus into the struct's storage.
Setting through `.fields` would not update the struct's
fields; it would replace a list that happens to contain
stringified field data. `.keys` and `.values` on maps are
also Getters: `.values` returns a `List(V)`, not a traversable
focus into the map. §8.5 classification: monadic (pure
positive-to-positive Kleisli maps in W), thunkable, central,
no polarity frame.

**Discipline/bracket evaluation order.** When bracket access
is applied to a disciplined variable — `$m['key']` where `m`
has a `.get` discipline — the evaluation order is: `.get`
fires on `$m` first (producing the Map value per CBV focusing
/ Prop 8550), then bracket projects from that value. Bracket
operates on the *value* produced by `.get`, not on the stored
slot. The discipline is transparent to bracket composition.


## References

[1] Tom Duff. "Rc — The Plan 9 Shell." 1990.
    `refs/plan9/papers/rc.ms` (with companion man page at
    `refs/plan9/man/1/rc`)

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

[9P] Plan 9 9P protocol. man pages section 5 in any Plan 9
    distribution; cited here as design inspiration for the
    coprocess conversation discipline, not for wire protocol.

[Honda98] Honda, Vasconcelos, Kubo. "Language Primitives and
    Type Discipline for Structured Communication-Based
    Programming." ESOP, 1998.

[CMS] Carbone, Marin, Schürmann. "A Logical Interpretation of
    Asynchronous Multiparty Compatibility." Proves the MCutF
    admissibility theorem: forwarders subsume classical
    coherence and capture all multiparty compatible
    compositions. Load-bearing justification for psh's star
    topology.
    `~/gist/logical-interpretation-of-async-multiparty-compatbility/`

[Clarke] Clarke, Boisseau, Gibbons. "Profunctor Optics, a
    Categorical Update." Compositionality, 2024.
    `~/gist/profunctor-optics/arxivmain.tex` (formal paper,
    primary citation for def-labels like `def:monadiclens`).
    `~/gist/DontFearTheProfunctorOptics/` is the three-part
    intuition introduction; read first, then Clarke for formal
    definitions.

[SPW] Spiwack. "A Dissection of L." 2014.
    `~/gist/dissection-of-l.gist.txt`

[Be] Haiku / BeOS. Application Kit, Interface Kit, I/O hierarchy.
    `reference/haiku-book/`

[SPEC] ksh26 Theoretical Foundation. `refs/ksh93/ksh93-analysis.md`

[SFIO] sfio Operational Semantics Reference.
    `refs/ksh93/sfio-analysis/README.md`

[SFIO-3] sfio Buffer Model.
    `refs/ksh93/sfio-analysis/03-buffer-model.md`

[SFIO-7] sfio Disciplines.
    `refs/ksh93/sfio-analysis/07-disciplines.md`

[VDC] psh VDC Framework Report. `docs/vdc-framework.md`

[CS] Cruttwell, Shulman. "A unified framework for generalized
    multicategories." Theory and Applications of Categories
    24(21), 2010, pp. 580–655. Introduces virtual double
    categories under their current name.

[Lei] Leinster. *Higher Operads, Higher Categories.* London
    Mathematical Society Lecture Note Series 298, Cambridge
    University Press, 2004. Introduces fc-multicategories
    (= virtual double categories).

[Bur] Burroni. "T-catégories (catégories dans un triple)."
    Cahiers de Topologie et Géométrie Différentielle
    Catégoriques 12(3), 1971, pp. 215–321. Original source
    (as "multicatégorie").
