---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [spiwack, system-l, sequent-calculus, polarized, shift, downshift, upshift, multiplicative, additive, exponential, dependent, classical, linear, two-sorts]
agents: [psh-sequent-calculus, psh-architect]
---

# Reference: A Dissection of L (Spiwack)

**Path.** `/Users/lane/gist/dissection-of-l.gist.txt`

**Citation.** Spiwack, *A Dissection of L*, 2014.

**Status.** Primary theoretical reference. **Structural reference for how psh's type system is built.**

## Summary

Dissects System L into its constituent parts, piece by piece:

1. **Two sorts** — terms and coterms.
2. **One reduction rule** — cut.
3. **Classical typing** — multiple conclusions in Δ.
4. **Linear typing** — managed resources.
5. **Polarity via shifts** — `↓N` (downshift, negative-to-positive) and `↑A` (upshift, positive-to-negative).
6. **Multiplicatives** — `⊗` / `⅋`.
7. **Additives** — `+` / `&`.
8. **Exponentials** — `!` / `?`.
9. **Dependent extension** — adding dependent types.

Each piece is introduced with the minimum prior material needed to make sense of it. For psh this is the step-by-step building block reference: when the spec adds a new connective or sort discipline, Dissection of L is where the formal building block lives.

## Concepts it informs in psh

- **Three sorts (producers/consumers/commands)** — `docs/spec/` §"The three sorts, made explicit". psh has **three** sorts (adding commands as a distinct sort beyond Spiwack's two terms/coterms); the command is the cut itself in Spiwack's formulation.
- **Polarity discipline** — positive/negative split, shifts `↓`/`↑`.
- **`decision/every_variable_is_a_list`** — the unit-free structure connects to Spiwack's linear fragment without the multiplicative unit.
- **`decision/try_catch_scoped_errort`** — ⊕/⅋ duality sits in Spiwack's additive/multiplicative structure.
- **`decision/let_is_mu_tilde_binder_cbpv`**, **`decision/unified_trap_three_forms`** — let/control duality as the two kinds of binder.
- **Coprocess protocol** — linear/session type flavor sits in Spiwack's linear fragment.

## Who consults it

- **sequent calculus agent** (primary, canonical): read after the Grokking paper.
- **psh-architect** (secondary): for the structural reference when implementing new connectives.

## Note

Spiwack treats μ and μ̃ **symmetrically**. psh adopts the **asymmetric** interpretation where `let` is μ̃ (value binding) and `trap` is μ (continuation binding) because the shell's cut discipline is operational, not purely structural. This is a deliberate departure from Dissection — noted in `decision/let_is_mu_tilde_binder_cbpv` and `decision/unified_trap_three_forms`. When the spec and Dissection disagree, the spec wins (spec is tier 1, papers are tier 5).
