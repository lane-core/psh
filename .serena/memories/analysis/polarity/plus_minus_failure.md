---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [plus-minus-equation, non-associativity, duploid, composition-failure, cbv-cbn-distinction, four-equations, three-hold-one-fails]
agents: [vdc-theory, psh-sequent-calculus, plan9-systems-engineer]
extends: analysis/polarity/_hub
related: [analysis/polarity/duploid_composition, analysis/polarity/dccache_witness, analysis/polarity/sh_prefix_critical_pair, analysis/polarity/frames]
---

# The (+,−) non-associativity equation

## Concept

In a duploid, four associativity equations relate the Kleisli (`•`, +) and co-Kleisli (`○`, −) compositions to each other. Three hold:

- (+,+): pure producer chains associate
- (−,−): pure consumer chains associate
- (−,+): producer-into-consumer respects bracketing

The fourth — **(+,−), where a Kleisli composition is bracketed against a co-Kleisli composition** — does NOT associate in general. That single failure **is** the CBV/CBN distinction, in algebraic form. Bracket one way and you get value-then-context; bracket the other and you get context-then-value, and effects observe the difference.

The (+,−) failure is what makes shift operators (`↓`, `↑`) primitive rather than derivable, what makes polarity frames necessary at runtime, and what makes "monadic vs comonadic" a real classification rather than a stylistic preference.

The empirical witnesses in the wild: sfio's Dccache (see `analysis/polarity/dccache_witness`) and ksh93's sh.prefix bug class (see `analysis/polarity/sh_prefix_critical_pair`).

## Foundational refs

- `reference/papers/duploids` — Mangel-Melliès-Munch-Maccagnoni state and prove the failure. The companion FoSSaCS paper (Munch-Maccagnoni 2014) gives the cleanest exposition.
- `docs/vdc-framework.md` §8.4 "The non-associativity failure" (line 835) — psh's framing of the failure as the discipline's justification.

## Spec sites

The spec doesn't name the (+,−) equation directly; it names its consequences. The relevant sections are `docs/spec/` §"Polarity discipline" (line 343) and §"Discipline functions" (line 502). The framework document is where the equation lives by name.

## Status

Settled at the theoretical level. Foundational. If a future contributor proposes a feature whose correctness depends on (+,−) holding, the proposal needs to be rejected — that equation fails by theorem, not by current limitation.
