---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [profunctor-optics, lens, prism, monomorphic, tambara, van-laarhoven, introduction, part-1, part-2, part-3]
agents: [psh-optics-theorist, psh-architect]
---

# Reference: Don't Fear the Profunctor Optics (three-part intro)

**Path.** `/Users/lane/gist/DontFearTheProfunctorOptics/`

**Status.** Accessible theoretical reference. Three-part introduction building from monomorphic lenses to the full profunctor representation with van Laarhoven encoding.

## Summary

- **Part 1** — monomorphic lenses with `view`/`update` signatures and the Lens laws (PutGet, GetPut, PutPut). The starting point.
- **Part 2** — polymorphic lenses and prisms (`preview`/`review`). Generalizes to the standard optic classes.
- **Part 3** — the profunctor representation. Tambara modules, the Choice profunctor for prisms, the Strong profunctor for lenses, how van Laarhoven encodings unify everything.

This is the **intuition layer** for the formal profunctor-optics paper by Clarke et al. Read first for grounding, then switch to Clarke et al. for the formal definitions.

## Concepts it informs in psh

- **`decision/postfix_dot_accessors`** — the Lens/Prism distinction determines which accessor type applies.
- **`docs/specification.md` §"Profunctor structure"** — redirections as Adapter (pure profunctor constraint), fd save/restore as Lens (Cartesian constraint).
- **`decision/codata_discipline_functions`** — the MonadicLens extension sits atop this hierarchy.
- **Optics activation table** in `docs/specification.md` §"Extension path" — maps psh types to their supported optic classes.

## Who consults it

- **optics agent** (primary, first read): intuition before formal definitions. The formal paper (`reference/papers/profunctor_optics_clarke`) is the canonical source; this one is the grounding.
- **psh-architect** (secondary): for the practical feel of how optics compose.
