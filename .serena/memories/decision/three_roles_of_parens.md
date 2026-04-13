---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-11
importance: high
keywords: [parens, list, tuple, tagged-construction, disambiguation, comma, splicing, rc-heritage]
agents: [psh-architect, psh-sequent-calculus, vdc-theory]
related: [decision/tagged_construction_uniform, decision/struct_record_literal_construction, decision/every_variable_is_a_list, analysis/data_vs_codata, analysis/duff_principle]
---

# Decision: three roles of `()` — list, tuple, tagged construction

## Decision

Parentheses `()` play three structurally distinct roles in psh,
disambiguated by context and delimiters:

1. **List** — space-delimited elements. `(a b c)` is a three-
   element list. Inherited from rc. Splices structurally on
   substitution. Runtime arity — list length is determined at
   construction and can grow or shrink through splicing.

2. **Tuple** — comma-delimited elements. `(10, 20)` is a two-
   element tuple (anonymous product). The comma is the
   disambiguator. Tuples do **not** splice — they preserve their
   boundary. Static arity — the number of elements is fixed at
   the literal.

3. **Tagged construction** — prefixed by a `NAME(` token with
   **no space** between NAME and `(`. `ok(42)`, `some(v)`.
   Args inside the parens are a term expression whose form
   matches the constructor's declared shape. Used for enum
   variants with payloads only. Map uses brace literal
   `{'key': v}`, not tagged construction.
   See `decision/tagged_construction_uniform`.

Struct construction is **not** a fourth role of parens — structs
use type-prefixed brace literal `Pos { x = 10; y = 20 }` or
positional `Pos { 10, 20 }`. Map uses brace literal
`{'key': v}`. Both use braces, not parens.

## Why

This preserves rc's list literal (the primary use of parens in
rc) while adding tuple products and the uniform tagged-construction
rule without breaking disambiguation. The comma is the tuple
disambiguator; the no-space-before-`(` is the tagged-construction
disambiguator. Every other `()` is a list.

The structural consequence — lists splice into tagged construction,
tuples do not — is deliberate: lists represent sequences (rc
heritage), tuples represent products (type-theoretic structure),
and tagged construction accepts the former (sequence of args) and
not the latter (structured product).

The term-layer / type-layer symmetry:

- **Term layer**: `(a b c)` bare + space = list (primitive — runtime
  arity, splicable); `(a, b, c)` bare + comma = tuple (derived — static
  arity, positional).
- **Type layer**: `(A, B, C)` bare + comma = tuple type (primitive —
  the only way to name a sequence of types is positional-heterogeneous).
  There is no list-of-types form because types have no runtime arity.

## Consequences

- `(a b c)` is a list; `(a, b, c)` is a tuple (min 2 elements);
  `NAME(a b c)` (no space after NAME) is tagged construction for
  enum variants.
- `NAME (a b c)` (with a space after NAME) is the command `NAME`
  invoked with a list argument.
- Lists splice into tagged construction (`let args = (42 'msg');
  let r = err($args)`).
- Tuples do not splice — `let t = (10, 20); f($t)` passes the
  tuple as a single arg.
- The parser disambiguates all three roles with local context:
  whether there's a comma inside, and whether the `(` is preceded
  by a NAME token with no space.
- Struct and map construction use braces, not parens. Type-
  prefixed struct `Pos { ... }` and map literal `{'key': v}`
  are separate grammar forms.

## Spec

`docs/spec/` §"Tuples (products, ×)", §"Enums
(coproducts, +)", §"Structs (named products, ×)". Grammar in
`docs/spec/04-syntax.md` §Values (list, tuple, record_lit, variant_val).
