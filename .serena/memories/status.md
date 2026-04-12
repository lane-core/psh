---
type: status
status: current
created: 2026-04-10
last_updated: 2026-04-12
importance: high
keywords: [psh, design-complete, stub, rc, ksh93, bracket-dot, map-literal, double-quotes, tight-binding-dot, classical-notation, nil-coalescing, if-let, prism-preview]
agents: [plan9-systems-engineer, psh-session-type-agent, psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect]
verified_against: [docs/specification.md@HEAD, docs/syntax.md@HEAD, git-log@HEAD]
---

# psh status — session handoff (2026-04-12)

## What psh is

A new Plan 9 rc-derived system shell, standalone (no external
infrastructure required). Grounded in the λμμ̃-calculus of sequent
calculus, duploid semantics, virtual double categories (FVDblTT
as the internal language), profunctor optics, session types, and
the Plan 9 / ksh93 operational heritage.

## Design state: first draft complete

The spec (`docs/specification.md`) and grammar (`docs/syntax.md`)
have been through a six-agent review and verification pass. All
typing rules use classical sequent notation `Γ ⊢ t : A | Δ` with
derivable `[synth]`/`[check]` mode annotations. All formal claims
verified against cited references. `docs/deliberations.md` has
been retired — all content merged into the spec or deleted.

## Core design commitments

**Value model:**
- Every variable is a list. Substitution always splices.
- Tuples `(a, b)` (min 2). Lists `(a b c)`.
- Structs: type-prefixed brace construction. Named form
  `Pos { x = 10; y = 20 }` (semicolons), positional form
  `Pos { 10, 20 }` (commas, min 2). Self-typing via prefix.
- Enums: `ok(42)`, `err('msg')`, `none` (nullary bare name).
  User-declared with parametric type constructors.
- Maps: `Map(V)` with string keys. Brace literal
  `{'key': v, 'key': v}`. `.insert` builder chain.
  `Map.from_list` bulk constructor.

**Accessor system:**
- Bracket `$a[i]` for positional/key projection. Tight-binding.
  Lists/maps return `Option(T)`. Tuples return `T` directly
  (static bounds check, out-of-bounds is type error).
- Dot `$a.name` for named field/method/discipline. Tight-binding
  (no space required). `.` is always accessor, never free caret.
  Concatenation uses explicit `^` only.
- `??` nil-coalescing: `$l[0] ?? 'default'`.
- Prism previews: `$result.ok`, `$result.err` returning
  `Option(Payload)`. Compose with `??`.
- `if let` for refutable pattern branches.

**Strings:** Two forms. Single quotes literal (`'no $expansion'`).
Double quotes interpolate (`"hello $name"`). Inside double quotes:
`$var`, `$var[i]`, `` `{cmd} `` expanded. Dot accessors via
`${name.upper}`.

**Pattern matching:** `match` with enum/struct/tuple patterns,
`|` alternation, pure guards `if(cond)`. Guards restricted to
side-effect-free expressions (checker-enforced). let-else for
refutable bindings.

**Type system:** Bidirectional checking. Classical sequent rules
with [synth]/[check] mode annotations. Lambda parameter pinning
(synth-if-all-params-pinned from body operations). No unification,
no let-polymorphism.

**Option Display:** `some(v)` displays as `v`, `none` as empty
string. Display convention on the type, not REPL-special.

**Control flow:** `if`/`else`, `if let`, `while`, `for` with
`=> cmd` braceless form. `try`/`catch` as scoped ErrorT.
Unified `trap` (lexical/global/deletion). `$status : Int`,
`$pipestatus : List(Int)`.

**Coprocesses:** 9P-shaped discipline. Per-tag binary sessions
`Send<Req, (Recv<Resp, End> ⊕ Cancel<Recv<Flush_Ack, End>>)>`.
Cancellation is per-tag internal choice (⊕), not a separate
admin session. Negotiate on tag 0, orderly teardown via close
frame. Star topology, deadlock freedom by binary duality per
channel.

**Discipline functions:** `.get` (pure, CBV focused), `.refresh`
(effectful), `.set` (mutator). Mixed monadic lens per Clarke
def:monadiclens. Polarity frames at shift sites.

**Shell features:** Here documents (all rc forms + `<<-` tab
stripping). fd-targeted pipes `|[2]`, `|[n=m]`. Per-command
local variables `VAR=val cmd`. Process substitution `<{cmd}`.
Command substitution `` `{cmd} `` (rc heritage, kept over `$()`
after deliberation). Glob no-match passes through (rc heritage).

## Implementation state

Stub. `src/main.rs` exits 2. `src/parse.rs` has combine
boilerplate. `src/signal.rs` has self-pipe handler.
PLAN.md has 8-phase implementation roadmap.

## Knowledge tiers

1. `docs/specification.md` — source of truth
2. `docs/vdc-framework.md`, `refs/ksh93/ksh93-analysis.md`,
   `docs/implementation.md` — framework
3. serena memory store — project-shared knowledge base
4. vendored papers at `/Users/lane/gist/` — literature

## Where to read first

1. `CLAUDE.md` — project overview, load-bearing decisions
2. `docs/specification.md` — the current design
3. `docs/syntax.md` — grammar productions
4. `PLAN.md` — 8-phase implementation roadmap
5. `docs/vdc-framework.md` §4, §8, §9 — VDC framework
6. `refs/ksh93/ksh93-analysis.md` — ksh93 analysis
7. This `status` memory for project state

## How Lane works

Rapid iteration, terse responses. Corrects quickly — first
correction is the strongest signal. Dispatches agent roundtables
for hard decisions. No v1/v2 language — every feature is in the
design or explicitly a non-goal. No historical narrative in spec
— git carries history. The spec is the source of truth.
