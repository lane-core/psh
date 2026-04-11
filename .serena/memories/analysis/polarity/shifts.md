---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [shift, downshift, upshift, value-mode, computation-mode, polarity-boundary, focused-sequent-calculus, focusing, F-A, U-B]
agents: [vdc-theory, psh-sequent-calculus, psh-optics-theorist]
extends: analysis/polarity/_hub
related: [analysis/polarity/frames, analysis/polarity/cbv_focusing, analysis/polarity/duploid_composition, decision/let_is_mu_tilde_binder_cbpv, decision/def_vs_lambda]
---

# Shifts (↓/↑) — the value/computation boundary operators

## Concept

In the focused sequent calculus, **shifts** are the operators that mediate between positive (value-mode, expansion context) and negative (computation-mode, execution context) types. The downshift `↓` (CBPV's `U`, thunk) suspends a computation as a value; the upshift `↑` (CBPV's `F`, returner) wraps a value as a trivial computation to be forced. (Spec convention at `docs/specification.md` line 200: "↑A for returning"; line 228: "`def`/`let` + lambda split is CBPV's `U`/`F` adjunction" — `def` returns `F(Status)`, lambda is `U(...)`.) Every place a psh expression crosses the polarity boundary, a shift is happening — and per the polarity-frame discipline, every shift is bracketed by save/restore at runtime.

Shifts are the operator; **frames are the runtime realization**. Conversely, the (+,−) equation failure (see `analysis/polarity/plus_minus_failure`) is exactly what makes shifts necessary as primitive operations rather than derivable conveniences.

## Foundational refs

- `reference/papers/dissection_of_l` — Spiwack's dissection of System L treats shifts piece by piece. Read the polarity sections for the structural definition.
- `reference/papers/grokking_sequent_calculus` — Binder et al. introduce shifts in the CBPV-flavored Fun→Core compilation. More accessible than Spiwack.
- `reference/papers/duploids` — duploids paper. Shifts mediate the Kleisli (value) and co-Kleisli (computation) subcategories of the duploid; the (+,−) failure is what makes them non-trivial.
- `docs/vdc-framework.md` §6.2 "The Sequent Calculus as the Type Theory of Shell" (line 694) — psh's shift placement.

## Spec sites

- `docs/specification.md` §"CBV/CBN split" (line 345) — how the split is realized in psh.
- `docs/specification.md` §"Three operations, three roles" (line 386) — the three operational roles of a shift in psh syntax.
- `docs/specification.md` §"Two kinds of callable" (line 473) — `def` vs lambda is the F/U distinction at the syntax level.

## Status

Settled at the theoretical level. Architect should treat every `let` (μ̃ on F(A)) and every `def` invocation as a shift site.
