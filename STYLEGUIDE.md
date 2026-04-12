# Style Guide

Conventions for contributing to psh — code, prose, and formatting.
Observed by all contributors, human and agent alike.

---

## Rust Formatting

| Rule | Setting |
|------|---------|
| Formatter | `cargo fmt` via `rustfmt.toml` |
| Max line width | 100 |
| Import granularity | Per-crate (`use psh::{ast, value}`) |
| Import grouping | std, then external crates, then local (`group_imports = "StdExternalCrate"`) |
| Linter | `cargo clippy -- -D warnings` |

Run `cargo fmt` before every commit. Run `cargo clippy`
before pushing. Fix warnings — don't suppress them.

The `rustfmt.toml` is the source of truth. No editor-local overrides.

## Derive Order

When a type derives multiple traits, use this order:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
```

Only derive what's needed. Omit `Copy` unless the type is small and
value-semantic.

## Visibility

psh is a single binary crate. Visibility rules:

| Scope | Annotation |
|-------|------------|
| Public library API (when split into lib + bin) | `pub` |
| Test support | `pub` on the module, `#[cfg(test)]` on the test mod |
| Everything else | private (no annotation) |

## Error Handling

| Context | Pattern |
|---------|---------|
| Builtin commands | Return `Status::err(message)` — never panic |
| Parser errors | `anyhow::bail!` with position |
| Lexer errors | `anyhow::bail!` with position |
| fork/exec failures | Return `Status::err` with errno context |
| Discipline functions | Reentrancy guard, then run body |

psh uses the ⊕ error convention exclusively: every function
returns a status or result. No longjmp, no panic for control
flow. Rust's `Result` + `?` is the mechanism.

---

## Code Comments

Comments explain *why*, not *what*. If the code needs a *what*
comment, the code needs rewriting.

```rust
// Bad: check if the variable exists
if let Some(var) = scope.get(name) { ... }

// Good: discipline functions fire in the current scope so
// their side effects (e.g., restarting a service) are visible
if let Some(body) = self.env.get_fn(&format!("{name}.set")).cloned() { ... }
```

Do not add comments to code you didn't change. Do not add
docstrings to private functions unless the logic is genuinely
non-obvious.

### Heritage Annotations

psh draws from two shell traditions. Document the lineage
where it matters:

```rust
//! rc heritage: first-class lists, single-quote-only quoting,
//! string-valued status. (Duff, "Rc — The Plan 9 Shell", 1990)
//!
//! ksh93 heritage: discipline functions, coprocesses, namerefs.
//! (Korn/Bolsky, "The New KornShell", 1995)
```

### Reference repositories

