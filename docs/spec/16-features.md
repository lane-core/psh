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

- **Map type** — associative arrays with string keys, O(1)
  lookup. Full specification in 06-types.md §Map(V).

- **String methods on `Str`** — fork-free string operations
  registered as `def Str::name { }` accessor methods. `.length`,
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

- **Typed pipes for def-to-def composition.** When both sides
  of a pipe are psh-native `def` cells, the pipe can carry a
  session type that the type checker verifies at the `|` site.
  The pipe remains a kernel byte stream — the type annotation
  is a static check, not a serialization protocol. External
  pipes (`ls | grep foo`) are unaffected and stay untyped.

  The dialogue duploid commitment (§Calculus) resolved the
  theoretical obstacle previously cited as a non-goal: the
  linear classical L-calculus provides involutive negation
  `¬(−)` for session duality, the par `⅋` for the negative
  monoidal structure, and the currification adjunction
  `−⊗Y ⊣ ¬Y ⅋ −` for typing pipeline stages [Wad14]. The
  practical obstacle — separately compiled external processes
  share no typing context at `|` — remains and restricts typed
  pipes to shell-internal composition.

  **Syntax.** The pipe operator gains an optional type
  annotation in brackets. `|[T]` is sugar for `|[Stream(T)]`
  where `T` is the element type:

      def lines |[Str] count         # Stream(Str) on the pipe
      def parse |[Int] sum           # Stream(Int) on the pipe
      ls | grep foo                  # untyped, unchanged
      cmd |[2:Str] logger            # fd 2 typed: Stream(Str)

  The bracket annotation is lexically unambiguous with fd
  targeting: type names are uppercase (`Str`, `Int`,
  `Stream(Str)`), fd numbers are digits (`2`, `5`). The
  combined form `|[N:T]` targets fd N with element type T.
  The grammar extension (§Syntax):

      pipe_op |= '|[' TYPE ']'            -- typed pipe
              |  '|[' NUM ':' TYPE ']'     -- fd-targeted typed

  **Session type.** The session type behind `|[T]` is the
  recursive streaming protocol `Stream(T)` (§Types):

      Stream(T) = μX. (Send<T, X> ⊕ End)

  At each step, the producer either sends a value of type T
  and continues, or closes the stream (EOF). The consumer's
  dual type is `¬Stream(T) = νX. (Recv<T, X> & End)` — at
  each step, receive a T or acknowledge end-of-stream. The
  `⊕`/`&` are internal/external choice from session type
  theory [HVK98]; the `μ`/`ν` are recursive session type
  constructors. The producer has initiative at every step.

  Writing `|[Stream(T)]` explicitly is valid and equivalent
  to `|[T]`. The explicit form exists so that richer session
  types on pipes (future work) can use the same annotation
  site — e.g., `|[WindowedStream(T)]` for batched protocols.

  **Typing rule.** The typed pipe is a cut with the session
  type as the cut formula. In sequent notation:

      Γ₁ ⊢ cmd₁ : S | Δ₁        Γ₂, α : ¬S ⊢ cmd₂ | Δ₂
      ───────────────────────────────────────────────────── (Cut-pipe-S)
      Γ₁, Γ₂ ⊢ cmd₁ |[S] cmd₂ | Δ₁, Δ₂

  The consumer receives the dual type `¬S` — the involutive
  negation from the L-calculus [MMM, §9.3]. For the common
  case `S = Stream(T)`, the producer writes `T` values to
  stdout and the consumer reads `T` values from stdin.

  When both sides are `def` cells, the type checker has access
  to both signatures at the `|` site. The check is a simple
  compatibility verification: the left side's output stream
  type must equal the right side's expected input stream type
  (after negation). When either side is an external command
  or a `def` without a stream type annotation, the pipe falls
  back to untyped (`!Bytes` — classical byte stream).

  **Subsumption.** A def with a typed output can always be
  piped to an untyped consumer. The session structure is
  erased (`Stream(T) → Bytes`) and the result promoted to the
  classical zone (`Bytes → !Bytes` via the L-calculus promotion
  rule `A → !A`), collapsing the protocol to a classical byte
  stream. The promotion is justified because an untyped byte
  stream has no protocol obligations — nothing to violate by
  duplication or discard. The reverse direction — untyped
  producer to typed consumer — requires an explicit parsing
  boundary (a `def` that reads bytes and produces typed values),
  because lifting `!Bytes` to `Stream(T)` requires runtime
  verification that the byte stream conforms to T.

  **Deadlock freedom.** Unidirectional `Stream(T)` is
  deadlock-free by structure: the producer only sends, the
  consumer only receives, no interleaving of send/receive on
  a single endpoint. Combined with the acyclic pipeline
  topology (a linear chain, not a cycle), no circular wait is
  possible.

  **Relationship to coprocesses.** Typed pipes and coprocesses
  are complementary mechanisms serving distinct protocol
  shapes:

  | Property | Coprocess (§Coprocesses) | Typed pipe |
  |---|---|---|
  | Topology | Bidirectional socketpair | Unidirectional pipe |
  | Session shape | `Send<Req, Recv<Resp, End>>` per tag | `Stream(T)` |
  | Multiplexing | Per-tag sessions over one channel | Single session per pipe |
  | Initiative | Shell-initiated (asymmetric) | Producer-initiated |
  | Participants | Shell ↔ external process | def ↔ def (shell-internal) |
  | Lifecycle | Named, scoped, explicit teardown | Pipeline-scoped, EOF = close |

  Coprocesses serve bidirectional request-response IPC with
  external processes. Typed pipes serve unidirectional
  streaming between shell-internal defs. Converging the two
  would add unnecessary complexity without serving either use
  case better.

  **Three-tier pipe system:**

  | Mechanism | When | Session type | Check |
  |---|---|---|---|
  | Untyped `\|` | External processes | `!Bytes` (classical) | None |
  | Typed `\|[T]` | Def-to-def | `Stream(T)` | Static at `\|` site |
  | Coprocess `\|&` | Bidirectional IPC | `Send<Req, Recv<Resp, End>>` | Rust phantom types |

  In VDC terms: a typed pipe is a horizontal arrow with a
  refined type annotation. The §8.5 classification is
  **monadic** (clause 1) — the type annotation refines the
  positive intermediary on the pipe without adding polarity
  shifts or boundary crossings. No new AST sorts, no new
  shift placement, no new critical pairs. The existing
  three-sort structure (Term/Command/Expr) is unaffected.

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

