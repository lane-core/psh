---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [parens, list, tuple, tagged-construction, disambiguation, comma, splicing, rc-heritage]
agents: [psh-architect, psh-sequent-calculus, vdc-theory]
related: [decision/tagged_construction_uniform, decision/every_variable_is_a_list]
---

# Decision: three roles of `()` — list, tuple, tagged construction

## Decision

Parentheses `()` play three structurally distinct roles in psh, disambiguated by context and delimiters:

1. **List** — space-delimited elements. `(a b c)` is a three-element list. Inherited from rc. Splices structurally on substitution.
2. **Tuple** — comma-delimited elements. `(10, 20)` is a two-element tuple (product). The comma is the disambiguator. Tuples do **not** splice — they preserve their boundary.
3. **Tagged construction** — prefixed by a `NAME(` token with **no space** between NAME and `(`. `ok(42)`, `Pos(10 20)`, `Map(('k' 'v'))`. Args inside the parens are space-delimited (list-style). See `decision/tagged_construction_uniform`.

## Why

This preserves rc's list literal (the primary use of parens in rc) while adding tuple products and the uniform tagged-construction rule without breaking disambiguation. The comma is the tuple disambiguator; the no-space-before-`(` is the tagged-construction disambiguator. Every other `()` is a list.

The structural consequence — lists splice into tagged construction, tuples do not — is not an accident but a deliberate design: lists represent sequences (rc heritage), tuples represent products (type-theoretic structure), and tagged construction accepts the former (sequence of args) and not the latter (structured product).

Alternative considered: drop one of the three roles. Rejected because each serves a genuinely different purpose — lists for sequences, tuples for products, tagged construction for constructors.

## Consequences

- `(a b c)` is a list; `(a, b, c)` is a tuple; `NAME(a b c)` (no space after NAME) is tagged construction.
- `NAME (a b c)` (with a space after NAME) is the command `NAME` invoked with a list argument.
- Lists splice into tagged construction (`let xy = (10 20); Pos($xy)` binds `x=10 y=20`).
- Tuples do not splice — `let t = (10, 20); Pos($t)` would pass a single tuple as one arg, likely causing arity mismatch.
- The parser can disambiguate all three roles with local context: it only needs to know whether there's a comma inside, and whether the `(` is preceded by a NAME token with no space.

Spec: `docs/specification.md` §"Tuples", §"Sums", §"Structs". Ledger: `docs/deliberations.md` §"The three roles of (): list vs tuple vs tagged construction".
