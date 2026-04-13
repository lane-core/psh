---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [forwarders, carbone, marin, schurmann, cut-elimination, star-topology, multiparty-compatibility, propositions-as-types, compositionality]
agents: [psh-session-type-agent, vdc-theory, psh-architect, plan9-systems-engineer]
related: [decision/coprocess_9p_discipline, analysis/wire_format_horizontal_arrow, analysis/nine_p_discipline, reference/papers/carbone_forwarders]
verified_against: [/Users/lane/gist/logical-interpretation-of-async-multiparty-compatbility/forwarders.tex@HEAD lines 494-503, audit/psh-session-type-agent@2026-04-11]
---

# Forwarders as cut (Carbone-Marin-Schürmann)

## Concept

Carbone, Marin, and Schürmann's forwarders paper presents a sequent calculus for asynchronous multiparty session types in which **forwarders compose via cut elimination**. The paper proves the sequent calculus enjoys cut elimination, which means forwarders can be composed and the composition is still a forwarder. The cut elimination proof itself provides the **semantics** for forwarders, in the propositions-as-types style. (`forwarders.tex` §"A Note on Compositionality and Cut Elimination", lines 494–500.)

In psh, the shell IS the forwarder: it routes per-tag binary sessions between coprocesses and the user, mediating the conversation discipline. The star topology — shell-as-hub, no coprocess-to-coprocess communication — is what lets every coprocess interaction reduce to a binary session against a single forwarder. The forwarders paper provides the proof-theoretic ground: any composition of forwarders is still a forwarder, so any psh coprocess composition through the shell-hub is automatically well-formed at the proof-theoretic level.

**Important.** The paper grounds composition in **cut elimination at the proof-theoretic level**, not in any byte-level framing or wire-format property. psh's wire format (`analysis/wire_format_horizontal_arrow`) is the operational realization of the horizontal arrows the paper proves sound — the framing discipline is psh's engineering choice, not a Carbone et al. theorem.

The technical development of the cut elimination proof is deferred in the paper to an extended note (CMS21b) that is **not included in the locally vendored gist**. For psh purposes, the in-paper claim at lines 494–503 is the proximate citable source; CMS21b is one hop deeper.

## Foundational refs

- `reference/papers/carbone_forwarders` — Carbone, Marin, Schürmann. *A Logical Interpretation of Asynchronous Multiparty Compatibility via Forwarders.* Cut elimination claim at `~/gist/logical-interpretation-of-async-multiparty-compatbility/forwarders.tex` lines 494–503.
- `reference/papers/multiparty_automata` — companion operational perspective on multiparty compatibility.

## Spec sites

- `docs/spec/` §"Coprocesses (9P-shaped discipline)" line 763 — psh's coprocess design.
- `decision/coprocess_9p_discipline` — design decision; cites Carbone-Marin-Schürmann directly for star-topology justification.
- `analysis/wire_format_horizontal_arrow` — operational consequence; the framing is psh's choice, not a paper theorem.
- `analysis/nine_p_discipline` — the conversation shape per-tag session sits inside.

## Status

Settled. When asked "why is psh's coprocess star topology safe to compose", this is the citation: forwarders compose via cut elimination, the shell is the forwarder, every coprocess interaction reduces to a binary session against a single forwarder.
