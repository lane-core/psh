---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [polarity, polarity-frame, shift, duploid, plus-minus, non-associativity, focusing, cbv, dccache, sh-prefix, critical-pair, hub, polarity-discipline]
agents: [vdc-theory, psh-sequent-calculus, psh-optics-theorist, plan9-systems-engineer, psh-architect]
related: [analysis/decision_procedure_8_5, analysis/monadic_lens, decision/codata_discipline_functions, reference/sfio_analysis_suite, reference/papers/duploids, reference/ksh93_analysis]
---

# Polarity discipline cluster — hub

## Motivation

psh's polarity discipline is the load-bearing operational mechanism that makes the type system match its semantics. Seven spokes cover it because the discipline has both a theoretical side (duploid laws, focusing, shifts) and an empirical side (sfio Dccache, ksh93 sh.prefix bugs), and the connection between the two is what distinguishes psh from "yet another typed shell." The hub orients agents to the cluster shape and gives a reading order; spoke content lives in the spokes.

Audience: every theory agent (vdc-theory, sequent-calculus, optics) plus plan9-systems-engineer on the empirical side. psh-architect reads on demand when implementing frames in Rust.

## Spokes

- [analysis/polarity/frames](analysis/polarity/frames) — the save/restore-around-shift engineering principle
- [analysis/polarity/shifts](analysis/polarity/shifts) — ↓/↑ as the operator the frame brackets
- [analysis/polarity/cbv_focusing](analysis/polarity/cbv_focusing) — static focusing as the reentrancy semantics inside the frame
- [analysis/polarity/duploid_composition](analysis/polarity/duploid_composition) — the four composition laws and which one fails
- [analysis/polarity/plus_minus_failure](analysis/polarity/plus_minus_failure) — the (+,−) equation, named directly
- [analysis/polarity/dccache_witness](analysis/polarity/dccache_witness) — sfio §07 as the empirical (+,−) failure
- [analysis/polarity/sh_prefix_critical_pair](analysis/polarity/sh_prefix_critical_pair) — ksh93 bug class as critical-pair manifestation

## Reading order

**Theoretical-first** (for vdc-theory, sequent-calculus on a clean slate): `duploid_composition` → `plus_minus_failure` → `shifts` → `frames` → `cbv_focusing` → `dccache_witness` → `sh_prefix_critical_pair`. Build the algebra, then watch it manifest.

**Empirical-first** (debugging-driven, plan9-systems-engineer or psh-architect chasing a concrete issue): `sh_prefix_critical_pair` → `dccache_witness` → `frames` → `shifts` → `cbv_focusing` → `plus_minus_failure` → `duploid_composition`. Start from the bug, trace the discipline, land at the algebra.

## Open questions

- Whether the polarity frame discipline transfers to typed pipes — deferred per `PLAN.md` "session types on pipes" v2 item.
- Whether the Squier coherence story (`analysis/squier_critical_pair`, planned tier 3) belongs as a spoke here or stays separate. Current call: separate, because it's about meta-rewriting of the calculus, not about the frame discipline per se.

## Cross-cluster references

- `analysis/decision_procedure_8_5` — the §8.5 monadic/comonadic/boundary-crossing classifier consumes the duploid law machinery this cluster documents.
- `analysis/monadic_lens` — codata discipline functions are the Kl(Ψ) inhabitants the polarity frame protects.
- `decision/codata_discipline_functions` — operational consumer of the cluster (CBV focusing, the polarity frame as reentrancy guard).
- `reference/sfio_analysis_suite` — file-level reference for the empirical side (esp. §07 disciplines, §03 buffer model).
- `reference/papers/duploids` — paper-level reference for the theoretical side (Mangel-Melliès-Munch-Maccagnoni).
- `reference/ksh93_analysis` — paper-level reference for the manifestation in real interpreter code.

`docs/specification.md` §"Polarity discipline" (line 343), §"Reentrancy and the polarity frame" (line 629). `docs/vdc-framework.md` §9.3 (line 937). `refs/ksh93/ksh93-analysis.md` §"The save/restore pattern IS the shift" (line 425).
