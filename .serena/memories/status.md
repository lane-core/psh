---
type: status
status: current
created: 2026-04-10
last_updated: 2026-04-11
importance: high
keywords: [psh, design-phase, stub, rc, ksh93, bidirectional-checking, commitment-a, option-b, record-literal-structs, user-enums, parametric-type-constructors, pattern-lets]
agents: [plan9-systems-engineer, psh-session-type-agent, psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect]
verified_against: [docs/specification.md@HEAD, docs/syntax.md@HEAD, git-log@HEAD]
---

# psh status — session handoff

## What psh is

A new Plan 9 rc-derived system shell, standalone (no external
infrastructure required). Grounded in the λμμ̃-calculus of sequent
calculus, duploid semantics, profunctor optics, session types,
virtual double categories (FVDblTT as the internal language), and
the Plan 9 / ksh93 operational heritage.

## Where the design is

The spec is in active iteration. `docs/specification.md` and
`docs/syntax.md` describe the current design end-to-end.
`docs/deliberations.md` has been retired — all content merged
into the spec or deleted.
`docs/vdc-framework.md` holds the theoretical framework report
(§4 VDCs, §8 composition laws and decision procedure, §9
engineering principles are load-bearing). `refs/ksh93/ksh93-
analysis.md` is the ksh26 sequent-calculus analysis of ksh93.

**Core type-theoretic commitments:**

- **Three-sort AST** matching Grokking's three λμμ̃ categories:
  producers (Word/Value), consumers (synthesized, not stored),
  statements (Command). Plus `Expr` as an engineering layer for
  the profunctor transformations (pipelines, redirections). No
  separate `Binding` sort — let/assign are statements whose
  consumer is a μ̃-binder.

- **Every variable is a list.** Scalars are length-1 lists.
  Substitution always splices. Duff's principle. The semantic
  Duff lift per FVDblTT: contexts interpret as finite products of
  types, so tuples-at-the-type-layer and contexts-at-the-sequent-
  level are the same VDC object.

- **Polarity discipline.** CBV/CBN split follows the duploid's two
  subcategories. The (⊕,⊖) equation fails: `(h○g)•f ≠ h○(g•f)`
  — a comonadic step wrapping a monadic one. Operationally, psh's
  sequential evaluation + fork boundary + polarity frames prevent
  the failing bracketing.

