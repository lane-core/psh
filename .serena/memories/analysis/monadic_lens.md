---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
verified_against: [docs/specification.md@HEAD §647-675]
keywords: [monadic-lens, kleisli, kl-psi, putget, getput, putput, lens-laws, profunctor-optics, codata, discipline-functions, clarke, def-monadiclens]
agents: [psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect]
related: [analysis/polarity/cbv_focusing, analysis/polarity/frames, decision/codata_discipline_functions, decision/postfix_dot_accessors, reference/papers/profunctor_optics_clarke, reference/papers/dont_fear_profunctor_optics]
---

# MonadicLens (def:monadiclens)

## Concept

A **MonadicLens** is the Kleisli generalization of the ordinary Lens. Where an ordinary Lens has `view : S → A` and `update : (A, S) → S`, a MonadicLens lifts both into a Kleisli category Kl(M) for some monad M: `view : S → M A` and `update : (A, S) → M S`. The lens laws (PutGet, GetPut, PutPut) are stated up to the Kleisli equality, accommodating effects in either operation.

In psh, the relevant Kleisli category is **Kl(Ψ)** where Ψ is the shell's effect monad (process spawning, fd manipulation, environment access, signal delivery — everything a discipline function can touch). A psh variable equipped with `.get` and `.set` discipline functions is **exactly a MonadicLens in Kl(Ψ)**: `.get` is the Kleisli view (computation observer), `.set` is the Kleisli update (computation constructor).

The lens laws are not enforced by the runtime — they become **contracts the user must maintain** when they install discipline functions. Ordinary variables without disciplines are pure Lenses and get the laws for free. The CBV focusing discipline (`analysis/polarity/cbv_focusing`) ensures that within a single expression, `.get` and `.set` each fire once, which keeps the lens laws meaningful in the presence of effects — without focusing, repeated reads or consecutive writes within the same expression could observe stale or non-monotone state and the laws would degrade further.

## Foundational refs

- `reference/papers/profunctor_optics_clarke` — Clarke, Boisseau, Gibbons. **Definition `def:monadiclens` is the canonical formal definition.** Cite this when stating the laws.
- `reference/papers/dont_fear_profunctor_optics` — three-part introduction to the **lens hierarchy and profunctor representation** (build intuition for ordinary lenses → profunctor encoding here before reading Clarke). **Note:** Don't Fear does NOT cover monadic lenses; those appear only in Clarke et al. `def:monadiclens` at `~/gist/profunctor-optics/arxivmain.tex:1054–1058`.

## Spec sites

- `docs/specification.md` §"MonadicLens structure" (line 647) — authoritative.
- `docs/specification.md` §"Discipline functions §The codata model" (line 510) — operational embodiment.
- `docs/specification.md` §"Profunctor structure" (line 696) — the full optics activation table.
- `decision/codata_discipline_functions` — the design decision built on this anchor.

## Status

Settled. Note that the lens **laws** are user contracts in the discipline-equipped case, not runtime invariants. The optics activation table in `docs/specification.md` §"Optics activation" (line 1313) lists which variables get which optic class.
