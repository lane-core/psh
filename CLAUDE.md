Can't write temp file — compressing manually per skill rules.

## Communication style

Terse like caveman. Technical substance exact. Only fluff die.
Drop: articles, filler (just/really/basically), pleasantries, hedging.
Fragments OK. Short synonyms. Code unchanged.
Pattern: [thing] [action] [reason]. [next step].
ACTIVE EVERY RESPONSE. No revert after many turns. No filler drift.
Code/commits/PRs: normal. Off: "stop caveman" / "normal mode".

# psh project instructions

## What this is

psh: new Plan 9 rc-derived system shell, designed from parser up. Grounded in λμμ̃-calculus of sequent calculus, dialogue duploid semantics (linear classical L-calculus as internal type theory), profunctor optics, session types, virtual double categories, Plan 9 / ksh93 operational heritage. Standalone shell — no pane dependency, no external infrastructure.

**Current state:** design phase. Prior implementation retired during VDC reframing (commit 76be317). Source tree is stub: `src/main.rs` reports retirement + exits, `src/parse.rs` retains combine boilerplate (character predicates, trivia, keyword/name primitives) as starting point for next grammar, `src/signal.rs` retains self-pipe pattern. Next work round rebuilds grammar, AST, evaluator, value model against current spec.

## Read first

| Document | Role |
|---|---|
| `docs/spec/index.md` | Spec chapter directory — single source of truth. Start here. |
| `docs/spec/04-syntax.md` | Formal grammar — implementer's contract. |
| `docs/vdc-framework.md` | Theoretical framework report. §4 (VDCs), §8 (composition laws + decision procedure), §9 (engineering principles) load-bearing. |
| `refs/ksh93/ksh93-analysis.md` | ksh26 sequent-calculus analysis of ksh93. Source of polarity frame discipline + sh.prefix bug analysis. |
| `docs/impl/` | Implementation blueprint — 7-layer architecture (Σ, Val, AST, parser, checker, evaluator, environment). Start at `docs/impl/index.md`. |
| `PLAN.md` | Roadmap. |
| `STYLEGUIDE.md` | Coding conventions. |

## Load-bearing design decisions

All resolved; all in spec. No re-litigation without consulting relevant agent + Lane.

