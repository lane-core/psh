---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [postfix-dot, accessor, copattern, space-disambiguation, per-type-namespace, capitalization, lens, prism]
agents: [psh-architect, psh-optics-theorist, psh-sequent-calculus]
related: [decision/tagged_construction_uniform, decision/codata_discipline_functions]
---

# Decision: postfix dot accessors with required leading space, per-type namespace

## Decision

psh's accessor syntax is **postfix dot with a required leading space**:

```
$pos .0        # tuple projection — Lens
$name .upper   # type method — Str.upper
$result .ok    # sum preview — Prism
$count .get    # discipline function fire
```

The leading space is the disambiguator against rc's free caret concatenation (`$stem.c` with no space is `$stem ^ .c`).

Per-type namespaces via `def Type.ident { body }`. Capitalization disambiguates: **uppercase Type** registers a method on the type (`def Str.length`); **lowercase variable** registers a discipline on an individual variable (`def count.set`).

## Why

The postfix-dot form is **copattern-style** — a form from the sequent-calculus literature where codata observers are applied postfix to the value being observed. This matches psh's data/codata duality: accessors on positive types are Lens/Prism projections (data eliminators), and accessors on variables with disciplines are codata observers.

The required leading space is a deliberate lexical rule that avoids parse-time type lookup. Without the space, `$stem.c` could be either a field access or a free caret concatenation, and disambiguating would need type information. With the space, `$stem .c` is unambiguously an accessor and `$stem.c` is unambiguously rc-style concatenation.

Capitalization disambiguation for per-type namespace: `Type.name` vs `var.name` is a pattern from Smalltalk / ML tradition. Types are conventionally capitalized; variables conventionally lowercase; the parser uses the case of the identifier before the dot to decide which namespace the accessor lives in.

## Consequences

- `$pos .0 .1` = `first . second` left-to-right — ordinary function composition of profunctor optics.
- Struct declarations auto-generate both named accessors (`.x`, `.y`) and numeric accessors (`.0`, `.1`).
- Partial accessors return option sums: `$result .ok` gives `some(v)` or `none()` depending on whether the outer tag matches.
- Discipline functions like `def count.get { ... }` define codata observers on the variable `count`.
- Methods like `def Str.upper { ... }` extend the `Str` type uniformly across all Str-typed variables.
- The free caret rule from rc is preserved — `$stem.c` (no space) still concatenates.

Spec: `docs/specification.md` §"Syntax §Postfix dot accessors with required leading space", §"Tuples", §"Structs", §"Sums", §"Discipline functions". Ledger: `docs/deliberations.md` §"Accessor notation: copattern-style postfix dot".
