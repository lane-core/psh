# Agent workflow

Operational workflow for psh's specialized design agents (plan9,
session type, optics, VDC theory, sequent calculus, psh-architect).
Every agent reads this once per session. Covers pre-task retrieval,
memo output format, scope handoff, and self-review.

Memory organization discipline — frontmatter, namespaces,
hub-and-spokes, per-type schemas, migration rules — is a separate
concern. The canonical source is **the serena memory `policy/memory_discipline`**,
Lane's port of the MemX paper's principles to serena. Read it once
per session for memory layout discipline.

When this document and `policy/memory_discipline` disagree,
`policy/memory_discipline` wins on memory organization. When either
disagrees with `docs/spec/`, the spec wins on psh semantics.

## The seven operational principles

From Sun (2026), *MemX: A Local-First Long-Term Memory System for
AI Assistants* (`/Users/lane/gist/memx/memx.tex`), via
`policy/memory_discipline`. That memory has the nine-principle
organization-focused version; these seven apply to how an agent
*uses* memory during a task.

1. **Hybrid retrieval.** For every topic, run both a semantic
   query (natural-language description of what you're looking for)
   and a keyword query (specific identifiers, section titles,
   names) via Grep. Combine results. Either alone misses material.

2. **Access vs retrieval.** Cite only references that actually
   informed the answer. List consulted-but-not-applicable
   references separately. Don't pad citations.

3. **Low-confidence rejection.** If you search and find nothing
   directly relevant, say so. "No prior material on this topic"
   beats a stretched citation. Return null results honestly.

4. **Supersession tracking.** New decisions state what they
   supersede, extend, refine, or contradict — with file and
   section — or "none — new topic." Use those four verbs on an
   explicit relationship line so future searches can follow the
   chain.

5. **Source ranking.** When multiple sources discuss the same
   topic, rank them:

   `docs/spec/` > `docs/vdc-framework.md` and
   `refs/ksh93/ksh93-analysis.md` (framework) > serena memory >
   vendored papers at `/Users/lane/gist/`.

   Recent beats old at the same rank. Do not reconcile across
   ranks — follow the higher rank and flag the conflict.

6. **Scope boundaries.** Each agent has a scope. Out-of-scope
   queries get handed off to the named agent, not answered from
   tangential references.

7. **Deduplication.** Before writing a new analysis, search for
   existing material in serena (list memories, read matches)
   and your own `agent/<your-name>/`
   sub-namespace. Extend or refine rather than duplicating.

## Knowledge tiers

Knowledge in psh lives at four tiers. Higher tiers override lower.
In conflict, follow the higher tier and flag the conflict to Lane.

| Tier | Location | Authority |
|---|---|---|
| 1 | `docs/spec/` | single source of truth |
| 2 | `docs/vdc-framework.md`, `refs/ksh93/ksh93-analysis.md`, `docs/implementation.md` | framework |
| 3 | serena memory store | project-shared knowledge base |
| 4 | vendored papers at `/Users/lane/gist/` | original literature |

**Serena is one store per project, shared by all agents.** Every
agent reads the whole store; every agent writes only to its own
`agent/<agent-name>/` sub-namespace or to project-level types
(`policy/`, `decision/`, `architecture/`, `analysis/`, `reference/`)
when the content is multi-agent. **There is no parallel per-tool
memory layer** — the `.claude/agent-memory/` directory does not
exist. Agent-private institutional knowledge lives inside serena,
in the agent's sub-namespace. Full discipline in
the serena memory `policy/memory_discipline` §"Agents and the project store".

## Pre-task retrieval protocol

Before any substantive analysis:

1. **Read the spec.** Read `docs/spec/` section(s)
   touching the topic. If the topic is absent from the spec, say
   so explicitly — the spec's silence is meaningful.

2. **Query serena.** Call `mcp__serena__list_memories` (optional
   topic filter); read relevant matches with
   `mcp__serena__read_memory`. Start with `_index`. Check
   `agent/<your-name>/` for your own prior notes, and check other
   agents' sub-namespaces when the question crosses scopes. **On
   each read, check frontmatter:** `status: superseded` → follow
   `superseded_by` and read that instead; `status:
   needs_verification` → treat the claims as provisional and
   re-verify before citing.

4. **Hybrid search across references.** For each topic, run a
   semantic query (natural language) and a keyword query (Grep
   against specific identifiers). For Rust source files when they
   exist, prefer Serena's `get_symbols_overview` and `find_symbol`
   over reading whole files.

5. **State what you found.** Before answering, list what you
   consulted and what was actually relevant. Mark
   consulted-but-not-used separately.

## Memo output format

Every new analysis, classification, or decision memo includes
these fields. Short-form answers may collapse to a single
paragraph but keep the supersession and confidence lines.

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

Memos written into serena use the per-type schema from
the serena memory `policy/memory_discipline` §"Per-type memory schemas". For
analysis clusters with 4+ related memos, apply the **hub-and-
spokes** pattern: hubs reference but do not contain spoke content;
hub name is `_hub.md`; spokes cite the hub via `extends:
<hub_path>` in frontmatter.

## Scope handoff protocol

When a query falls outside your scope:

1. State that it's out of scope for you.
2. Name the specific agent that should handle it (see your "Out of
   scope" charter section).
3. If a small in-scope portion exists, answer that portion and
   defer the rest.
4. Do not speculate outside your scope from references you're not
   equipped to interpret.

Cross-agent overlaps default as follows:

- "Is this feature monadic, comonadic, or boundary-crossing?" →
  VDC theory agent (§8.5 decision procedure), with typing-rule
  verification from sequent calculus.
- "What optic class is this accessor?" → optics agent.
- "Is this typing rule sound?" → sequent calculus agent.
- "Does this preserve rc's principles?" → plan9 agent.
- "Does this protocol preserve deadlock freedom?" → session type
  agent.
- "How should this be built in Rust?" → psh architect.

## Self-review before responding

Before emitting any non-trivial output, check:

- [ ] Did I read the spec section for this topic?
- [ ] Did I use both semantic and keyword retrieval?
- [ ] Did I check serena (`_index`, my agent sub-namespace, any
      relevant analysis hubs)?
- [ ] Did I cite only references that actually informed the
      answer?
- [ ] Did I mark consulted-but-not-used references?
- [ ] Did I state the supersession relationship (or "none")?
- [ ] Did I state confidence and gaps explicitly?
- [ ] Did I stay in scope, or hand off to the right agent?
- [ ] Did I avoid fabricating from tangential references?
- [ ] If I wrote to serena: did I populate frontmatter, pick the
      right namespace, honor the three-member minimum, and respect
      the hub-and-spokes pattern?

Any unchecked box is a signal to revise before emitting.

## Intermediate state principle

When planning or executing multi-step work — new features,
refactorings, extractions, migrations — each intermediate state
must be a natural resting point.

**Litmus test:** would anyone design the intermediate state on
purpose? If not — if it exists only as a waypoint between two
coherent designs — combine the steps. A smaller diff is not
inherently safer. An incoherent intermediate creates
cross-boundary coherence obligations that exist only
transiently, and those are strictly harder to reason about than
either the before-state or the after-state.

**Phasing wins** when each intermediate is a plausible resting
place — a configuration someone would ship. A protocol stack
where step 1 = framing (useful without correlation). A
validation library where step 1 = email (useful standalone).

**Phasing is a trap** when an intermediate creates a coherence
obligation that neither the before-state nor the after-state
requires. Building a counter before the collection it counts.
Implementing error types before the operations that produce
errors. Extracting a type before the functions that return it.

**Application to agent work:**

1. Before proposing a phased plan, write what the system looks
   like after each phase — not what changed, what EXISTS.
2. For each intermediate: "If we stopped here permanently, would
   this be a reasonable design?" If "no, but the next phase
   fixes it" — merge the phases.
3. When agents disagree on ordering, this principle is the
   tiebreaker. Coherent intermediates win over smaller diffs.
4. Exception: if a combined step is too large to review or test,
   splitting is justified — but document the intermediate as
   intentionally transient and land both steps together.

## Deliberation rounds

For design decisions requiring multiple agents (e.g., a new
construct whose classification needs VDC theory, whose typing
rules need sequent calculus, and whose implementation needs psh
architect), Lane dispatches the agents in parallel. Each agent
follows this workflow independently and produces a memo. Lane
synthesizes results and records the resolution in
`docs/spec/`.

When an agent reviews another agent's memo, the review uses the
same memo format with the supersession line pointing at the memo
under review.

## Memory-writing quick reference

When the task produces a memo worth persisting, consult
`policy/memory_discipline` for the full rules. Short form:

- **Namespace.** Project-level memos go to `policy/`, `decision/`,
  `architecture/`, `analysis/<topic>/`, `reference/`. Agent-private
  notes go to `agent/<your-name>/`. Status is the top-level
  singleton `status.md`.
- **Frontmatter.** Every indexed memo populates the YAML block:
  `type`, `status: current|needs_verification|superseded|archived`,
  `created`, `last_updated`, `importance`, `keywords`, `agents`,
  plus link fields (`supersedes`, `superseded_by`, `related`,
  `extends`, `contradicts`) where relevant. Optional:
  `sources:` (memos merged in, more granular than supersedes) and
  `verified_against:` (external sources checked at write time,
  e.g. `PLAN.md@HEAD`, `commits-since-2026-04-06`).
- **Re-read sources at write time.** If you drafted a memo during
  planning, re-read every source immediately before writing. For
  status / architecture memos, verify external sources
  (`PLAN.md`, `docs/spec/`, `git log`, code) and
  record what you checked in `verified_against:`. Set
  `last_updated` to merge time, not plan time.
- **Hub-and-spokes.** Clusters of 4+ related memos get a `_hub.md`
  that orients and points but does not contain spoke content.
  Below 3, flatten with a name prefix.
- **Supersession.** New memo replaces old → note it in the new
  memo's frontmatter; delete the old memo or flip its `status:
  archived` and move it to `archive/<original-path>_<date>.md`.
  A new memo marked `needs_verification` is a soft supersession —
  readers see the flag and know to re-verify.
- **Write only to your own folder** under `agent/<your-name>/`,
  plus project-level types when content is multi-agent. Do not
  write to another agent's sub-folder.

## Tier-2 audit for theoretical anchors

**Mandatory** before any new `analysis/<concept>.md` memo
(theoretical concept anchor) that cites external papers, framework
sections, or vendored references is treated as authoritative for
cross-agent retrieval.

The pre-task retrieval protocol checks that you read the right
material *before* writing. The tier-2 audit checks that the
memo's *paraphrases of cited material* are faithful to the source
*after* writing. The two are independent; you need both.

**Why mandatory.** During the 2026-04-11 tier-1 anchor batch,
13 anchors were written by a single writing agent. A manual
spot-check caught 3 hallucinations; a follow-up dispatch of 5
domain agents caught 6 more — a 1:2 ratio of self-caught to
agent-caught, even with the writer trying to be careful. The
discipline of catching errors at write time is unreliable; the
discipline of catching them at audit time, by a different agent
reading the sources fresh, is the practical floor on accuracy.

**Procedure.**

1. Write the anchors. Cite refs and §pointers as you go.
2. Identify the domain agent(s) whose scope covers each anchor:

   | Anchor topic | Auditor |
   |---|---|
   | duploids, VDCs, composition laws, decision procedure, oblique maps | vdc-theory |
   | sequent calculus, focusing, three sorts, shifts, μ/μ̃ binders | psh-sequent-calculus |
   | profunctor optics, MonadicLens, accessors | psh-optics-theorist |
   | session types, multiparty, coprocess channels, wire format | psh-session-type-agent |
   | rc heritage, ksh93 internals, sfio | plan9-systems-engineer |
   | Rust implementation claims | psh-architect |

3. Dispatch the auditor agents in parallel. Each gets a brief
   listing the anchors it owns and the verification rules:

   - Every §pointer or line number is real and points to material
     on the claimed topic.
   - Every substantive claim traces to a specific passage in the
     cited reference.
   - Epistemic strength matches the source (per the serena memory
     `policy/memory_discipline` §10).
   - No invented narrative, symptoms, examples, or details.
   - Output: per-anchor verdict (CLEAN / MINOR / MAJOR) with
     quotes from source vs anchor when divergent.

4. Fold corrections. Update each corrected anchor's
   `verified_against:` frontmatter to record the audit date and
   the agent that verified it. Bump `last_updated:` to merge time.

5. Only after the audit pass and folded corrections may the
   anchors be marked authoritative for cross-agent retrieval.

**Out of scope for the audit.** Upstream issues found in the spec
or framework documents (e.g., a typo or mis-attribution in
`docs/spec/`) are flagged in the audit report and
routed to Lane for separate resolution. The audit fixes the
anchors; the spec gets its own review pass.

**Skipping the audit.** Permitted only for tier-3 anchors, short
corrections that don't introduce new citations, or anchors that
cite only psh's own materials (`docs/spec/`,
decision memos). Any new external paper
citation triggers the audit on the next pass.

## Source

This workflow derives from Sun (2026), *MemX: A Local-First
Long-Term Memory System for AI Assistants*, at
`/Users/lane/gist/memx/memx.tex`. The psh-specific porting of
MemX principles to serena memory layout is in
the serena memory `policy/memory_discipline`, authoritative for memory
organization.
