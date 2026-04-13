---
type: decision
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [struct, record-literal, brace-construction, named-fields, bidirectional-check, check-mode, nominal-product]
agents: [psh-architect, psh-sequent-calculus, psh-optics-theorist, vdc-theory]
related: [decision/tagged_construction_uniform, decision/three_roles_of_parens, analysis/data_vs_codata, analysis/constructor_as_opcartesian_cell]
verified_against: [docs/spec/@HEAD §Structs]
---

# Decision: struct construction uses brace record literal with named fields

## Decision

Struct values are constructed via a **brace record literal** with
named fields:

```
struct Pos { x: Int; y: Int }

let p : Pos = { x = 10; y = 20 }
```

The declaration uses brace with `field: Type` entries separated by
`;`; construction uses brace with `field = value` entries separated
by `;`. The two forms mirror each other exactly — `:` for field
type in declarations, `=` for field value in construction.

Struct construction is a **check-mode** expression under psh's
bidirectional type checking: the record literal carries no type-
level information at its surface, and the struct type is
determined by the expected-type context (annotation, parameter
type, return type, match arm). A bare `let p = { x = 10; y = 20 }`
with no annotation is a type error at the binding site.

## Why

- **Notation mirrors declaration.** The same brace/`;`/named-field
  grammar appears at both declaration and use sites. Readers learn
  one rule.
- **Named fields document intent at the construction site.** A
  reader of `{ x = 10; y = 20 }` sees which field gets which
  value without needing to consult the struct declaration.
- **Order-independence.** Field order in the literal does not
  matter; the declaration carries the canonical order.
- **Typo detection at parse time.** Missing fields and misspelled
  field names are caught by the checker immediately.
- **Name punning.** When the variable name matches the field name,
  the `= NAME` part may be elided: `{ x; y }` is sugar for `{ x =
  x; y = y }`.
- **Honest about structure.** Struct fields are positional-
  heterogeneous (tuple-shaped), and under the "don't lie with
  syntax" principle the notation should not imply the homogeneous
  list shape that space-delimited parens `(a b c)` carries at the
  term layer.

## Consequences

- There is no `Pos(10, 20)` tagged construction form for structs
  and no `Pos.mk(10, 20)` auto-generated constructor function.
  The brace record literal is the sole construction form.
- Tuples and structs are **disjoint types** even when they share
  representation. `(10, 20) : (Int, Int)` is always a tuple, never
  coerced to a `Pos`. To cross the boundary, the user writes
  `{ x = $t .0; y = $t .1 }` explicitly.
- Struct construction requires type context (annotation, return
  type, parameter type, match arm scrutinee). Under-determined
  bindings are type errors.
- The struct declaration still auto-generates named accessors
  `.x`, `.y` and positional accessors `.0`, `.1` in the per-type
  namespace.
- Pattern matching uses a brace record pattern symmetric with
  construction: `match ($p) { { x = 0; y = 0 } => ...; { x; y }
  => ... }`.
- Pattern let accepts struct patterns for destructuring:
  `let { x = px; y = py } = $p` or `let { x; y } = $p`.
- Whole-struct replacement is the mutation pattern via `let mut`:
  `p = { x = 30; y = $p .y }`. No field-level mutation sugar.
- Every record type requires a `struct` declaration. There are no
  anonymous records — the brace literal always references an
  expected nominal type.
- **VDC universality is preserved.** The struct declaration fixes
  the canonical field order and names; every valid brace literal
  elaborates to the same underlying constructor cell regardless
  of the user's field ordering in the literal. The opcartesian
  cell per struct type remains unique (see
  `analysis/constructor_as_opcartesian_cell`).

## Spec

`docs/spec/` §"Structs (named products, ×)".
Grammar in `docs/spec/04-syntax.md` §Bindings (struct_decl) and §Values
(record_lit).
