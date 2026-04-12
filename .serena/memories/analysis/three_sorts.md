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

- **`Word` / `Value`** — Producer sort. Values, variable
  references, command substitutions, lambdas, concatenations,
  tuples, list literals, struct brace record literals, enum
  variant construction. Everything in Γ.
- **`Expr`** — Engineering boundary, not a logical sort. Groups
  the consumer-side machinery for evaluator organization:
  pipelines, redirections, background. Logically part of the
  consumer apparatus; kept as a separate AST node type because
  the evaluator dispatches on it before the cut fires.
- **`Command`** — Statement sort. Every cut. Simple commands,
  pipelines at the statement level, assignments (⟨val | μ̃x.rest⟩
  — the μ̃-binder is the consumer slot inside the statement,
  not a separate sort), if/for/match/try/trap.

Consumers are **synthesized implicitly** by the evaluator from
the statement's shape rather than stored as first-class AST
nodes — the same way rc's consumers were implicit, made just
explicit enough for sort-directed evaluation. There is no
separate `Binding` sort or `Coterm` sort in the AST; μ̃-binders
live inside statements as the consumer slot of a cut.

## Foundational refs

- `reference/papers/grokking_sequent_calculus` — Binder et al.
  introduce the three sorts as the "Core" calculus on the
  Fun→Core compilation. Cleanest first read. Grokking §"Syntax"
  (gist ~line 12300) is explicit: "λμμ̃ uses three different
  syntactic categories: producers p, consumers c and statements
  s."
- `reference/papers/dissection_of_l` — Spiwack treats the
  calculus piece by piece in one-sided System L, with two sorts
  plus duality on types. psh uses Grokking's two-sided reading
  instead for the reasons described in §"Concept" above.
- `docs/vdc-framework.md` §3.1 "Two Sorts: Values and Commands"
  (line 180) and §3.2 "Cut as Execution" (line 207) — psh's
  framing in the framework document.

## Spec sites

- `docs/specification.md` §"The three sorts, made explicit" —
  authoritative.
- `docs/specification.md` §"Terms (producers) — Γ",
  §"Coterms (consumers) — Δ", §"Commands (cuts) — ⟨t | e⟩" —
  sort-by-sort treatment.
- `docs/specification.md` §"The AST's three sorts (plus
  engineering layer)" — the AST-level mapping.

## Status

Settled. Foundational. The three logical sorts plus engineering
`Expr` layer is the shape the parser and evaluator are built
against.
