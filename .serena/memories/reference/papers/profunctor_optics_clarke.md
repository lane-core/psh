---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [profunctor-optics, clarke, boisseau, gibbons, tambara-module, representation-theorem, mixed-optics, monadic-lens, lens, prism, adapter, traversal, affine-traversal, grate, setter, getter, fold]
agents: [psh-optics-theorist]
---

# Reference: Profunctor Optics, a Categorical Update (Clarke, Boisseau, Gibbons)

**Path.** `/Users/lane/gist/profunctor-optics/`

**Citation.** Clarke, Boisseau, Gibbons et al., *Profunctor Optics, a Categorical Update*, Compositionality 2024.

**Status.** Primary theoretical reference. **The formal paper. Formal definitions live here.**

## Summary

The categorical framework for profunctor optics:

- **Tambara modules** — profunctor modules over a monoidal action.
- **Representation theorem** — every profunctor optic is representable as a van Laarhoven-style polymorphic function.
- **Mixed optics** — optics where the view and update live in different categories.
- **Monadic lenses** — `def:monadiclens` gives:

  ```
  MndLens_Ψ((A,B),(S,T)) = W(S, ΨA) × W(S × B, ΨT)
  ```

  psh's discipline functions instantiate this with Ψ being the shell's effect monad.

- **The full optic hierarchy:** Adapter → Lens → Prism → AffineTraversal → Traversal → Grate → Setter → Getter → Fold. Each class has its profunctor constraint (Profunctor, Cartesian, Cocartesian, etc.).

## Citation conventions from the optics agent

Cite **definition labels** (e.g., `def:monadiclens`, `def:lens`, `def:prism`) rather than section numbers. arXiv versioning makes section numbers unstable. Definition labels are stable across revisions.

## Concepts it informs in psh

- **`decision/codata_discipline_functions`** — `def:monadiclens` is the formal definition. psh's discipline functions in Kl(Ψ) are a direct instance.
- **`docs/spec/` §"Profunctor structure"** — redirections as Adapter (`def:adapter`), fd save/restore as Lens (`def:lens`).
- **Optics activation table** in `docs/spec/` §"Extension path" — per-type mapping to optic classes (the full hierarchy).
- **`decision/postfix_dot_accessors`** — Lens projection, Prism preview, AffineTraversal (Cartesian + Cocartesian for mixed).

## Who consults it

- **optics agent** (primary, canonical): read after Don't Fear for the formal definitions.

## Related

- `reference/papers/dont_fear_profunctor_optics` — the intuition layer, read first.
