# Checker — bidirectional type checking

The checker walks the AST, queries Σ, and fills in type
annotations. It implements the bidirectional algorithm from
05-type-checking.md: every expression is either synth (type
determined by premises) or check (type supplied from context).
No unification variables, no constraint solver, no
backtracking. Linear time on AST size.

Spec correspondence: 05-type-checking.md defines the modes,
the pinning mechanism, and the subsumption rule. 06-types.md
typing rules carry `[synth]` / `[check]` annotations that
the checker implements directly.

### Core interface

```rust
/// Type checking context — threaded through the walk.
struct CheckCtx<'s> {
    sigma: &'s Sigma,
    gamma: Scope,          // Γ — variable bindings (Layer 7)
    theta: DefRegistry,    // Θ — def bindings (Layer 7)
    errors: Vec<TypeError>,
}

/// The two modes.
enum Mode {
    Synth,
    Check(TypeRef),        // expected type from context
}

/// A type error with source location.
struct TypeError {
    span: Span,
    kind: TypeErrorKind,
}

enum TypeErrorKind {
    Mismatch { expected: TypeRef, got: TypeRef },
    Ambiguous { expr: &'static str },
    UnpinnedParam { type_name: TypeId, param: u8 },
    UndeclaredType(Name),
    UndeclaredVariant(Name),
    UndeclaredVariable(Name),
    NonExhaustive { missing: Vec<Name> },
    LinearNotConsumed(Name),
    AffineUsedTwice(Name),
    ArityMismatch { expected: usize, got: usize },
    NotCallable(TypeRef),
    NotAccessible { ty: TypeRef, name: Name },
    // ...
}
```

### The walk

The checker is three mutually recursive functions, one per
AST sort:

```rust
impl<'s> CheckCtx<'s> {
    /// Term → TypeRef. Synth or check, depending on mode.
    fn check_term(&mut self, term: &mut Term, mode: Mode)
        -> TypeRef;

    /// Command → (). Commands don't produce types; they
    /// extend Γ/Θ or produce Status.
    fn check_command(&mut self, cmd: &mut Command);

    /// Expr → TypeRef (Status for most exprs).
    fn check_expr(&mut self, expr: &mut Expr, mode: Mode)
        -> TypeRef;
}
```

Each function dispatches on the AST variant and applies the
appropriate typing rule from 06-types.md. The `mode` parameter
determines synth vs check. The function fills in `ann.ty` on
the node before returning.

### Synth vs check dispatch

The mode table from 05-type-checking.md maps directly to
match arms:

```rust
fn check_term(&mut self, term: &mut Term, mode: Mode) -> TypeRef {
    match term {
        // [synth] — type determined by literal form
        Term::Literal(ann, val) => {
            let ty = self.synth_literal(val);
            self.reconcile(ann, ty, mode)
        }

        // [synth] — type from (x : A) ∈ Γ
        Term::Var(ann, ident) => {
            let ty = self.gamma.lookup(ident.name)
                .unwrap_or_else(|| self.error(ann, UndeclaredVariable(ident.name)));
            self.reconcile(ann, ty, mode)
        }

        // [synth-if-pinned] — payload pins params
        Term::Tagged(ann, qualifier, variant, payload) => {
            self.check_tagged(ann, qualifier, variant, payload, mode)
        }

        // [check] — no payload, no bottom-up info
        // (nullary variants like `none`)
        Term::Tagged(ann, _, variant, None) if mode == Mode::Synth => {
            self.error(ann, Ambiguous { expr: "nullary variant" });
        }

        // ... remaining arms
    }
}
```

### The reconcile function

The bridge between synth and check. After a node synthesizes
a type, `reconcile` checks it against the expected type if
one exists:

```rust
fn reconcile(&mut self, ann: &mut Ann, synth: TypeRef, mode: Mode)
    -> TypeRef
{
    ann.ty = Some(synth.clone());
    match mode {
        Mode::Synth => synth,
        Mode::Check(expected) => {
            if !self.types_eq(&synth, &expected) {
                self.error(ann, Mismatch {
                    expected: expected.clone(),
                    got: synth,
                });
            }
            expected
        }
    }
}
```

This implements the subsumption rule from 05-type-checking.md:

    Γ ⊢ e ⇒ T'    T' = T
    ─────────────────────
    Γ ⊢ e ⇐ T

### Type parameter pinning

