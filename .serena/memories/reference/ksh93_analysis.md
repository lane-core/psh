---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [ksh26, ksh93-analysis, sequent-calculus, polarity-frame, sh-prefix, bug, critical-pair, lambda-mu-mu-tilde, oblique-map, continuation-stack, xec, macro]
agents: [plan9-systems-engineer, vdc-theory, psh-sequent-calculus]
---

# Reference: ksh26 theoretical foundation (`ksh93-analysis.md`)

**Path.** `refs/ksh93/ksh93-analysis.md`

**Origin.** Lane's ksh26 project — a sequent-calculus analysis of ksh93's interpreter, vendored into psh as the empirical precedent for the polarity frame discipline. **Not** a psh design document — it documents the ksh93 interpreter and the ksh26 redesign direction.

## What's in the analysis

- **§The observation** — ksh93 already has three-sorted λμμ̃ structure (producers, consumers, commands). It's implicit, enforced by careful C coding rather than any structural invariant. When the boundaries are respected, everything works; when violated, you get bugs.

- **§Theoretical framework** — nine papers that provide formal scaffolding: Curien-Herbelin λμμ̃; Spiwack System L (polarized variant); Wadler dual calculus; Mangel-Melliès-Munch-Maccagnoni duploids; Munch-Maccagnoni thesis and FoSSaCS paper; Levy CBPV; Binder et al. Grokking the Sequent Calculus; Curien-Munch-Maccagnoni focused calculus; Kraus-von Raumer Squier in HoTT.

- **§The correspondence** — structural mapping between ksh93 and sequent calculus. Three sorts (macro.c producers, fault.h/sh.st.trap coterms, sh_exec commands); shifts at `$(cmd)`, `<(cmd)`, `eval`, `x=val`; let/control duality (`nv_setlist` vs `sh_debug`/`sh_trap`); ⊕/⅋ error duality (exit status vs traps); continuation stack (`sigjmp_buf`, `checkpt`, `sh.jmplist`, SH_JMP_PROPAGATE boundary); scoping as CDT viewpath (`dtview`); the monolithic `Shell_t` state.

- **§Where the structure breaks down** — **the critical pair**: `⟨(S).α | x.(T)⟩` — a covariable abstraction cut against a variable abstraction. The two reduction orders yield different results. In ksh93 this manifests concretely as the **`sh.prefix` bugs** (001, 002, 003a, 003b), where a compound assignment context (value mode, `sh.prefix` set) is cut against a DEBUG trap dispatch (computation mode), and which fires first determines whether state is corrupted.

- **§The save/restore pattern IS the shift** — the recurring `prefix = sh.prefix; sh.prefix = 0; ...; sh.prefix = prefix` pattern in ksh93's C code is not defensive programming; it is the **implementation of a polarity shift** from focused type theory. ksh93 reinvents this mechanism ad-hoc, one bug at a time. **psh's polarity frame discipline is the generalization.**

- **§Monadic and comonadic patterns in C** — `macro.c` expansion as Kleisli composition (monadic); `xec.c` / `fault.h` context management as co-Kleisli (comonadic); `sh_exec` dispatching on `Shnode_t` as an oblique map `P → N`. Associativity holds within each side; non-associativity appears at the polarity boundary (the (+,−) failure).

- **§The refactoring direction** — how ksh26 proposes to generalize the polarity frame API (`sh_polarity_enter` / `sh_polarity_leave`) so the save/restore pattern becomes structural rather than ad-hoc.

## Concepts it informs in psh

- **Polarity frame discipline** (`docs/vdc-framework.md` §9.3, `docs/implementation.md`) — psh's central operational discipline.
- **`sh.prefix` bugs as motivation** — the concrete failure mode psh's polarity frame discipline prevents.
- **Three-sort structure** — the empirical evidence that λμμ̃'s three sorts are already present in real shells, just unnamed.
- **⊕/⅋ error duality** — psh's `decision/try_catch_scoped_errort` and `decision/unified_trap_three_forms` both trace to ksh93's longjmp mode taxonomy.
- **Let/control duality** — `decision/let_is_mu_tilde_binder_cbpv` vs `decision/unified_trap_three_forms`. Dual binders, same discipline.
- **Discipline functions as codata** — `decision/codata_discipline_functions` generalizes ksh93's get/set discipline functions into a type-theoretic framework.

## Who consults it

- **plan9 agent** (primary, canonical): this is the authority for ksh93 architectural commentary. Cite sections by name.
- **vdc-theory agent**: for the framework-level claims (duploid (+,−) equation, polarity frame as shift).
- **sequent calculus agent**: for the three-sort mapping, critical pair analysis, let/control duality.

## Note

The bug reproducers and the ksh26 redesign document live in the ksh26 source tree, not in psh. psh reuses the empirical findings (polarity patterns, sh.prefix bugs, ⊕/⅋ duality) as the motivating design study; psh does not reproduce the bugs.
