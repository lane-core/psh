# Σ — the signature store

Σ is the global registry of type declarations: structs, enums,
newtypes, and their associated metadata (fields, variants,
constructors, renaming tables, capability traits). Every other
layer — parser, checker, evaluator — queries Σ but never
mutates it after the declaration pass. Σ is populated once
(prelude + user declarations) and then frozen.

Spec correspondence: every typing rule that has `... ∈ Σ` in
its premises reads from this store. See 06-types.md for the
rules.

### Core types

```rust
/// Interned type name. Cheap to copy, compare, hash.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TypeId(u32);

/// Interned variant/field/method name.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Name(u32);

/// A type reference — ground, parametric instance, or param variable.
#[derive(Clone, PartialEq, Eq)]
enum TypeRef {
    Ground(TypeId),                          // Str, Int, Bool, Fd, Bytes
    Applied(TypeId, Vec<TypeRef>),           // List(Int), Map(Str), Result(Int, Str)
    Param(u8),                               // T, E — de Bruijn-ish index into declaration params
}

/// Polarity sort (spec 02-calculus.md §The three sorts).
#[derive(Clone, Copy)]
enum Sort { Positive, Negative }

/// Resource zone (spec 03-polarity.md §Linear resources).
#[derive(Clone, Copy)]
enum Zone { Classical, Affine, Linear }
```

### Type trait hierarchy

The `Ty` trait is the uniform interface every type exposes to
the checker and parser. Subtraits capture capabilities.

```rust
trait Ty {
    fn id(&self) -> TypeId;
    fn sort(&self) -> Sort;
    fn default_zone(&self) -> Zone;
    fn params(&self) -> &[TypeRef];         // empty for ground types
    fn backing(&self) -> Option<TypeId>;    // Some for newtypes
    fn is_unit(&self) -> bool {             // true for zero-field structs
        false
    }
    fn eq_nominal(&self, other: &dyn Ty) -> bool {
        self.id() == other.id()
    }
}

/// Arithmetic-capable. Int, and newtypes backed by Int.
trait Number: Ty {}

/// Supports for/map/filter/each/fold iteration.
trait Iterable: Ty {}

/// Supports dot and bracket accessors.
trait Accessible: Ty {}
```

Ground types implement subtraits directly. Newtypes inherit
from their backing type — computed once at registration.

### Σ entries

Each declaration form produces an entry in Σ:

```rust
/// A struct declaration.
struct StructEntry {
    id: TypeId,
    params: Vec<Name>,                       // type params (uppercase)
    fields: Vec<(Name, TypeRef)>,            // (field_name, field_type)
}

/// An enum declaration.
struct EnumEntry {
    id: TypeId,
    params: Vec<Name>,
    variants: Vec<VariantEntry>,
}

struct VariantEntry {
    name: Name,
    payload: Option<TypeRef>,               // None = nullary in source
                                            // (but check is_unit on resolved type)
}

/// A newtype declaration.
struct NewtypeEntry {
    id: TypeId,
    params: Vec<Name>,
    backing: TypeId,
    renaming: HashMap<Name, Name>,          // old (backing) => new (this type)
    reverse: HashMap<Name, Name>,           // new => old (computed at registration)
}
```

### Constructor registry

The parser needs to look up constructors by name to determine
parent type and arity. This is a separate index over Σ,
built at registration time.

```rust
struct ConstructorInfo {
    parent: TypeId,
    variant: Name,                          // name in the parent enum
    payload: Option<TypeRef>,
    nullary: bool,                          // true when payload is_unit()
}

/// Maps constructor name → info. Populated from enums + newtypes.
/// Qualified lookup: Result::ok → direct.
/// Bare lookup: ok → search order per spec (06-types.md §Qualified variant syntax).
struct ConstructorRegistry {
    /// Qualified: (TypeId, Name) → ConstructorInfo
    qualified: HashMap<(TypeId, Name), ConstructorInfo>,
    /// Bare: Name → Vec<ConstructorInfo> (may be ambiguous)
    bare: HashMap<Name, Vec<ConstructorInfo>>,
}
```

Bare lookup returns multiple candidates when the same name
exists across types (e.g., `ok` in both `Result` and some
user enum). The checker resolves ambiguity via expected type
in check mode, or requires qualification in synth mode.

