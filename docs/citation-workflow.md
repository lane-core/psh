# Code Documentation and Citation Workflow

A workflow for projects with theoretical foundations — research
prototypes, systems work drawing on type theory or category
theory, compilers citing formal semantics, any project where the
code realizes ideas from published work. The workflow combines a
code documentation discipline, an annotated bibliography, a
mechanical check, and a semantic audit. Each layer has a narrow
scope and knows what it does not prove.

The workflow is designed for Rust (`//!` module docs and `///`
item docs rendered by `cargo doc`) but the principles apply to
any language with structured documentation comments. Replace
`//!` / `///` with your language's equivalent; keep the rest.

## Overview

Code that realizes ideas from published work owes three things
to the reader of the code:

- **Credit to the authors** whose ideas the implementation
  draws from.
- **Traceability** — a future reader can follow the citation
  back to understand *why* a construct is designed the way it
  is, not only *what* it does.
- **Verifiability** — an auditor can check whether the
  implementation faithfully realizes the cited construct. A
  citation is a testable claim: *this code implements the idea
  described in this reference.*

The workflow is built around these three obligations. Every
mechanism exists to make one of them checkable without imposing
more ceremony than the obligation demands.

## Principles

1. **The module is the primary documentation unit.** Module-level
   docs (`//!`) establish the conceptual frame. Function-level
   docs (`///`) fill in detail within that frame. A reader who
   only reads the module doc should be able to say where the
   module fits in the architecture and what theoretical
   foundations inform it.

2. **Document the role before the mechanism.** What a module or
   function is *for* comes before how it works. "Installs a
   one-shot continuation keyed on token X" comes before "locks
   the hashmap and inserts." A reader who cannot answer "why
   does this exist?" cannot understand the code.

3. **Cite what you drew from.** If a reference informed the
   design of a specific construct, cite it inline with the
   construct. If it informed the broader architecture, cite it
   at the module level. If it is background knowledge that did
   not inform the specific implementation, it belongs in the
   bibliography's annotation, not in the code.

4. **Credit authors by name.** Inline citations use a short key
   (e.g. `[AuthorYY]`) that resolves to a full entry in the
   bibliography. The bibliography carries the full author list
   in the order the source gives. The reader of the code sees
   the key; the reader following the key sees the full
   attribution.

5. **Citations follow the implementation.** When code moves,
   citations move with it. When code is deleted, citations are
   deleted. A citation is an attribute of a specific
   implementation, not a permanent annotation on a concept. The
   bibliography is maintained independently and outlives
   specific code, but the inline citations in code belong to
   whichever lines currently cite them.

6. **Epistemic strength matches the source.** When citing or
   paraphrasing a source, the epistemic strength of the citation
   must not exceed the source. If the source says "structurally
   analogous," the citation does not say "realizes." If the
   source notes "not formally verified," the citation carries
   the caveat. Inventing details the source does not contain is
   the same class of error in the writing direction — it looks
   authoritative because the rest of the documentation is
   well-sourced, but it is a hallucination. Hedge when the
   source hedges.

## The annotated bibliography

The project maintains a central bibliography file (conventional
name: `docs/citations.md`). For each entry:

- **Canonical key.** Short identifier used in code doc comments
  to cite the reference. Author initials plus two-digit year is
  the recommended default form: `[JHK24]`, `[TCP11]`. Single-
  author papers use a short author key: `[Hou07]`, `[Rit]`.
  Entries without resolved author metadata use a descriptive
  acronym and are flagged `NEEDS BACKFILL` until the author
  list can be confirmed against the primary source.
