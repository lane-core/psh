---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [memx, sun, memory, retrieval, diskann, fts5, reciprocal-rank-fusion, frontmatter, knowledge-management, local-first, hybrid-search]
agents: [plan9-systems-engineer, psh-session-type-agent, psh-optics-theorist, vdc-theory, psh-sequent-calculus, psh-architect]
related: [policy/memory_discipline]
---

# Reference: MemX (Sun 2026)

**Path.** `/Users/lane/gist/memx/memx.tex`

**Citation.** Sun (2026), *MemX: A Local-First Long-Term Memory System for AI Assistants*.

**Status.** Framework reference for knowledge management. The theoretical source behind psh's serena memory discipline.

## Summary

MemX is a local-first hybrid retrieval system for AI assistant memory. Key mechanisms:

- **Vector search** (DiskANN over dense embeddings) + **keyword search** (FTS5 full-text) run in parallel
- **Reciprocal Rank Fusion** (k=60) fuses the ranked lists
- **Four-factor reranking**: semantic 0.45, recency 0.25 (30-day half-life), frequency 0.05 (log-normalized), importance 0.10
- **Low-confidence rejection**: return ∅ when both the keyword set is empty AND vector similarity falls below τ=0.50
- **Link graph** with seven relation types (similar, related, contradicts, extends, supersedes, caused_by, temporal) — data model only, not yet search-integrated in the published system
- **One store per user** — the unit of consolidation

**Load-bearing empirical finding:** semantic density per record is the primary driver of retrieval quality at scale. Fact-level chunking doubles Hit@5 versus session-level chunking on LongMemEval. **Second:** deduplication is data-dependent — improves recall on tagged template data, hurts recall on tag-free atomic facts.

## Concepts it informs in psh

The MemX principles are ported to serena's simpler name-based retrieval architecture in **`policy/memory_discipline`** (the authoritative project memory). That memory is the source Lane and the psh agents read for memory organization discipline. The MemX paper itself is the theoretical ground.

Specifically:

- **Fact-level granularity** (principle 1) — the empirical finding applied to serena memory structure.
- **Frontmatter as manual reranker** (principle 2) — replaces MemX's automatic four-factor scoring.
- **Query-organized index** (principle 3) — replaces MemX's runtime reranking with design-time organization.
- **Low-confidence rejection** (principle 8) — agent discipline rather than algorithmic filter.
- **One store per project** (ported from "one store per user") — the unit of consolidation, adapted for multi-agent project settings.
- **Hub-and-spokes pattern** — organizational discipline that remains useful even when automatic retrieval is absent.

## Who consults it

- **All agents** (the principles are universal), but direct reading of the paper is only needed when the agent workflow doc (`docs/agent-workflow.md` or `policy/memory_discipline`) itself is being revised. For day-to-day use, consult `policy/memory_discipline` which contains the ported principles.

## Location note

The paper lives at `/Users/lane/gist/memx/memx.tex`, **not** `refs/memx.tex`. Two files accompany it: `references.bib` and figures. Lane maintains `/Users/lane/memx-serena.md` as the authoring copy of the psh-specific port; `policy/memory_discipline` is synced from that file.
