---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [three-sorts, producers, consumers, commands, gamma, delta, terms, coterms, cuts, statements, grokking, lambda-mu-mu-tilde, engineering-layer]
agents: [psh-sequent-calculus, vdc-theory, psh-architect]
related: [analysis/oblique_maps, analysis/polarity/shifts, decision/let_is_mu_tilde_binder_cbpv, decision/def_vs_lambda]
---

# The three sorts (producers Γ / consumers Δ / commands ⟨t|e⟩)

## Concept

The λμμ̃-calculus has **three syntactic sorts**, not one or two:

- **Producers (terms, Γ side):** things that compute values.
  Pure expressions, function bodies, anything that "produces an
  answer." μ-binder captures the current context.
- **Consumers (coterms, Δ side):** things that consume values.
  Continuations, evaluation contexts, anything that "expects an
  answer." μ̃-binder captures the current value.
- **Statements (commands, ⟨t | e⟩):** the result of putting a
  producer t against a consumer e — a cut. The execution rule
  of the calculus.

This is the foundational structure that distinguishes sequent
calculus from natural deduction. Natural deduction has one sort
(terms); sequent calculus has three. The three-sort structure
is what lets the calculus express the duality between value-
binding and continuation-binding directly, and what makes
critical pairs and focusing discipline possible to state.

psh adopts Grokking's two-sided reading of λμμ̃ (three syntactic
categories with μ distinct from μ̃) rather than Spiwack's one-
sided reading (two categories with duality handled at the type
level). The choice is driven by the observable operational
asymmetry between producers and consumers at the process
boundary: a producer is a value in hand, a consumer is a waiting
process reading from a pipe, and converting one to the other
costs real resources (forking or thunking).

## psh's AST shape

The AST has three logical sorts corresponding to Grokking's
categories, plus one engineering layer:

- **`Term`** — Producer sort. Values, variable
  references, command substitutions, lambdas, concatenations,
  tuples, list literals, struct brace record literals, enum
  variant construction. Everything in Γ.
- **`Expr`** — Engineering boundary, not a logical sort. Groups
  the consumer-side machinery for evaluator organization:
  pipelines, redirections, background. Logically part of the
  consumer apparatus; kept as a separate AST node type because
  the evaluator dispatches on it before the cut fires.
- **`Command` — Consumer sort (coterm). Command shapess) — ⟨t | e⟩" —
  sort-by-sort treatment.
- `docs/spec/` §"The AST's three sorts (plus
  engineering layer)" — the AST-level mapping.

## Status

Settled. Foundational. The three logical sorts plus engineering
`Expr` layer is the shape the parser and evaluator are built
against.
