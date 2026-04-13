---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [arithmetic, dollar-double-paren, in-process, int, shift, polarity, ksh93-heritage, posix]
agents: [psh-sequent-calculus, psh-architect, plan9-systems-engineer]
related: [decision/let_is_mu_tilde_binder_cbpv]
---

# Decision: `$((...))` arithmetic is in-process pure computation returning Int

## Decision

`$((...))` evaluates an arithmetic expression **in-process** (no fork) and returns an `Int`. Bare names inside refer to their integer values without needing a `$` prefix.

```
let n = $((3 + 4))            # n is Int, value 7
let doubled = $((n * 2))      # no $ needed on n inside $((...))
```

## Why

ksh93/POSIX heritage: `$((...))` is a universally expected parameter expansion for arithmetic. psh preserves the syntax but places it within the polarity discipline: **`$((...))` is a ↓→↑ shift**, the same shift type as command substitution, but evaluated in-process rather than via fork.

Forking for arithmetic would be absurd (trivially expensive for a trivial operation). Making it a shift preserves the theoretical framing: the arithmetic context is a negative computation that produces a positive value, and the shift mediates.

Alternative considered: provide arithmetic only as a builtin (`let n = expr 3 + 4`). Rejected because `$((...))` is so pervasively expected in shell usage that omitting it would be a gratuitous break from convention.

## Consequences

- No fork cost for `$((...))`. Contrast command substitution `` `{cmd} `` which forks.
- Bare names inside `$((...))` are coerced to their integer values. Non-integer values produce an error.
- The result type is `Int` (a list of one integer per `decision/every_variable_is_a_list`).
- The ↓→↑ shift framing means `$((...))` sits at the same level as command substitution in the sequent calculus — it's a polarity shift, not a special form.
- Typed integer operations (arithmetic, comparison) are available without leaving the polarity discipline.

Spec: `docs/spec/` §"rc's execution model as sequent calculus §shifts", §"Three operations, three roles".
