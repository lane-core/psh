# psh project instructions

## What this is

psh is a new Plan 9 rc-derived system shell being designed from the
parser up. It is grounded in a specific theoretical constellation:
the λμμ̃-calculus of sequent calculus, duploid semantics, profunctor
optics, session types, virtual double categories, and the Plan 9 /
ksh93 operational heritage. It is a standalone shell — no pane
dependency, no external infrastructure required.

**Current state:** design phase. The prior implementation was
retired during the VDC reframing (commit 76be317). The source tree
is a stub: `src/main.rs` reports the retirement status and exits,
`src/parse.rs` retains combine boilerplate (character predicates,
trivia, keyword/name primitives) as the starting point for the next
grammar implementation, `src/signal.rs` retains the self-pipe
pattern. The next round of work rebuilds the grammar, AST,
evaluator, and value model against the current spec.

## Read first

| Document | Role |
|---|---|
| `docs/spec/index.md` | Spec chapter directory — single source of truth. Start here. |
| `docs/spec/04-syntax.md` | Formal grammar — the implementer's contract. |
| `docs/vdc-framework.md` | Theoretical framework report. §4 (VDCs), §8 (composition laws + decision procedure), §9 (engineering principles) are load-bearing. |
| `refs/ksh93/ksh93-analysis.md` | ksh26 sequent-calculus analysis of ksh93. Source of the polarity frame discipline and the sh.prefix bug analysis. |
| `docs/implementation.md` | Dependency rationale and engineering principles (CLOEXEC, polarity frames, no global mutable state). |
| `PLAN.md` | Roadmap. |
| `STYLEGUIDE.md` | Coding conventions. |

## Load-bearing design decisions

All resolved; all in the spec. Do not re-litigate without consulting
the relevant agent and Lane.

- **Every variable is a list.** Scalars are lists of length 1. Type
  annotations refer to element types. Substitution always splices.
- **`let` binds the result of a computation (CBPV).** `let x = M`
  where `M : F(A)` is the μ̃-binder on monadic bind. Pure values
  are a trivial special case. Builtins return values directly.
- **`def` for named computations (Θ), `let` + lambda for values
  (Γ).** Lambda syntax is `|x| => expr` or `|x| { block }`.
- **Two accessor forms: bracket and dot.** Bracket `$a[i]` for
  projection by runtime value (tuples, lists, maps). Returns
  `Option(T)`. Dot `$x.name` for named field/method/discipline
  access — both bind tight, no space required. `.` is always
  accessor; concatenation uses explicit `^` (`$stem^.c`).
  `??` nil-coalescing: `$l[0] ?? 'default'`. Prism
  previews: `$result.ok ?? 'fallback'`. `if let` for
  refutable pattern branches. Per-type accessor namespaces via
  `def Type.ident`.
- **Uniform tagged construction.** `NAME(args)` with `NAME`
  immediately followed by `(` — space-delimited args. Covers
  enum variant construction (`ok(42)`, `err('msg')`).
- **Three roles of `()`.** Space-delimited list, comma-delimited
  tuple, tag-prefixed tagged construction. Lists splice into
  tagged construction; tuples do not.
- **Structs: type-prefixed brace construction.** Named form
  `Pos { x = 10; y = 20 }` (semicolons), positional form
  `Pos { 10, 20 }` (commas, min 2 fields). Self-typing via
  type prefix. Named accessors (`.x`, `.y`) auto-generated.
  `.fields` returns `List((Str, Str))` for generic traversal;
  `.values` returns `List(T)` on homogeneous structs.
- **Codata discipline functions.** `.get` is the codata observer
  (computes the value seen by the accessor), `.set` is the codata
  constructor. Both are `def` cells in Kl(Ψ). CBV focusing is the
  reentrancy semantics: within one expression, `.get` fires once
  per variable. Polarity frames prevent self-reentrance.
- **Unified `trap`.** Grammar: `trap SIGNAL (body body?)?`. Three
  forms: lexical (`trap SIGNAL { h } { body }`), global
  (`trap SIGNAL { h }`), deletion (`trap SIGNAL`). Precedence:
  innermost lexical > outer lexical > global > OS default.
- **Coprocesses.** Named bidirectional channels with per-tag binary
  sessions multiplexed over one socketpair. `print -p name 'query'`
  returns an Int tag. Shell-internal PendingReply tracking. Wire
  format: length-prefixed binary frames, MAX_FRAME_SIZE 16 MiB.
- **9P-derived coprocess discipline** (negotiate, request-response,
  error-at-any-step, orderly teardown). Star topology: shell as hub.
- **try/catch as scoped ErrorT.** Changes the sequencing combinator
  inside the body.
- **`$((...))` arithmetic.** In-process pure computation returning
  an `Int`.
