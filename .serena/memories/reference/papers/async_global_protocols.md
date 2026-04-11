---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [asynchronous, global-protocol, realizability, multiparty-session-type, async-interaction, buffer, reordering]
agents: [psh-session-type-agent]
---

# Reference: Asynchronous Global Protocols

**Path.** `/Users/lane/gist/asynchronous-global-protocols/`

**Status.** Theoretical reference. Async multiparty session types and realizability.

## Summary

Studies **asynchronous** global protocols (where messages can be buffered and reordered) vs synchronous global protocols (where each send is immediately followed by the matching receive). Asynchronous protocols are more realistic but require additional realizability conditions to ensure a given global spec can be implemented by concrete local participants.

For psh: coprocess interactions are fundamentally **asynchronous** because the socketpair has finite buffer space and processes run concurrently. Any formal treatment of coprocess protocols needs the async framework, not the simpler synchronous one.

## Concepts it informs in psh

- **`decision/coprocess_9p_discipline`** — the async nature of socketpair-based coprocess IPC. Sends don't block until the buffer fills; the shell and the coprocess can race on multiple outstanding tags.
- **PendingReply tracking** — the shell's bookkeeping for outstanding requests is the async counterpart to "how many messages are currently in flight."
- **Realizability** — when designing a new coprocess protocol, verify it's realizable asynchronously (not just synchronously).

## Who consults it

- **session type agent**: for the async framework applied to coprocess protocol design.

## Note

Related to `reference/papers/carbone_forwarders` (both are multiparty session type foundations) but asynchronous-focused.
