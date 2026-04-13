# Features and non-goals

## Features and non-goals

psh is conceived as a unified totality, not a staged roadmap.
Every feature is either in the design or explicitly out of
scope with reasoning. There is no "v1/v2" split; the spec
describes the shell as a whole.

### Features beyond the rc base

The following types and features are load-bearing members of
psh's type system beyond what rc had. Each is in the design;
full spec sections will be added or refined as the design
stabilizes.

- **Map type** — associative arrays with O(1) lookup.
  Parametric type constructor `Map(V)`. Keys are always
  strings — matching the unanimous convention of bash, zsh,
  ksh93, nushell, and ysh. Integer-keyed collections use
  lists; string-keyed associative lookup uses maps.

  **Construction** has three paths:

  *Map literal* — brace syntax with colon key-value separator
  and comma delimiter, syntactically distinct from struct
  record literals (`=` and `;`):

      let m : Map(Int) = {'name': 1, 'age': 2}

  Map literals can synthesize their type from entries
  (value types must agree). Empty `{}` is [check] — the
  expected type resolves whether it is an empty map or an
  empty struct.

  **Typing rule** (map introduction):

      Γ ⊢ vᵢ : V | Δ    (for each entry 'kᵢ': vᵢ)    [synth]
      ─────────────────────────────────────────────
      Γ ⊢ {'k₁': v₁, …, 'kₙ': vₙ} : Map(V) | Δ

  *Builder chain* — `.insert` is a pure functional update
  method on Map values (distinct from the discipline `.set`
  on variables), returning a new Map:

      let m : Map(Int) = Map.empty .insert 'name' 1 .insert 'age' 2

  *Bulk constructor* — `Map.from_list : List((Str, V)) → Map(V)`
  constructs from a list of key-value tuples:

      let pairs = (('name', 1) ('age', 2))
      let m : Map(Int) = Map.from_list $pairs

  **Access** uses bracket notation — `$m['key']` returns
  `Option(V)` (`some(v)` or `none`). Inside brackets is
  expression context (never glob); the index expression must
  be of type `Str`. **Insertion** via assignment —
  `m['key'] = v` on a `let mut` map desugars to
  `m = $m.insert 'key' v` (discipline-transparent).

  **Accessors:** `.keys` returns `List(Str)`, `.values`
  returns `List(V)`. Key-indexed view (`$m['key']`) is an
  affine traversal; iterate-all-values (`.values`) is a
  traversal.

- **String methods on `Str`** — fork-free string operations
  registered as `def Str.name { }` accessor methods. `.length`,
  `.upper`, `.lower`, `.split`, `.strip_prefix`,
  `.strip_suffix`, `.replace`, `.contains`. Partial operations
  return `Option(T)`; predicates return status. Replaces
  ksh93's `${var#pat}`/`${var%pat}` parameter expansion.

- **Job control builtins** — `fg`, `bg`, `jobs`, `wait` (with
  `-n` for any-child), `kill`. Job IDs as a new word form:
  `%N` expands to the PID of job N.

- **Here documents and here-strings** — rc heritage (rc.ms
  lines 906-973). Six forms from three orthogonal toggles:

  | Form | Expansion | Tab strip | fd |
  |---|---|---|---|
  | `<<EOF` | yes | no | stdin |
  | `<<'EOF'` | no (literal) | no | stdin |
  | `<<-EOF` | yes | tabs stripped | stdin |
  | `<<-'EOF'` | no | tabs stripped | stdin |
  | `<<[n]EOF` | yes | no | fd n |
  | `<<-[n]EOF` | yes | tabs stripped | fd n |

  Quoted marker (`'EOF'`) suppresses variable expansion. `<<-`
  strips leading tabs from body and closing marker (ksh93
  heritage, sh.1 line 3867). fd-targeted form `<<[n]` directs
  to fd n instead of stdin (rc heritage, rc.ms line 959).
  Here-string `<<<` is the degenerate case: `cmd <<< 'input'`
  is a one-line here document with no closing marker.

  In VDC terms: a cell with a constant (or expansion-computed)
  horizontal arrow on the specified fd. §8.5 classification:
  monadic (positive-to-positive), same as string literals.

- **`$((...))` arithmetic** — documented in §"rc's execution
  model"; in-process pure expression evaluation returning an
  `Int`, shaped as `μα.⊙(e₁, e₂; α)` per [BTMO23, §2.1].

