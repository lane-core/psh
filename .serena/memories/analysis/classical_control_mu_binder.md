---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [classical-control, mu-binder, curien-herbelin, continuation-capture, signal-continuation, trap-as-mu, let-control-duality, sigjmp-buf, ksh93, lexical-scoping, delimited-continuation, classical-contraction]
agents: [psh-sequent-calculus, plan9-systems-engineer, vdc-theory]
related: [decision/unified_trap_three_forms, decision/let_is_mu_tilde_binder_cbpv, analysis/cbpv_f_u_separation, analysis/error_duality_oplus_par, analysis/cut_as_execution, analysis/polarity/sh_prefix_critical_pair]
verified_against: [docs/spec/@HEAD §969-991, refs/ksh93/ksh93-analysis.md@HEAD §228-260, audit/psh-sequent-calculus@2026-04-11]
---

# Classical control as μ-binder

## Concept

In the λμμ̃-calculus, **μα.s** (the "mu" binder) captures the current **continuation** (evaluation context) and binds it to α. This is the dual of **μ̃x.s** (the "mu-tilde" binder), which captures the current **value** and binds it to x. The let/control duality (per `reference/papers/grokking_sequent_calculus`): variable binding and continuation binding are the same structural operation applied to opposite sides of the sequent.

**In psh, lexical `trap` is the operational μ-binder.** From `docs/spec/` §"trap — unified signal handling" line 974:

> "**Lexical** (two blocks): `trap SIGNAL { handler } { body }` — installs the handler for the duration of the body, the μ-binder of Curien-Herbelin [5, §2.1]. The handler captures a signal continuation scoped to the body. Inner lexical traps shadow outer for the same signal."

The handler binds α (a name for "the signal continuation") for the duration of `body`. When the signal fires, control jumps into the handler with the continuation in scope; the handler may `return N` to abort the body with status N (operationally invoking the captured continuation with a value).

The let/control duality is the structural reason `trap` and `let` look so similar at the grammar level: both bind a name in a context. They're not two separate mechanisms — they're μ vs μ̃, the two binders of the calculus. `decision/let_is_mu_tilde_binder_cbpv` is the value-binding side; `decision/unified_trap_three_forms` is the continuation-binding side.

**ksh93's classical control machinery** (`refs/ksh93/ksh93-analysis.md` §"Continuations and classical control" lines 228-260) is what psh tames. The table at lines 230-237 maps shell mechanisms to sequent-calculus analogs:

- `sigjmp_buf` / `struct checkpt` are continuation frames (reified coterms) — fault.h
- `sh.jmplist` is the continuation stack (μ-variable binding) — shell.h
- `sh_pushcontext` / `sh_popcontext` are save/restore for continuations — fault.h
- Traps (DEBUG, ERR, EXIT) are delimited continuations stored in `sh.st.trap[]`
- `break` / `continue` / `return` are named continuation jumps (`goto α`)
- Subshell `(...)` is **classical contraction** (fork the continuation) — xec.c TPAR handler

ksh93 implemented all of this with global mutation on `Shell_t`, leading to the bug class catalogued at `analysis/polarity/sh_prefix_critical_pair` (Bug 003a/003b are stale-context violations on the saved continuation state). **psh's response is lexical scoping**: the μ-binder is bound for exactly the duration of a lexical block, with no global mutation, no `sigjmp_buf` / `longjmp` in the Rust implementation. Signal delivery via self-pipe wake + poll (per `decision/unified_trap_three_forms`).

**The orthogonality with try.** Per `analysis/error_duality_oplus_par`, `trap` and `try` operate on different sorts: `trap` on signal continuations (⅋, codata), `try` on command status (⊕, data). The composition is orthogonal because the binders live on opposite sides of the cut — μ on the right (continuation), μ̃ on the left (value).

## Foundational refs

- Curien, Herbelin. *The Duality of Computation*. ICFP 2000. **Not vendored** in `~/gist/`; cited from `decision/unified_trap_three_forms` and `docs/spec/` line 974 as `[5, §2.1]`.
- `reference/papers/grokking_sequent_calculus` — Binder et al. The let/control duality is presented as a key insight of the functional pearl per the existing reference memo: "Let-bindings (μ̃) are *exactly dual* to control operators (μ). Variable assignment is dual to trap/label setup. Not two separate mechanisms — the same operation viewed from opposite sides of the cut."
- `refs/ksh93/ksh93-analysis.md` §"Continuations and classical control" lines 228-260 — the ksh93 negative example: classical control via global mutation, with the table mapping shell mechanisms to calculus analogs.

## Spec sites

- `docs/spec/` §"trap — unified signal handling (⅋ discipline)" line 969 — authoritative for psh's μ-binder framing.
- `docs/spec/` line 974 — explicit "the μ-binder of Curien-Herbelin §2.1" attribution.
- `decision/unified_trap_three_forms` — design decision; cites Curien-Herbelin and the lexical-scoping rationale.
- `analysis/cbpv_f_u_separation` — the let half of the let/control duality; this anchor is the trap half.
- `analysis/error_duality_oplus_par` — orthogonal composition with try.
- `analysis/polarity/sh_prefix_critical_pair` — the negative-example bug class psh's lexical scoping is designed to prevent. Bugs 003a/003b are stale-context violations where `sh_debug()`'s blanket restore of saved `sh.st` overwrites the handler's intentional `trap[]` mutations — the immediate failure mode is trap-state corruption, not continuation-capture per se, but the structural cause (global mutation of state that should be lexically scoped) is the same.

## Status

Settled. The let/control duality completes the symmetry in psh's binder catalog: `let` is μ̃ (value), `trap` is μ (continuation). When asked "what's the theoretical status of trap", this is the citation. Architect should treat lexical trap as a scope construct that introduces a continuation binding, not as a global side effect.
