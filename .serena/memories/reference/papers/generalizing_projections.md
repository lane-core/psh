---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [generalizing-projection, multiparty-session-type, global-protocol, local-projection, endpoint-projection]
agents: [psh-session-type-agent]
---

# Reference: Generalizing Projections in Multiparty Session Types

**Path.** `/Users/lane/gist/generalizing-projections-in-multiparty-session-types/`

**Status.** Theoretical reference. Generalizes the global-to-local projection used in multiparty session types.

## Summary

A **global protocol** describes the entire multiparty interaction from a bird's-eye view. A **local projection** takes the global and extracts, for each participant, just the protocol that participant sees. Classical multiparty session type theory uses a fixed projection function; this paper **generalizes** the projection to cover more cases without breaking soundness.

For psh: if there's ever a global specification of a multi-coprocess interaction (e.g., "when coprocess A sends X, coprocess B should respond with Y, and then the shell forwards to coprocess C"), the projection mechanism is what lets you derive each participant's local protocol from the global spec.

## Concepts it informs in psh

- **Future extension: global coprocess specs.** psh currently specifies coprocess protocols per-coprocess (star topology, one binary session per tag). A future extension could add global specs that get projected to per-coprocess local protocols.
- **Typed pipes with global protocols** — a typed pipe carrying a multiparty session could be specified globally and projected at compile time.

## Who consults it

- **session type agent**: when designing or reviewing a protocol where multiple coprocesses interact through the shell in a coordinated way. Not needed for current star-topology design.

## Low-confidence rejection note

For the current psh coprocess design (star topology, binary sessions per tag), projection machinery is overkill. Only consult this paper when a genuine multiparty use case arises.
