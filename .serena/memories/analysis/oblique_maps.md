---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [oblique-maps, P-to-N, cross-polarity, shell-commands, producer-consumer, boundary-crossing, table-1, duploids, dialogue-duploid]
agents: [vdc-theory, psh-sequent-calculus, plan9-systems-engineer]
related: [analysis/polarity/duploid_composition, analysis/decision_procedure_8_5, analysis/three_sorts, reference/papers/duploids]
---

# Oblique maps (cross-polarity arrows P → N)

## Concept

In a duploid, **oblique maps** are arrows from a positive object to a negative object: `P → N`. They are neither pure-monadic (Kleisli, P → P) nor pure-comonadic (co-Kleisli, N → N) — they live in the gap between the two subcategories.

In psh's reading, **every shell command has the structure of an oblique map**. A command takes a value-mode argument list (positive: lists of words after expansion) and produces a computation-mode effect (negative: process execution, fd manipulation, exit status). The shell command is exactly the place where producer meets consumer, where the value side meets the computation side, where effects happen. Whether the full duploid composition laws carry over to shell commands is not formally verified — the structural correspondence is what psh's framework consumes.

This is why shell commands need polarity frames: every oblique map is a boundary crossing, and per the (+,−) non-associativity (`analysis/polarity/plus_minus_failure`) every boundary crossing needs a save/restore. The framework decision procedure (`analysis/decision_procedure_8_5`) classifies new features by where they sit relative to oblique maps: monadic features stay positive, comonadic features stay negative, boundary-crossing features are oblique.

The Mangel-Melliès-Munch-Maccagnoni mapping (cited as `[9, Table 1]` in `refs/ksh93/ksh93-analysis.md` line 459) gives the PL concept mapping: thunk corresponds to one direction, return to another, and oblique maps are the residual that don't reduce to either pure subcategory. Operationally they are what shells **do**. (The FoSSaCS 2014 paper containing Table 1 is not vendored locally; ksh93-analysis is the proximate citable source within psh's own materials.)

## Foundational refs

- `reference/papers/duploids` — Mangel-Melliès-Munch-Maccagnoni. The locally vendored PACMPL paper at `/Users/lane/gist/classical-notions-of-computation-duploids.gist.txt` covers the (+,−) non-associativity proof (lines 7100–7185) but does **not** contain Table 1 or an "oblique" treatment (zero hits on either string per audit). The Table 1 / oblique-map mapping is in the **un-vendored FoSSaCS 2014 companion** (Munch-Maccagnoni); for psh purposes, cite `refs/ksh93/ksh93-analysis.md` §"Monadic and comonadic patterns in C" line 459 as the proximate source.
- `docs/vdc-framework.md` §5.4 "Cells = Commands" (line 443) — psh's framing of commands as cells, which line up with oblique maps.

## Spec sites

- `docs/specification.md` §"Three operations, three roles" (line 386) — operational realization in psh syntax.
- `docs/specification.md` §"Theoretical framework" (line 187) — the duploid framing the spec inherits.

## Status

Settled. This is the right vocabulary for "what is a shell command, in the calculus." When explaining psh to an outsider, "every command is an oblique map P → N" is the one-sentence theoretical summary.
