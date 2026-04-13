---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [lambda, syntax, pipe-delimited, rust-style, line-continuation, nullary]
agents: [psh-architect, psh-sequent-calculus]
related: [decision/def_vs_lambda, decision/backslash_escape_rules, analysis/cbpv_f_u_separation]
verified_against: [docs/spec/@HEAD, docs/spec/04-syntax.md@HEAD]
---

# Decision: lambda syntax is `|x| { body }` / `|x| => expr`

## Decision

psh lambdas use Rust-style pipe-delimited parameter lists:

- `|x| => expr` — single-expression body
- `|x| { block }` — block body
- `| | => expr` — nullary lambda

The prior `\x => body` form is **not adopted**.

## Why

Replacing `\` as the lambda introducer frees the backslash for line continuation (`\<newline>`) and character escapes (`\'`, `\$`). The pipe character inside a lambda parameter list is unambiguous because lambdas only appear in **value position**, where a leading `|` cannot be a shell pipe.

Alternative considered: keep `\x =>`, reserve backslash for lambdas only. Rejected because line continuation is an essential editor convenience and backslash escapes are too useful to give up.

Commit `e0ecaf5` landed the change. Decision history is in git.

## Consequences

- Line continuation via `\<newline>` is available throughout psh source.
- Backslash escapes (`\'`, `\$`, `\\`, `\n` as literal `n`) are available in single-quoted strings.
- Nullary lambda uses a space inside the pipes (`| |`) to avoid confusing with `||` (the boolean-OR operator).
- The pipe position is unambiguous because lambdas appear in value position (RHS of `let`, argument to a higher-order function, etc.); in command position a leading `|` remains the pipeline operator.

Spec: `docs/spec/` §"Two kinds of callable", §"Syntax". Ledger: Decision history is in git.
