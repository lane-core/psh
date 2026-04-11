---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-11
importance: high
verified_against: [/Users/lane/gist/fcmonads.gist.txt@2026-04-11 §1-§11 section headers, restrictions location]
keywords: [fcmonads, cruttwell, shulman, virtual-double-category, vdc, generalized-multicategory, multicategory, composite, segal-condition, restriction, virtual-equipment]
agents: [vdc-theory, psh-architect]
---

# Reference: fcmonads paper (Cruttwell-Shulman)

**Path.** `/Users/lane/gist/fcmonads.gist.txt`

**Citation.** G.S.H. Cruttwell and M.A. Shulman, *A Unified Framework for Generalized Multicategories*, Theory and Applications of Categories 24(21), 2010, pp. 580–655.

**Status.** Primary theoretical reference. **Mathematical foundation for VDCs.** The paper that gives Virtual Double Categories their current name (after Burroni's 1971 "T-catégories" and Leinster's "fc-multicategories").

## Summary

A virtual double category has four kinds of data: objects, vertical arrows (compose strictly), horizontal arrows (do **not** necessarily compose), and cells (mediate between a sequence of horizontal arrows on top and a single horizontal arrow on bottom).

**Key insight:** horizontal arrows don't compose, but cells do. The compositional structure lives at the level of cells (two-dimensional), not at the level of arrows (one-dimensional). A sequence of horizontal arrows is just a sequence — it is not required to have a composite.

Sections psh cares about (section structure verified against `/Users/lane/gist/fcmonads.gist.txt` on 2026-04-11):

- **§2** "Virtual double categories" (line 2312, formal definition at line 2422). Load-bearing for any well-definedness check. Horizontal arrows are introduced at line 2432 as part of the §2 definition.
- **§3** "Monads on a virtual double category" (line 2776). The monad-theoretic side of the framework — where psh's effect monad Ψ would sit in the VDC.
- **§5** "Composites and units" (line 3930). When sequences of horizontal arrows have composites (opcartesian cells). Relevant to pipeline fusibility. **Note:** the "Segal condition" framing of pipeline fusibility is psh's own — see `docs/vdc-framework.md` §5.6 / §9.4. fcmonads itself has **zero** mentions of "Segal" (verified by grep on 2026-04-11).
- **§7** "Virtual equipments" (line 4472). VDCs with enough restriction structure to serve as a type-theoretic setting. **Restrictions** (vertical arrows acting on horizontal arrows) are introduced inside §7 around line 4541, not in §6 as a prior revision of this memo claimed. psh interprets restrictions as interface transformations on channel types (redirections, namespaces); psh's framework is a virtual equipment.

**Other section headers in the gist** (for reference): §1 Introduction (1861), §4 Generalized multicategories (3144), §6 $2$-categories of $T$-monoids (4283), §8 Normalization (5018), §9 Representability (5439), §10 Composites in $\Mod$ and $\HKl$ (5983), §11 Comparisons to previous theories (6253).

## Concepts it informs in psh

- **Virtual double category structure** — `docs/vdc-framework.md` §4 "Virtual Double Categories" reproduces the key definitions. psh's framework is a VDC instance where objects are process interfaces, vertical arrows are interface transformations (redirections), horizontal arrows are channels (pipes, argument lists, signals), cells are commands.
- **Cell composition** — operadic structure. `docs/vdc-framework.md` §5 "The Mapping §Cell Composition = Command Substitution and Piping".
- **Segal condition** — pipeline fusibility. `docs/vdc-framework.md` §5.6, §9.4.
- **Restrictions** — interface transformations. `docs/vdc-framework.md` §5.3 "Vertical Arrows = Interface Transformations".
- **`decision/coprocess_9p_discipline`** — coprocess channels as horizontal arrows carrying session types; shell as forwarder is a cell composition.

## Who consults it

- **vdc-theory agent** (primary, canonical). Read after the duploids paper in a new session.
- **psh-architect** (secondary): for the well-definedness arguments behind the AST's sort structure.

## Note

Related reading: Leinster, *Higher Operads, Higher Categories*, LMS 298 (2004) introduces fc-multicategories = VDCs. Burroni 1971 is the original "T-catégories" source. The fcmonads paper is the most accessible modern treatment and the one psh cites.