- **Full citation in ACM style** (or the citation style that
  matches the project's community conventions). Authors in the
  order the source gives, exact title, venue, year, volume /
  issue / article / page range, DOI or stable URL.
- **Aliases** (optional). A short-name alias can be declared for
  contexts where it reads more clearly than the author+year
  form. Example: `[JHK24]` with alias `[LinearActris]`. Both
  resolve to the same entry.
- **Annotation.** One paragraph describing what the reference
  contributes to the project — which part of the architecture
  it informs and why it matters. The annotation is the
  substance of the entry. An entry whose annotation does not
  name a specific thing the project draws from the reference
  is dead weight and should be removed.

Organize the entries by canonical key alphabetically. Include a
bibliography-index table at the top as a scannable directory.

**The bibliography is public-facing.** It should use academic
citation format exactly as it would appear in a paper. It must
not use local filesystem paths (`~/gist/...`, absolute home
paths, tmpdir paths). If the project's private knowledge store
needs to record where a reference lives on local disk, that
belongs in the private store, not in the public bibliography.
Publication and reproducibility both suffer if the authoritative
bibliography leaks workstation-specific paths.

**`NEEDS BACKFILL` discipline.** When adding an entry without
verified metadata, flag the missing fields explicitly:

```
### `[AGP]`

Kai Pischke, Jake Masters, and Nobuko Yoshida. *Asynchronous
Global Protocols, Precisely.* *NEEDS BACKFILL: venue, year,
DOI.*
```

Code should not cite `NEEDS BACKFILL` entries until the entry
is resolved. Resolving a marker means reading the primary
source (PDF, preprint, published journal version) and
extracting the metadata exactly as the source gives it.
Principle 6 applies: do not guess. Better to leave the marker
than to hallucinate.

## Module-level documentation

Every significant module gets a module-level doc comment. The
module doc establishes:

- **What this module is.** Its role in the architecture,
  stated in the vocabulary the project's design documents use.
  If the project has a formal structure (layers, sorts,
  phases, tiers), name which one this module belongs to.
- **What design documents it implements.** Direct references
  to spec sections, design docs, or architectural decisions.
  The module doc is the bridge between the design and the
  code.
- **What references informed it.** The theoretical or
  external sources that shaped the module's architecture —
  not every function's specific source, but the references
  that explain *why the module is structured this way.*
- **A `# References` section** at the end of the module doc,
  listing the canonical keys from the central bibliography.
  This gives the reader a focused reading list for
  understanding the module, scoped to what is load-bearing
  here.

**Template:**

```rust
//! # Module name — one-line role description
//!
//! Paragraph explaining the module's place in the architecture.
//! What it does, which architectural layer or phase it belongs
//! to, and how it relates to adjacent modules.
//!
//! Second paragraph on the design approach: what concepts from
//! the project's theoretical foundation are realized here, and
//! how they manifest in the code structure. Cite references at
//! the level of the architectural commitment, not at the level
//! of every function.
//!
//! See `docs/design-doc.md` §Relevant Section for the design
//! rationale.
//!
//! # References
//!
//! - `[Key1]` — what it contributes to this module
//! - `[Key2]` — what it contributes to this module
```

The `# References` section is the module's scoped bibliography.
It repeats the relevant keys with a one-line hook explaining
which aspect of the module each reference informs. Do not copy
the full bibliography entry — the central bibliography has
that.

## Function-level documentation

Function docs are secondary to the module doc. They fill in
specifics within the frame the module doc establishes. Most
function docs will not carry citations at all; only the
functions that draw from a specific reference need one.

**Content:**

- **Lead with the role.** "Evaluates X to produce Y" before
  "calls helper A then helper B." The reader needs the *what*
  before the *how*.
- **State preconditions and postconditions** when they are not
  obvious from the type signature. What the function expects,
  what it guarantees, what it may fail with.
- **Cite only when this specific function draws from a
  reference.** The threshold question: did we draw on this
  reference to construct *this function's* implementation? If
  the reference informed the module's architecture but not
  this function specifically, the module doc handles it. Do
  not repeat module-level citations at the function level
  unless the function has its own specific dependence.
- **Credit authors by name in the inline citation.** Write
  `/// Implements the X rule from [Author90] §3.2` — the reader
  sees the key without leaving the code. Full details are one
  lookup away in the bibliography.
- **Reference the spec or design doc** when the function
  implements a specific design decision, using the repo-
  internal path form: `/// See docs/design-doc.md §Section
  Name.`
- **Preserve hedges.** If the source says "structurally
  analogous," the citation does too.

**Example:**

```rust
/// Installs a one-shot reply continuation keyed on the request
/// token. Consumed by `fire_reply` or `fire_failed`; the entry
/// is removed on fire, realizing the one-shot semantics of the
/// X rule from the calculus.
///
/// Reference: [Author90] §3.2 (X-rule operational semantics).
pub fn insert(...) { ... }
```

## What not to document

- **Do not narrate the code.** If the implementation is clear
  from reading it, do not restate it in prose. Document *why*,
  not *what*. Well-named identifiers already say *what*.
- **Do not document private helpers** unless their behavior is
  surprising or their invariants are non-obvious. The module
  doc covers the module's internal structure at a high level.
  Not every private function needs its own doc comment.
- **Do not cite references you did not draw from.** A citation
  is a claim: "this reference informed this implementation."
  Citing a reference because it is prestigious, or because it
  is "background reading" for the topic, or because the author
  is respected, pollutes the audit trail. Every citation must
  be a testable claim of intellectual debt.
- **Do not cite references you have not read.** Reading an
  abstract does not qualify as reading a reference. If you
  cannot point at the specific section or theorem the
  implementation draws from, you did not draw from it.

## Citation hygiene

Citations are implementation attributes. They require
maintenance.

**When refactoring a function:** check whether its citations
still apply. If the function no longer implements the cited
construct, remove or update the citation. A stale citation is
worse than no citation — it actively misleads.

**When restructuring a module:** update the module-level
`# References` section. Add references the module now draws
from; remove ones it no longer needs.

**When adding a new module or function:** check the
bibliography for relevant references before writing docs. If
your implementation draws on a reference, cite it. If you are
unsure, note `// TODO: citation needs review` and flag for
audit.

**When retiring a module or function:** citations are deleted
with the code. They do not need to be preserved elsewhere. The
citation follows the implementation.

**Periodic audit:** grep for citation keys across the source
tree and verify each one points to a live module or function
that still implements the cited construct. Include this check
in the review process for any major refactor or architectural
change.

## Mechanical audit and its limits

A mechanical linter (conventional name: `cite-lint`) verifies
that every citation key in the code resolves to an entry in the
bibliography. Typical name: `cite-lint` rather than
`cite-audit` — *lint* is the conventional name for mechanical
syntactic checks and makes the scope explicit. *Audit* implies
comprehensive coverage and invites confusion.

**What the linter catches:**

- Typos in citation keys.
- Citations to keys that no longer have a bibliography entry
  (renamed, removed, or never added).
- Unused bibliography entries (entries no code cites).
- Alias conflicts (two entries claiming the same alias).
- Malformed keys.

**What the linter does not catch:**

- **Stale citations on refactored functions.** When a function's
  implementation changes but the citation stays, the citation
  may no longer reflect what the code does. The linter sees
  the citation; it cannot see whether it is accurate.
- **Wrong paper attribution.** A citation that resolves
  syntactically can still cite the wrong paper for the
  implementation claim.
- **Epistemic strength violations.** A citation whose inline
  phrasing strengthens the source beyond what the source
  actually says. Principle 6 is a semantic rule; a mechanical
  check cannot enforce it.
- **Refactor-induced drift of any kind** that preserves
  syntactic validity.

**The linter's output must include a disclaimer on every run.**
Not as an optional verbose flag — as a first-line, unconditional
piece of output that cannot be silenced. Example:

```
cite-lint: 42 keys resolved across 18 files (3 unused entries).

Mechanical check only. Semantic correctness (whether citations
accurately reflect the code they document) is the reviewer's
and auditor's responsibility. Green cite-lint ≠ correct
citations.
```

The disclaimer exists because mechanical tools that answer
easy questions get used as proxies for hard questions. Without
the disclaimer, "green CI" starts to function as evidence of
citation correctness, which it is not. The disclaimer makes
the tool's scope visible on every invocation.

**The linter does not substitute for the semantic audit.** Make
this explicit in the project's style guide, in the linter's own
help text, and in the pull-request review template. Green
`cite-lint` is necessary but not sufficient.

## Semantic audit

The semantic audit is the substance. It is the only mechanism
that catches the failures the linter cannot catch. Two triggers:

1. **Per-change, at review time.** When a pull request touches
   a module or function that has a citation, the reviewer is
   required to re-read the cited reference and verify the
   citation still describes the code accurately. The reviewer
   leaves an explicit comment confirming the check:

   > Citations re-verified against `[Author90]` §3.2 (the X-rule
   > operational semantics match the updated `insert` body).
   > No drift.

   Running the linter does not satisfy this obligation. The
   linter pass is a pre-filter; the reviewer comment is the
   evidence that the semantic check happened.

2. **Periodic, by a dedicated auditor role.** A periodic sweep
   catches drift that per-change review missed — citations
   that were correct when added but drifted as the code
   evolved. The auditor re-reads each cited reference against
   the current implementation claim and produces a verdict:
   CLEAN, MINOR, or MAJOR. MINOR findings are folded as small
   corrections; MAJOR findings trigger a citation rewrite or
   removal.

**The auditor role.** Any project member can be the auditor if
they are willing to re-read primary sources. In practice, a
dedicated role works better than "whoever is around" — the
auditor develops a feel for the project's citation landscape
and can recognize drift faster. Assign the role explicitly and
make the audit cadence part of the project's process (weekly,
monthly, per-release — whatever matches the pace of code
change).

