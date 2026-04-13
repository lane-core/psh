# Val — the value model

Val is the runtime representation of every psh value. It is
pure positive data — Clone, no embedded effects, no computation-
mode signals. Effects live in the evaluator, not in the value
type. This is the "Val is inert" principle from §Implementation
principles.

Spec correspondence: the ground types section of 06-types.md
defines what Val must represent. The "every variable is a list"
commitment (01-foundations.md) defines how Val is stored.

### The Val enum

```rust
/// Runtime value. Pure data, Clone, no effects.
/// Corresponds to the positive (value) sort of the calculus.
#[derive(Clone, Debug, PartialEq)]
enum Val {
    Str(SmolStr),                           // 'hello', "interpolated"
    Int(i64),                               // 42, $((expr))
    Bool(bool),                             // true, false
    Path(SmallVec<[PathComponent; 4]>),     // /usr/bin/rc
    Fd(RawFd),                              // open, dup — resource handle
    Bytes(Vec<u8>),                         // untyped pipe content
    List(Vec<Val>),                         // (a b c) — homogeneous
    Tuple(Vec<Val>),                        // (a, b) — heterogeneous, fixed arity
    Map(IndexMap<SmolStr, Val>),            // {'k': v, ...} — insertion-ordered
    Struct(TypeId, Vec<Val>),               // Pos { x = 10; y = 20 }
    Tagged(TypeId, Name, Option<Box<Val>>), // ok(42), none, inl(v)
    Unit(TypeId),                           // Unit { }, or any zero-field struct
}

#[derive(Clone, Debug, PartialEq)]
enum PathComponent {
    Root,
    Parent,
    Cur,
    Normal(SmolStr),
}
```

### Design decisions in the enum

**SmolStr, not String.** Most shell strings are short (variable
names, flags, filenames, single words). SmolStr inlines strings
≤ 22 bytes, avoiding heap allocation for the common case. The
hot path (word expansion, argument construction) stays on the
stack.

**SmallVec for Path.** Most paths have ≤ 4 components
(`/usr/bin/rc` = 4). SmallVec inlines these. Only deeply nested
paths allocate.

**IndexMap for Map.** Preserves insertion order — `$m.keys`
returns keys in the order they were defined. BTreeMap would
sort alphabetically (surprising); HashMap would randomize order
(non-deterministic). IndexMap gives O(1) lookup with stable
iteration. One extra dependency — justified by the ergonomic
payoff.

**TypeId in Struct/Tagged/Unit.** The runtime carries the
nominal type identity. `Struct(TypeId, ...)` knows it's a `Pos`
vs a `Vec3`. `Tagged(TypeId, Name, ...)` knows it's a
`Result::ok` vs an `Either::inl`. This enables:
- Runtime type display (`echo $val` shows the type name)
- Dynamic method dispatch via `sigma.methods.get(type_id, name)`
- Pattern match exhaustiveness is checked statically, but
  runtime tag dispatch uses `TypeId + Name`

**Option<Box<Val>> for Tagged payloads.** Nullary variants
(payload is Unit) carry `None` at runtime — no allocation, no
Unit value constructed. The `is_unit()` check at registration
time determines this; the runtime never materializes a Unit
value. Box for the non-nullary case avoids making the entire
Val enum as large as its largest variant.

### The list wrapping layer

Every variable slot holds a `Vec<Val>`, not a bare `Val`.
This is the "every variable is a list" invariant, maintained
by the environment layer, not by Val itself.

```rust
/// A variable slot. The outer Vec is the list wrapper.
/// Type annotations refer to the element type.
/// $#x queries the Vec length.
/// Substitution splices the Vec into argument position.
struct Slot {
    elements: Vec<Val>,
    type_id: TypeId,       // element type, not List(T)
    zone: Zone,            // classical / affine / linear
    exported: bool,        // mark-for-projection to child env
}
```

Val knows nothing about the list wrapper. A `Val::Int(42)` is
just an integer. The environment wraps it: `Slot { elements:
vec![Val::Int(42)], type_id: INT_ID, ... }`. This is the
"representation-level, below the type horizon" resolution from
the D7 discussion.

### Val ↔ Σ interface

Val carries TypeId but does not hold a reference to Σ. All
type queries go through Σ, using the TypeId as a key:

```rust
// Display
fn display_val(val: &Val, sigma: &Sigma) -> String;

// Method dispatch: $val.name
fn resolve_method(val: &Val, name: Name, sigma: &Sigma)
    -> Option<&MethodEntry>;

// Pattern matching: does val match this pattern?
fn match_pattern(val: &Val, pat: &Pattern, sigma: &Sigma)
    -> Option<Bindings>;
```

Val is dumb data. Σ gives it meaning.

### Conversions

```rust
impl Val {
    /// Coerce to Str for string contexts (interpolation,
    /// external command arguments, environment export).
    fn to_str(&self) -> SmolStr;

    /// Coerce to Int for arithmetic contexts.
    fn to_int(&self) -> Result<i64, TypeError>;

    /// Coerce to Bool for condition contexts.
    fn to_bool(&self) -> bool;

    /// Coerce to Path for filesystem operations.
    fn to_path(&self) -> Result<PathBuf, TypeError>;
}
```

`to_str` always succeeds — every Val has a string
representation (Display). `to_int` and `to_path` can fail —
type errors at the boundary. `to_bool` follows shell convention:
the empty string and zero are false; everything else is true.
These are the exit points from the typed world to the untyped
external command interface.

### What Val does NOT contain

- **No closures.** Lambdas are AST nodes with captured
  environments, stored in Θ (the def/let-lambda registry).
  Val is pure data; closures are computations.
- **No Status.** Status is the return type of commands, not a
  storable value. Commands produce Status; the evaluator
  inspects it. It never lands in a Slot.
- **No Stream(T).** Streams are type-level annotations on pipe
  channels, not runtime values. The pipe carries bytes; the
  type checker verified the protocol statically.
- **No effects.** No IO handles, no continuation captures, no
  mutable state. Effects live in the evaluator.


