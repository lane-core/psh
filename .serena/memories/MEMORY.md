---
type: index
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [index, memory-index, landing-page, query-organized, hub, navigation]
agents: [plan9-systems-engineer, psh-session-type-agent, psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect]
---

# psh memory index

This is the top-level project index — itself a hub per `policy/memory_discipline` §"Hubs reference, they do not contain". It orients agents to query categories and points at memories with one-line hooks. It does **not** contain memo content.

**Sources of truth.** `docs/specification.md` is tier 1 (the spec wins). `docs/deliberations.md` is tier 2 (decision history). `docs/vdc-framework.md` / `refs/ksh93/ksh93-analysis.md` / `docs/implementation.md` are tier 3 (framework). Serena memories (below) are tier 4 (project-shared knowledge base). Vendored papers at `/Users/lane/gist/` are tier 5.

## Start here (every session)

- [status](status) — where psh is now, what's next, known open questions
- [policy/memory_discipline](policy/memory_discipline) — frontmatter, namespaces, hub-and-spokes, per-type schemas, archive, migration, staleness prevention. Synced from `/Users/lane/memx-serena.md`. **Read once per session.**
- `docs/agent-workflow.md` — operational protocol (not a serena memory): pre-task retrieval, memo format, scope handoff, self-review. Read once per session.

## When you need to look up a resolved design decision

All decisions live under `decision/`. These are short records: Decision / Why / Consequences. Use them when you need the quick answer; drill into `docs/specification.md` for the full treatment.

Foundational:
- [decision/every_variable_is_a_list](decision/every_variable_is_a_list) — Duff's principle extended across the type system
- [decision/let_is_mu_tilde_binder_cbpv](decision/let_is_mu_tilde_binder_cbpv) — `let` binds `F(A)` effectful computations (μ̃-binder)
- [decision/def_vs_lambda](decision/def_vs_lambda) — command vs value callable, CBPV F/U surfaced

Syntactic:
- [decision/tagged_construction_uniform](decision/tagged_construction_uniform) — `NAME(args)` for sums, structs, maps
- [decision/three_roles_of_parens](decision/three_roles_of_parens) — list vs tuple vs tagged construction
- [decision/postfix_dot_accessors](decision/postfix_dot_accessors) — `$x .field` with required leading space, per-type namespace
- [decision/struct_positional_only_forever](decision/struct_positional_only_forever) — no named struct construction, ever
- [decision/lambda_syntax](decision/lambda_syntax) — `|x| => expr` / `|x| { block }`
- [decision/backslash_escape_rules](decision/backslash_escape_rules) — literal escapes, `\<whitespace>` as trivia
- [decision/single_quotes_only](decision/single_quotes_only) — no double-quote form
- [decision/arithmetic_in_process](decision/arithmetic_in_process) — `$((...))` as in-process shift
- [decision/parameter_expansion_destructors](decision/parameter_expansion_destructors) — `$#x` length, `$"x` join
- [decision/catch_binding_parenthesized](decision/catch_binding_parenthesized) — `catch (e)` for grammar consistency

Semantic / behavioral:
- [decision/codata_discipline_functions](decision/codata_discipline_functions) — `.get`/`.set` as codata observer/constructor (supersedes conservative model, commit 7afc97d)
- [decision/try_catch_scoped_errort](decision/try_catch_scoped_errort) — `try` as ErrorT with `;ₜ` sequencer (⊕ discipline)
- [decision/unified_trap_three_forms](decision/unified_trap_three_forms) — lexical / global / deletion trap forms (⅋ discipline)
- [decision/coprocess_9p_discipline](decision/coprocess_9p_discipline) — 9P-shaped, per-tag binary sessions, star topology

## When classifying a new feature as monadic / comonadic / boundary-crossing

- `docs/vdc-framework.md` §8.5 — canonical decision procedure. **First stop.**
- [reference/papers/duploids](reference/papers/duploids) — theoretical ground (Mangel-Melliès-Munch-Maccagnoni). For theory agents only.
- [reference/papers/fcmonads](reference/papers/fcmonads) — VDC mathematical foundation.

## When designing / reviewing a coprocess protocol

- `docs/specification.md` §"Coprocesses" — single source of truth.
- [decision/coprocess_9p_discipline](decision/coprocess_9p_discipline) — quick reference.
- [reference/papers/carbone_forwarders](reference/papers/carbone_forwarders) — justification for star topology.
- [reference/papers/deadlock_free_async_rust](reference/papers/deadlock_free_async_rust) — Rust implementation substrate.
- [reference/papers/multiparty_automata](reference/papers/multiparty_automata) — operational reasoning.

## When reasoning about discipline function semantics

- `docs/specification.md` §"Discipline functions" — authoritative.
- [decision/codata_discipline_functions](decision/codata_discipline_functions) — quick reference + supersession note.
- [reference/papers/profunctor_optics_clarke](reference/papers/profunctor_optics_clarke) — `def:monadiclens` formal definition.
- [reference/papers/duploids](reference/papers/duploids) — CBV focusing as critical pair resolution.
- [reference/sfio_analysis_suite](reference/sfio_analysis_suite) — empirical precedent (ksh93 discipline functions, Dccache).

