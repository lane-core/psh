---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
verified_against: [docs/specification.md@HEAD §556-602, §677]
keywords: [cbv-focusing, static-focusing, focusing, downen, reentrancy, fire-once-per-expression, focused-sequent-calculus, critical-pair, mu-mu-tilde]
agents: [vdc-theory, psh-sequent-calculus, psh-optics-theorist]
extends: analysis/polarity/_hub
related: [analysis/polarity/frames, analysis/polarity/shifts, decision/codata_discipline_functions]
---

# CBV focusing as reentrancy semantics

## Concept

**Focusing** is a structural discipline on the sequent calculus that resolves the critical pair between μ (continuation-binding) and μ̃ (value-binding) by mandating one evaluation order. **CBV (call-by-value) focusing** picks the order where the value side wins: when both rules apply, μ̃ fires first, the value lands in the variable, and that value is reused at every subsequent consumption site within the same expression.

In psh, CBV focusing is the operational answer to "when does `.get` fire on a discipline-equipped variable?" Once per expression, at first use; subsequent uses of the same variable read the produced value. `echo $cursor $cursor` fires `.get` exactly once. Across separate expressions (next line, separate command), `.get` fires fresh.

This is **not memoization-as-optimization**. It is the focusing discipline of the focused sequent calculus realized at the polarity boundary — Downen et al.'s static focusing made operational. Memoization is what it looks like from outside; focusing is what it is.

## Foundational refs

- `reference/papers/grokking_sequent_calculus` — Binder et al. introduce focusing as critical-pair resolution. Cleanest first read.
- `reference/papers/dissection_of_l` — Spiwack treats focusing structurally as a phase of System L.
- `docs/vdc-framework.md` §6.2 "The Sequent Calculus as the Type Theory of Shell" (line 694) — psh's focusing commitment, citing Downen et al. style static focusing.
- `reference/papers/duploids` — focusing as critical-pair resolution at the categorical level.

## Spec sites

- `docs/specification.md` §"CBV focusing as the reentrancy semantics" (line 556) — authoritative for psh.
- `docs/specification.md` §"Discipline functions §Reentrancy and the polarity frame" (line 629) — the operational consumer.
- `decision/codata_discipline_functions` — the design decision built on this anchor.

## Status

Settled. Note: cross-variable consistency across separate expressions is **not** guaranteed and is documented as a known caveat in `docs/specification.md` §"Known caveat: cross-variable consistency" (line 677).
