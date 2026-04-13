# Agent workflow

Op workflow for psh design agents (plan9, session type, optics, VDC theory, sequent calculus, psh-architect). Each agent reads once per session. Covers pre-task retrieval, memo format, scope handoff, self-review.

Memory org discipline — frontmatter, namespaces, hub-and-spokes, per-type schemas, migration rules — separate concern. Canonical source: **the serena memory `policy/memory_discipline`**, Lane's port of MemX paper to serena. Read once per session for memory layout discipline.

This doc and `policy/memory_discipline` disagree → `policy/memory_discipline` wins on memory org. Either disagrees with `docs/spec/` → spec wins on psh semantics.

## The seven operational principles

From Sun (2026), *MemX: A Local-First Long-Term Memory System for AI Assistants* (`/Users/lane/gist/memx/memx.tex`), via `policy/memory_discipline`. That memory has nine-principle org-focused version; these seven govern how agent *uses* memory during task.

1. **Hybrid retrieval.** Every topic: run semantic query (natural-language description) and keyword query (specific identifiers, section titles, names) via Grep. Combine. Either alone misses material.

2. **Access vs retrieval.** Cite only refs that actually informed answer. List consulted-but-not-applicable separately. No citation padding.

3. **Low-confidence rejection.** Search finds nothing relevant → say so. "No prior material on this topic" beats stretched citation. Return null honestly.

4. **Supersession tracking.** New decisions state what they supersede, extend, refine, or contradict — with file and section — or "none — new topic." Use those four verbs on explicit relationship line so future searches follow chain.

5. **Source ranking.** Multiple sources on same topic, rank:

   `docs/spec/` > `docs/vdc-framework.md` and
   `refs/ksh93/ksh93-analysis.md` (framework) > serena memory >
   vendored papers at `/Users/lane/gist/`.

   Recent beats old at same rank. Don't reconcile across ranks — follow higher rank, flag conflict.

6. **Scope boundaries.** Each agent has scope. Out-of-scope → hand off to named agent, not answered from tangential refs.

7. **Deduplication.** Before writing new analysis, search existing material in serena (list memories, read matches) and your own `agent/<your-name>/` sub-namespace. Extend or refine, don't duplicate.

## Knowledge tiers

Knowledge in psh: four tiers. Higher overrides lower. Conflict → follow higher tier, flag to Lane.

| Tier | Location | Authority |
|---|---|---|
| 1 | `docs/spec/` | single source of truth |
| 2 | `docs/vdc-framework.md`, `refs/ksh93/ksh93-analysis.md`, `docs/implementation.md` | framework |
| 3 | serena memory store | project-shared knowledge base |
| 4 | vendored papers at `/Users/lane/gist/` | original literature |

**Serena: one store per project, shared by all agents.** Every agent reads whole store; writes only to own `agent/<agent-name>/` sub-namespace or project-level types (`policy/`, `decision/`, `architecture/`, `analysis/`, `reference/`) when content is multi-agent. **No parallel per-tool memory layer** — `.claude/agent-memory/` directory doesn't exist. Agent-private institutional knowledge lives in serena, agent's sub-namespace. Full discipline in the serena memory `policy/memory_discipline` §"Agents and the project store".

## Pre-task retrieval protocol

Before any substantive analysis:

1. **Read spec.** Read `docs/spec/` section(s) touching topic. Topic absent from spec → say so explicitly — spec's silence is meaningful.

2. **Query serena.** Call `mcp__serena__list_memories` (optional topic filter); read relevant matches with `mcp__serena__read_memory`. Start with `_index`. Check `agent/<your-name>/` for own prior notes, check other agents' sub-namespaces when question crosses scopes. **On each read, check frontmatter:** `status: superseded` → follow `superseded_by` and read that instead; `status: needs_verification` → treat claims as provisional, re-verify before citing.

4. **Hybrid search across refs.** Each topic: run semantic query (natural language) and keyword query (Grep against specific identifiers). Rust source files when they exist → prefer Serena's `get_symbols_overview` and `find_symbol` over reading whole files.

5. **State findings.** Before answering, list what consulted and what was relevant. Mark consulted-but-not-used separately.

## Memo output format

Every new analysis/classification/decision memo includes these fields. Short-form answers may collapse to single paragraph but keep supersession and confidence lines.

