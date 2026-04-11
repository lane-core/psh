---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [grokking, binder, tzschentke, muller, ostermann, sequent-calculus, lambda-mu-mu-tilde, functional-pearl, compiler-ir, downen, focusing, let-control-duality, oplus-par, case-of-case, data-codata]
agents: [psh-sequent-calculus, psh-architect, vdc-theory]
---

# Reference: Grokking the Sequent Calculus (Binder et al.)

**Path.** `/Users/lane/gist/grokking-the-sequent-calculus.gist.txt`

**Citation.** Binder, Tzschentke, Müller, Ostermann, *Grokking the Sequent Calculus (Functional Pearl)*, 2023.

**Status.** Primary theoretical reference. **The most accessible introduction to λμμ̃ in the collection.** Read first in any new sequent calculus session.

## Summary

Presents λμμ̃ as a compiler intermediate language, compiling a surface language **Fun** into a sequent-calculus-based **Core**. Written as a functional pearl — programmer-facing, concrete examples, minimal category theory. Key insights:

- **First-class evaluation contexts.** The μ̃-binder reifies "what happens next" as a bindable object. This is what ksh93's `struct checkpt` already is (see `reference/ksh93_analysis`).
- **Let/control duality.** Let-bindings (μ̃) are *exactly dual* to control operators (μ). Variable assignment is dual to trap/label setup. Not two separate mechanisms — the same operation viewed from opposite sides of the cut.
- **Case-of-case falls out as μ-reduction.** Commutative conversions (important compiler optimizations) are just ordinary β-reduction in the sequent calculus.
- **⊕ vs ⅋ error handling.** Tagged error return (like `$status` / Rust's `Result`) is dual to continuation-based error handling (like traps / JS onSuccess/onFailure callbacks). The shell has both conventions; the sequent calculus explains why they coexist.
- **Data vs codata duality.** Data types are defined by constructors (eliminated by pattern matching); codata types are defined by destructors (eliminated by copattern matching).
- **Downen et al. static focusing** is referenced here; psh uses it via `docs/vdc-framework.md` §6.2 for argument expansion focusing.

## Concepts it informs in psh

- **λμμ̃ three sorts** — `docs/specification.md` §"The three sorts, made explicit" cites Curien-Herbelin and, through them, this paper's introduction.
- **Let/control duality** — `decision/let_is_mu_tilde_binder_cbpv` vs `decision/unified_trap_three_forms`. The dual binders.
- **⊕/⅋ error handling** — `decision/try_catch_scoped_errort` (⊕, data) vs `decision/unified_trap_three_forms` (⅋, codata).
- **`decision/codata_discipline_functions`** — data/codata duality. Discipline-backed variables are codata; ordinary variables are data.
- **CBV focusing** — Downen static focusing applied to argument expansion, realized operationally in psh.

## Who consults it

- **sequent calculus agent** (primary, canonical — first read).
- **psh-architect** (primary): programmer-facing introduction; good background before implementing the AST.
- **vdc-theory agent** (secondary).
- **plan9 agent**: for the ksh93 `struct checkpt` connection in `reference/ksh93_analysis`.
