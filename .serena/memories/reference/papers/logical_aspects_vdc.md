---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [fvdbltt, vdc, type-theory, hayashi, das, protype, channel-type, restriction, interface-transformation, comprehension-type, protocol-state]
agents: [vdc-theory, psh-session-type-agent]
---

# Reference: Logical Aspects of VDCs / FVDblTT (Hayashi, Das et al.)

**Path.** `/Users/lane/gist/logical-aspects-vdc.gist.txt`

**Status.** Theoretical reference. Introduces FVDblTT — the type theory of virtual double categories.

## Summary

FVDblTT is a type theory in which types have "shapes" analogous to the horizontal/vertical distinction in a VDC. Key concepts for psh:

- **Protypes** — types with both positive and negative aspects. Map to psh's channel types carrying protocols with both directions.
- **Restrictions** ↔ **interface transformations on channel types**. A vertical arrow acting on a horizontal arrow becomes a type-theoretic coercion on the protocol.
- **Comprehension types** ↔ **observation of protocol state**. A comprehension type lets you name and reason about the state of a channel without consuming it.

For psh, these are the type-theoretic ingredients for coprocess protocols and any future typed-channel extension. The FVDblTT framework is what would make "typed pipes" formally grounded.

## Concepts it informs in psh

- **Virtual double category structure** — psh's framework as a VDC, viewed type-theoretically.
- **`decision/coprocess_9p_discipline`** — protypes as session types on coprocess channels.
- **Typed pipes (future extension)** — typed pipes would need comprehension types to observe pipe buffer state.
- **Thunkable = central** — comprehension types relate to thunkability in the VDC setting.

## Who consults it

- **vdc-theory agent** (primary): for type-theoretic grounding questions ("is this new connective well-grounded in FVDblTT?").
- **session type agent** (secondary): for protype interpretations of coprocess channels.

## Note

Not a first-read reference for most questions. Consult on demand when a type-theoretic grounding check is needed. The paper is denser than the duploids paper and assumes more VDC background.