```
## <topic>

**Scope check:** <in scope, or name the right agent and stop>

**Supersedes / extends / refines / contradicts:**
  <prior entry, with file and section>, or
  "none — new topic"

**Consulted (used):**
- <ref 1> — <specific section> — <how it bore on the answer>
- <ref 2> — ...

**Consulted (not applicable):**
- <ref 3> — <one-line reason it didn't fit>

**Analysis:** <reasoning with citations inline>

**Confidence:** <what you verified, what you inferred, what's a
guess — same sentence as the claim>

**Open questions / gaps:** <what sources don't cover>
```

Memos in serena use per-type schema from the serena memory `policy/memory_discipline` §"Per-type memory schemas". Analysis clusters with 4+ related memos → **hub-and-spokes** pattern: hubs reference but don't contain spoke content; hub name `_hub.md`; spokes cite hub via `extends: <hub_path>` in frontmatter.

## Scope handoff protocol

Query outside your scope:

1. State out of scope.
2. Name specific agent that should handle it (see your "Out of scope" charter section).
3. Small in-scope portion exists → answer that, defer rest.
4. Don't speculate outside scope from refs you're not equipped to interpret.

Cross-agent overlap defaults:

- "Is this feature monadic, comonadic, or boundary-crossing?" → VDC theory agent (§8.5 decision procedure), with typing-rule verification from sequent calculus.
- "What optic class is this accessor?" → optics agent.
- "Is this typing rule sound?" → sequent calculus agent.
- "Does this preserve rc's principles?" → plan9 agent.
- "Does this protocol preserve deadlock freedom?" → session type agent.
- "How should this be built in Rust?" → psh architect.

## Self-review before responding

Before emitting non-trivial output, check:

- [ ] Read spec section for this topic?
- [ ] Used both semantic and keyword retrieval?
- [ ] Checked serena (`_index`, my agent sub-namespace, relevant analysis hubs)?
- [ ] Cited only refs that actually informed answer?
- [ ] Marked consulted-but-not-used refs?
- [ ] Stated supersession relationship (or "none")?
- [ ] Stated confidence and gaps explicitly?
- [ ] Stayed in scope, or handed off to right agent?
- [ ] Avoided fabricating from tangential refs?
- [ ] If wrote to serena: populated frontmatter, picked right namespace, honored three-member minimum, respected hub-and-spokes?

Unchecked box → revise before emitting.

## Intermediate state principle

Planning/executing multi-step work — new features, refactorings, extractions, migrations — each intermediate state must be natural resting point.

**Litmus test:** would anyone design intermediate state on purpose? If not — exists only as waypoint between two coherent designs — combine steps. Smaller diff not inherently safer. Incoherent intermediate creates cross-boundary coherence obligations that exist only transiently, strictly harder to reason about than either before-state or after-state.

**Phasing wins** when each intermediate is plausible resting place — config someone would ship. Protocol stack where step 1 = framing (useful without correlation). Validation library where step 1 = email (useful standalone).

**Phasing is trap** when intermediate creates coherence obligation neither before-state nor after-state requires. Building counter before collection it counts. Implementing error types before operations that produce errors. Extracting type before functions that return it.

**Application to agent work:**

1. Before proposing phased plan, write what system looks like after each phase — not what changed, what EXISTS.
2. Each intermediate: "If we stopped here permanently, would this be reasonable design?" If "no, but next phase fixes it" — merge phases.
3. Agents disagree on ordering → this principle is tiebreaker. Coherent intermediates win over smaller diffs.
4. Exception: combined step too large to review/test → splitting justified — but document intermediate as intentionally transient, land both steps together.

## Deliberation rounds

Design decisions needing multiple agents (e.g., new construct needing VDC theory classification, sequent calculus typing rules, psh architect implementation) → Lane dispatches agents in parallel. Each agent follows this workflow independently, produces memo. Lane synthesizes, records resolution in `docs/spec/`.

Agent reviewing another agent's memo → same memo format, supersession line pointing at memo under review.

## Memory-writing quick reference

Task produces memo worth persisting → consult `policy/memory_discipline` for full rules. Short form:

