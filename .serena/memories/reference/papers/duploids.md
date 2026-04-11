---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [duploids, mangel, mellies, munch-maccagnoni, plus-minus-equation, non-associative, cbv, cbn, kleisli, cokleisli, thunkable-central, fuhrmann-thielecke, dialogue-duploid, oblique-map]
agents: [vdc-theory, psh-sequent-calculus]
---

# Reference: duploids paper (Mangel, Melliès, Munch-Maccagnoni)

**Path.** `/Users/lane/gist/classical-notions-of-computation-duploids.gist.txt`

**Citation.** Mangel, Melliès, Munch-Maccagnoni. *Classical Notions of Computation and the Hasegawa-Thielecke Theorem.* And the companion: Munch-Maccagnoni, *Models of a Non-Associative Composition of Programs and Proofs*, FoSSaCS 2014. Munch-Maccagnoni's thesis (*Syntax and Models of a Non-Associative Composition of Programs and Proofs*, 2013) is where duploids originate.

**Status.** Primary theoretical reference. **The semantic backbone** of psh's polarity discipline. Read first in any new vdc-theory or sequent-calculus session.

## Summary

Defines **duploids** as non-associative categories integrating call-by-value (Kleisli / monadic) and call-by-name (co-Kleisli / comonadic) computation. Three of four associativity equations hold; the fourth — the **(+,−) equation** — fails. This failure captures the CBV/CBN distinction.

Maps that restore full associativity are **thunkable** (pure, value-like). In a **dialogue duploid** (duploid + involutive negation), **thunkable = central** — purity and commutativity coincide in the presence of classical control. This is the Führmann-Thielecke theorem.

Table 1 of the FoSSaCS companion paper maps abstract structure to concrete PL concepts: thunk, return, Kleisli (monadic), co-Kleisli (comonadic), and **oblique maps** (the cross-polarity arrows that shell commands instantiate).

## Concepts it informs in psh

- **Polarity discipline** — CBV/CBN split in `docs/specification.md` §"Polarity discipline".
- **Polarity frame** — the operational realization of the shift operator mediating duploid subcategories. Every psh polarity frame is an instance.
- **(+,−) non-associativity** — `docs/vdc-framework.md` §8.4 "The non-associativity failure" cites this directly. The `sh.prefix` bugs are concrete manifestations.
- **Decision procedure** — `docs/vdc-framework.md` §8.5 (monadic/comonadic/boundary-crossing classification) derives from the composition laws in this paper.
- **`decision/codata_discipline_functions`** — CBV focusing, the codata model, MonadicLens structure in Kl(Ψ).
- **Führmann-Thielecke theorem** — cited in `docs/specification.md` §"Theoretical framework §The semantics".
- **Oblique maps** — shell commands as cross-polarity arrows `P → N` (producer meets consumer, effects happen).

## Who consults it

- **vdc-theory agent** (primary, canonical).
- **sequent calculus agent** (primary, canonical).
- **Other agents should NOT consult directly.** The distilled composition law decision procedure in `docs/vdc-framework.md` §8.5 is the operational distillation. Reading the paper is an authority move that only the theory agents should make.
