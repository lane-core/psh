---
type: policy
status: current
created: 2026-04-12
last_updated: 2026-04-12
importance: high
keywords: [citation, documentation, bibliography, cite-lint, semantic audit, epistemic strength]
agents: [all]
related: [policy/memory_discipline]
---

# Citation Workflow Policy

Process and rationale for code documentation with citations in psh.
Practical rules are in STYLEGUIDE.md §Theoretical Citations, §Citation format, §Citation hygiene.
This policy document covers the WHY and the full procedure.

## Three obligations

Code that realizes ideas from published work owes the reader:
1. **Credit** to the authors whose ideas the implementation draws from.
2. **Traceability** — follow the citation back to understand WHY.
3. **Verifiability** — check whether the implementation faithfully realizes the cited construct.

## Six principles

1. **Module is the primary documentation unit.** Module docs establish the frame; function docs fill in detail.
2. **Document the role before the mechanism.** What it's FOR before how it works.
3. **Cite what you drew from.** Module-level for architecture, function-level for specific dependence.
4. **Credit authors by name.** Short key `[AuthorYY]` resolving to `docs/spec/references.md`.
5. **Citations follow the implementation.** Move with code, delete with code.
6. **Epistemic strength matches the source.** Hedge when the source hedges. Never invent.

## Bibliography

Central bibliography: `docs/spec/references.md` (formerly `docs/citations.md`).
- Canonical key: author initials + two-digit year (`[CH00]`, `[MMM]`).
- ACM-style entries. No local filesystem paths.
- Every entry has an annotation naming what the project draws from it.
- `NEEDS BACKFILL` flags on unverified metadata. Code should not cite NEEDS BACKFILL entries.

## Two-step discipline

1. **Mechanical (cite-lint):** `tools/cite-lint.sh` checks key resolution, unused entries, alias conflicts. Unconditional disclaimer on every run — green ≠ correct.
2. **Semantic (reviewer):** Re-read the cited reference, verify the citation still describes the code. Reviewer comment on PR is the evidence.

The linter catches trivial breakage. The reviewer catches substantive drift. The second step is the one that matters.

## Heritage vs theoretical citations

Two disciplines, do not cross them:
- Heritage (rc, ksh93, Plan 9): `path:line` form. Verified against source repo.
- Theoretical (papers): `[Key]` form. Verified against bibliography.
- Internal design docs: `docs/spec/chapter.md §Section` form.

## Anti-patterns

- Citations as ornament (background reading, not specific dependence)
- Stale citations kept because removing feels wrong
- Bibliography entries without annotations
- Local filesystem paths in the public bibliography
- Linter disclaimers that can be silenced
- Treating green CI as evidence of citation correctness
- Citing references nobody on the project has read

## Semantic audit

Per-change: reviewer re-reads cited reference, confirms citation accuracy, leaves explicit comment.
Periodic: auditor sweeps all citations. Verdict per citation: CLEAN, MINOR, MAJOR.
