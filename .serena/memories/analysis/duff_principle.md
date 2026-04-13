---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [duff, every-variable-is-a-list, list-valued-variables, rc-heritage, structural-substitution, no-scalar-list-distinction, quoting-ceremony, plan9, foundational, no-rescanning]
agents: [plan9-systems-engineer, psh-architect, psh-sequent-calculus, vdc-theory]
related: [decision/every_variable_is_a_list, decision/parameter_expansion_destructors, decision/three_roles_of_parens, analysis/three_sorts, analysis/data_vs_codata, reference/rc_paper_and_man]
verified_against: [docs/spec/@HEAD §31-72, refs/plan9/papers/rc.ms@HEAD §Variables lines 107-200, refs/plan9/papers/rc.ms@HEAD §"Design Principles" lines 1329 1343-1344, audit/plan9-systems-engineer@2026-04-11]
---

# Duff's principle (every variable is a list)

## Concept

Tom Duff's 1990 rc paper states the principle directly (`refs/plan9/papers/rc.ms` §"Variables", lines 110-114): "UNIX's Bourne shell offers string-valued variables. Rc provides variables whose values are lists of arguments — that is, arrays of strings. **This is the principal difference between rc and traditional UNIX command interpreters.**"

The structural commitment: lists are first-class, substitution always splices structurally, "input is never scanned more than once" (Duff §"Design Principles", cited at `docs/spec/` lines 69-70). There is no scalar/list distinction. A scalar is just a list of length 1. The Bourne shell's "string containing separators that gets re-scanned through IFS" is the negative example psh and rc both reject.

**psh extends Duff's principle across the type system** (`docs/spec/` §"Foundational commitment" line 31): every psh variable holds a list, type annotations refer to **element types** (`: Int` means "list whose elements are Int"), length is runtime data not part of the type, and substitution always splices. Tuples, sums, structs are distinct types at the *element* level — they appear inside the list. `let pos : Tuple = (10, 20)` holds a list of one tuple; `$#pos` is 1.

The structural consequence is **no `"$var"` quoting ceremony is ever needed.** Variables always splice structurally. The Bourne tax is gone by construction, not by convention. From spec line 56: "This is Duff's principle extended across the type system: the list structure is carried as data, never destroyed and reconstructed."

This is the move that makes everything else in psh's design coherent — without it, the type system would have to constantly distinguish "scalar" from "list" cases, and the cut-based execution model (`analysis/cut_as_execution`) would not work uniformly across argument lists.

## Foundational refs

- `refs/plan9/papers/rc.ms` — Tom Duff, *Rc — The Plan 9 Shell* (Bell Labs, 1990). §"Variables" lines 107-200 introduces list-valued variables: "Rc provides variables whose values are lists of arguments." §"Design Principles" lines 1329, 1343-1344: "Input is never scanned more than once by the lexical and syntactic analysis code (except, of course, by the `eval` command...)." The spec's quote at line 69 is faithful to this passage.
- `reference/rc_paper_and_man` — the serena reference memo for the rc paper.
- `docs/vdc-framework.md` §1.1 "Duff's Observation" — psh's framing of the principle in the VDC framework. Sequences are the primitive structure on cell boundaries.

## Spec sites

- `docs/spec/` §"Foundational commitment: every variable is a list" lines 31-58 — authoritative for psh's framing and the type-system extension.
- `docs/spec/` §"rc's execution model as sequent calculus", **List-valued variables** bolded item (lines 66-72) — quotes Duff directly. (This is a bolded sub-bullet inside the larger section, not a standalone subsection.)
- `docs/spec/` line 56: "This is Duff's principle extended across the type system: the list structure is carried as data, never destroyed and reconstructed."
- `decision/every_variable_is_a_list` — design decision memo (the load-bearing decision this anchor grounds theoretically).
- `decision/parameter_expansion_destructors` — `$#x` and `$"x` are the type-specific eliminators on the List layer that follow from the principle.
- `decision/three_roles_of_parens` — list / tuple / tagged-construction, where list is the rc-heritage form.

## Status

Settled. Foundational. The principle is non-negotiable; every other psh design decision assumes it. When asked "why doesn't psh have scalar variables", this is the citation: there is no separate scalar type because Bourne's scalar/list distinction was the source of the quoting-ceremony pathology Duff explicitly designed rc to eliminate, and psh extends that elimination across the typed extension.
