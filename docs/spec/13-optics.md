# Optics activation

### Optics activation

The accessor system uses two syntactic forms: **bracket**
`$a[i]` for projection by runtime value, and **dot** `$a.name`
for named field/method/discipline access. Both bind tightly
to `$name` with no space required. This split reflects
an optic selection boundary: bracket selects an optic by a
runtime-valued index (the index is itself a producer), while
dot selects by a static symbol resolved at parse/check time.
Both sides produce standard profunctor optics; the syntax
determines how the optic is selected, not which class it is.

| Type | Access form | Optic | Profunctor constraint |
|---|---|---|---|
| Lists (rc base) | `for x in $l` | Traversal (iteration) | Traversing (Applicative) |
| Lists | `$l[n]` | Affine traversal (index) | Cartesian + Cocartesian |
| Tuples (products) | `$t[i]` (literal index) | Lens (projection) | Cartesian |
| Structs (named products) | `$s.field` | Lens (named) | Cartesian |
| Sums (coproducts) | `match` | Prism (case analysis) | Cocartesian |
| Map(V), key lookup | `$m['key']` | Affine traversal (partial) | Cartesian + Cocartesian |
| Map(V), all values | `.values` | Getter (read-only) | — |
| List slice | `$l[a..b]` | Affine fold (read-only) | (read-only restriction of AffineTraversal) |
| Path | `$p.parent` | Lens (drop last component) | Cartesian |
| Path | `$p.name`, `.stem` | Lens (last component) | Cartesian |
| Path | `$p.ext` | AffineTraversal (may not exist) | Cartesian + Cocartesian |
| Path | `$p[n]` | AffineTraversal (nth component) | Cartesian + Cocartesian |
| ExitCode | `$e.code`, `$e.message` | Lens (struct field) | Cartesian |
| fd table (save/restore) | (internal) | Lens | Cartesian |
| Redirections | (wrapping) | Adapter | Profunctor |

Discipline-equipped variables are not listed here — they are
mixed monadic lenses per `def:monadiclens`, orthogonal to the
type-shape classification above. See §"Mixed-optic structure"
in §Discipline functions.

**Bracket/dot partition.** The table partitions cleanly — no
row needs both accessor forms. Bracket covers indexed/keyed
access (tuples, lists, maps); dot covers named observers
(struct fields, type methods, discipline functions). Optic
composition across the boundary follows standard Tambara
module composition: `$t[0] .name` (Lens ∘ Lens = Lens),
`$m['key'] .name` (AffineTraversal ∘ Lens = AffineTraversal).

**Traversing / Applicative** is the Tambara-module class
corresponding to Clarke's power-series action [Clarke,
`def:traversal`]. It is the class for van-Laarhoven-style
traversals `forall f. Applicative f => (a -> f b) -> (s -> f t)`,
and matches the Haskell `profunctors` library convention.

**Affine traversal** requires a cartesian-closed base category
and symmetric-monoidal-closed cocartesian structure [Clarke,
`def:affine`]. For psh's pure value category W this is
satisfied; the "Cartesian + Cocartesian" profunctor constraint
is sufficient for user-facing classification.

**Map type** gets two rows because the two views are
structurally different: a single-key bracket lookup
(`$m['key']`) is a partial projection — affine traversal, may
or may not hit. `.values` is a Getter — it materializes a
new `List(V)`, not a traversable focus into the map.

**List element access** is an affine traversal, not a Lens,
because the index may be out of bounds. `$l[n]` returns
`Option(T)` — `some(value)` or `none`. This matches map key
lookup, which also returns `Option(V)`. The partiality is
inherent: `Int` does not encode bounds, and psh has no
dependent types to prove `0 ≤ n < len(l)` statically.

**`.fields` and `.values` on structs** are Getters (read-only,
always succeed, no update path) — not Lenses, because they
produce a new list, not a focus into the struct's storage.
Setting through `.fields` would not update the struct's
fields; it would replace a list that happens to contain
stringified field data. `.keys` and `.values` on maps are
also Getters: `.values` returns a `List(V)`, not a traversable
focus into the map. §8.5 classification: monadic (pure
positive-to-positive Kleisli maps in W), thunkable, central,
no polarity frame.

**Discipline/bracket evaluation order.** When bracket access
is applied to a disciplined variable — `$m['key']` where `m`
has a `.get` discipline — the evaluation order is: `.get`
fires on `$m` first (producing the Map value per CBV focusing
/ Prop 8550), then bracket projects from that value. Bracket
operates on the *value* produced by `.get`, not on the stored
slot. The discipline is transparent to bracket composition.