- **Namespace.** Project-level memos → `policy/`, `decision/`, `architecture/`, `analysis/<topic>/`, `reference/`. Agent-private notes → `agent/<your-name>/`. Status is top-level singleton `status.md`.
- **Frontmatter.** Every indexed memo populates YAML block: `type`, `status: current|needs_verification|superseded|archived`, `created`, `last_updated`, `importance`, `keywords`, `agents`, plus link fields (`supersedes`, `superseded_by`, `related`, `extends`, `contradicts`) where relevant. Optional: `sources:` (memos merged in, more granular than supersedes) and `verified_against:` (external sources checked at write time, e.g. `PLAN.md@HEAD`, `commits-since-2026-04-06`).
- **Re-read sources at write time.** Drafted memo during planning → re-read every source immediately before writing. Status/architecture memos → verify external sources (`PLAN.md`, `docs/spec/`, `git log`, code), record what checked in `verified_against:`. Set `last_updated` to merge time, not plan time.
- **Hub-and-spokes.** 4+ related memos → `_hub.md` that orients and points but doesn't contain spoke content. Below 3 → flatten with name prefix.
- **Supersession.** New memo replaces old → note in new memo's frontmatter; delete old or flip `status: archived`, move to `archive/<original-path>_<date>.md`. New memo marked `needs_verification` = soft supersession — readers see flag, know to re-verify.
- **Write only to your own folder** under `agent/<your-name>/`, plus project-level types when content is multi-agent. Don't write to another agent's sub-folder.

## Tier-2 audit for theoretical anchors

**Mandatory** before any new `analysis/<concept>.md` memo (theoretical concept anchor) citing external papers, framework sections, or vendored refs is treated as authoritative for cross-agent retrieval.

Pre-task retrieval checks you read right material *before* writing. Tier-2 audit checks memo's *paraphrases of cited material* are faithful to source *after* writing. Two are independent; need both.

**Why mandatory.** During 2026-04-11 tier-1 anchor batch, 13 anchors written by single writing agent. Manual spot-check caught 3 hallucinations; follow-up dispatch of 5 domain agents caught 6 more — 1:2 ratio self-caught to agent-caught, even with writer trying to be careful. Catching errors at write time is unreliable; catching at audit time, by different agent reading sources fresh, is practical floor on accuracy.

**Procedure.**

1. Write anchors. Cite refs and §pointers as you go.
2. Identify domain agent(s) whose scope covers each anchor:

   | Anchor topic | Auditor |
   |---|---|
   | duploids, VDCs, composition laws, decision procedure, oblique maps | vdc-theory |
   | sequent calculus, focusing, three sorts, shifts, μ/μ̃ binders | psh-sequent-calculus |
   | profunctor optics, MonadicLens, accessors | psh-optics-theorist |
   | session types, multiparty, coprocess channels, wire format | psh-session-type-agent |
   | rc heritage, ksh93 internals, sfio | plan9-systems-engineer |
   | Rust implementation claims | psh-architect |

3. Dispatch auditor agents in parallel. Each gets brief listing anchors it owns and verification rules:

   - Every §pointer or line number is real, points to material on claimed topic.
   - Every substantive claim traces to specific passage in cited ref.
   - Epistemic strength matches source (per the serena memory `policy/memory_discipline` §10).
   - No invented narrative, symptoms, examples, or details.
   - Output: per-anchor verdict (CLEAN / MINOR / MAJOR) with quotes from source vs anchor when divergent.

4. Fold corrections. Update each corrected anchor's `verified_against:` frontmatter with audit date and verifying agent. Bump `last_updated:` to merge time.

5. Only after audit pass and folded corrections may anchors be marked authoritative for cross-agent retrieval.

**Out of scope for audit.** Upstream issues found in spec or framework docs (e.g., typo in `docs/spec/`) → flagged in audit report, routed to Lane for separate resolution. Audit fixes anchors; spec gets own review pass.

**Skipping audit.** Permitted only for tier-3 anchors, short corrections without new citations, or anchors citing only psh's own materials (`docs/spec/`, decision memos). Any new external paper citation triggers audit on next pass.

## Source

This workflow derives from Sun (2026), *MemX: A Local-First Long-Term Memory System for AI Assistants*, at `/Users/lane/gist/memx/memx.tex`. psh-specific porting of MemX principles to serena memory layout in the serena memory `policy/memory_discipline`, authoritative for memory organization.