- **Two string forms.** Single quotes are literal (`'no $expansion'`).
  Double quotes interpolate (`"hello $name"`). Multi-element lists
  inside double quotes join with spaces. `\`-escapes in both forms.

## Quick start

The current source tree is a stub. The only command that does
anything is:

```
cargo build          # clean build, zero warnings
cargo run            # prints retirement notice and exits with 2
```

When the next implementation round begins, run commands will return.

## Agent workflow

Six specialized agents live in `.claude/agents/`:

- **plan9-systems-engineer** — rc heritage, Plan 9 conventions,
  ksh93 operational behavior.
- **psh-session-type-agent** — session types on coprocess channels,
  multiparty compatibility, deadlock freedom arguments.
- **psh-optics-theorist** — profunctor optics, accessor semantics,
  discipline function MonadicLens structure.
- **vdc-theory** — virtual double category framework, composition
  laws, classification of new features (monadic / comonadic /
  boundary-crossing).
- **psh-sequent-calculus** — λμμ̃ typing rules, sort classification,
  polarity assignment, critical pair analysis.
- **psh-architect** — Rust implementation, AST design, parser
  structure, evaluator architecture.

For significant design decisions, dispatch the relevant agents in
parallel for deliberation. Every agent follows the operational
workflow in **`docs/agent-workflow.md`** — pre-task retrieval,
supersession tracking, scope handoff, and the memo output format
are defined there. When an agent charter disagrees with the
workflow doc, the workflow doc wins; when either disagrees with
`docs/spec/`, the spec wins.

## Committing

After completing a planned task where the build is clean (and any
tests that exist pass), commit without asking. Use a descriptive
message.

### Commit message format

Two-paragraph body after the subject line.

**First paragraph** describes Lane's provenance in third person,
using his name: the decision procedure, thought process, design
direction. Include a concrete summary of what was asked.

**Second paragraph** begins with "Agent steps:" and describes what
the agent did, including model used and any consultations.

Every commit containing AI-generated code must end with:

```
Generated-with: Claude opus-4-6 (Anthropic) via Claude Code
```

## References

| Source | Location |
|---|---|
| rc paper (Duff 1990) | `refs/plan9/papers/rc.ms` |
| rc man page | `refs/plan9/man/1/rc` |
| ksh93u+m manpage | `refs/ksh93/sh.1` |
| sfio analysis (14 files) | `refs/ksh93/sfio-analysis/` |
| ksh26 theoretical foundation | `refs/ksh93/ksh93-analysis.md` |
| VDC framework report | `docs/vdc-framework.md` |
| Theoretical papers (duploids, VDCs, optics, session types, sequent calculus) | `~/gist/` — accessed via agent-specific reference lists |

## Discipline

- **The spec is the source of truth.** Framework documents inform
  it but do not override it. If you find a contradiction, the spec
  wins.
- **The current implementation is not sacred.** Lane retired the
  prior type-system implementation explicitly so future work can
  rebuild from the current spec without compatibility constraints.
  Don't preserve code for preservation's sake.
- **Don't pick design options autonomously** when the resolution is
  not already in the spec. Consult the agents, then
  present to Lane.
- **Low-confidence rejection.** If you search the references for
  guidance and find nothing directly relevant, say so. Don't stretch
  an unrelated reference to fit.
- **Citation discipline.** When an analysis or implementation
  decision draws on a reference, cite the reference and section.
  When the spec is silent on a detail, note your judgment call.

## Knowledge management

Knowledge in psh lives at four tiers: `docs/spec/` (tier
1, single source of truth) > framework documents
(`docs/vdc-framework.md`, `refs/ksh93/ksh93-analysis.md`,
`docs/implementation.md` — tier 2) > **serena memory store** (tier
3, project-shared knowledge base; every agent reads the whole store,
writes only to `agent/<name>/` or project-level types) > vendored
papers at `/Users/lane/gist/` (tier 4). Higher tiers override lower;
spec always wins.

There is **no parallel per-tool memory layer.** Agent-private
institutional knowledge lives inside serena under
`agent/<agent-name>/`, not in a separate directory. The
`.claude/agent-memory/` path does not exist.

Operational protocol (pre-task retrieval, memo format, scope
handoff, self-review) is in **`docs/agent-workflow.md`**. Memory
organization discipline (frontmatter, namespaces, hub-and-spokes,
per-type schemas, migration rules) lives in serena as the
**`policy/memory_discipline`** memory — Lane's port of the MemX
paper (Sun 2026, `/Users/lane/gist/memx/memx.tex`) to serena —
the project's canonical source of truth for memory discipline.

The seven operational principles, in short:

1. **Hybrid retrieval.** Semantic query AND keyword query. Combine.
2. **Access vs retrieval.** Cite what you used; list consulted-
   but-not-applicable separately.
3. **Low-confidence rejection.** "No prior material" beats a
   stretched citation.
4. **Supersession tracking.** New decisions state what they
   supersede / extend / refine / contradict, with file + section.
5. **Source ranking.** Follow higher tier; flag conflicts, don't
   reconcile.
6. **Scope boundaries.** Hand off out-of-scope queries by name.
7. **Deduplication.** Search serena (including your
   `agent/<name>/` folder) and the spec before writing new
   material.
