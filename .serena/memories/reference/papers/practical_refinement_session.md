---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [refinement-session-type, refinement-type, inference, protocol-inference, predicate]
agents: [psh-session-type-agent]
---

# Reference: Practical Refinement Session Type Inference

**Path.** `/Users/lane/gist/practical-refinement-session-type-inference/`

**Status.** Theoretical reference. Integrates refinement types with session types.

## Summary

**Refinement types** are types with attached predicates — e.g., `{ x : Int | x > 0 }` for positive integers. This paper shows how to integrate refinement types with session types: a message type can carry a predicate that constrains the exchanged value, and the type system verifies the predicate at communication time.

For psh, this is **future protocol inference**: if we have examples of command/response exchanges, the refinement type system could infer a minimal session type that captures them (e.g., "the first message is always a non-empty string").

## Concepts it informs in psh

- **Future extension: inferred coprocess protocols.** Not in v1. Could let users write coprocess servers without explicit session type declarations; the shell infers the protocol from usage.
- **Constraint checking on coprocess messages** — useful for validation but not currently in the design.

## Who consults it

- **session type agent**: for the refinement-types-with-sessions integration when it becomes relevant.

## Low-confidence rejection note

Not a current psh concern. Consult only if a protocol inference or refinement question arises.
