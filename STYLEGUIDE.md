# Style Guide

Conventions for psh contributions — code, prose, formatting.
All contributors follow, human + agent.

---

## Rust Formatting

| Rule | Setting |
|------|---------|
| Formatter | `cargo fmt` via `rustfmt.toml` |
| Max line width | 100 |
| Import granularity | Per-crate (`use psh::{ast, value}`) |
| Import grouping | std, then external crates, then local (`group_imports = "StdExternalCrate"`) |
| Linter | `cargo clippy -- -D warnings` |

Run `cargo fmt` before every commit. Run `cargo clippy` before pushing. Fix warnings — no suppression.

`rustfmt.toml` = source of truth. No editor-local overrides.

## Derive Order

Multiple trait derives use this order:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
```

Only derive what's needed. Omit `Copy` unless type small + value-semantic.

## Visibility

psh = single binary crate. Visibility rules:

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

psh uses ⊕ error convention exclusively: every function returns status/result. No longjmp, no panic for control flow. Rust's `Result` + `?` = mechanism.

---

## Code Comments

Comments explain *why*, not *what*. Code needs *what* comment → code needs rewriting.

```rust
// Bad: check if the variable exists
if let Some(var) = scope.get(name) { ... }

// Good: discipline functions fire in the current scope so
// their side effects (e.g., restarting a service) are visible
if let Some(body) = self.env.get_fn(&format!("{name}.set")).cloned() { ... }
```

No comments on unchanged code. No docstrings on private functions unless logic genuinely non-obvious.

### Heritage Annotations

psh draws from two shell traditions. Document lineage where it matters:

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

psh cites published refs using short bibliography keys resolving to entries in `docs/citations.md`. Full workflow in `docs/citation-workflow.md`. Summary:

- **Module-level docs** (`//!`) carry `# References` section listing keys informing module architecture.
- **Function-level docs** (`///`) cite only when specific function draws from reference. Most functions need no citation.
- **Every citation = testable claim.** "This code implements idea from this reference." Claim not defensible → remove citation.
- **Epistemic strength matches source.** Hedge when source hedges.

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
//! See `docs/spec/` §Relevant Section for the
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
/// See `docs/spec/` §Coprocess protocol.
pub fn install_continuation(...) { ... }
```

### Citation format

Two separate disciplines — don't cross:

| Tradition | Format | Audit procedure |
|-----------|--------|-----------------|
| Heritage (rc, ksh93, Plan 9) | `rc.ms:line` or `src/cmd/ksh93/sh/nvdisc.c:302` | Verify against cited source repo |
| Theoretical (papers) | `[Key]` resolving to `docs/citations.md` | Verify against bibliography entry |
| Internal design docs | `docs/specification.md §Section` | Verify against current spec |

Heritage annotations cite source-code lineage. Theoretical citations cite papers. Module drawing from both carries both, separately.

### Citation hygiene

- **`tools/cite-lint.sh`** catches typos + dangling keys. Run before committing. Green lint ≠ correct citations — mechanical check only.
- **Semantic audit** catches stale citations, wrong attribution, strengthened hedges. At review time: PR touches cited function → reviewer re-reads cited reference, confirms citation still accurate.
- **Citations follow code.** Move with refactors, delete with deletions, update when implementation changes.

---

## Technical Writing Voice

Same as pane: describe the machine. Present tense, active voice, concrete behavior. Short sentences. Code examples over prose.

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
| Test naming | `snake_case` describing claim tested |
| Test isolation | Each test creates own `Shell` instance |

Test names read as claims:

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

Load-bearing design decisions documented in `docs/spec/` and pane project's `docs/shell.md`.

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

Multi-step implementation safer when each intermediate state = natural resting point. Litmus test: **would anyone design intermediate state on purpose?** If not — exists only as waypoint between two coherent designs — combine steps.

**When phasing wins:** each intermediate = plausible resting place. Validation library: step 1 = email validation (useful standalone), step 2 = URL validation. Protocol stack: step 1 = framing (useful without correlation). Each phase = system someone would ship.

**When phasing = trap:** intermediate creates coherence obligation neither before-state nor after-state requires. Building counter separately from collection it counts. Implementing error types before operations producing them. Extracting type to new crate before functions returning it. Transient cross-boundary invariant strictly harder to reason about than either endpoint.

**How to apply:**

1. Before proposing phased plan, write down what system looks like after each phase — not what changed, what EXISTS.
2. Each intermediate: "If stopped here permanently, reasonable design?" If "no, but next phase fixes it" — merge phases.
3. Smaller diff not inherently safer. Incoherent intermediate strictly more dangerous than larger coherent step.
4. Exception: combined step too large to review/test as unit → splitting justified — but document intermediate as intentionally transient, land both steps in same review cycle.

---

## Divergence Protocol

When deviating from rc behavior:

1. Document divergence with rationale
2. Valid reasons: ksh93 influence, correctness improvement, Rust idiom, pane integration
3. Invalid reasons: "sounds better", didn't check what rc does
4. Known deliberate divergences:
   - `if cond { } else { }` with mandatory braces (fixes rc's `if not`)
   - Scope push/pop on function calls (ksh influence, not rc)
   - Newline splitting for command sub (not `$ifs`)
   - `$pipestatus` as list (not rc's `|`-separated string)