- **Parametric type constructors on user declarations.**
  Users may declare parametric struct and enum types with
  uppercase type parameters in the declaration header:
  `enum Result(T, E) { ok(T); err(E) }`,
  `struct Pair(A, B) { first: A; second: B }`. Type
  parameters live on type declarations only; `def` signatures
  reference fully-instantiated ground types
  (`def parse : Str -> Result(Int, Str)`), never polymorphic
  ones. This is strictly weaker than rank-1 prenex
  polymorphism on function signatures (see §Non-goals) and
  does not require Hindley-Milner machinery — only structural
  monomorphization at each use site.

### Non-goals

The following are explicitly out of scope. Adding them would
distort the type theory or bloat the implementation without
serving the focused shell psh is designed to be.

- **Parametric polymorphism on `def` signatures.** Rank-1
  prenex `∀` at function boundaries — `def map(T, U) : (T ->
  U) -> List(T) -> List(U)` or similar — is not part of the
  design. Function-level polymorphism requires either VETT's
  hyperdoctrine semantics (which abandons FVDblTT's single-
  VDC home and in-language protype-isomorphism reasoning) or
  the unpublished dependent-FVDblTT extension. The rank-1
  free-theorem benefit (`∀α. α → α` is uniquely identity) is
  too weak to justify elaborator complexity, annotation
  burden, and the categorical commitments required.
  Init-script robustness is delivered instead by rich
  ground-typed builtins and the polarity discipline that
  already carries Reynolds-style parametricity internally at
  the phase boundary [Sterling-Harper, logical-relations-as-
  types §4226-4243]. Generic combinators (`map`, `filter`,
  `fold`) are shell builtins whose types live at the Rust
  implementation layer and are never surfaced to the shell
  user. Parametric type *constructors* on type declarations
  (see Features above) are permitted because they require
  only monomorphization at use sites, which is structurally
  different from rank-1 prenex `∀` at function signatures.
  The `type` keyword remains reserved in case a concrete
  shell-level use case ever emerges that monomorphic ground
  types cannot serve.

- **Typed session channels on pipes.** Pipes are byte
  streams. The typed-IPC case is served entirely by
  coprocesses (§Coprocesses), which have session-type
  discipline per tag. Typing pipes would force every pipeline
  stage to commit to a protocol — breaking rc's text-stream
  pipeline ergonomics and conflating two different IPC
  mechanisms. There is no binding site at `|` to pin session
  types, because pipe producers and consumers are separately
  compiled processes with no compile-time link. Pipes are
  byte streams, and typed IPC goes through coprocesses.

- **Refinement session types on coprocess payloads.** Das et
  al. ("Practical Refinement Session Type Inference") prove
  that refinement session type checking is undecidable even
  for simple linear arithmetic refinements; practical
  implementations require SMT-solver integration (Rast ships
  Z3). The phantom-session-type substrate psh uses for
  coprocess protocols gets its guarantees from Rust's own
  type checker and does not compose with solver-based
  refinement checking. Base-type refinements on payload
  values (`NonEmpty(Str)`, `Positive(Int)`) are a value-layer
  question that could ride on a future refinement-types
  addition if psh ever acquires one; that is orthogonal to
  session types.

- **Pipeline fusion as a user-visible feature.** The Segal
  condition (fcmonads §5) gives the categorical account of
  when a sequence of pipeline stages has a composite. This is
  an implementation-level optimization opportunity — the
  evaluator may fuse adjacent stages for performance when
  their composite exists — not a user-facing construct.
  Exposing fusion as a user feature would require committing
  to the pseudo-double-category equations that a VDC
  instance may not satisfy in general, and would force users
  to reason about composite existence at sites where they
  should just be writing shell. Fusion stays in the
  implementation notes.

- **Parametric polymorphism on pipes.** See above. Both the
  "typed pipes" and the "parametric polymorphism" non-goals
  are self-reinforcing — the former requires type variables
  at pipeline stages, the latter rules out user-visible type
  variables in function signatures, and together they pin
  pipes at byte streams.

### Reserved keywords

`type` is reserved for possible future use if parametric
polymorphism on function signatures is ever reconsidered. It
is not currently in the grammar.

`enum` is active — user-declared enums land under the
"Features beyond the rc base" section above.

