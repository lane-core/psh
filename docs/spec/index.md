# psh specification

The specification of psh's type theory, execution model, and
design rationale. psh descends from rc (Duff 1990), not from
the Bourne shell.

## Reading order

Start with foundations and calculus for the design philosophy.
The syntax chapter is the implementer's contract. Everything
else can be read as needed.

| # | Chapter | Scope |
|---|---------|-------|
| 01 | [Foundations](01-foundations.md) | Design position, every-var-is-list, rc heritage, sfio insight |
| 02 | [Calculus](02-calculus.md) | λμμ̃, dialogue duploids, three sorts, L-calculus |
| 03 | [Polarity](03-polarity.md) | CBV/CBN, polarity frames, linear resources, exponentials |
| 04 | [Syntax](04-syntax.md) | Formal grammar — the implementer's contract |
| 05 | [Type checking](05-type-checking.md) | Bidirectional checking, modes, pinning |
| 06 | [Types](06-types.md) | Ground types, tuples, structs, enums, Path, ExitCode |
| 07 | [Callables](07-callables.md) | `def` vs `let`+lambda, two kinds of callable |
| 08 | [Discipline functions](08-discipline.md) | Codata model, .get/.refresh/.set, MonadicLens |
| 09 | [Redirections](09-redirections.md) | Profunctor structure, word expansion pipeline |
| 10 | [Coprocesses](10-coprocesses.md) | 9P protocol, per-tag sessions, wire format |
| 11 | [Namespace](11-namespace.md) | Three tiers, export, environment |
| 12 | [Error handling](12-errors.md) | ⊕/⅋, Status/ExitCode, try/catch, trap, signals |
| 13 | [Optics](13-optics.md) | Accessor taxonomy, activation table |
| 14 | [Invocation](14-invocation.md) | Startup, flags, profile sourcing *(stub)* |
| 15 | [Builtins](15-builtins.md) | Builtin reference *(stub)* |
| 16 | [Features](16-features.md) | Features, non-goals, reserved keywords |

## Annotated bibliography

[references.md](references.md) — full ACM-style entries with
annotations. Every `[Key]` citation in the chapters resolves
here.

## Relationship to other documents

| Document | Role | Tier |
|----------|------|------|
| This directory (`docs/spec/`) | Single source of truth | 1 |
| `docs/vdc-framework.md` | Categorical semantics (VDC theory) | 2 |
| `refs/ksh93/ksh93-analysis.md` | ksh26 sequent-calculus analysis | 2 |
| `docs/implementation.md` | Engineering principles, crate rationale | 2 |
| serena memory store | Project knowledge base | 3 |
| `~/gist/` | Vendored papers | 4 |

Higher tiers override lower. When this spec and a framework
document disagree, the spec wins.

## Man page correspondence

| Chapter(s) | Eventual man page |
|------------|-------------------|
| 01-03 | psh(7) — theoretical foundations |
| 04 | psh-syntax(7) — grammar reference |
| 05-06 | psh-types(7) — type system |
| 07-08 | psh-functions(7) — callables and disciplines |
| 09 | psh-redirections(7) — I/O model |
| 10 | psh-coprocess(7) — coprocess protocol |
| 11, 14 | psh(1) — invocation, environment |
| 12 | psh-errors(7) — error handling |
| 15 | psh-builtins(1) — builtin reference |

## Citation discipline

Practical rules: `STYLEGUIDE.md` §Theoretical Citations.
Full rationale: serena `policy/citation_workflow`.
Mechanical check: `tools/cite-lint.sh`.
