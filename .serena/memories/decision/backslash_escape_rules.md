---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [backslash, escape, trivia, line-continuation, literal-n, string-literal]
agents: [psh-architect, plan9-systems-engineer]
related: [decision/lambda_syntax, decision/single_quotes_only]
verified_against: [docs/deliberations.md@HEAD §"Backslash escape rules", git-log@c1512db]
---

# Decision: backslash escape rules

## Decision

- `\<non-whitespace>` is a **literal escape**: `\'` is a literal `'`, `\$` is a literal `$`, `\\` is a literal `\`, etc.
- `\<whitespace>` is **trivia**: `\<newline>` is line continuation; `\<space>` is a space-ignoring token joiner.
- `\n` is a literal `n`, **not** a C-style newline escape.

The escape rules apply inside single-quoted strings (psh's only string literal form).

## Why

rc used single quotes as the only string literal form and kept escape rules minimal. psh preserves this and adds enough escape discipline to support line continuation (which rc lacked) without introducing C-style escape confusion.

Not adopting C-style `\n` for newline: keeps the lexical rule trivially simple (every `\X` is the literal `X` for non-whitespace; trivia for whitespace). Users needing an actual newline use a multi-line string literal.

Alternative considered: adopt C-style `\n`, `\t`, `\r` escapes. Rejected because it complicates the lexer with a special case that rc users don't expect.

Commit `c1512db` landed the rules. `docs/deliberations.md` §"Backslash escape rules (APPLIED)" has the resolution record.

## Consequences

- Line continuation works via `\<newline>` inside any context where whitespace is permitted.
- `\'` escapes an apostrophe inside a single-quoted string.
- `\$` escapes a dollar sign — useful when the next character would otherwise look like a variable reference.
- Multi-line string literals handle cases where users want embedded newlines.

Spec: `docs/syntax.md` §"Backslash escapes". Ledger: `docs/deliberations.md` §"Backslash escape rules (APPLIED)".