**The two-step discipline, in one sentence.** Run the linter
to catch the trivial breakage; re-read the source to catch the
substantive drift. The first step is fast and the second step
is slow, and the second step is the one that matters. Do not
conflate them.

## Ethical citation practices

The purpose of citing in code documentation is the three
obligations at the top of this document: credit, traceability,
verifiability. Every citation serves all three or it does not
belong. Practical rules that fall out:

- **Do not cite to impress.** Citing prestigious references in
  code that does not actually draw on them is dishonest
  signaling. It also pollutes the audit trail — a future
  reader cannot distinguish citations that matter from
  citations that do not.
- **Do not cite references you have not read.** Reading an
  abstract, a Wikipedia summary, or another paper's citation
  of the reference is not reading the reference. If you cannot
  point at the specific section or theorem, the citation is
  speculative.
- **Do not cite references that did not inform the specific
  implementation being documented.** A citation is a claim of
  intellectual debt specifically for *this implementation*.
  Background reading belongs in the bibliography's annotation
  (which describes what the reference contributes to the
  project overall) not in the code.
- **Make the claim honestly.** Every citation is a testable
  claim. The threshold question at review time is: "could I
  defend this citation against a skeptical reader who asks why
  this reference rather than some other?" If you cannot,
  remove or rephrase.
