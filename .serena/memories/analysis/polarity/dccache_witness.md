---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
verified_against: [refs/ksh93/sfio-analysis/07-disciplines.md@HEAD §229-265, docs/spec/@HEAD §133-169]
keywords: [dccache, sfio, discipline-cache, non-associativity-witness, sfdisc, buffer-mode-switch, empirical-precedent, plus-minus-failure]
agents: [plan9-systems-engineer, vdc-theory, psh-architect]
extends: analysis/polarity/_hub
related: [analysis/polarity/plus_minus_failure, analysis/polarity/frames, reference/sfio_analysis_suite]
---

# Dccache as (+,−) non-associativity witness

## Concept

sfio's **Dccache** (discipline cache) is the structural mechanism that reconciles a stream with already-buffered data when a new discipline gets pushed onto the discipline stack. Dccache exists because the two natural bracketings — "buffer the data, then push the discipline" vs "push the discipline, then process the data" — yield different results. Data already in value mode (sitting in the buffer) cannot be re-processed through a new computation discipline retroactively.

The `refs/ksh93/sfio-analysis/07-disciplines.md` analysis identifies Dccache as **structurally analogous** to the duploid (+,−) non-associativity failure: the file states the equation directly as `(h ○ g) • f ≠ h ○ (g • f)` and notes that data already buffered (in value mode) cannot be re-processed through a new computation discipline. The mapping is a pattern-match, **not a formal verification** — `docs/spec/` §"The sfio insight" is careful to say "The pattern matches; the full duploid composition laws have not been formally verified for sfio's discipline stack." The reference/sfio_analysis_suite memory carries the same caution. Cite Dccache as a motivating empirical correspondence, not as proof. psh's contribution is to generalize the Dccache pattern from "I/O library mechanism" to "universal discipline at every polarity boundary."

The witness matters for two reasons. First, it shows the failure isn't a theoretical curiosity — production code hits it. Second, it shows the workaround pattern (bracket the boundary, reconcile state, treat the brackets as primitive) is exactly what the duploid theory mandates. psh's contribution is to do this uniformly and on purpose.

## Foundational refs

- `refs/ksh93/sfio-analysis/07-disciplines.md` — load-bearing analysis section. Documents `Sfdisc_t` as endomorphism chain and Dccache as the non-associativity reconciler.
- `refs/ksh93/sfio-analysis/03-buffer-model.md` — the five-pointer buffer model (`_data`, `_next`, `_endr`, `_endw`, `_endb`) is the polarity-typed substrate Dccache reconciles over.
- `reference/papers/duploids` — the (+,−) equation in its categorical form.
- `docs/vdc-framework.md` §8.4 "The non-associativity failure" (line 835) — the framework citation that ties Dccache to the equation.

## Spec sites

`docs/spec/` §"The sfio insight" (line 133) cites this as the structural precedent for psh's polarity frame discipline.

## Status

Settled as the canonical empirical witness. Cite alongside `analysis/polarity/sh_prefix_critical_pair` whenever someone asks "what does the (+,−) failure actually look like in code." The mapping is structural analogy, not formal verification — see `reference/sfio_analysis_suite` §Note for the discipline of citing it as motivating rather than mathematical proof.
