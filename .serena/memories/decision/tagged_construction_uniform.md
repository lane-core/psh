---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-11
importance: high
keywords: [tagged-construction, enum, variant, NAME-paren, uniform-rule]
agents: [psh-architect, psh-sequent-calculus, psh-optics-theorist, vdc-theory]
related: [decision/three_roles_of_parens, decision/struct_record_literal_construction, decision/postfix_dot_accessors, analysis/data_vs_codata]
---

# Decision: uniform tagged construction — `NAME(args)` with NAME immediately followed by `(`

## Decision

psh uses a single uniform construction syntax for **coproducts**
— enum variants with payloads:

```
NAME(args)
```

where `NAME` is immediately followed by `(` (**no space**), and
args inside the parens are **space-delimited** (list-style). The
inner form is a term expression whose shape matches the
constructor's declared argument shape.

- Enum variants with payloads: `ok(42)`, `err('not found')`,
  `some('/tmp/file')`
- Enum variants with tuple-typed payloads: `error(('bad', 42))`
  (inner is a tuple literal)
- Nullary enum variants: **bare name**, not empty parens. `none`
  is the form, `none()` is not valid — `()` is reserved for the
  empty list.

**Map is NOT in the tagged construction family.** Map uses brace
map literal `{'key': 1, 'age': 2}` (colon separator, comma
delimiter), `.insert` builder chain, or `Map.from_list`. The old
`Map(('k1', 1) ('k2', 2))` form was dropped — it predated the
tuples-and-structs-disjoint commitment and the bracket/dot
accessor split. Map is `Map(V)` with string keys.

Struct construction is **not** in the tagged construction family;
structs use brace record literal `{ field = value; ... }`. See
`decision/struct_record_literal_construction`.

## Why

The uniform rule commits the parser to tagged-construction mode
as soon as it sees `NAME(` with no intervening space,
disambiguating cleanly from command invocation (where `NAME (...)`
with a space is a command with a list argument). The inner form
is a term expression — list, tuple, or single value — whose
shape matches the declared argument shape of `NAME`, verified at
type-check time under bidirectional checking.

List splicing works because args are list-style: `let xy = (10 20);
let m = Map($xy)` splices two args into the `Map` constructor if
each element has the right type. **Tuples do not splice** because
the comma is the disambiguator — see `decision/three_roles_of_parens`.

The narrower scope (coproducts + collection builtins, excluding
structs) reflects the categorical structure: products have a
single canonical constructor (the n-tuple), so nominal products
are constructed by their tuple-shaped brace literal under the
expected type; coproducts have N constructors (one per variant),
so nominal coproducts need the tag at the construction site to
discriminate which injection was used.

## Consequences

- The `NAME(` token (no space) commits the parser to tagged
  construction.
- `ok 42` (with space) is the command `ok` called with argument
  `42`, not enum construction.
- Nullary enum variants are bare: `none`, not `none()`. `()` is
  the empty list.
- The inner form of `NAME(...)` is an arbitrary term expression
  whose form depends on the constructor's declared shape. For
  single-payload variants like `ok(42)`, it's a single value.
  For tuple-payload variants like `error(('bad', 42))`, it's a
  tuple literal.
- Struct construction is **not** covered by this rule. Structs
  use the brace record literal form at use sites, which is
  handled by a separate grammar production and the bidirectional
  check-mode rule.

## Spec

`docs/specification.md` §"Enums (coproducts, +)" and §"Features
and non-goals §Map type". Grammar in `docs/syntax.md` §Values
(variant_val, nullary_variant).
