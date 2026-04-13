---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [ksh93, manpage, sh, compound-variable, discipline-function, namespace, coprocess, trap, here-doc, here-string]
agents: [plan9-systems-engineer, psh-architect]
---

# Reference: ksh93u+m manpage (`sh.1`)

**Path.** `refs/psh/refs/ksh93/sh.1`

**Origin.** AT&T / ksh93 community; ksh93u+m is the community fork. The canonical user-facing specification of ksh93's behavior.

## What's in the manpage

- **Command syntax** — authoritative reference for ksh93 grammar.
- **Variable and parameter expansion** — `${var#pat}`, `${var%pat}`, `${#var}`, etc. psh replaces these with Str method accessors (`decision/postfix_dot_accessors`) and prefix sigils (`decision/parameter_expansion_destructors`).
- **Compound variables** (`typeset -C`) — ksh93's struct system. `${x.field}` accessed fields; disciplines mediated access. psh's structs + postfix dot accessors are the typed version. See `decision/struct_positional_only_forever`.
- **Discipline functions** — `get`/`set`/`unset` functions attached to variables. Empirical precedent for psh's `decision/codata_discipline_functions`.
- **Arrays** — indexed and associative. psh's List and Map types correspond.
- **Type annotations** (`typeset -i`, `typeset -a`, etc.) — ksh93's implicit type system. `refs/ksh93/ksh93-analysis.md` analyzes this as the implicit type theory.
- **Namespaces** — ksh93's namespace construct. psh has a three-tier namespace (shell variables, process environment, filesystem) — see `docs/spec/` §"Namespace".
- **Coprocesses** (`|&`) — untyped byte-stream bidirectional channels. `decision/coprocess_9p_discipline` is psh's typed refinement.
- **Here-documents** (`<<EOF`) and **here-strings** (`<<<`) — inherited syntactic forms; here-string is on the v1 roadmap.
- **Traps** (DEBUG, ERR, EXIT, signal traps) — the `sh.prefix` bugs analyzed in `refs/ksh93/ksh93-analysis.md` manifest here.
- **Job control** (`fg`, `bg`, `jobs`, `wait`) — psh inherits the builtin set.
- **Builtins** — alias, bg, bind, break, builtin, cd, command, continue, echo, eval, exec, exit, export, false, fc, fg, getopts, hash, jobs, kill, let, pwd, read, readonly, return, set, shift, test, times, trap, type, typeset, ulimit, umask, unalias, unset, wait.

## Concepts it informs in psh

- `decision/codata_discipline_functions` — ksh93 discipline functions are the empirical precedent.
- `decision/coprocess_9p_discipline` — ksh93 `|&` is the pre-typed ancestor.
- `decision/struct_positional_only_forever` — ksh93 compound variables `typeset -C` are the untyped ancestor.
- `decision/unified_trap_three_forms` — ksh93 `trap` is the un-scoped ancestor.

## Who consults it

- **plan9 agent** (primary): the authority for "what does ksh93 actually do here?" Cite sections by name (e.g., "sh.1 under 'Compound Commands'").
- **psh-architect**: for ksh93-compatible user-facing choices and builtin semantics.