## When reasoning about polarity frames or the sh.prefix bug class

- `docs/vdc-framework.md` §9.3 — polarity frame discipline as engineering principle.
- [reference/ksh93_analysis](reference/ksh93_analysis) — the save/restore pattern IS the shift; the sh.prefix bugs.
- [reference/sfio_analysis_suite](reference/sfio_analysis_suite) — Dccache as non-associativity witness.
- [reference/papers/duploids](reference/papers/duploids) — (+,−) equation, focusing.
- `docs/specification.md` §"Polarity discipline", §"Discipline functions §Reentrancy and the polarity frame".

## When reasoning about error handling (⊕ / ⅋ duality)

- `docs/specification.md` §"Error model" — authoritative.
- [decision/try_catch_scoped_errort](decision/try_catch_scoped_errort) — ⊕ side.
- [decision/unified_trap_three_forms](decision/unified_trap_three_forms) — ⅋ side.
- [reference/papers/grokking_sequent_calculus](reference/papers/grokking_sequent_calculus) — where ⊕/⅋ duality is introduced.
- [reference/ksh93_analysis](reference/ksh93_analysis) — ksh93 longjmp mode taxonomy.

## When reasoning about rc / ksh93 heritage

- [reference/rc_paper_and_man](reference/rc_paper_and_man) — Duff 1990, foundational.
- [reference/ksh93_manpage](reference/ksh93_manpage) — ksh93 behavior reference.
- [reference/ksh93_analysis](reference/ksh93_analysis) — ksh26 theoretical analysis.
- [reference/sfio_analysis_suite](reference/sfio_analysis_suite) — sfio as implicit type theory.

## When reasoning about the profunctor optics hierarchy

- [reference/papers/dont_fear_profunctor_optics](reference/papers/dont_fear_profunctor_optics) — three-part intuition intro.
- [reference/papers/profunctor_optics_clarke](reference/papers/profunctor_optics_clarke) — formal definitions. Cite `def:monadiclens` etc.
- `docs/specification.md` §"Profunctor structure", §"Extension path" (optics activation table).

## When reasoning about sequent calculus / λμμ̃ / typing rules

- [reference/papers/grokking_sequent_calculus](reference/papers/grokking_sequent_calculus) — first read, most accessible.
- [reference/papers/dissection_of_l](reference/papers/dissection_of_l) — structural reference, piece by piece.
- [reference/papers/duploids](reference/papers/duploids) — categorical semantics.
- [reference/papers/squier_rewriting_hott](reference/papers/squier_rewriting_hott) — critical pair / coherence.
- [reference/papers/linear_logic_without_units](reference/papers/linear_logic_without_units) — unit-free MLL fragment.
- `docs/specification.md` §"The three sorts, made explicit".

## When reasoning about session types on channels

- [reference/papers/carbone_forwarders](reference/papers/carbone_forwarders) — forwarders as cut (star topology justification).
- [reference/papers/multiparty_automata](reference/papers/multiparty_automata) — operational perspective.
- [reference/papers/generalizing_projections](reference/papers/generalizing_projections) — global-to-local projection (future).
- [reference/papers/async_global_protocols](reference/papers/async_global_protocols) — async realizability.
- [reference/papers/dependent_session_types](reference/papers/dependent_session_types) — future extension.
- [reference/papers/practical_refinement_session](reference/papers/practical_refinement_session) — future extension.
- [reference/papers/safe_actor_multiparty](reference/papers/safe_actor_multiparty) — actor-model framing.
- [reference/papers/computational_complexity_interactive](reference/papers/computational_complexity_interactive) — decidability bounds.

## When the spec and a paper disagree

Spec wins. See `policy/memory_discipline` §"Low-confidence rejection over stretching" and `docs/agent-workflow.md` §"Source ranking" (tier 1 > tier 5).

## When reasoning about memory discipline itself

- [policy/memory_discipline](policy/memory_discipline) — the full ported principles. **Authoritative for all memory layout questions.**
- [reference/papers/memx](reference/papers/memx) — original MemX paper (theoretical source).

## Agent sub-namespaces

Empty at seed time. Each agent populates its own sub-folder as it produces institutional knowledge:

- `agent/plan9-systems-engineer/` — rc/ksh93 heritage notes, sfio cross-references, recurring misconceptions
- `agent/psh-session-type-agent/` — resolved coprocess justifications, theorem citations, async reasoning notes
- `agent/psh-optics-theorist/` — optic classifications, law verifications, MonadicLens composition patterns
- `agent/vdc-theory/` — §8.5 classifications, duploid law interactions, FVDblTT patterns
- `agent/psh-sequent-calculus/` — sort classifications, polarity assignments, critical pair resolutions
- `agent/psh-architect/` — AST module layout, parser module ownership, crate usage patterns, CLOEXEC/polarity-frame Rust patterns

Agents **read** any sub-folder; **write** only their own. To record cross-agent supersession, write a memo in your own folder with `supersedes:` / `contradicts:` frontmatter pointing at the other agent's memory.
