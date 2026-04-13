---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-12
importance: high
keywords: [quoting, strings, single-quotes, double-quotes, interpolation, free-caret, rc-heritage]
agents: [plan9-systems-engineer]
related: [decision/backslash_escape_rules]
---

# Decision: two string forms — single quotes (literal) and double quotes (interpolating)

## Decision

psh has two string literal forms:

- **Single quotes** — literal, no expansion. `'hello $name'` is
  the literal text `hello $name`.
- **Double quotes** — interpolating. `"hello $name"` expands
  `$name`. Inside double quotes: `$var`, `$var[i]`, and
  `` `{cmd} `` are expanded. Dot accessors require explicit
  delimiting: `"${name.upper}"`. `\$` escapes the dollar sign.
  Multi-element lists join with spaces (equivalent to `$"var`).

## Supersedes

`decision/single_quotes_only` (2026-04-10). The original design
followed rc's single-quote-only convention. Changed 2026-04-12
because:

1. rc's reason for dropping double quotes was Bourne-specific —
   Bourne's complex interpolation rules ($ ` \ ! but not globs)
   don't apply to psh's simpler expansion model.
2. `'hello '$name' welcome'` via free caret is the #1 interactive
   friction point.
3. Double quotes + the tight-binding dot change mean `"$name.txt"`
   works naturally (variable terminates at `.` per `var_char`).

## Spec

`docs/spec/` §"Two string forms". `docs/spec/04-syntax.md`
§Quoting (SQ_STRING, DQ_STRING productions).