- **Credit contributors, not only authors.** If an email
  thread or a conference conversation materially shaped an
  implementation decision, a `// Thanks to X for the
  observation that Y` comment is appropriate — but this is
  acknowledgment, not citation. Keep it distinct from the
  bibliography-key form.

## Heritage annotations vs theoretical citations

Some projects inherit design lineage from existing systems
(BeOS, Plan 9, Smalltalk, whatever). Source-code lineage
citations have a different shape from theoretical paper
citations, and the two coexist without overlap:

- **Heritage annotations** cite source-code lineage. Form:
  `path:line`, e.g. `src/kits/app/Looper.cpp:1162`. The claim
  is "system X did this at this location." Verification: the
  reader opens the cited file at the cited line and reads the
  analogous code.
- **Theoretical citations** cite papers and monographs. Form:
  `[Key]`, resolving to a bibliography entry in academic
  citation format. The claim is "this construct realizes an
  idea from paper Y." Verification: the reader opens the
  bibliography, finds the paper, and checks the cited section.

**Do not cross the streams.** A source-code file does not get
a bibliography entry. A paper does not get a `path:line`
citation. A module that draws from both cites both, separately,
in the same module doc block. The two disciplines have
different audit procedures (heritage citations are verified
against the cited source repository; theoretical citations are
verified against the bibliography) and keeping them distinct
keeps both audits tractable.

## Bootstrapping a project

When adopting this workflow for an existing project:

