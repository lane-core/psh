---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [multiparty-compatibility, communicating-automata, operational, protocol-interaction, automata-theory, session-type]
agents: [psh-session-type-agent]
---

# Reference: Multiparty Compatibility in Communicating Automata

**Path.** `/Users/lane/gist/multiparty-compatbility-in-communicating-automata/`

**Filename note.** "compatbility" is a typo in the gist directory name. Cite by author and topic, not filename.

**Status.** Theoretical reference. Automata-theoretic complement to the Carbone-Marin-Schürmann logical interpretation.

## Summary

Multiparty compatibility viewed through communicating automata rather than linear logic. Where Carbone et al. frame the question logically (forwarders as cut-elimination), this paper provides the **operational perspective**: step-by-step automata interaction, sound state-space exploration for deadlock detection.

The two papers are complementary: Carbone et al. give the "why any composition reduces to forwarders"; this paper gives the "what does the runtime look like during the interaction."

## Concepts it informs in psh

- **`decision/coprocess_9p_discipline`** — operational reasoning about protocol interactions. When you need to argue "what happens when Tag N is sent but Tag M's response arrives first?" — this is the framework.
- **PendingReply semantics** — the shell's internal tag tracking is an automaton state, and the automata paper gives the vocabulary for describing its transitions.
- **Debugging / tracing** — if a coprocess interaction goes wrong at runtime, the automata framing is what you use to diagnose.

## Who consults it

- **session type agent** (primary): operational follow-up to Carbone-Marin-Schürmann.

## Note

Read this after Carbone-Marin-Schürmann. The logical framework is the theoretical ground; the automata paper is how you apply it at runtime.
