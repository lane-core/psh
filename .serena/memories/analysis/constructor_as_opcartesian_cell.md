---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [constructor, struct, opcartesian-cell, composite, universal-property, VDC, fcmonads, data-codata, tuple, record-literal, brace-construction, operadic-composition]
agents: [vdc-theory, psh-sequent-calculus, psh-optics-theorist, psh-architect]
related: [decision/struct_record_literal_construction, decision/tagged_construction_uniform, decision/three_roles_of_parens, analysis/three_sorts, analysis/data_vs_codata, analysis/oblique_maps, reference/papers/fcmonads]
verified_against: [/Users/lane/gist/fcmonads.gist.txt@HEAD §5 Def 5.1 line 3957 lines 3999-4005 Theorem 5.4 line 4037, docs/spec/@HEAD §Structs, docs/vdc-framework.md@HEAD §5.4 line 443]
---

# Constructor as opcartesian cell

## Concept

A `struct` constructor in psh registers an **opcartesian cell**
in the VDC framework — the cell that witnesses a named composite
of the struct's field types' horizontal arrows. From
`docs/spec/` §Structs: "a struct declaration specifies
a cell with a fixed multi-source signature. `Pos : Int, Int → Pos`
says the constructor cell has two `Int` horizontal arrows on top
and one `Pos` horizontal arrow on the bottom. The named accessors
are destructor invocations — the codata view of the struct, dual
to the constructor's data view."

The universal property: by Cruttwell-Shulman §5 Def 5.1
(`~/gist/fcmonads.gist.txt:3957`), an opcartesian cell with top
boundary `(p₁, p₂, …, pₙ)` exists when every cell with that same
top boundary factors **uniquely** through it. For `struct Pos {
x: Int; y: Int }`, the constructor cell is the cell that every
two-Int-source cell must factor through. This is what makes the
struct declaration more than a syntactic shorthand — it is a batch
registration of a new universal factorization target in the VDC.

**Universality survives the brace record literal form.** The
declaration fixes the canonical field order and the field names.
Every brace literal `{ x = 10; y = 20 }` elaborates to the same
underlying cell regardless of the user's ordering at the
construction site: `{ y = 20; x = 10 }` and `{ x = 10; y = 20 }`
both normalize to the positional tuple in declaration order. The
cell is unique per struct type, and the record literal is a
surface syntax for producing that cell under check-mode (the
expected type from context determines which struct declaration
drives the elaboration). There is no second cell — field-order
flexibility at the literal is a parser-level normalization, not a
second factorization path.

The data/codata duality completes the picture. Constructor =
opcartesian cell (positive, data — see `analysis/data_vs_codata`).
Accessors `.x`, `.y`, `.0`, `.1` = factorizations through it
(negative, codata). The record literal at the construction site
is the syntactic entry point to the cell; the accessors are the
syntactic exit points.

**Scope — data layer only.** This anchor covers the **data layer**
of horizontal arrows (words, lists, tuples, structs). The
**channel layer** (pipes, fds, coprocess sessions) is a separate
horizontal-arrow classification — see
`analysis/wire_format_horizontal_arrow` and
`analysis/fvdbltt_protypes`. The opcartesian-cell story is
data-layer-specific.

## Foundational refs

- `reference/papers/fcmonads` — Cruttwell-Shulman §5 "Composites
  and units" (`~/gist/fcmonads.gist.txt` line 3930 onward). Def
  5.1 at line 3957 defines opcartesian cells with the uniqueness-
  of-factorization requirement. Units are the n=0 special case
  (lines 3999-4005). Theorem 5.4 at line 4037 gives the pseudo-
  double-category characterization.
- `docs/vdc-framework.md` §5.4 "Cells = Commands" (line 443) —
  psh's VDC mapping where commands are cells and constructors are
  cells with multi-source top boundaries.
- `docs/spec/` §Structs "In VDC terms" paragraph —
  psh's own framing of the constructor as a cell.
- `analysis/data_vs_codata` — the data/codata duality that
  constructor-as-opcartesian and accessor-as-factorization
  instantiate.
- `analysis/three_sorts` — the surrounding sort structure in
  which the composite sits.

## Spec sites

- `docs/spec/` §"Structs (named products, ×)" —
  authoritative struct treatment.
- `decision/struct_record_literal_construction` — the design
  decision for construction form. Brace record literal under
  check-mode; order-independent at the surface; canonical
  cell per struct type.
- `decision/tagged_construction_uniform` — the uniform `NAME(args)`
  rule, applied to enum variants and collection-shaped builtins.
  Struct construction is **not** in this family.
- `decision/three_roles_of_parens` — list (sequence) vs tuple
  (composite) distinction for bare-parens forms.

## Status

Settled. The universality argument is a structural fact about
psh's VDC instance: the canonical form from the declaration gives
one cell per struct type, and surface-level ordering at the
construction literal is normalized by the checker before it
reaches the cell layer.
