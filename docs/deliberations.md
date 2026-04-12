# psh: Active Deliberations

**Status:** Working document. Captures in-progress design
decisions and resolved reasoning not yet fully propagated to
`specification.md` or `syntax.md`. Items are deleted from this
file once their content is in the spec.


## Pattern matching

Patterns in match arms name shapes (constructors), not convenience
sequences. Multi-pattern alternation uses `|` in pattern position
(ML/Rust convention):

    match($x) {
        ()       => echo 'empty'
        (h t)    => echo 'head: '$h' tail: '$t
        ok(v)    => echo 'success '$v
        err(msg) => echo 'failure '$msg
        ok(v) | some(v) => handle $v
        _        => echo 'other'
    }

**Resolved:** Guards are deferred. Guards introduce a polarity
boundary inside pattern dispatch (the pattern is positive and
structural, the guard is negative and computational), adding
real implementation complexity. The workaround is `if` inside
the arm body, which is verbose but correct. When guards are
added later, the syntax will be `pattern if(cond) => body` — `if`
after the pattern, before `=>`, in parens per rc convention.


## The three roles of `()`: list vs tuple vs tagged construction

Inside parentheses, psh has three distinct interpretations
depending on what is outside the parens and what delimiter
appears inside:

| Form | Delimiter | Interpretation |
|---|---|---|
| `(a b c)` | space | List — free monoid, splicable sequence |
| `(a, b, c)` | comma | Tuple — single structured product value, fixed arity |
| `NAME(a b c)` | space (after tag) | Tagged construction — enum variant args |

The rule is local: commas switch the mode. `(a b c)` is a list.
Adding commas makes `(a, b, c)` a tuple. Prefixing a tag makes
`NAME(a b c)` an enum variant constructor that receives a list
and consumes it positionally against the variant's declared
payload shape.

Tagged construction covers **enum variants only** — `ok(42)`,
`err('msg')`, `some(v)`, user-defined variants. Struct
construction uses brace record literal `{ x = 10; y = 20 }`.
Map construction uses brace map literal `{'key': v}`.

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

### Lists splice, tuples do not

Tuples cannot be spliced into tagged construction. The asymmetry
is visible when constructing an enum variant from a tuple vs
from a list:

    let args = (42 'reason')   # list — two values
    let r = err($args)         # works — list splices, err receives 2 args

    let pair = (42, 'reason')  # tuple — one bundled value
    let r = err($pair)         # does NOT work — err receives 1 tuple, arity mismatch
    let r = err($pair[0] $pair[1])   # explicit destructure required

The friction reflects a real semantic difference between
sequences-of-values and single-structured-values. Lists are for
argument sequences — things that will be spliced, iterated, or
consumed positionally. Tuples are for structured values — things
that will be kept bundled and accessed by position via bracket
`$t[0]`, `$t[1]`.


## Signal handling and coprocess harmonization

From two rounds of research memos and Lane's review. These
decisions are ready to fold into the restructured spec alongside
the VDC reframing.

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
builtin calls and effectful expressions.

    let tag = print -p name 'request'    # direct — print -p returns a value
    let reply = read -p name              # direct — read -p returns a value
    let files = ls *.txt                  # direct — builtin returns a list

**Shell-internal PendingReply.** The shell tracks a `Vec<u16>` of
outstanding tags per coprocess. `read -p name` (no `-t`) reads
the oldest outstanding response (FIFO). `read -p name -t $tag`
reads a specific tag. Invalid or stale tags produce nonzero
status with a descriptive error. No user-visible linear handle.

**Negotiate validates protocol version only.** Tag 0 is reserved
for the negotiate exchange. Both sides send a version string
(`"psh/1"`). Mismatch kills the coprocess and returns nonzero
status.

**Wire format.** Length-prefixed binary frames:
`[4-byte LE u32 length][2-byte LE u16 tag][payload]`. Duff's
principle applied at the byte level — frame boundaries are
structural (length prefix), not content-scanned.

### Discipline functions with coprocess queries

`.get` disciplines may issue coprocess queries. The `.get` body
runs inside a polarity frame. A `print -p` / `read -p` pair
inside the body is a ↓→↑ shift — same pattern as command
substitution, with the additional property that the coprocess is
stateful.

**Failure propagation:** if the discipline body fails (dead
coprocess, command error), the variable access itself fails
— producing empty value and nonzero status, same as failed
command substitution.

**Reentrancy guard as CBV focusing (not memoization).** Within
a single expression's evaluation, `$x` fires `.get` the first
time and produces a value; subsequent occurrences of `$x` in the
same expression use the already-produced value. This is the
correct focusing behavior of the focused sequent calculus.

### EXIT handlers in subshells

Each process has its own EXIT handler. `@{ ... }` is classical
contraction — the continuation is duplicated, each copy evolves
independently. The subshell is an independent cell with its own
signal interface.

**Inherited handlers are copies.** If the subshell inherits a
global `trap EXIT { handler }` from the parent, the subshell gets
a copy of that handler. Modifications to the handler inside the
subshell do not affect the parent's copy. This is the Plan 9
`rfork` model.

### Practical concerns

- **Coprocess tag reuse after drop** — resolved by the
  drop-sends-cancel-frame (Tflush equivalent) decision.

- **Malformed coprocess frame** — `MAX_FRAME_SIZE` is 16 MiB.
  Exceeding it is a protocol violation: channel torn down,
  outstanding tags fail, coprocess killed.

- **Pipe deadlock on stderr** — genuine Unix hazard with no clean
  shell-level solution. Documented as a known limitation. The user
  manages explicitly with `>[2=1]` or similar redirections.
