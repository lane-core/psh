---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-11
importance: high
keywords: [let, mu-tilde, cbpv, f-a, binder, effectful, call-by-push-value, levy, curien-herbelin, monadic-bind]
agents: [psh-sequent-calculus, vdc-theory, psh-architect]
related: [decision/def_vs_lambda, decision/every_variable_is_a_list, analysis/three_sorts, analysis/cbpv_f_u_separation, analysis/cut_as_execution]
verified_against: [git-log@bdf0ca5]
---

# Decision: `let` binds the result of a computation (CBPV μ̃-binder on F(A))

## Decision

`let x = M` where `M : F(A)` is the μ̃-binder of Curien-Herbelin on monadic bind. It runs the computation `M`, captures the returned value as `x`, and continues. Pure values are the degenerate special case; effectful right-hand sides (builtin calls, pipelines, command substitution, coprocess queries) are the common case.

## Why

psh inherits Levy's Call-by-Push-Value (CBPV) distinction between values (positive types A) and computations (negative types B). The shift type `F(A)` is "a computation that returns a value of type A" — exactly what commands produce. Binding this with a μ̃-binder (dual of the μ control binder) gives the shell first-class let-bindings that accept effectful computations directly, without an extra "call this to get a value" step.

Alternative considered and rejected: ANF-style restriction to pure right-hand sides, which would force ugly workarounds for the common cases (command substitution capture, coprocess tag binding, etc.). Commit `bdf0ca5` formalized the decision to accept effectful RHS.

## Consequences

- `let tag = print -p srv 'query'` binds the returned Int tag from a coprocess send.
- `` let out = `{ls} `` captures stdout from a command substitution.
- `let x = 42` is a degenerate pure-case binding (the computation has no effects).
- `let` and `def` are complements: `let` + lambda names a **value** (first-class, storable, captures at definition); `def` names a **computation** (cut template, variadic args, dynamic scope). See `decision/def_vs_lambda`.
- The μ̃-binder is the dual of the μ control-binder; see `decision/unified_trap_three_forms` for the μ side.
- Builtins return values directly (a trivial `F(A)` where A is known).

Spec: `docs/spec/` §"Two kinds of callable", §"The three sorts, made explicit", §"Theoretical framework §The practice". Ground: Levy *Call-by-Push-Value* (Springer 2004); Curien-Herbelin "The Duality of Computation" ICFP 2000; Binder et al. "Grokking the Sequent Calculus" (functional pearl, 2023).
