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

### `.get` discipline as `def`, not pure lambda (APPLIED)

`.get` is now a `def` (effectful) with constraints:
1. Body's return value is discarded.
2. Body cannot modify the variable it's attached to.
3. Side effects permitted (logging, tracing, etc.).

Dropped the invented `cursor.refresh` workaround. See commit 6fbac31.

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
- **Records / compound variables** (§A.3.2): appendix proposes
  records as "a new kind of scalar — a single list element with
  internal structure," with `(x 3 y 4)` as the literal syntax. Lane
  finds this **interesting and wants to spend time on it**. The
  conceptual framing (a struct value occupies one element of a
  containing list) is compatible with our tuple/struct design, but
  the literal syntax differs. **Open — for further deliberation.**
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


## Open

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

**Open sub-items:**

- **Type name vs variable name in `def`.** `def x.set { }` is a
  discipline function on variable `x`. `def List.length { }` is a
  method on type `List`. How does the parser distinguish? Options:
  capitalization convention, explicit keyword, context-sensitive
  lookup. **On hold — Lane thinking.**

- **Lists as recursive structs.** The proposal treats lists as
  recursive structs with auto-generated numeric accessors. This
  gives both a pattern-matching view (cons/nil) and an accessor
  view (`.0`, `.1`, ...). **Lane has something to share before we
  finalize this.**

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

**Open:** Pattern alternation syntax.

- **(a)** No alternation — separate arms with same body
- **(b)** `|` in pattern position: `c | h => body` (ML/Rust convention)
- **(c)** `or` keyword: `c or h => body`

**Open:** Guards (predicates attached to arms) — include now or defer.

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

**Open:**
- Named construction (`Pos(x: 10, y: 20)`) — add now or defer?
- Field access: auto-generates `.x`, `.y` accessors from the
  declaration, plus numeric `.0`, `.1` fallback?
- Mutation: are struct fields mutable? How?

### Signal handling: rc style vs lexical trap

Current spec: `trap SIGNAL { handler } { body }` — lexically scoped
μ-binder. Principled but departs from rc.

rc uses `fn sigint { ... }` — a named function that fires on the
signal. Global, dynamic.

**Options:**
- Keep lexical `trap` (current)
- Restore rc's `def sigint { ... }` — global, dynamic, rc-familiar
- Both

**Lane has not weighed in yet.**


## Pending (not yet discussed in depth)

### String manipulation builtins

ksh93 had `${var#pattern}`, `${var%pattern}`, etc. psh should have
fork-free string operations. With the copattern accessor direction,
these become methods on `Str`:

    $name .length          # Int
    $name .upper           # Str
    $name .split ':'       # List
    $name .strip_prefix '/tmp/'   # some(...) or none()
    $name .replace 'old' 'new'    # Str

Details TBD.

### Associative arrays / maps

ksh93's `typeset -A` was heavily used. psh has no equivalent. Options:
- Add a `Map` type now
- Use lists of tuples `((k1 v1) (k2 v2))` with methods
- Reserved `struct` is close but not the same (structs have fixed
  fields; maps have dynamic keys)

Optic: AffineTraversal (partial lookup).

### Job control builtins

`fg`, `bg`, `jobs`, `wait` (with `-n` for any-child), `kill %N`.
Table stakes for interactive use. Need to spec the builtins and
their interaction with the job table.

### Here-string `<<<`

Trivial convenience: `cmd <<< 'input'` is `echo 'input' | cmd`.
Maps to `lmap` with constant input source. Add?

### List indexing via accessor

We agreed to move from rc's `$x(n)` to accessor notation. With the
copattern direction, this becomes `$list .0`, `$list .1`, etc.
AffineTraversal (partial — list might be shorter than the index).
Returns `option` sums per the partial access rule.

### Generalized destructor notation

The `$#x` / `$"x` sigil notation needs to unify with the postfix
accessor direction. Questions:

- Are `$#x` and `$"x` kept as special sigils, or replaced by
  `$x .length` and `$x .join` accessors?
- If kept, are they exceptions or part of a general prefix-sigil
  convention?

### Practical concerns from verification round

Flagged by the roundtable verification pass, not yet addressed:

- **Coprocess tag reuse after drop** — when `PendingReply` is dropped
  with a response in-flight, the drop path needs to drain-and-discard
  or send a cancel frame to prevent stale responses.

- **Pipe deadlock on stderr** — a pipeline stage writing to both
  stdout and stderr can deadlock if the consumer blocks on one while
  the buffer on the other fills. Real Unix hazard, not addressed.

- **Malformed coprocess frame** — length-prefixed frames with wrong
  length could hang the reader. Need timeout or max-frame-size guard.


## Resolved share items

Items that were flagged as "Lane has something to share" in earlier
sessions. Both have now been shared:

- **Lists as recursive structs** — Lane presented the VDC report
  which reframed lists as primitive sequence structure on cell
  boundaries, not recursive data types. See `docs/vdc-framework.md`.

- **A larger report** — the VDC report itself (saved as
  `docs/vdc-framework.md`) plus the integration appendix that
  Lane reviewed in this session.
