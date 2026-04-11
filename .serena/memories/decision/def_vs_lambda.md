---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [def, lambda, callable, cut-template, computation, value, cbpv, f-u-adjunction, ksh93-compound-variables]
agents: [psh-sequent-calculus, vdc-theory, psh-architect]
related: [decision/let_is_mu_tilde_binder_cbpv, decision/lambda_syntax]
---

# Decision: `def` for command-level bindings, `let` + lambda for value-level

## Decision

psh distinguishes two kinds of callable:

- **`def name { body }`** names a **command** — a cut template. Sort: Θ (commands). Variadic, positional args (`$1`, `$2`, `$*`). Dynamic scope (reads current scope). CBPV type `F(Status)`. rc's `fn` renamed.
- **`let name = |x| => body`** (or `|x| { block }`) names a **value** — a thunked computation. Sort: Γ (producers). Fixed arity, named args. Captures at definition. CBPV type `U(A → B)` or `U(A → F(B))`. No rc analog (psh extension).

Both are invoked as `name arg1 arg2`. The disambiguation is the definition site.

## Why

ksh93's compound variables (`typeset -C`) were its struct system, never named as such. The interpreter needed both effectful procedures (functions) and inert data accessors (compound variable fields), but conflated them in the `Namval_t` machinery. psh's `def`/lambda split is informed by this: the two roles are genuinely different sorts in the sequent calculus and should be syntactically distinct.

`def` is the neutral name — it defines a named computation without claiming its role in a cut (which only happens at the invocation site). rc's `fn` was a misnomer because it doesn't define a function in the value-theoretic sense; it defines a cut template.

`let` + lambda is the CBPV F/U adjunction surfaced as syntax: values are thunked computations (`U(B)`), commands are computations returning values (`F(A)`). Making both first-class separates the concerns.

## Consequences

- `def name` goes in Θ (command sort). `let name = |x| => ...` goes in Γ (producer sort).
- Lambda syntax: `|x| => expr` (single expression) or `|x| { block }` (block body). Nullary: `| | => expr`. See `decision/lambda_syntax`.
- Type-variable disambiguation in dotted names uses capitalization: `def Str.length { }` (uppercase → method on type Str); `def count.set { }` (lowercase → discipline on variable count).
- A lambda's captures are resolved at definition time; a `def` reads its enclosing scope dynamically at invocation.
- Effects: lambdas have inferred purity (thunkable when pure); `def` bodies may have arbitrary effects.

Spec: `docs/specification.md` §"Two kinds of callable" (authoritative table).
