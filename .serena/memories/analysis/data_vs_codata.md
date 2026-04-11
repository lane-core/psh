---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [data, codata, constructors, destructors, pattern-match, copattern-match, observers, duality, list, fd, codata-discipline, copattern-style]
agents: [psh-sequent-calculus, psh-optics-theorist, vdc-theory, plan9-systems-engineer]
related: [decision/codata_discipline_functions, decision/postfix_dot_accessors, analysis/monadic_lens, analysis/three_sorts, analysis/error_duality_oplus_par, reference/papers/grokking_sequent_calculus]
verified_against: [docs/vdc-framework.md@HEAD §3.5 lines 277-310, docs/specification.md@HEAD §510-655, audit/psh-optics-theorist@2026-04-11]
---

# Data vs codata (constructors vs destructors)

## Concept

The sequent calculus reveals **data types** and **codata types** as perfectly dual (`docs/vdc-framework.md` §3.5 "Data and codata" lines 277–310):

- **Data types** are defined by their **constructors** (how to build a value). The **consumer must pattern-match**. The producer chooses which constructor to use; the consumer must handle every case. Example: a list is `Nil` or `Cons(head, tail)`. Lists of arguments are data — the caller chose how many; the script must `switch` on `$#*`.
- **Codata types** are defined by their **destructors** (how to observe a value). The **producer must handle all observations**. The consumer chooses which destructor to invoke; the producer must respond. Example: a stream has destructors `hd` and `tl`. File descriptors are codata — the caller chooses to redirect, pipe, or close; the process must be ready to write to whatever fd is observed.

The vdc-framework section (line 310) summarizes: "The sequent calculus makes this duality first-class: constructors and destructors, pattern matches and copattern matches, are symmetric. Rc does not formalize this symmetry, but it is already present in the design."

In psh this duality becomes structural:

- **Argument lists are data.** The script pattern-matches via `match` or `switch`. The constructors are the list-builders; the eliminator is case analysis. (Spec example at vdc-framework lines 289–298: the rc `switch($#*)` idiom.)
- **Discipline-equipped variables are codata.** A variable with `.get` and `.set` is observed by accessor postfix syntax (`$x .get`, `$x .field`). The producer (the discipline body) responds to the observation. This is **copattern matching**: the destructor is on the left of `=` instead of constructors.
- **File descriptors are codata** in the rc tradition (vdc-framework lines 302–308). Redirection syntax is the destructor invocation.

The codata framing is what justifies `decision/codata_discipline_functions`: a discipline-backed variable is genuinely codata in the technical sense, not "data with side effects." `.get` is the codata observer (computes the value seen at the access site); `.set` is the codata constructor (mediates the assignment). Together they form a **MonadicLens** in Kl(Ψ) (`analysis/monadic_lens`).

The data/codata duality is also where `decision/postfix_dot_accessors` lives: postfix dot is **copattern-style** syntax — the form from the sequent calculus literature where codata observers are applied postfix to the value being observed.

The duality also surfaces in the error model (`analysis/error_duality_oplus_par`): ⊕ (status, eliminated by case-match) is a data type; ⅋ (trap, eliminated by copattern-match — see `analysis/error_duality_oplus_par`) is a codata type.

## Foundational refs

- `docs/vdc-framework.md` §3.5 "Data and codata" lines 277–310 — psh's framing with the rc examples (argument list as data, fd as codata).
- `reference/papers/grokking_sequent_calculus` — Binder et al. introduce data/codata duality in the Fun→Core compilation. The cleanest first read.

## Spec sites

- `docs/specification.md` §"Discipline functions §The codata model" line 510 — the codata reading of discipline functions.
- `docs/specification.md` §"MonadicLens structure" line 647 — the optic class that follows.
- `decision/codata_discipline_functions` — the design decision built on this duality.
- `decision/postfix_dot_accessors` — copattern-style syntax.
- `analysis/monadic_lens` — the optic class consequence.
- `analysis/error_duality_oplus_par` — the same duality applied to error handling.

## Status

Settled. The data/codata duality is foundational. When asked "is X a data type or a codata type", the test is: who picks the elimination form? Producer picks (constructors) → data. Consumer picks (destructors) → codata.
