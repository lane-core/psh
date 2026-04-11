---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [three-sorts, producers, consumers, commands, gamma, delta, terms, coterms, cuts, ast-four-sorts, mode, lambda-mu-mu-tilde]
agents: [psh-sequent-calculus, vdc-theory, psh-architect]
related: [analysis/oblique_maps, analysis/polarity/shifts, decision/let_is_mu_tilde_binder_cbpv, decision/def_vs_lambda]
---

# The three sorts (producers Γ / consumers Δ / commands ⟨t|e⟩)

## Concept

The λμμ̃-calculus has **three syntactic sorts**, not one or two:

- **Producers (terms, Γ side):** things that compute values. Pure expressions, function bodies, anything that "produces an answer." Bind with μ.
- **Consumers (coterms, Δ side):** things that consume values. Continuations, evaluation contexts, anything that "expects an answer." Bind with μ̃.
- **Commands (cuts, ⟨t | e⟩):** the result of putting a producer t against a consumer e. The execution rule of the calculus.

This is the foundational structure that distinguishes sequent calculus from natural deduction. Natural deduction has one sort (terms); sequent calculus has three. The three-sort structure is what lets the calculus express the duality between value-binding and continuation-binding directly, and what makes critical pairs and focusing discipline possible to state.

In psh, the three sorts get a **fourth extension** in the AST: a **Mode** sort that captures the polarity-frame state (positive/expansion vs negative/execution mode). The four AST sorts are the structural inheritance of the three-sort calculus extended with the polarity-frame discipline.

## Foundational refs

- `reference/papers/grokking_sequent_calculus` — Binder et al. introduce the three sorts as the "Core" calculus on the Fun→Core compilation. Cleanest first read.
- `reference/papers/dissection_of_l` — Spiwack treats them piece by piece in System L.
- `docs/vdc-framework.md` §3.1 "Two Sorts: Values and Commands" (line 180) and §3.2 "Cut as Execution" (line 207) — psh's framing in the framework document.

## Spec sites

- `docs/specification.md` §"The three sorts, made explicit" (line 232) — authoritative.
- `docs/specification.md` §"Terms (producers) — Γ" (line 251), §"Coterms (consumers) — Δ" (line 270), §"Commands (cuts) — ⟨t | e⟩" (line 302) — sort-by-sort treatment.
- `docs/specification.md` §"The AST's four sorts" (line 321) — the Mode extension.

## Status

Settled. Foundational. Architect treats the four-sort AST as load-bearing for the parser and evaluator structure — see `decision/let_is_mu_tilde_binder_cbpv` and `decision/def_vs_lambda` for the operational consequences.
