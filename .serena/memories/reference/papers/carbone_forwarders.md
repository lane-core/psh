---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [carbone, marin, schurmann, multiparty-compatibility, forwarder, linear-logic, cut-elimination, star-topology, deadlock-freedom, async, session-type]
agents: [psh-session-type-agent, vdc-theory]
---

# Reference: Logical Interpretation of Asynchronous Multiparty Compatibility (Carbone-Marin-Schürmann)

**Path.** `/Users/lane/gist/logical-interpretation-of-async-multiparty-compatbility/`

**Citation.** Carbone, Marin, Schürmann, *Logical Interpretation of Asynchronous Multiparty Compatibility*.

**Filename note.** "compatbility" is a typo in the gist directory name. Cite the paper by author and topic, not filename.

**Status.** Primary theoretical reference. **The load-bearing justification for psh's star topology.** Read first in any new session type session.

## Summary

Proves that **forwarders capture all multiparty compatible compositions** via a linear-logic argument. A forwarder is a cell that mediates between participants in a multiparty session, and cut-elimination over forwarders yields the composite protocol.

**Key theorem:** any multiparty session composition reduces to one in which the participants communicate through a forwarder. This means star-topology protocols (with a central forwarder and leaf participants) are not a restricted case — they cover the entire space of multiparty-compatible compositions.

## Concepts it informs in psh

- **`decision/coprocess_9p_discipline`** — the **star topology** justification. psh's shell is the forwarder; each coprocess is a leaf. The Carbone-Marin-Schürmann theorem is why this topology is not a restriction — it's the general case.
- **Deadlock freedom** — as a corollary of the forwarder framing, the shell's asymmetric initiator/responder discipline (shell always initiates; coprocess always responds) gives deadlock freedom for the whole multiparty composition.
- **Coprocess channels as horizontal arrows** — `docs/vdc-framework.md` §4-5 (VDC channels carrying session types).

## Who consults it

- **session type agent** (primary, canonical): read first in any new session. This paper is the single most important reference for coprocess design.
- **vdc-theory agent**: for the forwarder / cell correspondence.

## Note

When citing: "Per Carbone-Marin-Schürmann §N, the star topology is sound because..." — cite the paper by the author names and section number.