1. **Create `docs/citations.md`.** Populate it with the
   references the project currently draws on, using ACM style.
   Mark unverified fields `NEEDS BACKFILL`. Do not start by
   backfilling all entries — start with the few references
   that are load-bearing for recent work and add entries as
   code cites them.
2. **Write the style guide section.** Copy this document's
   module-level and function-level templates into the
   project's `STYLEGUIDE.md` (or equivalent). Adapt the
   language-specific comment syntax if the project is not
   Rust.
3. **Write the mechanical linter.** A POSIX shell script plus
   `awk` and `grep` is sufficient for the mechanical check;
   the whole script fits in 50–100 lines. Make the disclaimer
   unconditional.
4. **Add the two-step discipline to the review process.** The
   PR template should include a checkbox or explicit comment
   requirement: "If this PR touches a cited function,
   re-verify the citation against the source and confirm in a
   comment."
5. **Assign the auditor role.** Decide who owns the periodic
   semantic audit and how often it runs.
6. **Backfill incrementally.** Do not try to retrofit
   citations across the whole codebase in one pass. Add
   citations as code is touched for other reasons. The
   backfill will converge because every refactor that touches
   cited code refreshes the citations.

Existing projects may have legacy documentation comments
without citations. Do not delete them; add citations
incrementally as functions get touched. Document the workflow
first, enforce it on new work, and let coverage grow.

## Anti-patterns

- **Citations as ornament.** `/// See [Paper95] for context.`
  Context is background, not specific dependence. If the paper
  did not inform the function's implementation, it does not
  belong in the function's doc.
- **Stale citations kept because removing them feels wrong.**
  A citation whose code has moved on is misinformation. Remove
  or update.
- **Bibliography entries without annotations.** An entry that
  is just a citation with no description of what it
  contributes is dead weight. Write the annotation or remove
  the entry.
- **Citation keys that duplicate paper-internal abbreviations.**
  If the paper itself calls the construct X and the
  bibliography key is `[Paper95]`, do not write
  `[Paper95-X]`. The paper's internal abbreviation belongs in
  the citation's inline phrasing, not the key.
- **Local filesystem paths in the public bibliography.** The
  bibliography is published alongside the code. It should look
  like an academic reference list, not a file index.
- **Linters whose disclaimers can be silenced.** Any flag that
  turns off the "mechanical check only" disclaimer defeats the
  containment discipline. Do not add such a flag.
- **Treating green CI as evidence of citation correctness.**
  CI green proves the keys resolve. It does not prove the
  citations are correct. The reviewer comment on the PR is
  the evidence that the semantic check happened.
- **Citing references nobody on the project has read.** Every
  citation is a claim of intellectual debt by someone. If no
  one on the project can defend the citation, it does not
  belong.

## Summary in one page

- Module docs establish the frame; function docs fill in
  detail; citations appear at whichever level drew from the
  reference.
- Every citation uses a short key that resolves to an entry
  in a central bibliography. The bibliography is public-
  facing, uses academic citation format, carries no local
  filesystem paths.
- Epistemic strength matches the source. Hedges preserved.
  Details not invented.
- A mechanical linter catches trivial breakage (typos,
  resolved keys, unused entries). Its output carries an
  unconditional disclaimer: mechanical check only; green ≠
  correct.
- A semantic audit catches the failures the linter cannot —
  stale citations, wrong attribution, strengthened hedges,
  refactor-induced drift. The semantic audit runs per-change
  (reviewer) and periodically (auditor). Reviewer comment on
  each PR is the evidence that the semantic check happened.
- Citations are implementation attributes. They move with the
  code, they are deleted with the code, they are updated when
  the code is refactored.
- The workflow exists because mechanical tools that answer
  easy questions get used as proxies for hard questions. Every
  mechanism in the workflow has a narrow scope and knows what
  it does not prove.

---

*This workflow was developed in the context of a systems
project with theoretical foundations (session types, duploids,
profunctor optics). The specifics are Rust-flavored, but the
principles generalize to any language with structured
documentation comments and any project with references it
wants to cite responsibly.*
