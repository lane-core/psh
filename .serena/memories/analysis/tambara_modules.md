---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [tambara, tambara-modules, profunctor, representation-theorem, pastro-street, optics, lens, prism, monoidal-actions, generalized-tambara, mixed-optics, optic-activation]
agents: [psh-optics-theorist, vdc-theory]
related: [reference/papers/profunctor_optics_clarke, analysis/monadic_lens, decision/postfix_dot_accessors]
verified_against: [/Users/lane/gist/profunctor-optics/arxivmain.tex@HEAD §sec:tambaratheory line 1850-1920, 488, 561-562, 326, 35, /Users/lane/gist/profunctor-optics/optics.bib@HEAD entry tambara06 line 357, audit/psh-optics-theorist@2026-04-11]
---

# Tambara modules (profunctor optics representation)

## Concept

A **Tambara module** is a profunctor equipped with a compatible action of a monoidal category — originally Daisuke Tambara's construction (Tambara 2006, cited in `~/gist/profunctor-optics/optics.bib:357` as `tambara06`). In Clarke-Boisseau-Gibbons (`~/gist/profunctor-optics/arxivmain.tex` §8 "Tambara theory" line 1850), Tambara modules are the **algebraic structure** that profunctor optics represent.

From the paper at line 488: "...algebraic structure called a **Tambara module**. Optics under this encoding are called **profunctor optics**." The Pastro-Street representation theorem (cited at `arxivmain.tex:561-562`) is the technical apparatus that characterises these profunctor optics as profunctor maps polymorphic over Tambara modules of a particular monoidal action.

A **lens** is a profunctor map polymorphic over **cartesian** Tambara modules. A **prism** is polymorphic over **cocartesian** Tambara modules. The various optic classes correspond to different choices of what Tambara structure the profunctor must respect. The paper's keywords (line 35) list "optics, lens, tambara module" together — the connection is foundational.

Clarke-Boisseau-Gibbons generalizes Tambara modules to **mixed Tambara modules** for mixed optics (`arxivmain.tex` §"Generalized Tambara modules" line 1912). This generalization is necessary for things like monadic lenses where the view and update operations live in different categories. From line 1898: "we need to go even further and generalize the definition of Tambara [modules]." The generalized form is what makes the optic activation table in `docs/spec/` §"Optics activation" (line 1313) coherent: each row's **profunctor constraint** (cartesian, cocartesian, monoidal, etc. — the spec table's actual column heading) corresponds to a different generalized Tambara structure. **Note:** the correspondence "row's profunctor constraint = generalized Tambara structure" is the optics-theorist reading per Clarke §8; the spec table itself lists profunctor constraints, not Tambara structures, and a careful reader should not mistake this anchor for a spec quotation.

**Why psh cares.** The optic classes psh exposes (Lens, Prism, MonadicLens, etc., per `analysis/monadic_lens`) compose because they are all profunctor maps factoring through the same generalized-Tambara-modules representation. When psh adds a new optic class to the activation table, the composition question reduces to "does its profunctor structure live in the same generalized Tambara framework." The Pastro-Street theorem is the technical apparatus that lets psh's optics activation be extensible without bespoke composition rules.

**psh's design only depends on the representation theorem holding** (which it does, by Pastro-Street + Clarke generalization), not on the deep theory of Tambara modules per se. Treat this anchor as the theoretical-grounding pointer; the operational consumer is the optics activation table.

## Foundational refs

- `reference/papers/profunctor_optics_clarke` — Clarke, Boisseau, Gibbons. **§8 "Tambara theory" at `~/gist/profunctor-optics/arxivmain.tex` line 1850** is the canonical treatment for psh purposes. Generalized Tambara modules at line 1912.
- The original Tambara paper (Tambara 2006, cited as `tambara06` in `optics.bib:357`) is **not vendored locally**. Clarke et al. is the proximate source.
- The Pastro-Street result that Clarke uses is cited at `arxivmain.tex:561-562` as "Pastro and Street on Tambara theory" — the original Pastro-Street paper is also not vendored.

## Spec sites

- `docs/spec/` §"Profunctor structure" line 696 — psh's profunctor optics framing.
- `docs/spec/` §"Optics activation" line 1313 — the activation table whose rows correspond to generalized Tambara structures.
- `analysis/monadic_lens` — the MonadicLens consumer of the generalized Tambara framework.
- `decision/postfix_dot_accessors` — the syntactic surface where the optic classes are exposed.

## Status

Settled. Tambara theory is the technical apparatus; psh's design depends on it operationally only via the activation table composing. When asked "why does psh's optics activation table compose," the answer is "every row is a profunctor map polymorphic over a generalized Tambara module structure, and Pastro-Street + Clarke says these compose." Read this anchor for the keyword landing; consult `reference/papers/profunctor_optics_clarke` directly for the technical details.
