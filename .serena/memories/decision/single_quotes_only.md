---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [quoting, single-quote, string-literal, rc-heritage, escape, line-continuation, no-double-quote]
agents: [plan9-systems-engineer, psh-architect]
related: [decision/backslash_escape_rules]
---

# Decision: single quotes only for string literals

## Decision

psh uses **single quotes only** as the string literal form. There is no double-quote form. Backslash escapes apply inside single-quoted strings:

- `\'` — literal apostrophe
- `\$` — literal dollar sign
- `\\` — literal backslash
- `\<whitespace>` — trivia (including `\<newline>` for line continuation)
- `\n` — literal `n`, **not** a C-style newline escape (see `decision/backslash_escape_rules`)

## Why

rc used single quotes only. Double-quoted strings in Bourne/POSIX/ksh are a source of endless confusion: variable interpolation inside quotes, partial word splitting, $-sign ambiguity. rc's rule is simpler: strings are quoted or they are not, and variable interpolation happens at the word level, not inside quotes.

psh preserves the rc convention. Adding a double-quote form would reintroduce the complexity rc deliberately removed. Alternative: adopt ksh93-style double quotes. Rejected — runs counter to rc heritage and the whole "no rescanning" principle.

## Consequences

- `'hello $name'` is the literal eight-character string `hello $name`. To interpolate, leave the quotes: `'hello ' $name` or `'hello '^$name`.
- No `"..."` form; no need to learn which characters are special inside double quotes.
- Backslash escapes inside single quotes handle the rare cases where you need a literal apostrophe or a multi-line literal.
- Concatenation via rc's free caret (`'a'^'b'` → `ab`) handles composition when interpolation isn't enough.
- Users migrating from Bourne/POSIX need to unlearn `"$var"` quoting — but they also need to relearn every-variable-is-a-list, which eliminates the need for the defensive quoting in the first place.

Spec: `docs/specification.md` §"Syntax §Single quotes only for string literals", §"Foundational commitment: every variable is a list". Syntax: `docs/syntax.md` §"Backslash escapes".