Parametric types need their parameters pinned at the
construction site. The checker uses write-once slots:

```rust
/// Write-once slot for a type parameter.
struct ParamSlot {
    param_index: u8,
    pinned: Option<TypeRef>,
}

impl ParamSlot {
    /// Pin the parameter. Error if already pinned to a
    /// different type.
    fn pin(&mut self, ty: TypeRef, span: Span, errors: &mut Vec<TypeError>)
    {
        match &self.pinned {
            None => self.pinned = Some(ty),
            Some(existing) if existing == &ty => {},
            Some(existing) => errors.push(TypeError {
                span,
                kind: TypeErrorKind::Mismatch {
                    expected: existing.clone(),
                    got: ty,
                },
            }),
        }
    }
}
```

For `ok(42)` in `Result(T, E)`:
1. Allocate slots `[T: None, E: None]`
2. Check payload `42 : Int` → pin `T = Int` (synth, bottom-up)
3. If in check mode with expected `Result(Int, Str)` → pin
   `E = Str` (check, top-down)
4. After both directions, any unpinned slot → error

Lambda parameter pinning uses the same mechanism: body
operations with monomorphic signatures write to the slot.

### Newtype-aware checking

Tagged construction with newtypes: the checker resolves
through the renaming table.

```rust
fn check_tagged(&mut self, ann: &mut Ann,
    qualifier: &Option<Ident>, variant: &Ident,
    payload: &mut Option<Box<Term>>, mode: Mode)
    -> TypeRef
{
    // 1. Look up variant in constructors registry
    let info = self.resolve_constructor(qualifier, variant, mode);

    // 2. If parent is a newtype, consult renaming table
    //    to find the backing variant and its payload type
    let payload_ty = if let Some(nt) = self.sigma.newtypes.get(&info.parent) {
        let backing_name = nt.reverse.get(&variant.name);
        self.sigma.variant_payload(nt.backing, *backing_name)
    } else {
        info.payload.clone()
    };

    // 3. Check/synth payload against resolved type
    if let (Some(pl), Some(ty)) = (payload, &payload_ty) {
        self.check_term(pl, Mode::Check(ty.clone()));
    }

    // 4. Build result type with pinned params
    // ...
}
```

### Exhaustiveness checking

Match arms must cover all variants. The checker collects
the set of matched variants and compares against Σ:

```rust
fn check_exhaustive(&self, type_id: TypeId,
    arms: &[MatchArm], span: Span)
{
    let all_variants = self.sigma.variant_names(type_id);
    let covered: HashSet<Name> = arms.iter()
        .flat_map(|arm| arm.patterns.iter())
        .filter_map(|pat| pat.variant_name())
        .collect();

    // Wildcards cover everything
    if arms.iter().any(|arm| arm.has_wildcard()) { return; }

    let missing: Vec<_> = all_variants
        .difference(&covered)
        .collect();

    if !missing.is_empty() {
        self.error(span, NonExhaustive {
            missing: missing.into_iter().copied().collect()
        });
    }
}
```

For newtypes, `variant_names` returns the renamed names (not
the backing names). Exhaustiveness is checked at the newtype's
abstraction level.

### Linear resource tracking

The checker tracks linear and affine bindings to verify
consumption:

```rust
struct LinearTracker {
    /// Bindings that must be consumed exactly once.
    linear: HashMap<Name, (Span, bool)>,   // (decl site, consumed?)
    /// Bindings that may be consumed at most once.
    affine: HashMap<Name, (Span, u32)>,    // (decl site, use count)
}
```

At scope exit, the checker verifies: every linear binding has
`consumed == true`, every affine binding has `use_count <= 1`.
Affine bindings with `use_count == 0` trigger cleanup (e.g.,
Tflush for ReplyTag). This is the three-zone model from
03-polarity.md realized as a bookkeeping pass.

### What the checker does NOT do

- **No unification.** Write-once slots, not unification
  variables. No occurs check, no constraint propagation.
- **No cross-expression inference.** Each binding is checked
  at its site. The type of `x` is determined where `x` is
  bound, not where `x` is used.
- **No AST transformation.** The checker annotates in place.
  No desugaring, no elaboration into a core language.
- **No effects tracking.** The checker does not distinguish
  pure from effectful terms (that's the evaluator's job via
  the polarity frame mechanism). It tracks resource zones
  (linear/affine/classical) but not effect types.


