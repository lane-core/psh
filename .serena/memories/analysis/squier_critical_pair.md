---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [squier, critical-pair, coherence, local-confluence, rewriting, kraus, von-raumer, hott, cell-completion, multiple-resolution-mechanisms, deferred-verification]
agents: [vdc-theory, psh-sequent-calculus]
related: [reference/papers/squier_rewriting_hott, analysis/polarity/cbv_focusing, analysis/polarity/_hub, reference/papers/grokking_sequent_calculus]
verified_against: [reference/papers/squier_rewriting_hott, /Users/lane/gist/squier-rewriting-hott.gist.txt@HEAD lines 508-541, 590-598, 1105, audit/vdc-theory@2026-04-11]
---

# Squier critical pair (coherence from local confluence)

## Concept

**Squier's theorem** (Kraus and von Raumer, *A Rewriting Coherence Theorem with Applications in Homotopy Type Theory*, `~/gist/squier-rewriting-hott.gist.txt:508`) gives a **local-to-global coherence result** for rewriting systems: if all critical pairs have confluent completions (local confluence), then the rewriting system has higher-dimensional coherence. The cells reading — reduction steps as 1-cells, local confluence diagrams filling critical-pair peaks with 2-cells — is the standard reading of higher-dimensional rewriting the paper works in (cf. `~/gist/squier-rewriting-hott.gist.txt` lines 590–598, 1105). Critical pairs themselves are *peaks*; the diagrams *filling* them sit in the 2-cell layer.

In psh, this matters when **multiple resolution mechanisms operate on the same critical pair structure** and we need to know they compose coherently. psh has at least four resolution mechanisms that operate at the polarity boundary:

1. **Polarity frames** (save/restore around shifts; `analysis/polarity/frames`)
2. **CBV focusing** (once-per-expression value reuse; `analysis/polarity/cbv_focusing`)
3. **Lexical trap scoping** (μ-binder with block-scope lifetime; `decision/unified_trap_three_forms`)
4. **`try`'s `;ₜ` sequencer** (monadic bind with status check; `decision/try_catch_scoped_errort`)

A Squier-style argument would give the local-to-global coherence: if each mechanism's local confluence diagram is verified, the chain of diagrams composes to global coherence. When a psh feature interacts with multiple resolution mechanisms, the Squier framework is the right tool to check the local pieces chain.

**Important caveat** (from `reference/papers/squier_rewriting_hott` §"Low-confidence rejection note"): Squier gives coherence from local to global, **not** the local analysis itself. If you don't have the local confluence data for each mechanism, Squier doesn't help. The current state in psh: local confluence is **claimed informally** for each mechanism but **not formally verified**. Squier is the framework to use **when** that verification work is done; it is not a substitute for the verification.

The μ vs μ̃ critical pair (`reference/papers/grokking_sequent_calculus`) is the one psh resolves operationally via CBV focusing (`analysis/polarity/cbv_focusing`): `⟨μα.s₁ | μ̃x.s₂⟩` reduces in two ways (CBV vs CBN), and CBV focusing fixes the order. Squier would let psh prove this resolution composes coherently with the other resolution mechanisms — work psh has not yet undertaken.

## Foundational refs

- `reference/papers/squier_rewriting_hott` — Kraus and von Raumer. The statement of Squier's theorem in HoTT (paper title "A Rewriting Coherence Theorem with Applications in Homotopy Type Theory" at gist line 508; abstract at line 515). The reference memo flags the low-confidence rejection note.
- `reference/papers/grokking_sequent_calculus` — for the standard statement of the μ/μ̃ critical pair (Binder et al.), which is the local data Squier's theorem would compose.

## Spec sites

The spec does not cite Squier directly. The cited dependency is on **CBV focusing** (`docs/spec/` §"CBV focusing as the reentrancy semantics" line 556), which resolves the most prominent critical pair operationally. Squier is the meta-rewriting tool that would let psh prove all four resolution mechanisms compose coherently — work psh has not yet undertaken.

## Status

**Open / deferred verification.** This is the framework to use **if and when** psh formally verifies the composition coherence of its resolution mechanisms. Currently the composition is justified informally. The polarity hub's open questions section (`analysis/polarity/_hub`) flags this as a deferred coherence question.

**Note for tier-3 expansion.** **Signal precedence** (innermost lexical > outer lexical > global > OS default, per `decision/unified_trap_three_forms`) is arguably a **fifth resolution mechanism** — it resolves a critical pair between competing trap handlers. The current four-mechanism enumeration above folds it into "lexical trap scoping" for compactness; the eventual tier-3 Squier writeup should treat it explicitly, since the precedence ordering is exactly the kind of local-confluence data Squier consumes.
