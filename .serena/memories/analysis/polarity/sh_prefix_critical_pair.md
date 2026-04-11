---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
verified_against: [refs/ksh93/ksh93-analysis.md@HEAD §353-423, §425]
keywords: [sh-prefix, critical-pair, ksh93-bug, missing-polarity-frame, state-leak, value-computation-boundary, ksh26, manifestation]
agents: [plan9-systems-engineer, vdc-theory, psh-sequent-calculus]
extends: analysis/polarity/_hub
related: [analysis/polarity/plus_minus_failure, analysis/polarity/dccache_witness, analysis/polarity/frames, reference/ksh93_analysis]
---

# sh.prefix bug class as critical-pair manifestation

## Concept

ksh93 has a class of recurring bugs in compound-name resolution and trap handling, all rooted in the same structural failure: the global `Shell_t.sh.prefix` field (the compound-LHS-name prefix used by `name.c` during expressions like `foo[x].bar=value`) is read across the value/computation boundary without a save/restore frame. `refs/ksh93/ksh93-analysis.md` §"Non-associativity made concrete: the sh.prefix bugs" (line 353) catalogues four:

- **Bug 001** — `typeset -i` combined with compound-associative array expansion produces "invalid variable name." The lexer's `S_DOT` handler in `lex.c:873` resets `varnamelength` on `]` followed by `.`, which is correct for `foo[x].bar=value` but wrong inside `${T[k].arr[@]}` because no nesting-level guard exists. Fixed in commit `91f0d162`.
- **Bug 002** — `typeset` inside a `DEBUG` trap during compound assignment fails because `sh_debug()` runs the trap handler without saving/restoring `sh.prefix`. The compound-assignment context (positive, value mode) is corrupted by the DEBUG trap's computation-mode intrusion. Fixed by a polarity-lite frame in `sh_debug()`.
- **Bug 003a** — `trap - DEBUG` inside a DEBUG trap has no effect because `sh_debug()` does a blanket restore of saved `sh.st`, overwriting the handler's intentional `trap[]` mutation.
- **Bug 003b** — same as 003a but the failure mode is use-after-free: the saved `sh.st` copy holds a freed trap pointer, the blanket restore writes it back, and the next DEBUG event dereferences it.

In sequent-calculus terms, Bug 002 is the (+,−) non-associativity from `analysis/polarity/plus_minus_failure` manifest in real interpreter code — `refs/ksh93/ksh93-analysis.md` §354–380 derives it explicitly as `(h ○ g) • f ≠ h ○ (g • f)` where `f` is parameter expansion, `g` is the compound-assignment context entry, and `h` is the DEBUG trap dispatch. Insert the polarity frame and the right bracketing contains `sh.prefix` within the computation frame; leave it out and `h ○ (g • f)` fires `h` around the result with `sh.prefix` exposed. Bugs 003a/b are **stale-context violations** (a different category in the taxonomy), where the missing fix is selective restoration rather than a frame per se. `sh_polarity_leave` implements both — it restores `sh.prefix`, `sh.namespace`, and `sh.var_tree` while preserving handler-side trap mutations.

Where Dccache is the empirical structural correspondence in sfio (I/O), sh.prefix bugs are the empirical structural correspondence in the shell interpreter. psh inherits the lesson and bakes the frame discipline into the architecture from the start.

## Foundational refs

- `refs/ksh93/ksh93-analysis.md` §"The critical pair" (line 297) — the theoretical framing.
- `refs/ksh93/ksh93-analysis.md` §"Non-associativity made concrete: the sh.prefix bugs" (line 353) — the bug-class taxonomy.
- `refs/ksh93/ksh93-analysis.md` §"The save/restore pattern IS the shift" (line 425) — the prescription that polarity frames are exactly the missing ingredient.
- `reference/ksh93_analysis` — the memory anchor for the source document.

## Spec sites

`docs/specification.md` §"The sfio insight" (line 133) and §"Polarity discipline" (line 343) consume this as the negative example that psh's discipline is built to prevent.

## Status

Settled as canonical historical lesson. The architecture commitment (`docs/implementation.md` §"no global mutable state") is the operational consequence — psh has no `Shell_t` analogue, so the bug class cannot exist by construction.
