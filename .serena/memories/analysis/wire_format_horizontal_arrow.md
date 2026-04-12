---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [wire-format, horizontal-arrow, vdc, channel, length-prefix, framing, max-frame-size, coprocess, 9p, binary-frames]
agents: [vdc-theory, psh-session-type-agent, psh-architect, plan9-systems-engineer]
related: [decision/coprocess_9p_discipline, reference/papers/carbone_forwarders, reference/papers/fcmonads]
---

# Wire format as horizontal arrow discipline

## Concept

In a **virtual double category**, horizontal arrows are the "channel" component of the structure — they connect objects (interfaces) and form the spans that vertical arrows transform. In psh's VDC mapping (`docs/vdc-framework.md` §5.2), **horizontal arrows are channels**: the actual byte-carrying things between processes.

The **wire format discipline** is psh's commitment to treating every channel — pipe, fd, coprocess socketpair, future typed pipe — as a **typed horizontal arrow with an explicit framing protocol**. For coprocesses specifically, this means:

- **Length-prefixed binary frames.** Every message is a `u32` length followed by that many bytes of payload. No newline framing, no escape characters, no in-band signaling.
- **MAX_FRAME_SIZE 16 MiB.** Hard cap, enforced at both ends. Frames larger than this are a protocol violation, not a degraded condition.
- **Per-tag binary sessions multiplexed over one socketpair.** Each in-flight request-response pair has a tag; the tag selects which binary session the frame belongs to.

The horizontal arrow framing is what lets the **§8.5 decision procedure** classify coprocess operations cleanly. Forwarders (Carbone-Marin-Schürmann) compose horizontal arrows via **cut elimination** at the proof-theoretic level (`forwarders.tex` lines 494–503: "The sequent calculus we presented enjoys cut elimination. That means that forwarders can be composed, and their composition is still a forwarder."). psh's wire format is the operational realization of those arrows in bytes — the framing discipline is psh's engineering choice, not a theorem from Carbone et al.

## Foundational refs

- `docs/vdc-framework.md` §5.2 "Horizontal Arrows = Channels" (line 417) — the VDC mapping that licenses the framing.
- `docs/vdc-framework.md` §9.2 "The Horizontal Arrow Discipline" (line 923) — engineering principle.
- `reference/papers/fcmonads` — Cruttwell-Shulman **§2** "Virtual double categories" (`fcmonads.gist.txt` line 2312, formal definition at line 2422). Horizontal arrows are introduced at line 2432 as part of the §2 definition. (§3 at line 2776 is "Monads on a virtual double category" — section numbers in the existing reference memo for §5/§6/§7 may also need re-verification per the same audit.)
- `reference/papers/carbone_forwarders` — Carbone-Marin-Schürmann. Forwarders compose via cut elimination (`forwarders.tex` §"A Note on Compositionality and Cut Elimination" lines 494–503). Carbone et al. does **not** discuss byte-level framing; the wire format is psh's operational realization of the horizontal arrows the paper proves sound proof-theoretically.

## Spec sites

- `docs/specification.md` §"Wire format" (line 883) — frame layout (length-prefix, tag, payload).
- MAX_FRAME_SIZE 16 MiB cap is in specification.md §Wire format.
- `docs/specification.md` §"Coprocesses (9P-shaped discipline)" (line 763) — the protocol context.
- `decision/coprocess_9p_discipline` — design decision.

## Status

Settled. The 16 MiB cap is a v1 commitment and not subject to negotiation per session. Future typed-pipe work (deferred to v2 per `PLAN.md`) will inherit the horizontal-arrow discipline; only the framing protocol may be revisited.
