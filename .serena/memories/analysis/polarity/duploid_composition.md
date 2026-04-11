---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [duploid, composition-laws, kleisli, cokleisli, monadic, comonadic, cut, associativity, three-of-four-equations, dialogue-duploid]
agents: [vdc-theory, psh-sequent-calculus]
extends: analysis/polarity/_hub
related: [analysis/polarity/plus_minus_failure, analysis/polarity/shifts, analysis/oblique_maps, analysis/decision_procedure_8_5]
---

# Duploid composition laws (•, ○, cut)

## Concept

A **duploid** (Mangel-Melliès-Munch-Maccagnoni) is a non-associative category that integrates three composition operations:

- **`•` (Kleisli composition, monadic side):** how producers chain effects. CBV-flavored. Used for pipelines and Kleisli pipelines.
- **`○` (co-Kleisli composition, comonadic side):** how consumers chain context. CBN-flavored. Used for sequential composition with shared context.
- **Cut (⟨t | e⟩, fundamental interaction):** how a producer meets a consumer. The execution rule of the calculus.

The duploid laws say: `•` is associative, `○` is associative, cut interacts coherently with both — **but the four-way associativity equation between `•` and `○` (the (+,−) equation) fails**. Three of four equations hold; one doesn't. That failure is the entire reason duploid theory exists, and it's the entire reason psh's polarity discipline exists.

In **dialogue duploids** (duploid + involutive negation), the additional **Hasegawa-Thielecke theorem** holds: thunkable maps = central maps. Purity and commutativity coincide in the presence of classical control. (Note: `docs/specification.md` line 211 currently calls this the "Führmann-Thielecke theorem" — that is an upstream mis-attribution; the duploids paper macro `\FH` at line 6863 expands to "Hasegawa-Thielecke" and the theorem is attributed to Hasegawa per Mellies-Tabareau at line 9608. Führmann is credited at lines 7903–7906 for the related but distinct work distinguishing thunkability from centrality. Spec needs upstream fix.)

## Foundational refs

- `reference/papers/duploids` — Mangel, Melliès, Munch-Maccagnoni. The composition laws and the (+,−) failure are stated in the locally vendored PACMPL paper (`/Users/lane/gist/classical-notions-of-computation-duploids.gist.txt`) — the four-case enumeration of `(ε, ε')` is at lines 7100–7185. The FoSSaCS 2014 companion paper (Munch-Maccagnoni, *Models of a Non-Associative Composition of Programs and Proofs*) is cited in the bibliography (line 5818) but is **not vendored locally**; for the PL concept mapping (thunk, return, Kleisli, co-Kleisli, oblique maps), use `refs/ksh93/ksh93-analysis.md` §"Monadic and comonadic patterns in C" line 459 as the proximate source — it cites the FoSSaCS paper as `[9, Table 1]` and is the citable anchor in psh's own materials.
- `docs/vdc-framework.md` §8.1 "Pipeline composition (•, Kleisli/monadic)" (line 779), §8.2 "Sequential composition (○, co-Kleisli/comonadic)" (line 797), §8.3 "Cut (⟨t|e⟩)" (line 820).

## Spec sites

- `docs/specification.md` §"Polarity discipline" (line 343) — what the discipline enforces operationally.
- `docs/specification.md` §"Theoretical framework §The semantics" (line 202) — Führmann-Thielecke citation.

## Status

Settled. This is theory psh consumes, not theory psh derives. Treat the duploids paper as authoritative; consult only via vdc-theory or sequent-calculus agent.
