# Types

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
        |p| => Pos { x = $p.x + 1; y = $p.y }
    }

    # function argument — type explicit at the call site
    def distance : Pos -> Pos -> Int {
        |a b| => $(( abs($a.x - $b.x) + abs($a.y - $b.y) ))
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
    echo $p.x                # 10
    echo $p.y                # 20

Generic traversal:

    for (name, val) in $p.fields {
        echo $name '=' $val  # 'x = 10', 'y = 20'
    }

Homogeneous typed iteration:

    let vals = $p.values     # List(Int) = (10 20)

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
        Pos { x = x; y = y }   => echo $x $y;        # named
        Pos { _, y }            => echo 'y=' $y       # positional
    }

All declared fields must appear in named patterns (wildcards
`_` are fine for fields you don't care about).

**Pattern let** accepts struct patterns, binding multiple
names from a single destructuring:

    let Pos { x, y } = $p            # positional
    let Pos { x = px; y = py } = $p  # named, explicit binding names

**Mutation** requires `let mut`. Struct fields are immutable by
default; mutation takes the form of whole-struct replacement:

    let mut p = Pos { x = 10; y = 20 }
    p = Pos { x = 30; y = $p.y }

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

**Qualified variant syntax.** Variant names can be qualified
with their parent type using `::` (Rust convention):

    let r = Result::ok(42)
    let m = MenuResult::err('not interactive')
    match $choice {
        MenuResult::selected(v) => handle $v;
        MenuResult::cancelled   => ();
        MenuResult::err(e)      => echo $e
    }

Qualification is **never required** when the bidirectional
checker can resolve the variant from context. Bare `ok(42)` is
valid when the expected type pins the enum. Qualification is
available for:

- **Explicitness** — making code self-documenting when multiple
  enums share variant names (`err` on both `Result` and
  `MenuResult`).
- **Disambiguation** — when the checker cannot determine which
  enum a variant belongs to (rare, requires a context where
  two enums with the same variant name are both candidates).

When disambiguation is needed and the user writes a bare
variant, the checker produces a targeted error:

    error: ambiguous variant `err` — could be Result::err or MenuResult::err
      --> script.psh:12:5
       | match $x {
       |     err(e) => ...
       |     ^^^
       = help: qualify as Result::err(e) or MenuResult::err(e)

The `TYPENAME::` prefix is syntactic — it resolves at parse
time to the enum declaration in Σ. It is not a runtime
namespace lookup.

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
        ok(val)  => echo "got $val";
        err(msg) => echo "failed: $msg"
    }

    match ($c) {
        success(p)                        => echo "built: $p";
        warning(w)                        => echo "warning: $w";
        error(ErrorInfo { message = msg; line = ln }) => echo "error: $msg at $ln";
        error(ErrorInfo { message; line })            => echo "error: $message at $line"
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
type-prefixed brace form as struct construction (named and
positional).

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
error. Sort: consumer (the pattern match + else branch
is a single focused elimination with two arms.

**`if let`** — refutable pattern match in branch position:

    if let ok(v) = $result {
        echo "got $v"
    } else {
        echo 'failed'
    }

The complement of `let-else`: `if let` branches on pattern
match success, `let-else` branches on failure. The bound
variables are scoped to the success body only. The else body
is optional — without it, the `if let` is a conditional that
does nothing on pattern mismatch.

**`??` nil-coalescing operator** — extracts a value from
`Option(T)` with a default:

    $l[0] ?? 'default'       # value or default
    $m['key'] ?? ''          # value or empty string
    $result.ok ?? 0          # extract ok payload or default

Typing rule:

    Γ ⊢ M : Option(T) | Δ     Γ ⊢ N : T | Δ      [synth]
    ─────────────────────────────────────────
    Γ ⊢ M ?? N : T | Δ

Sugar for `match(M) { some(x) => x; none => N }`. Sort:
producer. RHS `N` is lazily evaluated (only when `M` is
`none`). Precedence: bracket > dot > `??` > caret. So
`$l[0] ?? 'default' .upper` = `$l[0] ?? ('default' .upper)`.

`??` is the primary ergonomic tool for the common "access
or default" pattern. For cases where the failure path needs
real logic, use `let-else` or `match`.

**Enum Prism previews** — `.ok`, `.err`, and user-variant
preview methods:

    $result.ok               # some(v) or none
    $result.err              # some(msg) or none
    $opt.some                # some(v) or none (identity on Option)

These are `def Result.ok`, `def Result.err` etc. — discipline
functions in the standard per-type namespace, returning
`Option(PayloadType)`. They compose naturally with `??`:

    $result.ok ?? 'fallback'    # extract ok or default
    $result.err ?? 'unknown'    # extract error or default

`match` remains the canonical form for multi-arm dispatch.
Prism previews are for the common one-variant extraction
case. Profunctor constraint on the Prism structure:
Cocartesian.

**Option Display convention.** `some(v)` displays as `v`'s
Display representation; `none` displays as the empty string.
This is a Display/toString convention on the type, not a
REPL special case — behavior is identical in scripts and at
the REPL. `echo $l[0]` prints `hello`, not `some(hello)`.
`echo $m['missing']` prints nothing. The typed value in the
pipeline is still `some('hello')` or `none` — pattern
matching, `??`, and conditionals see the full tagged value.
Debug/inspect output shows the full `some(...)` wrapping.
This parallels Rust's Display vs Debug distinction.

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
arms. Pure guards are thunkable, and thunkable ⇔ central by
the Hasegawa-Thielecke theorem [MMM, §9.6] — they compose
freely and backtrack safely because no state was modified.
The converse is the deeper justification: an effectful guard
is non-thunkable, hence non-central by HT contrapositive,
and its side effects interact with match dispatch in
composition-order-dependent ways — exactly the `(⊕,⊖)` non-
associativity failure at the guard boundary. Effectful
conditions belong in `if` inside the arm body, where the
effect is explicit and no backtracking occurs.

Enums are positive (value sort), admit all structural rules.
They are inert data — Clone, no embedded effects.


## Path (component sequences)

A path is not a string — it is a sequence of components with
structural boundaries. This is Duff's principle applied to the
filesystem: component boundaries are part of the data, not
recovered by scanning for `/`. A filename containing `/` in
one component cannot be confused with a directory separator,
for the same reason that a list element containing spaces
cannot be confused with a list separator.

**Representation.** Path is a list of typed components:

    Path = List(PathComponent)

    enum PathComponent {
        root;             # / (Unix root)
        parent;           # ..
        cur;              # .
        normal(Str)       # a directory or file name
    }

The leading `root` component distinguishes absolute from
relative paths. `..` and `.` are preserved as components —
normalization (resolving `..` against the parent) is a
filesystem operation, not a type-level operation. This matches
Rust's `std::path::Components` design: `..` components are
kept literal in the iterator, resolved only by `canonicalize`.

**Literals.** Path literals use filesystem syntax directly:

    /usr/bin/rc           # Path: (root, normal(usr), normal(bin), normal(rc))
    ./src/main.rs         # Path: (cur, normal(src), normal(main.rs))
    ../lib                # Path: (parent, normal(lib))

The parser decomposes path literals at parse time. The `/` at
the start is the root component; internal `/`s are component
separators. This parallels how `(a b c)` is parsed into a list
at parse time — the spaces are separators, not content.

**Interpolation.** `"$path"` joins with `/` to produce a Str.
This is the display/exec form — the string that goes to a
syscall. The component structure is the canonical
representation; the joined string is derived. Multi-element
path lists join each path with space (standard list
interpolation), then each path internally with `/`.

**Accessors.** Path has optic-backed accessors:

| Accessor | Optic class | Returns | Example |
|----------|-------------|---------|---------|
| `$p.parent` | Lens | Path (all but last component) | `/usr/bin/rc` → `/usr/bin` |
| `$p.name` | Lens | Str (last component) | `/usr/bin/rc` → `rc` |
| `$p.stem` | Lens | Str (name without extension) | `/src/main.rs` → `main` |
| `$p.ext` | AffineTraversal | Option(Str) (may not exist) | `/src/main.rs` → `some(rs)` |
| `$p.components` | Getter | List(Str) (component strings) | `/usr/bin` → `(usr bin)` |
| `$p[n]` | AffineTraversal | Option(PathComponent) (nth component) | `/usr/bin`[1] → `normal(bin)` |

**Join.** Path joining is component concatenation — the second
path's components are appended to the first:

    $dir / $file          # / as infix join operator on Path
    path.join($base $rel) # explicit builtin

If the right operand is absolute (starts with `root`), it
replaces the left operand entirely — matching Rust's
`PathBuf::push` semantics and POSIX path resolution.

**Relationship to Str.** Path is not a subtype of Str. The
conversion is explicit:

- Path → Str: interpolation (`"$path"`) or `str $path`
  (joins components with `/`)
- Str → Path: `path $str` builtin (decomposes on `/`)
  or path literal syntax

This prevents accidental treatment of paths as strings (which
would lose component boundaries) or strings as paths (which
would misparse embedded separators). The type system catches
`echo $path` (list splice — each component becomes a separate
argument) vs `echo "$path"` (interpolation — the joined string).

**Non-UTF-8 filenames.** On Unix, filenames are arbitrary bytes
(except NUL and `/`). psh's Str is UTF-8. Filenames that are
not valid UTF-8 produce a runtime error at the Str → Path or
Path → Str boundary, with a diagnostic naming the offending
bytes. This is a deliberate restriction: psh's value model is
string-based, and silently mangling non-UTF-8 filenames would
violate Duff's principle (structure should not be corrupted by
the representation). The 99.9% case (UTF-8 filenames on modern
systems) works cleanly; the edge case fails loudly.

**Classical zone.** Path is a value type, freely copyable and
discardable (`!Path` in the classical zone). No resource
semantics — a Path names a location but does not hold a handle.

Path is positive (value sort), admits all structural rules.
Inert data — Clone, no embedded effects.


## ExitCode and Status

### ExitCode — the error payload

ExitCode is a ground type representing a process termination
result. It carries both the POSIX numeric code and an optional
descriptive message:

    struct ExitCode {
        code : Int;        # 0-255 for external, wider for builtins
        message : Str      # descriptive error (empty on success or
                           # when unavailable from external commands)
    }

The `code` field is the POSIX reality: `waitpid` returns
0-255 for external processes. Shell builtins and `def`
functions may use the full Int range (ksh93 precedent: sh.1
lines 1699-1709), clamped to 0-255 at process exit boundaries.

The `message` field preserves rc's string-status heritage
[Duf90, §Exit status]: "On Plan 9 status is a character string
describing an error condition." On Unix, builtins populate the
message with descriptive errors (`'file not found'`,
`'permission denied'`). External commands leave it empty —
the POSIX interface provides no string channel. Signal death
synthesizes a message (`'killed by SIGTERM'`). The message is
informational, not load-bearing for control flow — conditionals
and `try`/`catch` inspect `code`, not `message`.

**Accessors:**

    $e.code       # Int
    $e.message    # Str

**Construction:** builtins and `exit` produce ExitCode values.
The user writes `exit 1` (bare code, empty message) or
`exit 1 'not found'` (code + message). External commands
produce ExitCode from `waitpid` with an empty message.

ExitCode is positive (value sort), classical zone (`!ExitCode`),
freely copyable and discardable. It is inert data — no resource
semantics.

### Status — the ⊕ connective

Status is the return type of every command. It is a genuine
two-case coproduct (⊕), not a bare integer with an implicit
predicate:

    Status = Result((), ExitCode)

- **Success:** `ok(())` — the command succeeded. No payload.
- **Failure:** `err(ExitCode)` — the command failed. The
  ExitCode carries the code and optional message.

This is the ⊕ from linear logic: constructors `Inl(t)` /
`Inr(t)`, eliminated by case dispatch. `try`/`catch` is the
coproduct elimination form:

    try { body } catch (e) {
        # e : ExitCode — the error payload
        echo $e.code $e.message
    }

Conditionals (`if`, `&&`, `||`) consume Status via ⊕
elimination — they inspect the tag (ok vs err), not a
numeric value. The shell convention "0 = truthy" is a
consequence of the ⊕ structure: success is the left injection
(the "continue" case), failure is the right injection (the
"abort/branch" case). **Bool never enters the picture at the
type level for command status.** If the user wants an explicit
boolean, `$status.is_success` returns Bool via a named
projection.

**`$status` and `$pipestatus`:**

    $status     : ExitCode       # most recent exit code
    $pipestatus : List(ExitCode) # per-component codes

`$status` holds the ExitCode from the most recent command.
On success, `$status.code` is 0 and `$status.message` is
empty. For a simple command (not a pipeline), `$pipestatus`
is `($status)` — a single-element list. For a pipeline,
`$pipestatus` holds the ExitCode of each component in order,
following bash/zsh convention with psh's native list type.

### In the VDC

ExitCode is the type of the bottom-boundary horizontal arrow
of every command cell. The co-Kleisli extract that `&&`/`||`
observe is `ε : W(Context) → ExitCode` — naming what was
previously implicit. `$pipestatus` is the sequence of
bottom-boundary ExitCode annotations from each cell in a
pipeline, naturally preserved because the VDC framework
maintains pipelines as sequences, not composites (Duff's
principle generalized: structure is never destroyed and
reconstructed).


## Stream(T) (typed pipe sessions)

`Stream(T)` is a recursive session type describing a
unidirectional stream of typed values — the session protocol
carried by typed pipes (§Features, "Typed pipes for def-to-def
composition").

**Definition.** `Stream(T)` is a recursive session type using
the internal/external choice operators from session type
theory [HVK98]:

    Stream(T) = μX. (Send<T, X> ⊕ End)

Read this: at each step, the producer either sends a value of
type `T` and continues with the protocol (recursion via `X`),
or closes the stream (`End`). The `⊕` is internal choice —
the producer decides which branch to take. `μ` is the
recursive session type constructor (least fixed point).

The consumer's dual type is obtained by involutive negation
`¬(−)` from the L-calculus [MMM, §9.3]:

    ¬Stream(T) = νX. (Recv<T, X> & End)

At each step, the consumer either receives a `T` and
continues, or acknowledges end-of-stream. The `&` is
external choice — the consumer must be prepared for either
branch. `ν` is the greatest fixed point (dual to `μ`). The
duality `¬¬Stream(T) ≅ Stream(T)` holds by involutivity.

**Operational correspondence.** `Send<T, X>` is a `write`
to the pipe fd. `Recv<T, X>` is a `read` from the pipe fd.
`End` is EOF (pipe closure). The session type describes the
protocol that the bytes on the pipe follow — it does not
change what the kernel transmits. A `Stream(Str)` pipe
carries newline-delimited strings; a `Stream(Int)` pipe
carries the string representation of integers. The
serialization boundary is Display/FromStr, same as rc's
text convention.

**Recursion.** `Stream(T)` is the first recursive type in
psh's type system. The recursion is **structural** — the type
unfolds to a fixed alternation of `Send` and `⊕`/`End`
choice. There is no arbitrary recursive type machinery; the
`μ`/`ν` constructors are restricted to session types on pipe
endpoints. User code does not write `μX. ...` directly —
`Stream(T)` is a named type constructor that encapsulates
the recursion, like `List(T)` encapsulates cons-cell
recursion at the implementation level.

**Sugar.** `|[T]` in the pipe operator is sugar for
`|[Stream(T)]`. The explicit form `|[Stream(T)]` is valid
and equivalent. The sugar exists because the streaming
protocol is the overwhelmingly common case for pipes, and
writing `Stream(...)` at every pipe site would be noise. The
explicit form exists for future extensibility — richer
session types on pipes (e.g., batched or windowed protocols)
can use the same annotation site.

**Typing rule** (stream introduction / producer):

    Γ ⊢ v : T | Δ          Γ ⊢ rest : Stream(T) | Δ     [synth]
    ────────────────────────────────────────────────────
    Γ ⊢ send(v); rest : Stream(T) | Δ

    ─────────────────── [synth]
    Γ ⊢ close : End | Δ

The producer builds the stream by sending values and
eventually closing. In practice, these are not explicit
`send`/`close` calls — they are `echo`, `print`, and pipe
closure. The typing rule describes the logical structure;
the evaluator maps it onto standard I/O operations.

**Zone.** `Stream(T)` as a type descriptor is classical
(`!Stream(T)`) — the type name is freely copyable and
usable in annotations. A *pipe endpoint* carrying a stream
is **affine** — consumed by the pipeline mechanism, not
directly manipulated by user code. Users do not hold
`Stream(T)` values; the type exists in the type system to
annotate pipe channels, not as a first-class value type.

**Not user-constructible.** Unlike `List(T)` or `Option(T)`,
`Stream(T)` values are not directly constructed by user code
with a literal syntax. They are produced by the pipeline
mechanism when a `def` writes to stdout. The `Stream(T)`
type appears in:

- Pipe operator annotations: `|[T]` or `|[Stream(T)]`
- Future: `def` signature stream type declarations
- Type-level reasoning about pipe compatibility

Stream(T) is positive (the data that flows on the pipe is
positive). The session type structure (the protocol state
machine) is a type-level property checked statically, not a
runtime representation.


