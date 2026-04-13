---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [foundational, list, scalar, rc, duff, splicing, type-annotation, no-rescan, element-type]
agents: [plan9-systems-engineer, psh-sequent-calculus, psh-architect, psh-optics-theorist, vdc-theory]
related: [analysis/duff_principle, decision/parameter_expansion_destructors, decision/three_roles_of_parens, decision/let_is_mu_tilde_binder_cbpv]
---

# Decision: every variable is a list

## Decision

Every psh variable holds a list. There is no separate "scalar" type distinct from "list of length 1." Type annotations refer to **element types**. Substitution always splices structurally.

## Why

rc (Duff 1990) made list-valued variables the foundational move. Duff's design principle: "input is never scanned more than once." psh extends this across the entire type system — structure is carried as data, never destroyed and reconstructed. The VDC framework reinforces the same principle: sequences are primitive on cell boundaries, never forced into a single composite. Alternative considered and rejected: a scalar/list distinction with `"$var"` quoting ceremony, which would preserve rescanning holes at substitution sites.

## Consequences

- `let count : Int = 0` is shorthand for `let count : Int = (0)`. Both denote a list of one int. `$#count` is 1.
- `: Int` means "list whose elements are Int." Length is runtime data, not part of the type.
- A "scalar" binding splices one element; a list binding splices its elements. Structural substitution is uniform across the distinction.
- Tuples, sums, structs are distinct types at the **element level** — they appear inside the list. `let pos : Tuple = (10, 20)` holds a list of one tuple. `$#pos` is 1. `$pos .0` is `10`.
- **No `"$var"` quoting ceremony is ever needed.** Variables always splice structurally.
- The entire type system is designed so that `List` is the outermost layer and everything else is an element type.

Spec: `docs/spec/` §"Foundational commitment: every variable is a list". Ground: `refs/plan9/papers/rc.ms` §Design Principles; `docs/vdc-framework.md` §9.1 "Duff's Principle Generalized".
