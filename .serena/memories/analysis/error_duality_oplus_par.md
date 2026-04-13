---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [oplus, par, error-duality, status, trap, try-catch, de-morgan, linear-logic, additive, multiplicative, callback, result-type, data-codata, orthogonal-composition]
agents: [psh-sequent-calculus, vdc-theory, plan9-systems-engineer]
related: [decision/try_catch_scoped_errort, decision/unified_trap_three_forms, analysis/data_vs_codata, analysis/cbpv_f_u_separation, reference/papers/grokking_sequent_calculus, reference/ksh93_analysis]
verified_against: [docs/spec/@HEAD §946-1044, refs/ksh93/ksh93-analysis.md@HEAD §206-226, /Users/lane/gist/grokking-the-sequent-calculus.gist.txt@HEAD lines 11509-11552, audit/psh-sequent-calculus@2026-04-11]
---

# ⊕ / ⅋ error-handling duality

## Concept

psh has two co-existing conventions for error handling, dual in the sense of linear logic:

- **⊕ (positive — caller inspects):** Every operation returns `Status(pub String)`. The caller pattern-matches on the status (`if cmd { ok } else { err }`, `try { body } catch (e) { handler }`). Like Rust's `Result<T,E>`. **Data type, eliminated by case/pattern-match.**
- **⅋ (negative — callee invokes a continuation):** Trap handlers fire on signals. The callee chooses which continuation to invoke; the caller doesn't need to check anything. Like passing onSuccess/onFailure callbacks. **Codata type, eliminated by copattern-match** (`analysis/data_vs_codata`).

Both conventions are present because both are legitimate. The sequent calculus explains the relationship: ⊕ and ⅋ are **De Morgan duals**, connected by the same involutive negation that swaps CBV and CBN (`refs/ksh93/ksh93-analysis.md` §"⊕ / ⅋ error-handling duality" lines 206–226).

In psh's spec, the operational realization at `docs/spec/` §"⊕ and ⅋" lines 953–955: "`$status` is ⊕ (positive — caller inspects a tagged value). Traps are ⅋ (negative — callee invokes a continuation). Both are present."

- **`try`/`catch` is the ⊕ sequencer.** `try { body } catch (e) { handler }` changes the sequencing combinator within `body` from unconditional `;` to monadic `;ₜ` that checks Status after each command. On nonzero status, execution aborts to the handler. The handler binding `e` is a μ̃-binder on the error case. (Spec line 957.)
- **`trap` is the ⅋ binder.** A lexical `trap SIGNAL { handler } { body }` is the **μ-binder of Curien-Herbelin §2.1**: it captures a signal continuation scoped to the body. (Spec line 974.)

The orthogonal-composition property at `docs/spec/` lines 1040–1044 is the operational consequence: "`trap` and `try` compose orthogonally because they operate on different sorts: `trap` on signal continuations (⅋), `try` on command status (⊕)." A lexical `trap` inside a `try` body fires first when a signal arrives; if the trap returns a status, try inspects it through its normal status-check mechanism.

**Note on linear-logic notation.** ⊕ and ⅋ are the **two disjunctions** of linear logic — ⊕ is additive disjunction, ⅋ is multiplicative disjunction. Grokking (`~/gist/grokking-the-sequent-calculus.gist.txt:11509`) introduces them as such: "In addition to these two different kinds of conjunction, we also have two different kinds of disjunction. These two disjunctions are written σ ⊕ τ (pronounced 'plus') and σ ⅋ τ (pronounced 'par') and correspond to two different ways to handle errors." This pairing is **standard linear logic**, not a deviation. ksh93-analysis cites grokking as `[7]` for this framing. The "De Morgan duals" language (positive/negative duality between caller-inspects and callee-invokes) is from `refs/ksh93/ksh93-analysis.md` line 225 — grokking itself uses "duality between the two different ways of handling exceptions" (gist line 11552).

## Foundational refs

- `reference/papers/grokking_sequent_calculus` — Binder, Tzschentke, Müller, Ostermann. ⊕/⅋ error handling presented as a duality in the functional pearl. The reference memo summarizes: "Tagged error return (like `$status` / Rust's `Result`) is dual to continuation-based error handling (like traps / JS onSuccess/onFailure callbacks)."
- `refs/ksh93/ksh93-analysis.md` §"⊕ / ⅋ error-handling duality" lines 206–226 — the table of ⊕ vs ⅋ conventions and the De Morgan duality framing. Cites `[7]` (grokking) for the error-handling interpretation.
- `reference/papers/linear_logic_without_units` — Houston's thesis. Cited as background for the unit-free MLL substrate; not the direct source of the error-handling interpretation.

## Spec sites

- `docs/spec/` §"⊕ and ⅋" line 946 — authoritative for psh's framing.
- `docs/spec/` §"try/catch — scoped ErrorT (⊕ discipline)" line 957 — the ⊕ side.
- `docs/spec/` §"trap — unified signal handling (⅋ discipline)" line 969 — the ⅋ side.
- `docs/spec/` lines 1040–1044 — orthogonal composition.
- `decision/try_catch_scoped_errort` — design decision (⊕).
- `decision/unified_trap_three_forms` — design decision (⅋).

## Status

Settled. The orthogonal-composition property at spec §1040 is the operational consequence: try-blocks and trap-handlers don't interfere because they're typed on different sorts. Architect should treat the two as truly independent control mechanisms in the evaluator.
