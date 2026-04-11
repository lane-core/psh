---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [dependent-session-type, protocol, value-dependent, message-type, future-extension]
agents: [psh-session-type-agent]
---

# Reference: Dependent Session Types

**Path.** `/Users/lane/gist/dependent-session-types/`

**Status.** Theoretical reference. Dependent session types — where message types can depend on previously exchanged values.

## Summary

Classical session types fix message types in advance: "send an Int, then receive a String." Dependent session types let **later** message types depend on **earlier** values: "send an Int n, then receive a list of length n."

For psh, this is aspirational. Current coprocesses use simple binary sessions (`Send<Req, Recv<Resp, End>>`) with fixed message types per tag. A future extension could allow message types to depend on values previously exchanged in the same session.

## Concepts it informs in psh

- **Future extension: dependent coprocess sessions.** Not in v1. A coprocess could, e.g., send a schema announcement as the first message and have subsequent messages typed by that schema.
- **Typed pipes (future extension)** — if typed pipes carry a protocol, and the protocol depends on an initial handshake, dependent session types are the framework.

## Who consults it

- **session type agent**: when the question is "could we strengthen the coprocess type discipline with value-dependent messages?" Not needed for current protocol design.

## Low-confidence rejection note

psh does not currently use dependent session types. If asked about them, check whether the question is about the **current** coprocess protocol (→ `reference/papers/carbone_forwarders`) or about **future extensions** (→ this reference).
