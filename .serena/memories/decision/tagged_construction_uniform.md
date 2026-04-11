---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [tagged-construction, sum, struct, map, NAME-paren, uniform-rule, positional, list-splicing]
agents: [psh-architect, psh-sequent-calculus, psh-optics-theorist, vdc-theory]
related: [decision/three_roles_of_parens, decision/struct_positional_only_forever, decision/postfix_dot_accessors]
---

# Decision: uniform tagged construction — `NAME(args)` with NAME immediately followed by `(`

## Decision

psh uses a single uniform construction syntax for **all** tagged values — sums, structs, and maps:

```
NAME(args)
```

where `NAME` is immediately followed by `(` (**no space**), and args inside the parens are **space-delimited** (list-style), not comma-delimited.

- Sums: `ok(42)`, `err('not found')`, `some('/tmp/file')`, `none()`
- Structs: `Pos(10 20)`, `Rgb(255 0 0)`
- Maps: `Map(('k1' 'v1') ('k2' 'v2'))`

## Why

The uniform rule replaces what would otherwise be three or four distinct construction syntaxes with one. It commits the parser to tagged-construction mode as soon as it sees `NAME(` with no intervening space, disambiguating cleanly from command invocation (where `NAME (...)` with a space is a command with a list argument).

List splicing works uniformly because args are list-style: `let xy = (10 20); let p = Pos($xy)` splices two args into the `Pos` constructor. **Tuples do not splice** because the comma is the disambiguator — see `decision/three_roles_of_parens`.

Alternative considered: separate syntaxes for each tagged form (e.g., `Pos { x: 10, y: 20 }` for structs, `ok(42)` for sums). Rejected because the uniform rule is simpler to parse, simpler to teach, and makes splicing work across all tagged types.

## Consequences

- The `NAME(` token (no space) commits the parser to tagged construction.
- `ok 42` (with space) is the command `ok` called with argument `42`, not sum construction.
- Struct construction is **positional only** — fields bound by declaration order. No `Pos(x: 10, y: 20)` named form, now or in any future version. See `decision/struct_positional_only_forever`.
- Anonymous records (`(x 3 y 4)` style) are **not adopted**. Every record type requires a `struct` declaration.
- Map entries use nested tagged construction: `Map(('k' 'v') ...)` — the inner `('k' 'v')` is a list of two elements (an unlabeled pair).

Spec: `docs/specification.md` §"Sums", §"Structs", §"Syntax §Uniform tagged construction". Ledger: `docs/deliberations.md` §"Tagged construction: the uniform rule".
