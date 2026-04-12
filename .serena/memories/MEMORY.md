---
type: index
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [index, memory-index, landing-page, query-organized, hub, navigation, theoretical-concept-anchors, analysis-namespace]
agents: [plan9-systems-engineer, psh-session-type-agent, psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect]
---

# psh memory index

This is the top-level project index — itself a hub per `policy/memory_discipline` §"Hubs reference, they do not contain". It orients agents to query categories and points at memories with one-line hooks. It does **not** contain memo content.

**Sources of truth.** `docs/specification.md` is tier 1 (the spec wins). `docs/vdc-framework.md` / `refs/ksh93/ksh93-analysis.md` / `docs/implementation.md` are tier 2 (framework). Serena memories (below) are tier 3 (project-shared knowledge base). Vendored papers at `/Users/lane/gist/` are tier 4.

## Start here (every session)

- [status](status) — where psh is now, what's next, known open questions
- [policy/memory_discipline](policy/memory_discipline) — frontmatter, namespaces, hub-and-spokes, per-type schemas, archive, migration, staleness prevention, epistemic-strength rule. **Project's canonical source of truth for memory discipline. Read once per session.**
- `docs/agent-workflow.md` — operational protocol (not a serena memory): pre-task retrieval, memo format, scope handoff, self-review. Read once per session.

## Theoretical concept anchors

Concept-named landings into the foundational material. Each is a short anchor (150–300 words) pointing at the canonical reference sections. Drill into the references for depth; come back to the anchor for the keyword landing. These supplement — they do not replace — the query-organized sections below.

### Polarity discipline cluster (hub-and-spokes)

- [analysis/polarity/_hub](analysis/polarity/_hub) — orientation hub. **Read first** for any polarity-related question. Includes both reading orders (theoretical-first / empirical-first).
- [analysis/polarity/frames](analysis/polarity/frames) — save/restore-around-shift engineering principle
- [analysis/polarity/shifts](analysis/polarity/shifts) — ↓/↑ as the operator the frame brackets
- [analysis/polarity/cbv_focusing](analysis/polarity/cbv_focusing) — static focusing as reentrancy semantics
- [analysis/polarity/duploid_composition](analysis/polarity/duploid_composition) — •, ○, cut and the four equations
- [analysis/polarity/plus_minus_failure](analysis/polarity/plus_minus_failure) — the (+,−) equation, named directly
- [analysis/polarity/dccache_witness](analysis/polarity/dccache_witness) — sfio §07 as empirical (+,−) failure
- [analysis/polarity/sh_prefix_critical_pair](analysis/polarity/sh_prefix_critical_pair) — ksh93 bug class as critical-pair manifestation

### Standalone tier-1 anchors

- [analysis/decision_procedure_8_5](analysis/decision_procedure_8_5) — **First stop** for "should psh add feature X." The monadic / comonadic / boundary-crossing classifier from `docs/vdc-framework.md` §8.5.
- [analysis/three_sorts](analysis/three_sorts) — producers Γ / consumers Δ / commands ⟨t|e⟩, plus the AST's four-sort extension (with Mode)
- [analysis/oblique_maps](analysis/oblique_maps) — every shell command is a P→N cross-polarity arrow
- [analysis/monadic_lens](analysis/monadic_lens) — `def:monadiclens` from Clarke et al., Kl(Ψ) structure of discipline functions
- [analysis/wire_format_horizontal_arrow](analysis/wire_format_horizontal_arrow) — coprocess wire format as VDC horizontal arrow

### Tier-2 anchors

- [analysis/forwarders_as_cut](analysis/forwarders_as_cut) — Carbone-Marin-Schürmann; star topology justified by cut elimination at the proof-theoretic level
- [analysis/nine_p_discipline](analysis/nine_p_discipline) — negotiate / req-resp / error / teardown as the conversation shape psh borrows from 9P
- [analysis/error_duality_oplus_par](analysis/error_duality_oplus_par) — ⊕/⅋ as the two disjunctions of linear logic; try (⊕) vs trap (⅋) compose orthogonally
- [analysis/sfio_as_implicit_type_theory](analysis/sfio_as_implicit_type_theory) — the framing claim: sfio is the structure ksh93 built right at I/O and failed to propagate to the shell
- [analysis/cut_as_execution](analysis/cut_as_execution) — ⟨t | e⟩ as the execution rule; pipe = cut; assignment = cut against μ̃-binder
- [analysis/cbpv_f_u_separation](analysis/cbpv_f_u_separation) — Levy CBPV F/U adjunction surfaced as `def`/lambda
- [analysis/data_vs_codata](analysis/data_vs_codata) — constructors vs destructors, pattern vs copattern; data and codata are perfectly dual
- [analysis/squier_critical_pair](analysis/squier_critical_pair) — local-to-global coherence framework (deferred verification — psh has not yet undertaken the local-confluence work Squier consumes)

### Tier-3 anchors

- [analysis/duff_principle](analysis/duff_principle) — every variable is a list; rc heritage; the foundational commitment psh extends across the type system
- [analysis/hasegawa_thielecke](analysis/hasegawa_thielecke) — thunkable = central in dialogue duploids (corrected attribution from Führmann-Thielecke per the tier-1 audit)
- [analysis/tambara_modules](analysis/tambara_modules) — Pastro-Street + Clarke generalized Tambara representation; why optics compose
- [analysis/fvdbltt_protypes](analysis/fvdbltt_protypes) — Hayashi-Das FVDblTT; protypes as channel types (forward-looking, v2)
- [analysis/classical_control_mu_binder](analysis/classical_control_mu_binder) — μ as continuation capture; lexical trap is the operational μ-binder; the trap half of the let/control duality

### Follow-up from 2026-04-11 type-theory investigation

- [analysis/constructor_as_opcartesian_cell](analysis/constructor_as_opcartesian_cell) — struct constructor as opcartesian cell witnessing a composite; universality is load-bearing on `decision/struct_positional_only_forever`

`analysis/shell_t_monolithic_state` was considered for tier 3 but **dropped** as duplicating `analysis/polarity/sh_prefix_critical_pair` (per the merge test in `policy/memory_discipline` §7).

**All three anchor batches (tier 1, tier 2, tier 3) passed mandatory `docs/agent-workflow.md` §"Tier-2 audit for theoretical anchors" verification by domain agents.** Cumulative audit findings: tier 1 caught 3 hallucinations + 6 minor issues; tier 2 caught 1 MAJOR + 6 MINOR; tier 3 caught 0 MAJOR + 5 MINOR. All folded back in.

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
