---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-11
importance: high
keywords: [bracket-accessor, postfix-dot, accessor, copattern, space-disambiguation, per-type-namespace, capitalization, lens, affine-traversal, bracket-dot-split]
agents: [psh-architect, psh-optics-theorist, psh-sequent-calculus]
related: [decision/tagged_construction_uniform, decision/codata_discipline_functions, analysis/monadic_lens, analysis/data_vs_codata]
---

# Decision: two accessor forms — bracket and dot

## Decision

psh has **two accessor forms** for projecting into values:

**Bracket `$a[i]`** — projection by runtime value. Tuples
(`$t[0]`), lists (`$l[n]`), maps (`$m['key']`). Returns
`Option(T)`. Binds immediately after `$name` with no space
(no-space rule mirrors rc's `$path(2)` subscript convention).

```
$pos[0]        # tuple projection — Lens (literal index only)
$list[n]       # list element — AffineTraversal
$m['key']      # map lookup — AffineTraversal
$list[1..3]    # slice — AffineFold
$list[-1]      # last element — negative indexing
```

**Dot `$x.name`** — named field/method/discipline access.
Binds tightly to `$name` — no space required (space is
optional). The `.` is always an accessor, never free caret.
Concatenation uses explicit `^` only: `$stem^.c`.

```
$s.x           # struct field — Lens
$name.upper    # type method — Str.upper
$count.get     # discipline function fire
$result.ok     # Prism preview — returns Option
```

Inside brackets is expression context (never glob). Tuple bracket
requires literal `Int` index (result type varies by position).
No `[*]`/`[@]` — psh's "every variable is a list" makes them
unnecessary.

Per-type namespaces via `def Type.ident { body }`. Capitalization
disambiguates: **uppercase Type** registers a method on the type
(`def Str.length`); **lowercase variable** registers a discipline
on an individual variable (`def count.set`).

## Why

The postfix-dot form is **copattern-style** — a form from the sequent-calculus literature where codata observers are applied postfix to the value being observed. This matches psh's data/codata duality: accessors on positive types are Lens/Prism projections (data eliminators), and accessors on variables with disciplines are codata observers.

As of 2026-04-12, the dot binds tightly — no space required.
`.` is always an accessor; concatenation uses explicit `^`
(`$stem^.c`). rc's free caret on `.` was an accident of the
general rule, not a deliberate design choice (Duff: "user
demand has dictated that rc insert carets"). psh claims `.`
for accessors as a first-class language feature.

Capitalization disambiguation for per-type namespace: `Type.name` vs `var.name` is a pattern from Smalltalk / ML tradition. Types are conventionally capitalized; variables conventionally lowercase; the parser uses the case of the identifier before the dot to decide which namespace the accessor lives in.

## Consequences

- Bracket and dot compose freely: `$t[0] .name` (Lens ∘ Lens),
  `$m['key'] .name` (AffineTraversal ∘ Lens). Standard Tambara
  module composition across the bracket/dot boundary.
- The bracket/dot split mirrors an index-vs-symbol selection
  boundary, not an optic class boundary. Both sides produce
  standard profunctor optics; the syntax determines how the
  optic is *selected* (runtime value vs static symbol).
- Struct declarations auto-generate named accessors (`.x`, `.y`),
  `.fields` (generic `List((Str, Str))`), and `.values` (typed
  `List(T)` on homogeneous structs only). No bracket positional
  access on structs.
- `m['key'] = v` on `let mut` maps desugars to
  `m = $m.insert 'key' v` (discipline-transparent).
- Discipline functions like `def count.get { ... }` define
  codata observers on the variable `count`.
- Methods like `def Str.upper { ... }` extend the `Str` type
  uniformly across all Str-typed variables.
- `.` is never free caret. Concatenation uses `^` explicitly.

Spec: `docs/spec/` §"Two accessor forms: bracket and
dot", §"Tuples", §"Structs", §"Map type", §"Optics activation".
Grammar: `docs/spec/04-syntax.md` §"Accessor syntax".
