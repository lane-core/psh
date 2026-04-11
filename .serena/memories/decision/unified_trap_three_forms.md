---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [trap, signal, lexical, global, deletion, mu-binder, classical-control, signal-continuation, ksh93-sigjmp]
agents: [psh-sequent-calculus, plan9-systems-engineer, psh-architect, vdc-theory]
related: [decision/try_catch_scoped_errort, decision/let_is_mu_tilde_binder_cbpv, analysis/error_duality_oplus_par, analysis/cut_as_execution, analysis/data_vs_codata, analysis/classical_control_mu_binder]
---

# Decision: unified `trap` — three forms by block count

## Decision

psh's `trap` uses a unified grammar `trap SIGNAL (body body?)?` that distinguishes three forms by block count:

- **Lexical** (two blocks): `trap SIGNAL { handler } { body }` — installs the handler for the duration of `body`. This is the **μ-binder** of Curien-Herbelin: `body` runs with the handler's signal continuation bound to SIGNAL in Δ. Inner lexical traps shadow outer.
- **Global** (one block): `trap SIGNAL { handler }` — registers at the top-level signal interface. Persists until overridden or removed.
- **Deletion** (no block): `trap SIGNAL` — removes a previously-installed global handler.

Precedence at signal delivery: **innermost lexical > outer lexical > global > OS default**.

## Why

ksh93 used `sigjmp_buf` / `checkpt` with global mutation (`sh.prefix`, `sh_getscope`) for continuation handling. The `SPEC, §"Continuations and classical control"` analysis identifies repeated stack corruption bugs from this approach. psh tames classical control by making the μ-binder **lexically scoped** — the handler is bound for exactly the duration of a lexical block, with no global state mutation.

The three forms unify into one grammar (`trap SIGNAL (body body?)?`) rather than three separate keywords. Block count disambiguates: zero blocks → deletion, one block → global registration, two blocks → lexical scoping. Users learn one grammar and one precedence rule.

Lexical as a μ-binder is the **dual** of `let` as a μ̃-binder (`decision/let_is_mu_tilde_binder_cbpv`). Both bind a name in a context; the binder-type differs by which side of the sequent the binding lives on.

## Consequences

- Signals fire at **interpreter step boundaries** (between commands, or wake-from-block during child waits), via a self-pipe pattern.
- `trap` and `try` compose orthogonally because they operate on different sorts: `trap` on signal continuations (⅋), `try` on command status (⊕). A lexical `trap` inside a `try` body fires first when a signal arrives; if the trap returns a status, `try` inspects it through its normal status-check mechanism.
- `@{ cmds }` duplicates the continuation (classical contraction — each copy evolves independently in its own process).
- **EXIT handler** (`trap EXIT`) is synthesized on process exit — rc heritage (Duff's `sigexit`).
- **EINTR policy:** builtins retry on EINTR by default; external commands handle EINTR themselves.
- No `sigjmp_buf` / `longjmp` in the Rust implementation. Signal delivery via self-pipe wake + poll.

Spec: `docs/specification.md` §"Error model §trap — unified signal handling", §"Polarity discipline §Classical control", §"Signal delivery model", §"Signal interaction with try blocks". Ledger: `docs/deliberations.md` §"Signal handling: rc style vs lexical trap (RESOLVED)".
