---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [actor-model, multiparty-session-type, safe-actor, mailbox, hub-and-spoke, shell-as-hub]
agents: [psh-session-type-agent, psh-architect]
---

# Reference: Safe Actor Programming with Multiparty Session Types

**Path.** `/Users/lane/gist/safe-actor-programming-with-multiparty-session-types/`

**Status.** Theoretical + implementation reference. Integrates the actor model with multiparty session types.

## Summary

Actors communicate by message passing through mailboxes. Multiparty session types can discipline actor interactions so that the type system prevents the usual actor-model failure modes (stuck receives, unhandled messages, deadlock).

For psh, this paper informs the **shell-as-hub** design: the shell is an actor (with a mailbox of pending coprocess messages), and each coprocess is an actor (with its own mailbox). The star topology makes the shell the **hub actor** that all other actors communicate through, which matches the `decision/coprocess_9p_discipline` star topology.

## Concepts it informs in psh

- **`decision/coprocess_9p_discipline`** — the actor model framing of shell-as-hub. Each coprocess has its own mailbox (its socketpair buffer); the shell's PendingReply table is the hub actor's state.
- **Deadlock avoidance via type discipline** — same theme as Carbone-Marin-Schürmann but from an actor-model angle.
- **Rust implementation** — complements `reference/papers/deadlock_free_async_rust` with an actor-oriented perspective.

## Who consults it

- **session type agent**: for the actor-model framing when reasoning about shell/coprocess roles.
- **psh-architect** (secondary): for the mailbox / hub implementation patterns.

## Note

Less load-bearing than Carbone-Marin-Schürmann for the theoretical justification of psh's topology, but useful for the operational pattern (hub-and-spoke actor model). Read after the forwarder paper.