- **Typed pipes between external processes.** Pipes between
  separately compiled external processes remain untyped byte
  streams. There is no compile-time link at `|` between
  `/usr/bin/sort` and `/usr/bin/uniq` — no shared typing
  context exists. The typed-pipe feature (§Features above)
  applies only to shell-internal `def`-to-`def` composition
  where both sides inhabit psh's type system. Attempting to
  extend typed pipes to external processes would require
  either runtime protocol verification (expensive, fragile)
  or a type manifest system for external binaries (outside
  psh's scope). The `!Bytes` classical byte stream is the
  correct type for external pipes — it is the `!`-promoted
  degeneration of any session type, carrying no protocol
  discipline.

- **Structured data serialization over pipes.** Typed pipes
  are a static type check on kernel byte streams (scenario A),
  not a structured-data serialization protocol (scenario B).
  The pipe carries the same bytes it always would — the type
  annotation constrains what the type checker accepts, not
  what the kernel transmits. Shells that serialize structured
  data over pipes (PowerShell's .NET objects, nushell's
  tables) create a two-world seam at the external-command
  boundary: internal commands pass structured data, external
  commands pass text, and the boundary between them is a
  source of confusion and bugs. psh avoids this by keeping
  pipes as byte streams at every level. rc's principle holds:
  the kernel object is a file descriptor carrying bytes
  [Duf90, §Design Principles].

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

- **Parametric polymorphism on pipes.** Typed pipes
  (§Features above) use monomorphic ground types at the `|`
  site — `|[Str]`, `|[Int]`, `|[Stream(MyRecord)]` — not
  type variables. There are no universally quantified pipe
  types (`|[∀T. Stream(T)]`), because parametric polymorphism
  on `def` signatures is itself a non-goal (see above).
  Typed pipes compose with the existing monomorphic type
  system; they do not require or introduce type variables.

### Rejected features (heritage deviations)

- **`select` loop.** POSIX/ksh93's `select` prints a numbered
  menu to stderr, reads a choice, sets a variable, loops until
  `break`. psh's `menu` builtin supersedes it: typed
  `MenuResult(T)` return instead of string-in-variable,
  explicit cancellation tag instead of EOF, single-shot
  instead of implicit loop. `select`'s untyped looping model
  is the pattern psh's type system exists to improve on.
  `select` is not reserved — the word is available for user
  definitions.

### Reserved keywords

`type` is reserved for possible future use if parametric
polymorphism on function signatures is ever reconsidered. It
is not currently in the grammar.

`enum` is active — user-declared enums land under the
"Features beyond the rc base" section above.

