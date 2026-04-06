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

### Theoretical Annotations

When code implements a concept from the duploid framework or
sequent calculus, cite the source concisely:

```rust
//! Theoretical basis: redirections are profunctor transformations
//! (lmap/rmap on the IO profunctor). Left-to-right evaluation is
//! the canonical left-associated bracketing in the duploid.
```

Keep theoretical annotations brief. One sentence identifying the
concept and its source.

### Citation format

| Tradition | Format |
|-----------|--------|
| rc | `(Duff 1990, §section)` or `rc.ms:line` |
| ksh93 | `src/cmd/ksh93/sh/nvdisc.c:302` (path relative to ksh93 root) |
| ksh26 SPEC.md | `(SPEC.md §section, line N)` |
| Papers | Author, title, year, section |

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
