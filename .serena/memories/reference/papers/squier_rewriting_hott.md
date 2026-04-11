---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [squier, rewriting, hott, homotopy-type-theory, kraus, von-raumer, critical-pair, local-confluence, higher-coherence, cell-completion]
agents: [vdc-theory, psh-sequent-calculus]
---

# Reference: Squier's Theorem in HoTT (Kraus-von Raumer)

**Path.** `/Users/lane/gist/squier-rewriting-hott.gist.txt`

**Citation.** Kraus and von Raumer, *Squier's theorem in homotopy type theory*.

**Status.** Theoretical reference for rewriting and coherence.

## Summary

Develops Squier's theorem in the setting of homotopy type theory. Squier's theorem gives a **local-to-global coherence result** for rewriting systems: if all critical pairs have confluent completions (local confluence), the whole rewriting system has higher-dimensional coherence.

The rewriting-as-cells interpretation lets you read a shell's evaluation step as a cell, confluence as cell completion, and critical pairs as 2-cells.

## Concepts it informs in psh

For psh this is relevant when reasoning about whether **multiple resolution mechanisms compose coherently**. psh has at least four resolution mechanisms that all operate on the same critical pair structure:

- **Polarity frames** (save/restore around boundary crossings)
- **CBV focusing** (once-per-expression value reuse)
- **Lexical trap scoping** (μ-binder with block-scope lifetime)
- **`try`'s `;ₜ` sequencer** (monadic bind with status check)

Squier-style analysis gives the local-to-global coherence argument for why these compose. When a psh feature interacts with multiple resolution mechanisms, the Squier framework checks that the local confluence diagrams chain into a global coherence.

## Concepts in psh docs

- **`decision/codata_discipline_functions`** — CBV focusing as critical pair resolution.
- **`docs/vdc-framework.md` §9.3** — polarity frame discipline.
- **`decision/unified_trap_three_forms`** — lexical trap as μ-binder.
- **`decision/try_catch_scoped_errort`** — `;ₜ` sequencer as monadic bind.

## Who consults it

- **vdc-theory agent** (primary): when composition of multiple resolution mechanisms needs a coherence argument.
- **sequent calculus agent** (primary): for critical pair analysis and confluence.

## Low-confidence rejection note

Squier gives coherence from local to global, not the local analysis itself. If you don't have the local confluence data, Squier doesn't help. Say so in that case.
