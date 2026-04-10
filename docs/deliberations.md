# psh: Active Deliberations

**Status:** Working document. Captures in-progress design decisions from
ongoing sessions that have not yet been applied to `specification.md`
or `syntax.md`. Items here are drafts — they may change, be rejected,
or be refined further before landing in the live docs.

Organized by status:
- **Decided (not yet applied)** — tentatively agreed, awaiting the
  moment we update the live specs
- **Open** — under active discussion, some sub-decisions made
- **Pending** — identified but not yet discussed in depth


## Decided (not yet applied)

### Lambda syntax: `|x| { body }` / `|x| => expr`

Replaces `\x => body`. Frees `\` for line continuation and other
escape uses. Rust-familiar.

    |x| => $((x * 2))                      # single expression form
    |x| { echo $x; return $x }             # block form
    |x y| => $((x + y))                    # multiple params
    | | => echo 'nullary'                  # no params

Both forms (`=>` for single expression, `{...}` for block) are
available. The pipe character inside a lambda parameter list is
unambiguous — `|` in value position cannot be a shell pipe.

### Backslash escape rules

`\<non-whitespace>` is a literal escape. `\<whitespace>` is trivia.

- `\<newline>` — line continuation (standard rc)
- `\<space>`, `\<tab>` — trivia (equivalent to the whitespace char)
- `\'` inside single quotes — literal single quote (nicer than rc's
  `''` convention)
- `\\` — literal backslash
- `\n`, `\t`, etc. — literal characters (NOT C-style escape sequences;
  `\n` is literal `n`, not newline)

If you need a real newline in a string, use a multi-line quoted string.

### `.get` discipline as `def`, not pure lambda

Previous spec claimed `.get` bodies are pure lambdas. This was
inconsistent — examples showed logging/tracing, which are effects.

`.get` bodies should be defined as `def`, allowing side effects
(logging, tracing, metrics), with constraints:

1. The body's return value is discarded. `$x` always evaluates to
   the stored value.
2. The body cannot modify the variable it's attached to (`x` is free
   in the body of `x.get` — no self-recursion).
3. Effects are limited to "observation effects" that don't change
   what subsequent shell expressions observe.

The roundtable's cross-coprocess discipline chain concern still
applies — impure `.get` that queries external mutable state can
create inconsistent reads across a single expression. This is a
documented caveat, not a prohibition.

Remove `cursor.refresh` from the spec — it was an invented workaround
for enforced purity, unnecessary now.

### `$#x` and `$"x` are parameter expansion destructors

Acknowledge in the spec that psh does have parameter expansion
operators — just in prefix sigil form rather than ksh93's suffix
form. `$#list : List → Int` (length), `$"list : List → Str` (join).
These are eliminators for the List type.

The existence of these affects how we think about the general
destructor notation problem.


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


## Items Lane needs to share

- **Lists as recursive structs** — Lane has "something to show" that
  may unify several treatments. Blocks finalization of the accessor
  notation and struct system.

- **A larger report** that may change how we proceed.
