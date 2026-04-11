---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [rc, plan9, duff, shell, no-rescan, list-values, fn, sigexit, holmdel]
agents: [plan9-systems-engineer, psh-architect, vdc-theory, psh-sequent-calculus]
---

# Reference: rc paper (Duff 1990) and man page

**Paths (vendored).**

- `refs/plan9/papers/rc.ms` — Tom Duff, *Rc — The Plan 9 Shell*, 1990. The foundational document for psh. Short. **Read first** in any new plan9 session.
- `refs/plan9/man/1/rc` — authoritative rc(1) man page.
- `refs/plan9/LICENSE` — Plan 9 Foundation MIT license.

**Origin.** Plan 9 from Bell Labs. Vendored into psh to preserve the design lineage.

## What's in the paper

- **§Design Principles** — "Input is never scanned more than once." The load-bearing sentence. Everything psh extends across its type system descends from this.
- **List-valued variables** — `path=(. /bin)` is two strings, never rescanned. The foundational move.
- **Consistent syntax** — `{}` for grouping, `'` for quoting (single quotes only), `()` for lists. rc started fresh after decades of Bourne shell accretion.
- **`fn name { body }`** — function definitions as named command forms. psh renames this to `def` to distinguish from value-level functions (see `decision/def_vs_lambda`).
- **Signal handlers as `fn sigint { ... }`** — named continuations triggered by signals. Pre-parsed cells, not re-interpreted strings. psh generalizes via `decision/unified_trap_three_forms`.
- **`sigexit`** — artificial signal synthesized on process exit. psh preserves as `EXIT`.
- **Exit status as string** — "On Plan 9 status is a character string describing an error condition. On normal termination it is empty." psh preserves this.
- **`$ifs` kept only as "indispensable"** — Duff's one reluctant compromise with Bourne-style rescanning. psh removes it, closing the last rescanning hole (command substitution splits on newlines, not `$ifs`).
- **`eval` as the deliberate rule-breaker** — "whose raison d'être is to break the rule" of not rescanning input. psh preserves this as the only place structure is deliberately flattened.
- **The Holmdel example** — a complete worked rc script. `docs/vdc-framework.md` §5.14 walks through it as a VDC pasting diagram.

## Concepts it informs in psh

- `decision/every_variable_is_a_list` — foundational.
- `decision/three_roles_of_parens` — rc's `()` is the list-literal role; tuples and tagged construction extend it.
- `decision/single_quotes_only` — rc's quoting convention.
- `decision/def_vs_lambda` — rc's `fn` renamed to `def`; `let` + lambda is psh's extension.
- `decision/unified_trap_three_forms` — rc's `fn sigint { }` generalized.
- `decision/backslash_escape_rules` — rc's minimal quoting rules; line continuation added.

## Who consults it

- **plan9 agent** (primary, canonical): read in full in every new session.
- **psh-architect**: for rc-heritage choices in parser and grammar integration.
- **vdc-theory agent**: `docs/vdc-framework.md` §5 walks rc concept by concept against the VDC mapping.
- **sequent calculus agent**: the three-sort structure is implicit in rc and named in psh.
