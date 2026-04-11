---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [polarity-frame, save-restore, shift, sh-polarity-enter, sh-polarity-leave, reentrancy, value-computation-boundary, engineering-principle]
agents: [vdc-theory, psh-sequent-calculus, psh-optics-theorist, plan9-systems-engineer, psh-architect]
extends: analysis/polarity/_hub
related: [analysis/polarity/shifts, analysis/polarity/cbv_focusing, analysis/polarity/dccache_witness, decision/codata_discipline_functions]
---

# Polarity frames (engineering principle)

## Concept

A **polarity frame** is the save/restore pattern that wraps any operation crossing the boundary between value-mode (positive, expansion context) and computation-mode (negative, execution context). The frame saves positive-mode state, clears it, runs the computation, and restores the state on exit. Every shift in the focused calculus is realized at runtime as a frame; conversely, every place psh leaves a frame out is a place where the calculus's discipline has been violated and bugs follow.

The pattern is inherited directly from ksh93's `sh_polarity_enter` / `sh_polarity_leave` calls and from sfio's discipline-cache reconciliation. psh's contribution is to lift the pattern from "ad hoc workaround" to **uniform engineering principle at every polarity boundary**.

## Foundational refs

- `docs/vdc-framework.md` §9.3 "The Polarity Frame Discipline" (line 937) — the engineering principle as first-class framework rule.
- `reference/papers/duploids` — Mangel-Melliès-Munch-Maccagnoni. Frames are the operational realization of the shift operator that mediates duploid subcategories.
- `refs/ksh93/sfio-analysis/07-disciplines.md` — Dccache as the empirical precedent (sfio reconciles a discipline-stack push by flushing and rechaining; the mechanism plays the same role a polarity frame plays at the I/O layer — see `analysis/polarity/dccache_witness` for the epistemic discipline of citing it as structural analogy, not formal verification).
- `refs/ksh93/ksh93-analysis.md` §"The save/restore pattern IS the shift" (line 425) — the explicit identification.

## Spec sites

- `docs/specification.md` §"Polarity discipline" (line 343) — psh's discipline statement.
- `docs/specification.md` §"Discipline functions §Reentrancy and the polarity frame" (line 629) — the codata case.
- `docs/specification.md` §"Three operations, three roles" (line 386) — frame structure at the operational level.

## Status

Settled. The frame discipline is a v1 commitment, with `decision/codata_discipline_functions` as the currently-implemented operational consumer. Architect should treat every shift in the AST evaluation pass as a frame insertion site.
