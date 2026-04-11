---
type: status
status: current
created: 2026-04-10
last_updated: 2026-04-11
importance: high
keywords: [psh, design-phase, implementation-retired, vdc-reframing, stub, rc, ksh93, serena-seeding, phase-1-parser]
agents: [plan9-systems-engineer, psh-session-type-agent, psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect]
verified_against: [PLAN.md@HEAD, docs/specification.md@HEAD, git-log@HEAD, git-log@76be317]
---

# psh status

## Where we are

psh is a new Plan 9 rc-derived system shell. Standalone (no pane dependency, no external infrastructure). Grounded in the λμμ̃-calculus of sequent calculus, duploid semantics, profunctor optics, session types, virtual double categories, and the Plan 9 / ksh93 operational heritage.

**Design: complete**, pending one final VDC reframing pass. Resolved decisions are in `docs/specification.md` and `docs/syntax.md`. Decision history with supersession is in `docs/deliberations.md`. Theoretical framework is in `docs/vdc-framework.md`. ksh93 empirical analysis is vendored at `refs/ksh93/ksh93-analysis.md`. Dependency rationale is in `docs/implementation.md`.

**Implementation: retired** at commit `76be317`. The prior ~8800-line implementation encoded design decisions that have since moved. The current source tree is a deliberate stub:

- `src/main.rs` (~35 lines) — binary stub; reports retirement and exits 2
- `src/parse.rs` (~130 lines) — combine boilerplate (char predicates, trivia, name primitives)
- `src/signal.rs` (~130 lines) — self-pipe signal handling, type-system neutral, preserved

**Dependencies** unchanged per `docs/implementation.md`: `anyhow`, `bpaf`, `combine`, `fnmatch-regex`, `libc`, `rustix`, `smallvec`, `signals_receipts`. No `pane`, no `par`.

**Reference material:** all vendored. rc paper + man page at `refs/plan9/papers/rc.ms` / `refs/plan9/man/1/rc`. ksh93 manpage at `refs/ksh93/sh.1`. ksh93 interpreter analysis at `refs/ksh93/ksh93-analysis.md`. sfio analysis suite (14 files) at `refs/ksh93/sfio-analysis/`. Theoretical papers at `/Users/lane/gist/`.

**Agent ecosystem:** six specialized design agents under `.claude/agents/` (plan9-systems-engineer, psh-session-type-agent, psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect). Operational workflow at `docs/agent-workflow.md`. Memory discipline at serena `policy/memory_discipline`.

**Serena memory store:** seeded 2026-04-10 with `policy/memory_discipline` (project's canonical MemX-serena principles, edited in place), this `status`, decision memos for load-bearing design choices, reference pointers for vendored material and theoretical papers. Agent sub-namespaces (`agent/<agent-name>/`) are initially empty and populate as agents work.

## What's next

**Immediate:** VDC reframing pass on `docs/specification.md` — restructure the spec so the Virtual Double Category framework is the top-level presentation (currently organized around sequent calculus with duploids/CBPV/optics as supporting apparatus). The reframing preserves every resolved decision but reorganizes. Handoff memo for this work is in Lane's possession, not committed.

**After reframing:** implementation Phase 1 (parser). Ten-phase roadmap in `PLAN.md` §"Implementation roadmap": parser → AST and value model → evaluator → control flow and error model → coprocesses → primitive type methods → Map type → job control and interactive → spec conformance tests → polish.

## Known open questions

None blocking. The deferred-to-v2 items in `PLAN.md` §"Open items from deliberations.md" are deliberate non-decisions:

- Parametric polymorphism (the `type` keyword is reserved)
- User-defined `enum` types (built-in sum tags suffice for v1)
- Named struct construction (positional-only is permanent, not deferred)
- Pattern guards (workaround: `if` inside arm body)
- Session types on pipes (pipes remain byte streams; the VDC framework accommodates the future extension)
- Pipeline fusion (Segal-condition optimization; not a correctness requirement)
