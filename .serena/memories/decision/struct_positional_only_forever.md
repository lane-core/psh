---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [struct, positional, named-construction, rejected, permanent, ksh93-compound-variables, arity]
agents: [psh-architect, psh-sequent-calculus, psh-optics-theorist]
related: [decision/tagged_construction_uniform, analysis/data_vs_codata]
verified_against: [git-log@ad4dd61]
---

# Decision: struct construction is positional only, forever

## Decision

Struct construction uses **positional args** bound by declaration order. There is **no named form** (`Pos(x: 10, y: 20)`) — now or in any future version of psh. The prohibition is permanent, not deferred.

```
struct Pos { x: Int; y: Int }

let p = Pos(10 20)        # positional — x=10, y=20
```

Arity mismatch is a binding-time error.

## Why

Named construction sits uncomfortably against psh's uniform tagged construction rule (`NAME(args)` with space-delimited args). Adopting a named form would require either a second construction syntax (breaking uniformity) or inline syntax like `Pos(x: 10, y: 20)` (breaking the list-style args convention). Both options compound complexity without solving a real problem: positional is clear, short, and consistent with the rest of the tagged construction family (sums, maps).

The `struct` declaration already auto-generates **named accessors** (`.x`, `.y`) and positional accessors (`.0`, `.1`), so field access is both named and positional. Construction is uniformly positional; access is uniformly dual.

ksh93's compound variables allowed `typeset -C p=(x=10 y=20)` named assignment. psh's struct is the typed version of compound variables but deliberately departs from this: declaration-order binding is simpler to parse, simpler to teach, and consistent with the uniform rule.

Alternative considered and rejected in commit `ad4dd61`: reserve named construction for a future version. Rejected as permanent because reviving it later would re-introduce the complexity we're avoiding now.

## Consequences

- `Pos(10 20)` binds `x=10 y=20` because `x` is declared first.
- `Pos(x: 10, y: 20)` is a parse error (or reinterpreted under some other grammar rule) — never a struct constructor.
- Anonymous records (`(x 3 y 4)` style pseudo-records) are also **not adopted**. Every record type requires a `struct` declaration.
- Whole-struct replacement is the mutation pattern: `p = Pos(30 $p .1)`. No field-level mutation syntax in v1.
- List splicing into positional struct construction works uniformly: `let xy = (10 20); Pos($xy)` binds `x=10 y=20`.

Spec: `docs/specification.md` §"Structs". Ledger: `docs/deliberations.md` §"Struct definitions", §"Why no anonymous records".