### Per-type method registry

Methods defined via `def Type::name { }` (spec 04-syntax.md
§Type::name()) live in Θ but are indexed by type for accessor
resolution.

```rust
struct MethodEntry {
    parent: TypeId,
    name: Name,
    return_type: TypeRef,                   // e.g., Option(T) for prism previews
    def_id: DefId,                          // pointer into Θ for the body
}

/// Maps (TypeId, Name) → MethodEntry.
struct MethodRegistry {
    methods: HashMap<(TypeId, Name), MethodEntry>,
}
```

Dot accessor resolution on `$val.name`: look up `val`'s type
in Σ, then check `MethodRegistry` for `(type_id, name)`. The
`::` prefix form `Type::name(val)` does the same lookup.

### The Sigma struct

```rust
struct Sigma {
    // Type declarations
    ground: HashMap<TypeId, Box<dyn Ty>>,
    structs: HashMap<TypeId, StructEntry>,
    enums: HashMap<TypeId, EnumEntry>,
    newtypes: HashMap<TypeId, NewtypeEntry>,

    // Indexes (built once at registration, then frozen)
    constructors: ConstructorRegistry,
    methods: MethodRegistry,

    // Name interning
    interner: Interner,
}
```

Σ is built in two passes:

1. **Prelude pass.** Register ground types (Str, Int, Bool,
   Fd, Bytes), prelude structs (Unit), prelude enums (Either),
   prelude newtypes (Option, Result). Build constructor and
   method registries for prelude types.

2. **User declaration pass.** Walk the AST's top-level
   declarations. Register each struct/enum/newtype. Extend
   constructor and method registries. After this pass, Σ is
   frozen — no further mutations.

### Parser ↔ Σ interface

The parser queries Σ during parsing for exactly two decisions:

1. **Constructor arity.** Is `NAME(` a constructor call? If so,
   does the constructor take arguments? Query:
   `sigma.constructors.qualified(type_id, name).nullary`.

2. **Name classification.** Is a bare uppercase name a type or
   a constructor? Query: `sigma.is_type(name)` vs
   `sigma.constructors.bare(name)`.

All other Σ queries happen in the checker, not the parser.
The parser produces an AST with unresolved names; the checker
resolves them against Σ.

### Checker ↔ Σ interface

The checker queries Σ for:

- **Type lookup.** `sigma.lookup_type(id) → &dyn Ty` — sort,
  zone, params, backing type.
- **Struct fields.** `sigma.structs[id].fields` — field names
  and types for named/positional construction and dot access.
- **Enum variants.** `sigma.enums[id].variants` — variant
  names and payloads for construction and exhaustiveness.
- **Newtype resolution.** `sigma.newtypes[id].backing` and
  `.renaming` — map constructor names through the renaming
  table, verify payloads against the backing enum.
- **Constructor disambiguation.** `sigma.constructors.bare(name)`
  → candidates, filtered by expected type in check mode.
- **Method resolution.** `sigma.methods.get(type_id, name)` →
  return type and def body reference.
- **Capability queries.** Does this type implement `Number`?
  `Iterable`? Checked via trait objects on `&dyn Ty`.

### Design principles

**Σ is frozen after declaration.** No runtime mutation. This
guarantees that type information is consistent across the
entire program — no declaration can invalidate a prior check.

**Interning.** Type names, field names, variant names are all
interned as `u32` indices. Comparison is integer equality.
HashMap lookups are fast. This matters because Σ queries are
on every hot path (every constructor call, every accessor,
every pattern match).

**Trait queries, not type-matching.** The checker never asks
"is this a newtype?" — it asks "does this type implement
Number?" and gets the right answer regardless. Newtypes are
transparent to capability queries. This is the implementation-
level analogue of `Adapter ∘ Prism = Prism`.

### Upper/lower bounds on elaborator complexity

If type inference becomes an albatross, the escape hatch is
weakening inference by leaning on the namespace system to
disambiguate. Weaken inference, not the type system.

Features naturally suggested by the type system should stay in
the design. `set -o` can disable non-essential features for
performance, but this is not an excuse to make core safety
features slow and then justify it with an off-switch. Safety
features achievable within the design goals must be implemented
well. The default is safe; opting out is conscious and rare.


