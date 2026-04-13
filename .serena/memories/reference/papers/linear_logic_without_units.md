---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [houston, linear-logic, unitless, mll, multiplicative, promonoidal-category, tensor, par, unit-free]
agents: [vdc-theory, psh-sequent-calculus]
---

# Reference: Linear Logic Without Units (Houston)

**Path.** `/Users/lane/gist/linear-logic-without-units.gist.txt`

**Citation.** Houston, *Linear Logic Without Units*, thesis (arXiv:1305.5032).

**Status.** Theoretical reference for unitless multiplicative linear logic (MLL).

## Summary

Studies promonoidal categories as models for multiplicative linear logic without multiplicative units. The tensor (⊗) and par (⅋) structure without units is the fragment that applies to psh's unit-free type system.

psh **has no unit types** — the type system is unit-free, which brings it into the unitless MLL fragment. Houston's thesis provides the categorical machinery for reasoning about this fragment.

## Concepts it informs in psh

- **`decision/every_variable_is_a_list`** — no unit type; lists of length 0 are `()` (the empty list), not a separate Unit.
- **`docs/spec/`** implicitly unitless — no Unit type in the value model.
- **⊕/⅋ error duality** — when the linear side of the duality is relevant, the unitless MLL fragment is the correct framework.
- **Coprocess channels** — session types as linear propositions sit in this fragment.

## Who consults it

- **vdc-theory agent** (primary): for the unit-free MLL fragment when the linear side of the framework is in play.
- **sequent calculus agent** (primary): for ⊕/⅋ duality when the linear side is relevant.

## Low-confidence rejection note

If asked a question about units (which psh doesn't have), this paper is the authority for "why unit-free is consistent." If the question is about non-linear structure, Houston may not apply — say so.
