# psh: Active Deliberations

**Status:** Working document. Captures in-progress design decisions.
Items marked **applied** are now in `specification.md` or `syntax.md`;
items marked **open** or **pending** are staged for later work.

Organized by status:
- **Applied** — landed in the live specs; kept here as a changelog until
  we archive them.
- **Open (pending VDC reframing)** — direction confirmed but presentation
  is waiting on the broader spec restructure.
- **Pending** — identified but not yet discussed in depth.


## Applied

The following items from a previous deliberation session have been
applied to the live specs.

### Lambda syntax: `|x| { body }` / `|x| => expr` (APPLIED)

Replaced `\x => body` with Rust-style `|x|` delimiters. Frees `\` for
line continuation. Both forms available:

- `|x| => expr` — single expression
- `|x| { block }` — block body
- `| | => expr` — nullary

The pipe character inside a lambda parameter list is unambiguous
because lambdas only appear in value position, where a leading `|`
cannot be a shell pipe. See commit e0ecaf5.

### Backslash escape rules (APPLIED)

`\<non-whitespace>` is a literal escape. `\<whitespace>` is trivia.
`\n` is literal `n`, not a C-style newline escape. See §Backslash
escapes in syntax.md and commit c1512db.

### `.get` discipline: codata model (SUPERSEDED → APPLIED)

**Initial applied version** (commit 6fbac31): `.get` as a `def`
with "return discarded, x free in body, side effects for
logging only" constraints. This was the conservative model.

**Superseded by the codata model** (commit 7afc97d): `.get` is
now the codata observer — the body computes the value seen by
the accessor, not just a hook whose output is discarded. `.set`
is the codata constructor. Both are `def` cells in Kl(Ψ). CBV
focusing is the reentrancy semantics: within one expression,
`.get` fires once per variable and the produced value is reused.
The polarity frame discipline prevents reentrant self-invocation
during the shift.

The `cursor.refresh` workaround is gone either way — it was
unnecessary once `.get` could compute values directly.

Live spec: `specification.md` §Discipline functions. Live
grammar: `syntax.md` §Discipline functions.

### `$#x` and `$"x` as parameter expansion destructors (APPLIED)

Documented that `$#x` and `$"x` are type-specific eliminators for the
List type — length and join destructors. psh uses the prefix-sigil
convention from rc, not ksh93's suffix form. See commit 30f5f6c.


## Open (pending VDC reframing)

These items reached tentative agreement but are waiting on the broader
spec restructure around the Virtual Double Category framework (see
`docs/vdc-framework.md`). The direction is confirmed; the final
presentation will happen as part of the VDC reframing.

### Integration of the VDC appendix

Lane also provided an appendix to the VDC report, "Integrating Rc and
Ksh93 in the Virtual Double Category Framework" (not yet saved as a
doc). The appendix extends the VDC framework with concrete guidance on
integrating features from both shells. Lane has reviewed it and
selected which parts to adopt:

**Adopt wholesale (framework-level):**
- The generalized Duff principle (§A.6.1): "structure is never
  destroyed and reconstructed" — covers lists, types, polarity,
  session protocols.
- The horizontal arrow discipline (§A.6.2).
- The polarity frame discipline (§A.6.3).
- The Segal condition as optimization guide (§A.6.4).
- Named cells over eval (§A.6.5).
- The decision procedure for new features (§A.5.4): value-level →
  monadic threading, computation-level → save/restore, boundary-
  crossing → polarity frame.
- The duploid composition laws table (§A.5).
- The correspondence table (§A.7) — cleaner than what we currently
  have in specification.md.

**Reconcile with existing decisions:**
- **Accessor notation** (§A.3.4): appendix uses `$config.db.host`
  (no space, no braces). Lane's decision stands — space required
  before postfix dot. Rewrite appendix examples to use our form.
- **Records / compound variables** (§A.3.2): **resolved.** Lane
  decided: no anonymous records, every record type requires a
  `struct` declaration. The conceptual framing (a struct value
  occupies one element of a containing list) is adopted; the
  literal syntax question is avoided entirely. Tuples `(10, 20)`
  for "quick pair," named structs `Pos(10 20)` for "real record."
  No middle ground.
- **Discipline function semantics** (§A.3.3): appendix treats
  disciplined variables as codata — `.get` computes the value seen
  by the accessor. Our session decision was more conservative: `.get`
  is a def whose return is discarded. Lane is **willing to see what
  happens with the codata model now that we have the VDC framework
  as theoretical scaffolding**. The polarity frame discipline is
  supposed to prevent the bug class that made us conservative. The
  session's constrained-def model is **superseded by the codata
  model** pending verification in the restructured spec.
