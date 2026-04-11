---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [decision-procedure, monadic, comonadic, boundary-crossing, classification, vdc-framework-8-5, new-feature-classifier, kleisli, cokleisli, oblique]
agents: [vdc-theory, psh-sequent-calculus, psh-optics-theorist, plan9-systems-engineer, psh-architect, psh-session-type-agent]
related: [analysis/polarity/duploid_composition, analysis/polarity/plus_minus_failure, analysis/oblique_maps, analysis/polarity/_hub, analysis/monadic_lens, reference/papers/duploids]
---

# The §8.5 decision procedure (monadic / comonadic / boundary-crossing)

## Concept

`docs/vdc-framework.md` §8.5 contains a **decision procedure** for classifying any proposed psh feature into one of three categories:

- **Monadic (Kleisli, +):** the feature lives on the producer side. It composes with `•`. Effects flow forward through value composition. Examples: `let`-binding (μ̃ on F(A)), pipeline `|`, command substitution.
- **Comonadic (co-Kleisli, −):** the feature lives on the consumer side. It composes with `○`. Context flows backward through co-Kleisli composition. Examples: `;` sequencing, `trap` continuation handlers, `try`/`catch` scoped ErrorT.
- **Boundary-crossing (oblique, P → N):** the feature is a shell command in the technical sense — a producer meeting a consumer at the polarity boundary. These are the **oblique maps** (`analysis/oblique_maps`). Every shell command instantiates one. Polarity frames (`analysis/polarity/frames`) are required at every boundary crossing.

The procedure: identify the feature's interaction with `•`, `○`, and cut. If it associates cleanly with one of the two compositions, classify accordingly. If it's a producer-consumer interaction site, it's boundary-crossing — and the (+,−) non-associativity (`analysis/polarity/plus_minus_failure`) tells you you need a frame.

## Foundational refs

- `docs/vdc-framework.md` §8.5 "Decision procedure for new features" (line 865) — **the canonical source. First stop for any new-feature classification question.**
- `docs/vdc-framework.md` §8.1–8.4 (lines 779–865) — the composition law machinery the procedure consumes.
- `reference/papers/duploids` — theoretical justification of the three-way classification.

## Spec sites

The spec doesn't restate §8.5 — it consumes it. Every feature in the spec was classified through this procedure during design. The framework document is where the procedure lives by name.

## Status

Settled. **Authoritative for any "should psh add feature X" question.** When dispatching agents in parallel for a design decision, the vdc-theory agent's job starts here.