| Source | Location |
|--------|----------|
| ksh93u+m | `/Users/lane/src/ksh93` (github: `ksh93/ksh`) |
| ksh26 | `/Users/lane/src/ksh/ksh` (Lane's fork) |
| Plan 9 | `/Users/lane/src/lin/pane/reference/plan9/` vendored |
| pane | `/Users/lane/src/lin/pane` |

### Theoretical Citations

psh cites published references using short bibliography keys
that resolve to entries in `docs/citations.md`. The full
workflow is in `docs/citation-workflow.md`. Summary:

- **Module-level docs** (`//!`) carry a `# References` section
  listing the keys that inform the module's architecture.
- **Function-level docs** (`///`) cite only when the specific
  function draws from a reference. Most functions need no
  citation.
- **Every citation is a testable claim.** "This code implements
  the idea described in this reference." If the claim is not
  defensible, remove the citation.
- **Epistemic strength matches the source.** Hedge when the
  source hedges.

**Module-level template:**

```rust
//! # Module name — one-line role description
//!
//! Paragraph on the module's place in psh's architecture.
//! Which layer (value/expression/command), which phase
//! (parse/eval/exec), how it relates to adjacent modules.
//!
//! Design approach: what concepts from psh's theoretical
//! foundation are realized here. Cite at the level of the
//! architectural commitment, not at every function.
//!
//! See `docs/specification.md` §Relevant Section for the
//! design rationale.
//!
//! # References
//!
//! - `[Key1]` — what it contributes to this module
//! - `[Key2]` — what it contributes to this module
```

**Function-level example:**

```rust
/// Installs a one-shot reply continuation keyed on the request
/// token. Consumed by `fire_reply`; removed on fire, realizing
/// the linear usage from [HVK98] §3.
///
/// See `docs/specification.md` §Coprocess protocol.
pub fn install_continuation(...) { ... }
```

### Citation format

Two separate disciplines — do not cross them:

| Tradition | Format | Audit procedure |
|-----------|--------|-----------------|
| Heritage (rc, ksh93, Plan 9) | `rc.ms:line` or `src/cmd/ksh93/sh/nvdisc.c:302` | Verify against cited source repo |
| Theoretical (papers) | `[Key]` resolving to `docs/citations.md` | Verify against bibliography entry |
| Internal design docs | `docs/specification.md §Section` | Verify against current spec |

Heritage annotations cite source-code lineage. Theoretical
citations cite papers. A module drawing from both carries both,
separately.

### Citation hygiene

- **`tools/cite-lint.sh`** catches typos and dangling keys.
  Run before committing. Green lint ≠ correct citations — it
  is a mechanical check only.
- **Semantic audit** catches stale citations, wrong attribution,
  strengthened hedges. Performed at review time: if a PR touches
  a cited function, the reviewer re-reads the cited reference
  and confirms the citation is still accurate.
- **Citations follow the code.** Move with refactors, delete
  with deletions, update when the implementation changes.

---

## Technical Writing Voice

Same as pane: describe the machine. Present tense, active voice,
concrete behavior. Short sentences. Code examples over prose.

| Guideline | Example |
|-----------|---------|
| Present tense | "The lexer reads from the input" not "will read" |
| Active voice | "The parser wraps redirections" not "redirections are wrapped" |
| State consequences | "Panics if the fd is closed" not "may fail" |
| No hedging | "does" not "may", "fails" not "might fail" |

---

## Testing

| Rule | Detail |
|------|--------|
| Framework | `#[test]` with `cargo test` |
| Test naming | `snake_case` describing the claim being tested |
| Test isolation | Each test creates its own `Shell` instance |

Test names should read as claims:

```rust
#[test]
fn concat_pairwise() { ... }

#[test]
fn redirect_order_left_to_right() { ... }

#[test]
fn discipline_set_fires_on_assignment() { ... }
```

---

## Architecture Patterns

These are load-bearing design decisions documented in
`docs/specification.md` (the principled design document)
and the pane project's `docs/shell.md`.

| Pattern | Rule |
|---------|------|
| Value model | Typed enum (Unit, Bool, Int, Str, Path, List). Bare assignment stays Str (rc heritage); `let` bindings run type inference. |
| AST structure | Three sorts: values (Word), expressions (Expr), statements (Statement). Profunctor redirections. |
| Error convention | ⊕ only (Status returns). No longjmp, no panic for control flow. |
| Evaluation | CBV for words, CBN for pipelines. |
| Discipline functions | fn x.get / fn x.set — the MonadicLens at the shell level. |
| Dependencies | par (hard), pane-proto + pane-session (feature-gated). No fp-library. |
| Scope | Decide: rc (no scoping) vs ksh (local via typeset). Document the choice. |
| Shell struct | Keep decomposed. Separate concerns into separate sub-structs. Prevent Shell_t trajectory. |

---

## Intermediate State Principle

Multi-step implementation is safer when each intermediate state
is a natural resting point. The litmus test: **would anyone
design the intermediate state on purpose?** If not — if it
exists only as a waypoint between two coherent designs — combine
the steps.

**When phasing wins:** each intermediate is a plausible resting
place. A validation library: step 1 = email validation (useful
standalone), step 2 = URL validation. A protocol stack: step 1 =
framing (useful without correlation). Each phase is a system
someone would ship.

**When phasing is a trap:** an intermediate creates a coherence
obligation that neither the before-state nor the after-state
requires. Building a counter separately from the collection it
counts. Implementing error types before the operations that
produce errors. Extracting a type to a new crate before the
functions that return it. The transient cross-boundary invariant
is strictly harder to reason about than either endpoint.

**How to apply:**

1. Before proposing a phased plan, write down what the system
   looks like after each phase — not what changed, what EXISTS.
2. For each intermediate: "If we stopped here permanently, would
   this be a reasonable design?" If "no, but the next phase fixes
   it" — merge the phases.
3. A smaller diff is not inherently safer. An incoherent
   intermediate is strictly more dangerous than a larger coherent
   step.
4. Exception: if a combined step is too large to review or test
   as a unit, splitting is justified — but document the
   intermediate as intentionally transient and land both steps in
   the same review cycle.

---

## Divergence Protocol

When deviating from rc behavior:

1. Document the divergence with rationale
2. Valid reasons: ksh93 influence, correctness improvement,
   Rust idiom, pane integration
3. Invalid reasons: "sounds better", didn't check what rc does
4. Known deliberate divergences:
   - `if cond { } else { }` with mandatory braces (fixes rc's `if not`)
   - Scope push/pop on function calls (ksh influence, not rc)
   - Newline splitting for command sub (not `$ifs`)
   - `$pipestatus` as list (not rc's `|`-separated string)