- **Every variable is a list.** Scalars = length-1 lists. Type annotations refer to element types. Substitution always splices.
- **`let` binds computation result (CBPV).** `let x = M` where `M : F(A)` is μ̃-binder on monadic bind. Pure values trivial special case. Builtins return values directly.
- **`def` for named computations (Θ), `let` + lambda for values (Γ).** Lambda: `|x| => expr` or `|x| { block }`.
- **Two accessor forms: bracket and dot.** Bracket `$a[i]` for projection by runtime value (tuples, lists, maps). Returns `Option(T)`. Dot `$x.name` for named field/method/discipline access — both bind tight, no space needed. `.` always accessor; concatenation uses explicit `^` (`$stem^.c`). `??` nil-coalescing: `$l[0] ?? 'default'`. Prism previews: `$result.ok ?? 'fallback'`. `if let` for refutable pattern branches. Per-type accessor namespaces via `def Type.ident`.
- **Uniform tagged construction.** `NAME(args)` with `NAME` immediately followed by `(` — space-delimited args. Covers enum variant construction (`ok(42)`, `err('msg')`).
- **Three roles of `()`.** Space-delimited list, comma-delimited tuple, tag-prefixed tagged construction. Lists splice into tagged construction; tuples do not.
- **Structs: type-prefixed brace construction.** Named: `Pos { x = 10; y = 20 }` (semicolons), positional: `Pos { 10, 20 }` (commas, min 2 fields). Self-typing via type prefix. Named accessors (`.x`, `.y`) auto-generated. `.fields` returns `List((Str, Str))` for generic traversal; `.values` returns `List(T)` on homogeneous structs.
- **Codata discipline functions.** `.get` = codata observer (computes value seen by accessor), `.set` = codata constructor. Both `def` cells in Kl(Ψ). CBV focusing = reentrancy semantics: within one expression, `.get` fires once per variable. Polarity frames prevent self-reentrance.
- **Unified `trap`.** Grammar: `trap SIGNAL (body body?)?`. Three forms: lexical (`trap SIGNAL { h } { body }`), global (`trap SIGNAL { h }`), deletion (`trap SIGNAL`). Precedence: innermost lexical > outer lexical > global > OS default.
- **Coprocesses.** Named bidirectional channels w/ per-tag binary sessions multiplexed over one socketpair. `print -p name 'query'` returns `ReplyTag` (affine resource). Shell-internal PendingReply tracking. Wire format: length-prefixed binary frames, MAX_FRAME_SIZE 16 MiB.
- **9P-derived coprocess discipline** (negotiate, request-response, error-at-any-step, orderly teardown). Star topology: shell as hub.
- **try/catch as scoped ErrorT.** Changes sequencing combinator inside body.
- **`$((...))` arithmetic.** In-process pure computation returning `Int`.
- **Two string forms.** Single quotes literal (`'no $expansion'`). Double quotes interpolate (`"hello $name"`). Multi-element lists in double quotes join w/ spaces. `\`-escapes in both.
- **Dialogue duploid commitment.** Full Hasegawa-Thielecke theorem (thunkable ⇔ central). Linear classical L-calculus as internal type theory. Exponentials `!`/`?` partition typing context.
- **Three-zone linear resource model.** Classical (`!A`, default for value types), affine (ReplyTag — drop triggers Tflush), linear (bare `A` — must consume). `set -o linear` for whole-script linear mode. `let !x` for classical promotion.
- **Path is a component list**, not string. Duff's principle for filesystem paths. Root marker + `List(Str)`. Not subtype of Str.
- **ExitCode = `{ code: Int, message: Str }`.** Status refactored to `Result((), ExitCode)` — genuine ⊕ coproduct. `catch (e : ExitCode)`.
- **Typed pipes for def-to-def composition.** `|[T]` annotates pipe w/ element type T (sugar for `|[Stream(T)]`). Static type check only — pipe stays kernel byte stream. External pipes untyped. `Stream(T) = μX. (Send<T, X> ⊕ End)`. Cut rule w/ ¬S for consumer. VDC classification: monadic. Complementary to coprocesses (streaming vs request-response).
- **`set -o` option system.** 13 options, 6 axes. noclobber/pipefail default ON. No errexit (try/catch). No nounset (type checker). `set -o emacs` / `set -o vi` for editor mode.

## Quick start

Source tree is stub. Only working command:

```
cargo build          # clean build, zero warnings
cargo run            # prints retirement notice and exits with 2
```

Next implementation round → run commands return.

## Agent workflow

Six agents in `.claude/agents/`:

- **plan9-systems-engineer** — rc heritage, Plan 9 conventions, ksh93 operational behavior.
- **psh-session-type-agent** — session types on coprocess channels, multiparty compatibility, deadlock freedom.
- **psh-optics-theorist** — profunctor optics, accessor semantics, discipline function MonadicLens structure.
- **vdc-theory** — virtual double category framework, composition laws, feature classification (monadic / comonadic / boundary-crossing).
- **psh-sequent-calculus** — λμμ̃ typing rules, sort classification, polarity assignment, critical pair analysis.
- **psh-architect** — Rust implementation, AST design, parser structure, evaluator architecture.

Big design decisions → dispatch relevant agents in parallel. All agents follow **`docs/agent-workflow.md`** (pre-task retrieval, supersession tracking, scope handoff, memo format). Agent charter disagrees w/ workflow doc → workflow doc wins. Either disagrees w/ `docs/spec/` → spec wins.

## Committing

Build clean + tests pass after planned task → commit without asking. Descriptive message.

### Commit message format

Two-paragraph body after subject line.

**First paragraph:** Lane's provenance in third person, using his name — decision procedure, thought process, design direction. Concrete summary of ask.

**Second paragraph:** starts "Agent steps:" — what agent did, model used, consultations.

AI-generated code commits must end with:

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
| Theoretical papers (duploids, VDCs, optics, session types, sequent calculus) | `~/gist/` — via agent-specific reference lists |

## Discipline

- **Spec = source of truth.** Framework documents inform but don't override. Contradiction → spec wins.
- **Current implementation not sacred.** Lane retired prior type-system implementation so future work rebuilds from current spec w/o compatibility constraints. Don't preserve code for preservation's sake.
- **No autonomous design picks** when resolution not in spec. Consult agents, present to Lane.
- **Low-confidence rejection.** Search refs, find nothing relevant → say so. Don't stretch unrelated reference to fit.
- **Citation discipline.** Decision draws on reference → cite reference + section. Spec silent → note judgment call.

## Knowledge management

Four tiers: `docs/spec/` (tier 1, source of truth) > framework docs (`docs/vdc-framework.md`, `refs/ksh93/ksh93-analysis.md`, `docs/impl/` — tier 2) > **serena memory store** (tier 3, project-shared; agents read whole store, write only to `agent/<name>/` or project-level types) > vendored papers at `/Users/lane/gist/` (tier 4). Higher tiers override lower; spec always wins.

**No parallel per-tool memory layer.** Agent-private knowledge lives in serena under `agent/<agent-name>/`, not separate directory. `.claude/agent-memory/` path does not exist.

Operational protocol in **`docs/agent-workflow.md`**. Memory discipline (frontmatter, namespaces, hub-and-spokes, per-type schemas, migration rules) in serena as **`policy/memory_discipline`** — Lane's port of MemX paper (Sun 2026, `/Users/lane/gist/memx/memx.tex`) to serena — canonical source of truth for memory discipline.

Seven operational principles:

1. **Hybrid retrieval.** Semantic query AND keyword query. Combine.
2. **Access vs retrieval.** Cite what used; list consulted-but-not-applicable separately.
3. **Low-confidence rejection.** "No prior material" beats stretched citation.
4. **Supersession tracking.** New decisions state what they supersede / extend / refine / contradict, w/ file + section.
5. **Source ranking.** Follow higher tier; flag conflicts, don't reconcile.
6. **Scope boundaries.** Hand off out-of-scope queries by name.
7. **Deduplication.** Search serena (including `agent/<name>/`) + spec before writing new material.