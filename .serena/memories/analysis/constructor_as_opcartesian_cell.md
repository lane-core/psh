---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [constructor, struct, opcartesian-cell, composite, universal-property, positional-only, VDC, fcmonads, data-codata, tuple, operadic-composition, load-bearing-commitment]
agents: [vdc-theory, psh-sequent-calculus, psh-optics-theorist, psh-architect]
related: [decision/struct_positional_only_forever, decision/tagged_construction_uniform, decision/three_roles_of_parens, analysis/three_sorts, analysis/data_vs_codata, analysis/oblique_maps, reference/papers/fcmonads]
verified_against: [/Users/lane/gist/fcmonads.gist.txt@HEAD §5 Def 5.1 line 3957 lines 3999-4005 Theorem 5.4 line 4037, docs/specification.md@HEAD §Structs lines 1194-1199 §1106-1203, docs/vdc-framework.md@HEAD §5.4 line 443, audit/vdc-theory@2026-04-11]
---

# Constructor as opcartesian cell

## Concept

A `struct` constructor in psh registers an **opcartesian cell** in the VDC framework — the cell that witnesses a named composite of the struct's field types' horizontal arrows. From `docs/specification.md` §Structs lines 1194–1199: "a struct declaration specifies a cell with a fixed multi-source signature. `Pos : Int, Int → Pos` says the constructor cell has two `Int` horizontal arrows on top and one `Pos` horizontal arrow on the bottom. The named accessors are destructor invocations — the codata view of the struct, dual to the constructor's data view."

The universal property: by Cruttwell-Shulman §5 Def 5.1 (`~/gist/fcmonads.gist.txt:3957`), an opcartesian cell with top boundary `(p₁, p₂, …, pₙ)` exists when every cell with that same top boundary factors **uniquely** through it. For `struct Pos { x: Int; y: Int }`, the constructor `Pos(_ _)` is — conditional on the positional-only commitment discussed next — the cell that **every** two-Int-source cell must factor through. This is what makes the struct declaration more than a syntactic shorthand — it is a batch registration of a new universal factorization target in the VDC.

**Universality is conditional on psh's positional-only commitment.** `decision/struct_positional_only_forever` is load-bearing here. If psh later added a named-field form like `Pos(x: 10, y: 20)`, there would be a **second** cell witnessing the same composite — field names and positions both factoring into the constructor — and the uniqueness-of-factorization clause of Def 5.1 would fail unless a new disambiguation rule were imposed. The "no named form, now or later" commitment preserves a theoretical invariant, not just an ergonomic preference. This is the settled content of the 2026-04-11 vdc-theory + sequent-calculus investigation — see the Provenance section at the bottom of this anchor for the full scope.

The data/codata duality completes the picture. Constructor = opcartesian cell (positive, data — see `analysis/data_vs_codata`). Accessors `.x`, `.y`, `.0`, `.1` = factorizations through it (negative, codata). List splicing into a constructor works because lists present *n* unit arrows to the multi-source (operadic slot-filling); tuple splicing fails because a tuple is **one** composite arrow and the constructor expects *n* slots — `Pos($tuple)` is arity-mismatch by construction, not by convention.

**Scope — data layer only.** This anchor covers the **data layer** of horizontal arrows (words, lists, tuples, structs). The **channel layer** (pipes, fds, coprocess sessions) is a separate horizontal-arrow classification — see `analysis/wire_format_horizontal_arrow` and `analysis/fvdbltt_protypes`. The opcartesian-cell story is data-layer-specific.

## Foundational refs

- `reference/papers/fcmonads` — Cruttwell-Shulman §5 "Composites and units" (`~/gist/fcmonads.gist.txt` line 3930 onward). Def 5.1 at line 3957 defines opcartesian cells with the uniqueness-of-factorization requirement. Units are the n=0 special case (lines 3999-4005). Theorem 5.4 at line 4037 gives the pseudo-double-category characterization.
- `docs/vdc-framework.md` §5.4 "Cells = Commands" (line 443) — psh's VDC mapping where commands are cells and constructors are cells with multi-source top boundaries.
- `docs/specification.md` §Structs "In VDC terms" paragraph (lines 1194-1199) — psh's own framing of the constructor as a cell. This anchor extends that framing with the universality claim and the load-bearing-on-positional-only note.
- `analysis/data_vs_codata` — the data/codata duality that constructor-as-opcartesian and accessor-as-factorization instantiate.
- `analysis/three_sorts` — the surrounding three-sort structure in which the composite sits.

## Spec sites

- `docs/specification.md` §"Structs" lines 1103-1200 — authoritative struct treatment.
- `decision/struct_positional_only_forever` — the design decision whose theoretical stake this anchor records. The positional-only-forever commitment is what makes the constructor universal. The decision memo has been updated to cross-reference this anchor.
- `decision/tagged_construction_uniform` — the uniform `NAME(args)` rule, of which the struct constructor is an instance.
- `decision/three_roles_of_parens` — list (sequence) vs tuple (composite) vs tagged construction distinction that the operadic reading depends on.

## Status

Settled conditionally on the positional-only commitment. **This anchor is the load-bearing rationale for the "forever" clause in `decision/struct_positional_only_forever`** — a future contributor tempted to revisit "why not named-field construction like `Pos(x: 10, y: 20)`?" should read this anchor and understand that the answer is VDC-level, not ergonomic. Adding a named form would break the universal property by creating a second factorization path; the universality would need re-verification against whatever new disambiguation rule was chosen.

**Provenance.** The universality claim and the "load-bearing on positional-only" framing were established during the 2026-04-11 vdc-theory + sequent-calculus investigation of Lane's three-observation research memo on psh's emerging type theory. The full investigation scope (Observations 2 and 3) was partially rejected; Observation 1 stands, and this anchor is the settled content from that observation.
