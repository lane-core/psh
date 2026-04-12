---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [parameter-expansion, sigil, length, join, rc-heritage, list-destructor, prefix-sigil, dollar-hash, dollar-quote]
agents: [plan9-systems-engineer, psh-architect]
related: [decision/every_variable_is_a_list, analysis/data_vs_codata, analysis/duff_principle]
verified_against: [docs/specification.md@HEAD, docs/syntax.md@HEAD]
---

# Decision: `$#x` and `$"x` are type-specific eliminators for List

## Decision

Two parameter expansion sigils are reserved for List destructors:

- **`$#x`** — list **length** (number of elements in `x`)
- **`$"x`** — list **join** (concatenate elements of `x` with a separator)

These are **prefix-sigil** forms in the rc tradition, not ksh93's suffix form (`${#x}` / `${var#pat}`).

## Why

List is the outermost layer of every psh variable (`decision/every_variable_is_a_list`), so it needs type-specific eliminators with high user-facing frequency. The two most common operations on a list are "how many?" (length) and "flatten to string" (join). Making them parameter-expansion sigils puts them at the syntactic level where shell users expect them.

rc used prefix sigils for shell-level operations (`$x` for value, `$#x` for length). psh preserves this convention rather than adopting ksh93's `${#x}` braced suffix form, which doesn't extend cleanly to other destructors and introduces a second bracketing layer inside the parameter expansion.

Alternative considered: method-style accessors like `$x .length` and `$x .join`. These remain available as Str/List methods but are longer to type for the two most common operations. The sigils are the short forms; the accessors are the long forms; they coexist.

Commit `30f5f6c` landed the decision. Decision history is in git.

## Consequences

- `$#count` returns the length of `count`. For a scalar (list of length 1), returns 1.
- `$"path` joins list elements with a conventional separator (colon by default, matching shell PATH conventions).
- The same operations are also available via `$count .length` and `$path .join` method accessors (see `decision/postfix_dot_accessors`).
- ksh93's `${var#pat}` / `${var%pat}` pattern-matching forms are **not** adopted; use Str method accessors instead.

Spec: `docs/specification.md` §"Extension path" (String methods on Str), §"Foundational commitment" (every variable is a list). Ledger: Decision history is in git.