- **Polarity frames** wrap the ↓→↑ shift sites: command
  substitution (`` `{cmd} ``), `$((...))` arithmetic (trivial
  frame), `.refresh` and `.set` discipline bodies. Pure `.get` and
  process substitution do not need a frame. Framework §9.3.

- **Three disciplines for codata variables** (`.get`, `.refresh`,
  `.set`). `.get` is **pure** `W(S, A)`; `.refresh` is effectful
  `Kl(Ψ)`, invoked as `varname.refresh` at step boundaries;
  `.set` is the mutator. Together `.get` and `.set` form a
  **mixed monadic lens** per Clarke `def:monadiclens` — pure
  view, effectful update. CBV focusing of `.get` is a theorem
  via Duploids Proposition 8550 (thunkable ⇒ central, forward
  direction of Hasegawa-Thielecke; does not require dialogue
  duploid structure).

- **⊕ and ⅋ duality for errors.** `$status` is ⊕ (data, tagged
  return, case-eliminated); `trap` is ⅋ (codata, cocase with
  callee-jump). Same codata framing as discipline cells. De
  Morgan duals.

- **Coprocesses** use 9P-shaped conversation discipline: negotiate,
  request-response per tag, error at any step, orderly teardown.
  Per-tag binary sessions `Send<Req, Recv<Resp, End>>`
  multiplexed over one socketpair. Shell is the hub (star
  topology), justified by Carbone-Marin-Schürmann forwarder
  theorem (MCutF admissibility, §5). Wire format: length-prefixed
  frames with first-byte dispatch for request/response/error
  (`'!'`)/tflush (`'#'`)/rflush (`'#'`). Admin session
  `Send<Cancel, Recv<Flush_Ack, End>>*` carries cancel frames;
  tag reuse gated on session termination.

**Value construction forms:**

- **Tuples** (anonymous products): `(a, b, c)` bare parens, comma-
  delimited. Static arity from literal.
- **Lists** (homogeneous collections): `(a b c)` bare parens,
  space-delimited. Runtime arity.
- **Structs** (nominal products): `{ field = value; field = value }`
  brace record literal. Named fields. Check-mode under
  bidirectional type checking — type determined by expected-type
  context (annotation, parameter type, return type, match arm).
  No bare-tuple-to-struct coercion: tuples and structs are
  disjoint types even when they share representation. No `Pos.mk`
  auto-generated constructor — the brace literal is the sole
  path. Name-pun shorthand `{ x; y }` for `{ x = x; y = y }`.
- **Enums** (nominal coproducts): `tag(payload)` for variants
  with payloads, bare `tag` for nullary variants. No `()` on
  nullary — `()` is the empty list. User-declared with `enum
  Name(T, E) { variant(T); nullary; ... }`. Multi-field payloads
  reference a declared struct.
- **Maps**: `Map(('k1', 1) ('k2', 2))` tagged construction with a
  space-delimited list of comma-delimited key-value tuples.
  Under discussion — Lane flagged the construction form may want
  revisiting.

**Type system:**

- **Bidirectional type checking**. Synth mode (bottom-up from
  literals and typed constructors) + check mode (top-down from
  annotations, return types, parameter types). No unification
  variables, no cross-expression inference, no let-polymorphism.
  Ambiguous bindings are errors at the binding site, not
  deferred holes. The full algorithm is in spec §"Bidirectional
  type checking." Expected footprint ~300-600 lines of Rust vs
  ~1200 for Hindley-Milner.

- **Parametric type constructors on user type declarations**:
  `enum Result(T, E) { ok(T); err(E) }`,
  `struct Pair(A, B) { first: A; second: B }`. Type parameters
  are uppercase (Rust convention, matching psh's capitalization
  rule for types). `def` signatures reference fully-instantiated
  ground types like `Result(Int, Str)`, never polymorphic ones.

- **Parametric polymorphism on `def` signatures is not in the
  design.** Function-level ∀ is a non-goal. Generic combinators
  live at the Rust implementation layer. See spec §"Non-goals"
  for the full reasoning (FVDblTT incompatibility with VETT-
  style polymorphism; rank-1 free-theorem benefit too weak;
  Reynolds parametricity carried internally by the polarity
  phase distinction per Sterling-Harper logical-relations-as-
  types §4226-4243).

**Pattern matching and let:**

- **Match** uses symmetric patterns: brace record form for
  structs (`{ x = 0; y = 0 }`), tagged form for enums (`ok(v)`,
  `error({ message; line })`). Name-pun shorthand works in
  patterns too.
- **Pattern let** with wildcards (`let _ = expr`), tuple
  destructuring (`let (a, b) = expr`), and struct destructuring
  (`let { x; y } = expr`). Irrefutable patterns only; refutable
  patterns use `let-else`: `let some(v) = lookup else { ... }`.

**Return:**

- **Explicit `return expr`** available in value-returning defs
  (and `return N` for status in status-returning defs — rc
  heritage).
- **Implicit return from final expression** in value-returning
  defs: if the last item in a body is a bare value expression
  of the declared return type, it is the return value. Rust-
  style.

**Non-goals (explicit, with reasoning in spec §Non-goals):**
parametric polymorphism on `def` signatures; typed session
channels on pipes (pipes stay byte streams, typed IPC goes
through coprocesses); refinement session types on coprocess
payloads; pipeline fusion as a user-visible feature.

## Where the code is

Design phase. The prior implementation was retired at commit
`76be317` during the VDC reframing. Current source tree is a
deliberate stub:

- `src/main.rs` — prints retirement status and exits with 2
- `src/parse.rs` — combine boilerplate for the future parser
- `src/signal.rs` — self-pipe signal handling, preserved
  unchanged (type-system-neutral)

`Cargo.toml` dependencies unchanged from `docs/implementation.md`:
`anyhow`, `bpaf`, `combine`, `fnmatch-regex`, `libc`, `rustix`,
`smallvec`, `signals_receipts`. No `pane`, no `par`.

Implementation work has not restarted. The sketch that will
drive parser work should come from psh-architect based on the
current spec, and the parser is Phase 1 of `PLAN.md`.

## Branch state

Currently on `redesign` branch, ahead of master by multiple
commits — the theoretical hardening pass and the subsequent
design consolidation. Not pushed. Recent commits (most recent
first): `9904b2f Remove historical annotations from spec`,
`d880f6c Land bidirectional checking, record literal structs,
user enums`, `7f9602d Tier 3 cleanup`, `313f5cb Tier 2 cleanup`,
`e987d21 Tier 1 Option B codata rewrite`. The redesign branch
is the current working branch and has not been merged to
master.

## Open threads

These are design discussions Lane paused mid-flight. Each is
live:

- **Map construction form — RESOLVED.** Map literal uses brace
  syntax `{'key': 1, 'age': 2}` (colon, comma). Map is `Map(V)`
  with string keys. Old `Map(...)` tagged form dropped. Builder
  via `.insert` chain, bulk via `Map.from_list`. Access via
  bracket `$m['key']` returning `Option(V)`. Landed in spec.

- **psh-architect's checker sketch.** The bidirectional type
  checker has a spec but no Rust sketch. The ~300-600 line
  footprint estimate is unverified against a published
  implementation.

- **Serena memory updates — RESOLVED (2026-04-12).** Updated
  `decision/tagged_construction_uniform` (narrowed to coproducts
  only, Map removed), `decision/postfix_dot_accessors` (rewritten
  for bracket/dot split). `decision/struct_positional_only_forever`
  was already deleted.

- **Pipe construction under Lane's "list of structs" framing.**
  Tangentially related to Map. Not actively in discussion but
  may be worth revisiting.

- **PLAN.md restructure.** `PLAN.md` still has Phase 1-10
  structure and a "Features not in v1" section that use
  staging language Lane has explicitly rejected in
  `docs/specification.md`. The phase ordering can stay (it's
  build ordering for the focused first release), but the
  "Features not in v1" section should be recast as either
  non-goals (with explicit reasoning) or features-in-the-design
  to match the spec's §"Features and non-goals" restructure.

## How Lane works

Key process notes for continuity:

- **No v1/v2 language.** psh is conceived as a unified totality,
  not a staged roadmap. Every feature is either in the design
  or explicitly out of scope with reasoning. Staged phrasing
  implies value preferences that age badly and confuse readers.

- **No historical narrative in the spec.** specification.md and
  syntax.md state what IS. Decision history is in git. Git
  ledger with supersession. Git carries history. Do not
  duplicate. Phrases like "an earlier draft committed to X"
  or "the reversal is motivated by Y" do not belong in the
  spec; they create the same staleness trap as v1/v2 framing.

- **Rapid iteration with directness.** Lane moves fast and
  expects terse, technical responses. He corrects quickly
  when a proposal drifts from his intent — the first correction
  is usually the strongest signal of what he wants.

- **Theoretical grounding is load-bearing.** Citations to
  Grokking, Dissection of L, Duploids, Clarke profunctor
  optics, fcmonads, FVDblTT (logical-aspects-of-vdc), Carbone-
  Marin-Schürmann, Deniélou-Yoshida, Das et al. are live
  references. The spec commits to the theory actually holding,
  not just being gestured at.

- **Roundtable pattern for hard decisions.** When a design
  question has multiple legitimate positions, Lane dispatches
  specialized agents in parallel (plan9, session-type, optics,
  vdc, sequent-calculus, architect) for deliberation, then
  synthesizes. The polymorphism roundtable that converged on
  Commitment A (untyped-at-surface with parametric type
  constructors on declarations) is the canonical example.

- **"Exhaust the options" for decisions.** Lane prefers thorough
  investigation over quick resolution. If a non-goal call is
  being reconsidered, the next step is usually a deep-audit
  roundtable that pulls from previously-unconsulted references
  before finalizing the call.

- **Write agents specific briefs, not shotgun prompts.** Each
  roundtable agent gets a targeted brief naming the references
  to consult, the sub-questions specific to their seat, and
  the format of the expected output. Generic "what do you
  think?" briefs produce shallow work.

## Where to read first (new session)

1. `CLAUDE.md` — project overview, load-bearing decisions
   (outdated in places; trust the spec over this when they
   disagree).
2. `docs/specification.md` — the current design. Read §"The
   three sorts," §"Polarity discipline," §"Discipline
   functions," §"Bidirectional type checking," §"Structs,"
   §"Enums," §"Coprocesses," §"Features and non-goals."
3. `docs/syntax.md` — grammar productions, including the
   pattern, record-literal, enum, and let-else additions.
4. Git log — decision history is in commit messages.
   Read only when investigating the history of a specific
   choice; do not let it seep into new spec text.
5. `docs/vdc-framework.md` §4, §8, §9 — VDC framework, load-
   bearing §8.5 decision procedure.
6. `refs/ksh93/ksh93-analysis.md` — ksh93 empirical analysis;
   source of the polarity frame discipline.
7. This `status` memory for the project state.
8. Relevant decision memos via `mcp__serena__list_memories` +
   `mcp__serena__read_memory`. Start with `decision/*` for
   load-bearing commitments and `analysis/*` for theoretical
   anchors. Watch for the stale ones flagged in "Open threads"
   above.

## What to work on next

The natural next step depends on Lane's priority:

- **If continuing design iteration**: the Map construction
  discussion is the most concrete open thread. Lane's
  "effectively a list of structs" observation is sharp and
  the resolution may propagate back to how collection types
  are syntactically distinguished from product types.

- **If pivoting to implementation**: psh-architect should
  produce a Rust sketch of the bidirectional type checker,
  the four-sort AST (Word, Expr, Command — with the consumer
  sort synthesized rather than stored), the value model
  including structs-as-nominal-tuples, and the record literal
  elaboration. Phase 1 of `PLAN.md` is the parser; the checker
  sketch is informally Phase 2. Both need spec-following
  sketches before committing to specific encodings.

- **If cleaning up the knowledge base**: the serena decision
  memos flagged in "Open threads" need rewrites, and PLAN.md
  needs its v1/v2 language expunged per Lane's principle.
  This is lower-stakes maintenance work that stabilizes the
  handoff surface for future sessions.

When starting a new session, ask Lane which direction before
picking one — all three have real value and none is
obviously blocking the others.