- **Type annotations AND the everything-is-a-list model** (§A.3.1):
  appendix uses `count : int = (0)`. Lane committed to both the
  annotation syntax and the underlying model: **every variable holds
  a list. A "scalar" is a list of length 1.** `$#count` for `count
  = (0)` is 1. Type annotations refer to element types — `: Int`
  means "list whose elements are Int." Length is a runtime property,
  not part of the type. `let x : Int = 42` is sugar for `let x : Int
  = (42)`. Substitution always splices a list (rc's exact model).
  Tuples, sums, and structs remain distinct types at the element
  level — they can appear inside the list. `let pos : Tuple = (10,
  20)` is a list containing one tuple, `$#pos` is 1.

  **Lane also noted: the current implementation is not sacred. We
  can scrap and rebuild from the parser up.** This frees the spec
  from compatibility constraints when applying the architectural
  decisions.
- **`eval` as escape hatch** (§A.3.6, §A.6.5): appendix retains
  `eval` as the explicit "force the Segal condition" escape hatch.
  Lane: **include it for now**. Easy to remove later if unused.
- **Name references**: appendix proposes `ref = *target` with no
  stated reasoning. Lane's decision: **stick with our `ref name =
  target` keyword form**.
- **Coprocess harmonization**: appendix treats coprocesses as
  "bidirectional horizontal arrows carrying session types" at the
  framework level. Our session-specific design (9P-shaped, tagged,
  PendingReply) is compatible but more detailed. **Need to harmonize
  our design with the VDC framework view. See research memo.**
- **Signal handling**: appendix implicitly endorses rc-style `fn
  sigint { ... }`. Our current spec has lexical `trap SIGNAL { }
  { }`. **Open — see research memo.**


## Resolved design areas (pending propagation to live specs)

These areas had open sub-questions in earlier sessions. All
sub-questions are now resolved. The decisions below are ready
to fold into the restructured specification.md — some have been
partially propagated, others await the restructure.

### Accessor notation: copattern-style postfix dot

**Direction:** abandon `${x.op}` (ksh93-derived brace form) for a
postfix dot notation inspired by Agda copatterns. Each type has a
namespace of accessors; users can extend it with `def Type.ident`.

    $pos .0                # tuple projection
    $pos .1
    $name .upper           # string method (nullary)
    $name .split ':'       # string method (parameterized, curried)
    $items .length         # user-defined accessor

**Decisions so far:**

- **Partial access returns option sums.** Out-of-bounds or wrong-tag
  accessors return `none()`; successful accesses return `some(v)` or
  the plain value (TBD whether total accessors also wrap in option).
  Users pattern-match on the option.

- **Space required before postfix dot.** `$x .0` is accessor, `$x.0`
  is free caret (`$x ^ .0`). Unambiguous parsing, no type-level
  disambiguation needed.

- **Global, last-wins scoping for user accessors.** `def List.length`
  registers `.length` on List for the rest of the session.
  Re-definition replaces the earlier one. Documented.

- **Parameterized methods are curried.** `.split` returns a lambda
  bound to the receiver, which is then applied to the argument:

      let by_colon = $name .split       # bound method, lambda waiting for arg
      let parts = $name .split ':'      # applied immediately

  0-argument methods return the value directly (no lambda wrapper).

**Resolved sub-items:**

- **Type name vs variable name in `def`** — **capitalization
  convention.** Type names start with an uppercase letter
  (`List`, `Str`, `Tuple`, `Pos`). Variable names start with a
  lowercase letter. The parser distinguishes by inspecting the
  first character before the dot:
  - `def x.set { }` — lowercase `x`, discipline function
  - `def List.length { }` — uppercase `List`, type method
  
  No keyword needed, no context-sensitive lookup. The convention
  is already implicit in the spec (the primitive types are
  capitalized, user variables are lowercase). Make it explicit.

- **Lists as recursive structs** — **resolved by the VDC
  reframing.** Lists are not recursive structs; they are the
  primitive sequence structure on cell boundaries. See the VDC
  framework (`docs/vdc-framework.md`) and the "Foundational
  commitment: every variable is a list" section in
  specification.md. Accessor notation `$list .0`, `$list .1`
  operates on the primitive sequence structure, not on a
  recursive cons/nil decomposition.

### Pattern matching as principled constructor syntax

Patterns in match arms name shapes (constructors), not convenience
sequences. `(h t)` is the cons pattern (head, tail), not "match c or
h." Multi-pattern alternation needs separate syntax.

    match($x) {
        ()       => echo 'empty'
        (h t)    => echo 'head: '$h' tail: '$t
        ok(v)    => echo 'success '$v
        err(msg) => echo 'failure '$msg
        _        => echo 'other'
    }

**Resolved:** Pattern alternation is `|` in pattern position —
option (b), ML/Rust convention. `ok(v) | some(v) => handle $v`.
The pipe character is unambiguous inside `match` arms because
patterns appear before `=>`, syntactically distinct from pipeline
position. The parser is inside a match block reading patterns;
no pipeline can form there.

**Resolved:** Guards are deferred. Guards introduce a polarity
boundary inside pattern dispatch (the pattern is positive and
structural, the guard is negative and computational), adding
real implementation complexity. The workaround is `if` inside
the arm body, which is verbose but correct. When guards are
added later, the syntax will be `pattern if(cond) => body` — `if`
after the pattern, before `=>`, in parens per rc convention.

### Struct definitions

    struct Pos {
        x: Int
        y: Int
    }

    let p = Pos(10 20)     # positional constructor

Space-delimited positional arguments — treats the constructor input as
a programmable sequence. `Pos(10 20)` and `Pos($vals)` where
`$vals = (10 20)` behave uniformly. The tag determines the
interpretation.

**Resolved:**

- **Positional construction is the only form.** `Pos(10 20)` is
  the only struct constructor syntax — now and permanently.
  There is no named construction form planned (no
  `Pos(x: 10, y: 20)` ever). This is consistent with sum
  construction (`ok(42)`) and with the uniform tagged-
  construction rule `NAME(args)` where args is a space-delimited
  word list.

  Fields are ordered by declaration: `struct Pos { x: Int; y:
  Int }` fixes `x` at position 0 and `y` at position 1.
  Construction binds by declaration order. Arity mismatch is a
  binding-time error. The asymmetry between construction
  (positional only) and access (named `.x`/`.y` and numeric
  `.0`/`.1`) is intentional: construction is a single
  structural act, access is a repeated operation where names
  earn their keep.

- **Field access: auto-generate both named and numeric
  accessors** from the declaration. `struct Pos { x: Int; y: Int
  }` registers `.x`, `.y`, `.0`, and `.1` on the Pos type
  namespace. Named accessors are the primary form; numeric
  accessors are for generic programming (iterating over fields
  by index). Both are Lens projections in the optic hierarchy.

- **Struct fields are immutable by default. Mutation requires
  `mut`** and takes the form of whole-struct replacement:

        let mut p = Pos(10 20)
        p = Pos(30 $p .1)          # rebind with new value

  No field-level mutation syntax (`p.x = 30`) initially.
  Whole-struct replacement is consistent with the value model —
  structs are positive (inert data, Clone), and mutation means
  rebinding the variable. Field-level mutation sugar can come
  later as `p .x = 30` desugaring to whole-struct replacement —
  this is the Lens `set` operation.

- **Anonymous records are NOT added.** Every record type requires
  a `struct` declaration. `(10, 20)` (tuple, anonymous) handles
  the "quick pair" case; named structs handle the "real record"
  case. No middle ground. See the "Tagged construction: the
  uniform rule" section below for the full clarification.

- **`enum` (user-defined sum types) is deferred.** The `enum`
  keyword is reserved (alongside `type` and `struct`) but not
  implemented in v1. Built-in sum tags (`ok`, `err`, `some`,
  `none`) cover the common error-handling and optionality
  cases. User-defined sum types via `enum Color { red | green
  | blue }` or similar syntax are a future extension. For v1,
  users can emulate enums with convention-based string tags
  (`let color = 'red'`) and glob match arms, or with the
  built-in sum tags where appropriate. The reserved keyword
  prevents name collisions so the feature can be added later
  without breaking existing code.

### Signal handling: rc style vs lexical trap (RESOLVED)

Resolved by the unified `trap` decision in the "Signal handling and
coprocess/VDC harmonization: resolved decisions" section below.
Grammar: `trap_cmd = 'trap' SIGNAL (body body?)?`. Three forms
distinguished by block count: lexical (two blocks), global (one
block), deletion (no blocks). Precedence: innermost lexical > outer
lexical > global > OS default. See the resolved-decisions section
for full details.


## Resolved from Pending

### String manipulation as Str type methods

Resolved via the accessor direction. All string operations are
`def Str.name { }` methods — type-level methods on `Str`,
distinguished from discipline functions by the capitalization
convention (uppercase `Str` = type, lowercase variable = discipline).

    $name .length          # Int
    $name .upper           # Str
    $name .lower           # Str
    $name .split ':'       # List
    $name .strip_prefix '/tmp/'   # some(rest) or none()
    $name .strip_suffix '.txt'    # some(rest) or none()
    $name .replace 'old' 'new'    # Str

Partial operations (strip_prefix, strip_suffix) return option
sums. Total operations (upper, lower, replace, length) return
plain values. `.contains` is a predicate that returns a status
(usable in `if` position), not an option sum — users care about
the boolean, not a payload.

### Map type (associative arrays)

Add a `Map` type — resolved. Lists of tuples are structurally
adequate but ergonomically hostile (linear scan on lookup,
painful update syntax). Maps give O(1) lookup and natural
set/get methods.

**Literal syntax uses tagged construction**, consistent with
`Pos(10 20)` and `ok(42)`:

    let mut env : Map = Map()                         # empty map
    env = Map(('HOME' '/home/lane') ('PATH' '/bin'))  # literal
    echo $env .get 'HOME'                             # some('/home/lane') or none()
    env .set 'EDITOR' 'vim'                           # requires mut

`Map(...)` takes a list of pairs as arguments. Each pair is
itself a list of two elements (key and value). This falls out
of the uniform tagged construction rule — `Map` is a tag,
`Map(args)` commits to tagged construction, and the args are
space-delimited list-style. List splicing works uniformly:

    let entries = (('HOME' '/home/lane') ('PATH' '/bin'))
    let env = Map($entries)   # splices — Map receives 2 pair-args

Optic: AffineTraversal (partial — key might not exist).
`.get` returns an option sum, consistent with other partial
accessors. `.set` takes a key-value pair and updates. `.keys`
and `.values` return lists.

In the "every variable is a list" model, a Map value is a single
list element. `$#env` is `1` — the variable holds one map. A list
of maps is possible but unusual.

The "medium confidence" flag from earlier deliberations is
removed — the answer fell out of the tagged-construction
decision.

### Job control builtins

`fg`, `bg`, `jobs`, `wait` (with `-n` for any-child), `kill`.
These are implementation work, not design questions. The only
design decision: job IDs (`%1`, `%2`) are a new word form,
analogous to `$x` for variables. `%N` expands to the PID of
job N. This keeps builtins simple — `kill %1` is `kill` with
argument `%1`, which expands to a PID before the builtin runs.

### Here-string `<<<`

**Add it.** Trivial desugaring: `cmd <<< 'input'` is equivalent
to `echo 'input' | cmd`, more precisely a cell with an embedded
constant horizontal arrow on stdin (the same structure as a here
document, inline). No fork for echo — the shell writes directly
to the pipe.

### List indexing via accessor

Confirmed: `$list .0`, `$list .1`, etc. AffineTraversal — returns
`some(value)` or `none()` if the index is out of bounds. Replaces
rc's `$list(1)` (which was 1-indexed). psh uses 0-indexed to
match the tuple accessor convention.

**Multi-index sub-lists** (rc's `$list(2 1 2)` returning a
sub-list of elements at indices 2, 1, 2) are NOT provided as a
primitive. Use a `for` loop or a `.at` method returning a list of
selected elements if genuinely needed. The multi-index case is
uncommon enough that a loop suffices for v1.

### Generalized destructor notation (direction noted)

Direction confirmed by the VDC copattern accessor framework. The
canonical form is the accessor: `$x .length` and `$x .join` are
the per-type namespace destructors on the List type. `$#x` and
`$"x` are rc heritage sugar — specific prefix-sigil forms for two
particular List destructors.

**Resolution:** keep `$#x` and `$"x` as sugar aliases for
`$x .length` and `$x .join` respectively. The accessor form is
canonical; the sigil form is rc-faithful ergonomic shorthand.
When the restructured spec documents the List type's accessor
namespace, it should note the sigil aliases as equivalent shorter
forms. No other prefix-sigil destructors are added — `$#` and
`$"` are the only two inherited from rc.

### Practical concerns from verification round

Flagged by the roundtable verification pass. Status below:

- **Coprocess tag reuse after drop** — **RESOLVED** by the
  drop-sends-cancel-frame (Tflush equivalent) decision in the
  coprocess protocol section. The shell's internal tracking
  handles stale responses by cancelling on drop.

- **Malformed coprocess frame** — **RESOLVED.** `MAX_FRAME_SIZE`
  is 16 MiB. Any frame whose length prefix exceeds this bound
  is treated as a protocol violation: the coprocess channel is
  torn down, outstanding tags fail with error status, and the
  coprocess process is killed. This is a defensive constant to
  bound memory use against buggy or hostile peers; it is not a
  semantic limit on legitimate payloads. 16 MiB is large enough
  for any structured data a coprocess would reasonably
  exchange, small enough that a malformed frame claiming
  gigabytes of length is detected as a protocol violation
  rather than allocated.

- **Pipe deadlock on stderr** — genuine Unix hazard with no clean
  shell-level solution. A pipeline stage writing to both stdout
  and stderr can deadlock if the downstream consumer blocks on one
  while the buffer on the other fills. **Resolution: document as
  a known limitation.** psh does not attempt to auto-merge stderr
  into stdout or provide implicit buffering; the user is expected
  to manage this explicitly with `>[2=1]` or similar redirections
  when needed. This matches the behavior of every other Unix
  shell and is an inheritance of the Unix pipe model, not a psh
  design flaw.


## Resolved share items

Items that were flagged as "Lane has something to share" in earlier
sessions. Both have now been shared:

- **Lists as recursive structs** — Lane presented the VDC report
  which reframed lists as primitive sequence structure on cell
  boundaries, not recursive data types. See `docs/vdc-framework.md`.

- **A larger report** — the VDC report itself (saved as
  `docs/vdc-framework.md`) plus the integration appendix that
  Lane reviewed in this session.


## Signal handling and coprocess/VDC harmonization: resolved decisions

From two rounds of research memos and Lane's review. These decisions
are ready to fold into the restructured spec alongside the VDC
reframing. Core recommendations from the researcher's v2 memo, with
refinements and resolutions by Lane.

### Unified `trap` syntax

Grammar:

    trap_cmd = 'trap' SIGNAL (body body?)?

Three forms distinguished by presence/count of blocks:

- `trap SIGNAL { handler } { body }` — lexical (μ-binder scoped to
  the body). Inner traps shadow outer for the same signal. Handler
  may `return N` to abort the body with status N.
- `trap SIGNAL { handler }` — global (cell registered at the
  top-level object's signal interface). Persists until overridden.
- `trap SIGNAL` (with no block) — delete a global handler.

Parser disambiguation is LL(1) via peek after SIGNAL: `\n`/`;`/`}`
after SIGNAL means deletion; `{` means a block follows. This is the
same strategy rc uses for `fn name` (no body = delete). No `-d`
flag — flag form is ksh93 heritage, doesn't fit psh's
keyword-before-braces convention.

Signal masking uses an empty handler: `trap SIGINT { } { body }`
for lexical, `trap SIGINT { }` for global.

Precedence at signal delivery: innermost lexical > outer lexical >
global > OS default.

`EXIT` is an rc-derived artificial signal synthesized when the
shell is about to exit. Attributed to Duff's rc paper §22, not
Plan 9.

### Signal delivery model

Signals fire at **interpreter step boundaries**, which include:

1. Between-command points (between `;`-separated statements in a
   block).
2. Wake-from-block points during child waits.

The second case is load-bearing. The shell's main loop uses
`poll(2)` on both the child-status fd and the self-pipe. When a
signal arrives during a child wait, the self-pipe wakes the poll
loop. The shell handles the signal *before* resuming the wait. For
SIGINT specifically, the shell forwards the signal to the child's
process group (`kill(-pgid, SIGINT)`), giving the child a chance to
terminate, then resumes waiting.

The "between commands" model in the researcher's v2 memo is an
idealization that doesn't cover blocking waits. The corrected model:
signals are checked whenever the shell is about to block or resume
from a block.

### EINTR policy

Builtins retry on EINTR by default. External commands handle
EINTR themselves. If an external command exits nonzero due to
interruption, the status flows through `try` normally. This
matches POSIX convention for shell builtins and avoids spurious
`catch` triggers from transient EINTR returns.

### Signal interaction with try blocks

Four cases, all with precise operational behavior:

**Case 1: Signal between commands inside try.**

- (1a) Handler calls `return N`: try body terminates, status N
  propagates to catch.
- (1b) Handler does not return: execution resumes at next command,
  handler side effects have occurred, try continues normally.
- (1c) Handler calls `exit`: shell (or subshell) exits, EXIT
  handler fires during shutdown.

**Case 2: Signal interrupts a blocking builtin.** Builtin retries
on EINTR. Signal flag is still set. Handler fires at next
signal-checking point after the builtin completes.

**Case 3: Lexical trap inside try.** Trap handler fires first
(μ-binder, ⅋); if it returns a status, try inspects it (ErrorT, ⊕).
Clean composition because trap and try operate on different sorts.

**Case 4: Outer trap, inner try.** Outer trap fires at
signal-checking point. If it doesn't return, try continues and
inspects status normally. Handler and try-check fire in sequence
at the same signal-checking point.

### Coprocess protocol

**`print -p name 'request'` returns an Int tag.** Tags are plain
Ints identifying outstanding requests, not opaque handles. They
fit psh's "every variable is a list" model as a list of one Int.

**`let` binds effectful computations — decided.** `let` is
CBPV's μ̃-binder on `F(A)`, not just on `A`. The RHS of a `let`
binding may be any computation that produces a value, including
builtin calls and effectful expressions. No command substitution
ceremony needed.

    let tag = print -p name 'request'    # direct — print -p returns a value
    let reply = read -p name              # direct — read -p returns a value
    let files = ls *.txt                  # direct — builtin returns a list

This is Levy's original CBPV semantics: `let x = M` where
`M : F(A)` is standard monadic bind. `M` is evaluated (its
effects happen), the resulting value is bound to `x`, execution
continues. psh's earlier command-substitution workaround
(`let tag = `{ print -p name 'request' }`) was a symptom of
not having committed to this sort-system choice. Committing
removes the workaround.

**The sort system implications.** `let` no longer distinguishes
between pure value bindings and effectful computation bindings.
Both go through the same μ̃-binder. The RHS can be:

- A pure value: `let x = 42` (list of one Int, no effects)
- An effectful computation: `let tag = print -p name 'query'`
  (computation runs, returned value is bound)
- A command substitution: `let files = `{ ls }` (subprocess
  forked, stdout captured, bound — remains available as a
  mechanism for forking, but not required for binding a
  builtin's return value)

The distinction between pure and effectful RHS is carried in
the type system (CBPV's `F(A)` vs `A`), not in the surface
syntax. A `let` binding always extends Γ with a value; how
that value is produced is the RHS's business.

**For builtins:** `print -p`, `read -p`, and similar
value-returning builtins directly return values that can be
bound with `let`. The return type is part of the builtin's
signature. The previous "returns an Int tag as output on
stdout" framing is obsolete — there is no stdout involvement.
`print -p name 'query'` returns an Int (wrapped in a list per
the uniform model), and `let tag = print -p name 'query'`
captures it.

**For the spec:** the restructured specification.md should
present `let` in CBPV terms from the start: "`let` binds the
result of a computation." Pure values are a special case
(trivially thunkable computations). This is the cleanest
framing and matches the theory directly.

**Shell-internal PendingReply.** The shell tracks a `Vec<u16>` of
outstanding tags per coprocess. `read -p name` (no `-t`) reads
the oldest outstanding response (FIFO). `read -p name -t $tag`
reads a specific tag. Invalid or stale tags produce nonzero
status with a descriptive error. No user-visible linear handle.

**Negotiate validates protocol version only.** Tag 0 is reserved
for the negotiate exchange. Both sides send a version string
(`"psh/1"`). Mismatch kills the coprocess and returns nonzero
status. No fallback to untyped mode. Application-level type
mismatches surface as runtime errors on malformed responses.

**Wire format.** Length-prefixed binary frames:
`[4-byte LE u32 length][2-byte LE u16 tag][payload]`. This is
Duff's principle applied at the byte level — frame boundaries are
structural (length prefix), not content-scanned (no newline
delimiters). The same role that list boundaries play for argument
sequences.

### Discipline functions with coprocess queries (codata model)

`.get` disciplines may issue coprocess queries. The `.get` body
runs inside a polarity frame (§A.6.3 of the appendix). A `print
-p` / `read -p` pair inside the body is a ↓→↑ shift — same
pattern as command substitution, with the additional property
that the coprocess is stateful.

**Failure propagation:** if the discipline body fails (dead
coprocess, command error), the variable access itself fails
— producing empty value and nonzero status, same as failed
command substitution.

**Reentrancy guard as CBV focusing (not memoization).** Within
a single expression's evaluation, `$x` fires `.get` the first
time and produces a value; subsequent occurrences of `$x` in the
same expression use the already-produced value.

This is not memoization-as-optimization. It is the correct
focusing behavior of the focused sequent calculus: in CBV with
positive types, a producer is evaluated (focused) once and the
resulting value is used at each consumption site. The `.get`
discipline is a ↓→↑ shift from computation to value; once the
shift lands, the result is a value, and values in CBV are used
without re-evaluation.

Cross-variable consistency across a single expression follows
for free: each discipline-backed variable is computed at most
once per expression. Inconsistency between expressions remains
possible (the backing state can change), and is documented
behavior, not a bug.

### EXIT handlers in subshells

Each process has its own EXIT handler. `@{ ... }` is classical
contraction — the continuation is duplicated, each copy evolves
independently. The subshell is an independent cell with its own
signal interface.

**Inherited handlers are copies.** If the subshell inherits a
global `trap EXIT { handler }` from the parent, the subshell gets
a copy of that handler. Modifications to the handler inside the
subshell do not affect the parent's copy. This is the Plan 9
`rfork` model: the child gets a copy of the parent's namespace
(including signal handlers), and the two copies evolve
independently.

`exit` from a subshell terminates the subshell and fires the
subshell's EXIT handler. The parent's EXIT fires when the parent
terminates.


## Tagged construction: the uniform rule

psh has one syntactic form for constructing tagged values:
`NAME(args)`, where `NAME` is immediately followed by `(` (no
space). The parser commits to tagged construction on seeing the
`NAME(` token. What the tag resolves to — sum injection, struct
construction — depends on the declared type, not on the syntax.

| Declaration | Construction | What it produces |
|---|---|---|
| (built-in sum tags) | `ok(42)` | Sum value, tag `ok`, payload `42` |
| (built-in sum tags) | `none()` | Sum value, tag `none`, no payload |
| `struct Pos { x: Int; y: Int }` | `Pos(10 20)` | Struct value, type `Pos`, fields x=10 y=20 |
| `struct Rgb { r: Int; g: Int; b: Int }` | `Rgb(255 128 0)` | Struct value, type `Rgb`, fields by position |

The arguments inside the parens are **space-delimited** — standard
psh word list, not comma-separated. This means list splicing works
uniformly:

    let xy = (10 20)
    let p = Pos($xy)          # splices — equivalent to Pos(10 20)

    let code = 42
    let r = ok($code)         # equivalent to ok(42)

The argument sequence is structural: substitution splices without
rescanning (Duff's principle), and the constructor receives exactly
as many arguments as the list has elements. Arity mismatch (wrong
number of arguments for the struct's field count) is a binding-time
error.

### What `struct` does

A `struct` declaration does two things:

1. **Registers a constructor.** `Pos` becomes a valid tag in
   `NAME(args)` position.
2. **Registers named accessors.** `.x` and `.y` are auto-generated
   as Lens projections on `Pos`, alongside positional fallbacks
   `.0` and `.1`.

        struct Pos { x: Int; y: Int }

        let p = Pos(10 20)
        echo $p .x             # 10 — named accessor (auto-generated)
        echo $p .1             # 20 — positional accessor (auto-generated)

Without `struct`, you only get positional access (tuples). With
`struct`, you get both named and positional. The struct
declaration is a batch accessor registration — it saves you from
writing `def Pos.x { ... }` and `def Pos.y { ... }` by hand.

### Why no anonymous records

The appendix to the VDC report proposed an anonymous record
syntax: `(x 3 y 4)` for a record with fields x=3, y=4, no type
declaration required. psh does not adopt this, for three reasons:

1. **Accessor registration requires a type name.** Named accessors
   (`.x`, `.y`) are registered on a type (`def Pos.x { ... }` or
   auto-generated by `struct Pos`). An anonymous record has no
   type name, so there is nothing to register the accessors on.
   You would need a structural accessor mechanism that does not
   exist in psh's copattern model.

2. **Consistency with sums.** Sum values require a tag (`ok(42)`,
   not just `42`). Struct values should likewise require a tag
   (`Pos(10 20)`, not just `(10 20)`). The tag is what makes
   construction and pattern matching work — the tag identifies
   the shape.

3. **Tuples cover the anonymous case.** If you want a quick pair
   without declaring a type, use a tuple: `(10, 20)`. Tuples give
   positional access (`.0`, `.1`). If you need named access,
   declare a struct. This is a clean split: tuples for anonymous
   positional data, structs for named typed data.

### In VDC terms

Struct construction is a cell with a specific top boundary
signature. The struct declaration specifies the types of the
horizontal arrows in the constructor's multi-source:

    Pos : Int, Int → Pos

Two horizontal arrows of type `Int` on top, one horizontal arrow
of type `Pos` on the bottom. The constructor is the cell that
mediates. The named accessors (`.x`, `.y`) are destructor
invocations on the `Pos` type — the codata view of the struct,
dual to the constructor's data view.

The struct has both: data (constructed by `Pos(10 20)`) and codata
(destructed by `.x`, `.y`). This is the data/codata duality from
the sequent calculus, unified in a single type declaration. The
`struct` keyword is the syntactic form that batches the two views
together — registering the constructor (positive introduction)
and the projections (negative destructors) at once.


## The three roles of `()`: list vs tuple vs tagged construction

Inside parentheses, psh has three distinct interpretations
depending on what is outside the parens and what delimiter
appears inside:

| Form | Delimiter | Interpretation |
|---|---|---|
| `(a b c)` | space | List — free monoid, splicable sequence |
| `(a, b, c)` | comma | Tuple — single structured product value, fixed arity |
| `NAME(a b c)` | space (after tag) | Tagged construction — args consumed as a word list |

The rule is local: commas switch the mode. `(a b c)` is a list.
Adding commas makes `(a, b, c)` a tuple. Prefixing a tag makes
`NAME(a b c)` a constructor call that receives a list and
consumes it positionally against the tag's declared arity.

### Lists and tuples serve different purposes

A list is a **sequence of values** meant to be spliced, iterated,
or passed as arguments. A tuple is a **single structured value**
meant to be kept bundled and accessed by position. The comma
inside the parentheses is the mode switch between these two
readings.

Under the "every variable is a list" model, both store as lists
at the outer level, but the element types differ:

- `let xy = (10 20)` stores `[Int, Int]` — a list of two ints.
  `$#xy` is 2.
- `let xy2 = (10, 20)` stores `[Tuple(Int, Int)]` — a list of one
  tuple value. `$#xy2` is 1.

This is not an accident. It is the correct reading: a tuple is
one structured value, not two values grouped. The user who writes
the comma is telling the shell "keep this bundled."

### The practical consequence: lists splice, tuples do not

Tuples cannot be spliced into tagged construction. The asymmetry
is visible when constructing a struct from a tuple vs from a list:

    let xy = (10 20)          # list — two values
    let p1 = Pos($xy)          # works — list splices, Pos receives 2 args

    let xy2 = (10, 20)        # tuple — one bundled value
    let p2 = Pos($xy2)         # does NOT work — Pos receives 1 tuple, arity mismatch
    let p2 = Pos($xy2 .0 $xy2 .1)   # explicit destructure required

Users will hit this. A user who reaches for the tuple form because
the commas signal "this is a pair of things, not just two
arguments" then discovers that the structured form is the harder
one to use for construction. This is an ergonomic cost Lane has
accepted as the price of a clean type-theoretic distinction.

### Why the friction is kept

The friction reflects a real semantic difference between
sequences-of-values and single-structured-values. Lists are for
argument sequences — things that will be spliced, iterated, or
consumed positionally by a command or constructor. Tuples are for
structured values — things that will be kept bundled and accessed
by position. Tagged constructions take a sequence of arguments
and a tag.

Alternative designs considered and rejected:

- **Allow tuples to splice in tagged construction.** Define
  `Pos($tuple)` to desugar to `Pos($tuple .0 $tuple .1)` when the
  arity matches. Rejected because it requires context-sensitive
  parsing (the shell would need to know the type of `$tuple` at
  parse time to decide whether to splice), violating Duff's
  single-pass principle.

- **Drop tuples entirely, use lists for anonymous heterogeneous
  grouping.** Rejected because it loses a useful primitive for
  anonymous structured values — you'd have to declare a struct
  for every paired return or use weakly-typed lists.

Lane's position: accept the asymmetry, document it clearly, and
teach users that tuples are for bundling while lists are for
splicing. The workaround (explicit destructure when passing a
tuple to a constructor) is honest about the distinction.

### Spec presentation

When the restructured spec documents tuples and lists, it should
include an explicit example of the splicing behavior difference,
with the framing that lists are for sequences and tuples are for
bundled values. Something like:

> Tuples and lists look similar but serve different purposes. A
> list `(a b c)` is a sequence of values meant to be spliced,
> iterated, or passed as arguments. A tuple `(a, b, c)` is a
> single structured value meant to be kept bundled and accessed
> by position. The comma inside the parentheses is the mode
> switch.
>
> The practical consequence: lists splice into tagged
> construction, tuples do not.
>
>     let xy = (10 20)          # list, spliceable
>     let p1 = Pos($xy)          # works — list splices to 2 args
>
>     let xy2 = (10, 20)        # tuple, bundled
>     let p2 = Pos($xy2)         # does not work — tuple is 1 arg
>     let p2 = Pos($xy2 .0 $xy2 .1)   # explicit destructure
>
> Use a list when you want to pass components as arguments. Use
> a tuple when you want to keep components bundled as a single
> value.
