# psh: Implementation Blueprint

Implementation architecture for psh, organized by dependency
layer. Lower-numbered layers are foundations; higher layers
build on them.

## Layer directory

| # | Layer | Scope | File |
|---|-------|-------|------|
| 00 | [Principles](00-principles.md) | Dependencies, crate budget, source structure, engineering principles |
| 01 | [Σ — Signature store](01-sigma.md) | Type declarations, registries, trait hierarchy |
| 02 | [Val — Value model](02-val.md) | Runtime values, slots, conversions |
| 03 | [AST — Syntax tree](03-ast.md) | Three-sort AST, patterns, programs |
| 04 | [Parser](04-parser.md) | Six-layer combine architecture |
| 05 | [Checker](05-checker.md) | Bidirectional type checking |
| 06 | [Evaluator](06-evaluator.md) | Three-function core, polarity frames |
| 07 | [Environment](07-environment.md) | Scope chain, def registry, traps |

## Dependency graph

```
07-environment ←── 06-evaluator ←── 05-checker
       ↑                ↑               ↑
       └────────────────┼───────────────┘
                        │
                   04-parser
                        ↑
                   03-ast
                        ↑
                   02-val
                        ↑
                   01-sigma
                        ↑
                   00-principles
```

All layers query Σ (01). The evaluator (06) owns the
environment (07). The checker (05) reads the AST (03) and Σ.
The parser (04) produces AST nodes and queries Σ for
constructor arity.

## Companion documents

| Document | Role | Tier |
|----------|------|------|
| `docs/spec/` | Specification — source of truth | 1 |
| This directory (`docs/impl/`) | Implementation blueprint | 2 |
| `docs/vdc-framework.md` | Categorical semantics | 2 |
| `refs/ksh93/ksh93-analysis.md` | ksh26 sequent-calculus analysis | 2 |

## References

All citation keys resolve to `docs/spec/references.md`.
