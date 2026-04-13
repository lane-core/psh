---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [cbpv, F, U, returner, thunk, levy, value-computation, def-lambda, let-as-mu-tilde, F-A, U-B, adjunction, pure-vs-effectful]
agents: [psh-sequent-calculus, vdc-theory, psh-architect]
related: [analysis/three_sorts, analysis/polarity/shifts, analysis/cut_as_execution, decision/let_is_mu_tilde_binder_cbpv, decision/def_vs_lambda, reference/papers/grokking_sequent_calculus]
verified_against: [docs/spec/@HEAD §200-230, §473-500, decision/let_is_mu_tilde_binder_cbpv, audit/psh-sequent-calculus@2026-04-11]
---

# CBPV F/U separation (call-by-push-value)

## Concept

Levy's **Call-by-Push-Value** (CBPV) splits types into two kinds:

- **Value types** (positive, A): things that exist, can be stored, can be substituted.
- **Computation types** (negative, B): things that happen, can be sequenced, can produce values.

Two adjunction operators bridge them:

- **`F : Val → Comp`** — the **returner**. Given a value type A, `F(A)` is "a computation that returns a value of type A." This is the **upshift** ↑ in the focused-sequent-calculus reading. (Convention at `docs/spec/` line 200: "↑A for returning".)
- **`U : Comp → Val`** — the **thunk**. Given a computation type B, `U(B)` is "a value that suspends a computation of type B." This is the **downshift** ↓.

In psh, the `def`/lambda split is the F/U adjunction surfaced as syntax. From `docs/spec/` line 228: "psh's `def`/`let` + lambda split is CBPV's `U`/`F` adjunction surfaced as syntax."

- **`def` defines a computation.** A `def` cell has type `F(Status)` — it's a computation that, when run, produces a Status value. Builtins, named commands, and discipline functions are all `def` cells.
- **A lambda is a value.** `|x| => expr` is a value of type `U(...)` — a thunked computation, storable and substitutable, forced by application.

`let x = M` where `M : F(A)` is the **μ̃-binder of Curien-Herbelin on monadic bind** (`decision/let_is_mu_tilde_binder_cbpv`): it runs the computation M, captures the returned value as x, and continues. Pure values are the degenerate case (the computation has no effects). Effectful right-hand sides (builtin calls, command substitution, coprocess sends) are the common case.

The F/U separation is what makes psh's `let` accept effectful computations directly without an extra "call this to get a value" step. ANF-style restriction to pure RHS was considered and rejected; the decision was formalized at commit `bdf0ca5` (per `decision/let_is_mu_tilde_binder_cbpv`).

## Foundational refs

- Levy, P.B. *Call-by-Push-Value: A Functional/Imperative Synthesis*. Springer, 2004. **Not vendored in `~/gist/`**; cited from the existing `decision/let_is_mu_tilde_binder_cbpv` and the spec's "The practice" section line 226. Lane's prior decision memo treats Levy as the canonical source for the framework.
- Curien, Herbelin. *The Duality of Computation*. ICFP 2000. Also not vendored; cited from `decision/let_is_mu_tilde_binder_cbpv` for the μ/μ̃ binders.
- `reference/papers/grokking_sequent_calculus` — Binder et al. give the CBPV-flavored Fun→Core compilation. The accessible programmer-facing introduction.

## Spec sites

- `docs/spec/` §"Theoretical framework §The practice" lines 218–230 — Levy CBPV citation.
- `docs/spec/` §"Two kinds of callable" line 473 — `def` vs lambda as F/U.
- `docs/spec/` line 200 — convention "↑A for returning".
- `decision/let_is_mu_tilde_binder_cbpv` — design decision; the operational consequence.
- `decision/def_vs_lambda` — the design decision for the syntactic split.
- `analysis/polarity/shifts` — the focused-calculus reading of F/U as ↑/↓.
- `analysis/cut_as_execution` — `let` as a cut against a μ̃-binder; the operational realization of the F/U adjunction.

## Status

Settled. CBPV is the substrate. The F/U adjunction is what `def` vs lambda surface as syntax. Architect should treat every `def` invocation as a returner site and every lambda as a thunk site. Note that Levy's CBPV monograph is not vendored locally; the canonical source for psh's CBPV claims is the spec's "The practice" section plus `decision/let_is_mu_tilde_binder_cbpv`, not direct paper citation.
