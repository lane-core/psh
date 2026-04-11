---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [9P, plan9, negotiate, request-response, error-at-any-step, orderly-teardown, session-discipline, conversation-shape, protocol-family, per-tag-session, binary-session]
agents: [psh-session-type-agent, plan9-systems-engineer, psh-architect]
related: [decision/coprocess_9p_discipline, analysis/wire_format_horizontal_arrow, analysis/forwarders_as_cut, analysis/error_duality_oplus_par]
verified_against: [docs/specification.md@HEAD §763-922, decision/coprocess_9p_discipline, audit/psh-session-type-agent@2026-04-11]
---

# 9P discipline (negotiate / request-response / error / teardown)

## Concept

psh extracts Plan 9's 9P protocol **shape** — its conversation discipline — as the session pattern for bidirectional shell↔child channels. Per `decision/coprocess_9p_discipline`, the four-part discipline:

1. **Negotiate.** One round-trip confirming both sides speak the same protocol. psh's handshake is `"psh protocol v1"` and validates protocol version only.
2. **Request-response pairs.** Every request gets a response. No fire-and-forget. No ambiguity about whose turn it is. Each in-flight pair has a binary session type `Send<Req, Recv<Resp, End>>` — exactly one legal action at each step.
3. **Error at any step.** Failure is always a valid response, not a special case. Error responses produce nonzero status on `read -p` with the error message bound to the reply variable. Standard ⊕ error handling applies (`analysis/error_duality_oplus_par`).
4. **Orderly teardown.** Explicit close with reason. EOF is the crash fallback, not the normal path.

This is the **conversation shape**, not the bytes. psh borrows 9P's discipline without inheriting 9P's wire protocol — the wire format is psh's own choice (`analysis/wire_format_horizontal_arrow`). The 9P heritage is **conceptual**: the four-part discipline is what makes a protocol composable enough to multiplex per-tag binary sessions over a single channel.

The proof-theoretic ground for composing per-tag sessions through the shell-hub is forwarders compose via cut elimination (`analysis/forwarders_as_cut`); 9P provides the **shape**, forwarders provide the **soundness argument**.

## Foundational refs

- `decision/coprocess_9p_discipline` — authoritative protocol decision with full per-tag binary session structure, wire format, tag management, and the explicit citation that psh extracts 9P's conversation shape, not its bytes. **The decision memo is the canonical source for psh's 9P-shaped protocol.**
- The original 9P specification (Plan 9 manual section 5) is **not vendored locally** in `refs/plan9/`. A glob for `refs/plan9/**/9P*` returns no files. The 9P shape lives in psh's own materials by paraphrase from `decision/coprocess_9p_discipline` and the design lineage spec section, not by primary citation. If a future contributor needs to verify a fine-grained 9P claim against Plan 9's primary documentation, they need to consult Plan 9 manual section 5 externally.
- `analysis/forwarders_as_cut` — Carbone-Marin-Schürmann. The proof-theoretic ground for composing per-tag sessions through the shell-hub.

## Spec sites

- `docs/specification.md` §"Coprocesses (9P-shaped discipline)" line 763 — protocol context, design lineage, per-tag binary sessions, wire format, named coprocesses.
- `decision/coprocess_9p_discipline` — the full design decision.

## Status

Settled at the framing level. The four-part discipline is fixed for v1. The original Plan 9 9P primary source is a known gap in the locally vendored materials — psh treats `decision/coprocess_9p_discipline` as the canonical source for the discipline shape.
