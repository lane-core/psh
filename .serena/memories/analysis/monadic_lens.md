---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
verified_against: [docs/specification.md@HEAD §"Mixed-optic structure" line 757, /Users/lane/gist/profunctor-optics/arxivmain.tex:1054-1087]
keywords: [monadic-lens, mixed-optic, kleisli, kl-psi, putget, getput, putput, lens-laws, profunctor-optics, codata, discipline-cells, clarke, def-monadiclens, pure-view]
agents: [psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect]
supersedes: [analysis/monadic_lens@pre-2026-04-11 (the "both view and update in Kl(Ψ)" framing that contradicted Clarke def:monadiclens)]
related: [analysis/polarity/cbv_focusing, analysis/polarity/frames, decision/codata_discipline_functions, decision/postfix_dot_accessors, reference/papers/profunctor_optics_clarke, reference/papers/dont_fear_profunctor_optics]
---

# Monadic lens (def:monadiclens) — mixed optic with pure view

## Concept

A **monadic lens** in Clarke's sense [def:monadiclens] is a
**mixed optic** in which the view morphism is **pure** and the
update morphism is **effectful**:

    MndLens_Ψ((A,B),(S,T)) = W(S, A) × W(S × B, ΨT)

The decomposition category is `W` (pure base); the reconstruction
category is `Kl(Ψ)`. "Mixed" means the two base categories differ
— it is **not** a synonym for "impure." Clarke prop:monadiclens
explicitly establishes that monadic lenses *are* mixed optics.

A psh variable equipped with `.get` and `.set` discipline cells
instantiates `def:monadiclens` exactly:

- **`.get`** lives in `W(S, A)` — **pure** observation. The body
  reads the stored slot and returns a value without invoking
  effects. By default every disciplined variable has the trivial
  identity-on-slot `.get`; user-defined `.get` bodies must remain
  pure (typically a pure derived view of the slot).
- **`.set`** lives in `W(S × B, ΨT)` ≅ `Kl_Ψ(S ⋊ B, T)` —
  **effectful** mutation in the Kleisli category of the shell
  effect monad Ψ.

**`.refresh` is orthogonal to the lens structure.** A `.refresh`
body is a statement in `Kl(Ψ)` that writes the slot from outside
the lens framework. The view remains pure *between* refreshes;
`.refresh` is not part of the lens, and its presence or absence
does not change the lens laws.

## Monadic lens laws

Stated up to Kleisli equality on the update side [cited through
Clarke from AbouSaleh et al. 2016 §2.3]:

- **PutGet:** `set s b >>=_Ψ get ≡ return b`
- **GetPut:** `get s >>=_Ψ (λa. set s a) ≡_Ψ return s`
- **PutPut:** `set s b₁ >>=_Ψ (λs'. set s' b₂) ≡_Ψ set s b₂`

The laws become user contracts when discipline cells are
installed. For variables without discipline cells, the view is
identity in `W`, the update is trivial, and all three laws hold
unconditionally.

## Supersession note

The prior version of this memo claimed psh's construct was
"exactly a MonadicLens in Kl(Ψ)" with both view and update in
the Kleisli category. That framing:

1. **Contradicted Clarke def:monadiclens**, which puts the view
   in pure `W(S, A)`, not in `Kl(Ψ)`.
2. **Conflated Clarke's monadic lens with Riley 2018 §4.9's
   non-mixed effectful lens**, which really does put both legs
   in `Kl(Ψ)` but is a different optic class.
3. **Inverted Clarke's "mixed optic" terminology**, treating it
   as "impure optic" when it means "two different base
   categories."

The roundtable at 2026-04-11 (optics, sequent-calculus, vdc,
session-type, plan9) resolved this by narrowing `.get` to pure
and introducing `.refresh` as a separate effectful cell. With
`.get` pure, psh's construct *is* `def:monadiclens` exactly — no
contradiction with Clarke, no need for Riley's non-mixed variant.

## Foundational refs

- `reference/papers/profunctor_optics_clarke` — Clarke, Boisseau,
  Gibbons. **`def:monadiclens`** at `arxivmain.tex:1058` is the
  canonical definition. `prop:monadiclens` at line 1061
  establishes that monadic lenses are mixed optics.
- `reference/papers/dont_fear_profunctor_optics` — three-part
  introduction. Does NOT cover monadic lenses; build intuition
  for ordinary lenses here, then consult Clarke.
- `reference/papers/duploids` — Proposition 8550 ("thunkable ⇒
  central") justifies sharing the pure view within an expression
  without requiring the full Hasegawa-Thielecke theorem.

## Spec sites

- `docs/specification.md` §"Mixed-optic structure" (line 757) —
  authoritative.
- `docs/specification.md` §"Discipline functions §The codata
  model" (line 599) — operational embodiment.
- `docs/specification.md` §"Profunctor structure §Redirections
  as profunctor maps" (line 810) — optic classes for other
  shell constructs.

## Status

Settled 2026-04-11. The lens **laws** remain user contracts in
the discipline-equipped case; the difference from the prior memo
is that the *class* is now stated correctly (mixed optic per
Clarke, not the invalid "both sides effectful" formulation).
