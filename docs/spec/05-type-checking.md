# Type checking

## Syntax

The formal grammar and all syntactic decisions are in
`04-syntax.md`. This section summarizes the design rationale
that connects syntax to semantics.

rc's actual syntax is the baseline. Every convention from rc
is preserved unless explicitly departed from with justification.
The formal grammar in 04-syntax.md annotates each production
with its rc heritage or extension rationale.

Key syntactic decisions with semantic grounding:

- **`def` instead of `fn`** for command definitions. rc's `fn`
  was a misnomer — it defines a cut template, not a function.
  `def` names the sort honestly. Type/variable disambiguation
  in `def` names (`def List::length { }` vs `def x.set { }`) is
  by `::` vs `.`. See §Two kinds of callable.
- **`let` + lambda for functions.** Values in the value sort,
  first-class, with capture semantics. Lambdas use `|x| => expr`
  or `|x| { block }`; nullary is `| | => expr`. `let` is CBPV's
  μ̃-binder: it accepts any computation producing a value
  (pure, builtin call, pipeline, command substitution). See
  §Two kinds of callable.
- **rc parentheses** around conditions: `if(cond)`,
  `while(cond)`, `for(x in list)`, `match(expr)`.
- **`else` instead of `if not`.** Duff acknowledged rc's
  weakness here [Duf90, §Design Principles].
- **`match`/`=>` instead of `switch`/`case`.** rc's `case` arms
  are top-level commands in a list; psh's `match` uses structured
  `=>` arms with `;` separators. The operation is genuinely
  different. Patterns are constructor-shaped (`(h t)` for cons,
  `(a, b, c)` for tuple, `ok(v)` for sum, `Pos { x, y }` for
  struct destructuring). Pattern alternation uses `|` between patterns
  (ML/Rust convention, unambiguous inside match arms). Pure
  guards via `if(cond)` after the pattern — restricted to
  side-effect-free expressions (comparisons, arithmetic).
- **Two accessor forms: bracket and dot.** Both bind tightly
  to `$name` with no space required. Bracket `$a[i]` is
  projection by runtime value — tuples (`$t[0]`), lists
  (`$l[n]`), maps (`$m['key']`). Returns `Option(T)`. Dot
  `$x.name` is named field/method/discipline access.
  Concatenation uses explicit `^` only (`$stem^.c`). The `.`
  is always an accessor, never an implicit caret. Inside
  double quotes, `"$name.txt"` works naturally — `$name`
  terminates at `.` (not in `var_char`), and `.txt` is literal
  text. See §Accessors.
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
- **Two string forms: single and double quotes.** Single quotes
  are literal (no expansion): `'hello $name'` is the literal
  text. Double quotes interpolate: `"hello $name"` expands
  `$name`. Inside double quotes, `$var`, `$var[i]`, and
  `` `{cmd} `` are expanded; `\$` escapes the dollar sign.
  Dot accessors inside double quotes require explicit
  delimiting: `"${name.upper}"` — bare `"$name.txt"` is
  variable `$name` followed by literal `.txt` (`.` terminates
  the variable reference inside double quotes, matching the
  `var_char` boundary). Multi-element lists inside double
  quotes join with spaces (equivalent to `$"var`). `\`-escapes
  work in both forms. See 04-syntax.md §Quoting and §Backslash
  escapes.

  rc rejected double quotes because Bourne's double-quote rules
  were complex (interpolation of `$`, `` ` ``, `\`, `!` but not
  globs). psh's expansion model is simpler (no IFS splitting,
  no glob expansion inside quotes), so the "raft of obscure
  quoting problems" Duff cited [Duf90, §Design Principles] does
  not apply.


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
[BTMO23, §2], Curien-Herbelin [CH00]). Each rule carries a **mode
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
| Nil-coalescing `M ?? N` | [synth] | `Option(T) ?? T → T` — sugar for match |
| Prism preview `$r.ok` | [synth] | `T → Option(Payload)` — discipline method |
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
    let greet = |name| => "hello ${name.upper}"
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


