---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
verified_against: [docs/spec/@HEAD §".get — the pure observer" line 627, /Users/lane/gist/classical-notions-of-computation-duploids.gist.txt:8550-8555]
keywords: [cbv-focusing, thunkability, thunkable-central, proposition-8550, hasegawa-thielecke, duploid, reentrancy, fire-once-per-expression, pure-get, not-downen-static-focusing]
agents: [vdc-theory, psh-sequent-calculus, psh-optics-theorist]
supersedes: [analysis/polarity/cbv_focusing@pre-2026-04-11 (the "Downen static focusing" framing)]
extends: analysis/polarity/_hub
related: [analysis/polarity/frames, analysis/polarity/shifts, analysis/hasegawa_thielecke, decision/codata_discipline_functions]
---

# CBV focusing as reentrancy semantics — via thunkability

## Concept

Within a single expression, a pure `.get` discipline fires at
most once per variable, and its produced value is shared at
every subsequent consumption site. In psh this is a **theorem**,
not an operational convention:

**Proposition (Duploids [2] §7, line 8550):** Every thunkable
map is central.

Pure maps into the positive subcategory `P_t` are thunkable by
construction. Central maps may be reused at every consumption
site inside an expression without disturbing composition order.
Therefore CBV argument expansion evaluates `.get` once and
shares the result.

## What this is NOT

It is **not Downen et al.'s static focusing**. Downen's static
focusing is a **syntactic preprocessing pass** that rewrites a
program before evaluation to eliminate stuck terms in `L̄`; see
Grokking [7] §5.2 ("Focusing on Evaluation in Core", Def.
11178). psh does not run that pass. What psh does is plain CBV
evaluation of argument lists combined with the thunkability
theorem, which gives reuse as a categorical consequence, not as
a rewriting outcome.

The prior version of this memo cited Downen static focusing as
the justification. This was the spec's original framing and it
was incorrect: the framework document `docs/vdc-framework.md`
§6.2 references Downen for a related point (argument focusing
before command execution), but the runtime reuse property for
`.get` is a thunkability fact, not a focusing fact.

## What this is NOT (second error corrected)

It is also **not** an appeal to the full Hasegawa-Thielecke
theorem "thunkable = central." Hasegawa-Thielecke proves the
biconditional in a **dialogue duploid** (a duploid with
involutive negation), and psh has no involutive negation.
psh cites only the **forward direction** (thunkable ⇒ central),
which holds in every symmetric monoidal duploid and does not
require dialogue structure.

## What this IS

A straightforward consequence of two facts:

1. **`.get` bodies are pure** in the Option B design (committed
   2026-04-11). The body has type `W(S, A)` — no effects.
2. **Pure maps into `P_t` are thunkable.** Duploids §7 line 7643
   and §"Thunkable and central maps" lines 7554-8555. Pure
   value-producing computations trivially satisfy the
   thunkability equation.

Combined with Prop 8550, a pure `.get` is central, and a central
map can be reused at multiple sites in an expression. CBV
argument expansion does the actual reuse mechanically; the
theorem guarantees the reuse is semantically safe.

## Why Option B simplified this

Under the prior effectful-`.get` design (pre-2026-04-11),
`.get` bodies lived in `Kl(Ψ)`. Kleisli computations are not
automatically thunkable — psh's Ψ is non-commutative, so most
effectful `.get` bodies fail the thunkability equation. The
spec's appeal to "fire once per expression" therefore had no
categorical backing under the prior design; it was operational
memoization justified by nothing stronger than "we do it this
way."

Option B (pure `.get`, effectful `.refresh`) eliminates the gap.
Pure `.get` is thunkable by construction, and Prop 8550 gives
reuse as a theorem. The justification is structural, not
operational.

## Foundational refs

- `reference/papers/duploids` — Mangel, Melliès, Munch-Maccagnoni.
  **Proposition at line 8550** ("every thunkable map is
  central") is the load-bearing citation. Read after the §1
  "Emergence of non-associativity" section.
- `reference/papers/grokking_sequent_calculus` — Binder et al.
  For what static focusing actually is (so you can cite it
  correctly when it IS the right mechanism elsewhere in psh).
- `docs/vdc-framework.md` §6.2 "The Sequent Calculus as the
  Type Theory of Shell" — the framework's own citation of
  static focusing, which applies to argument-list pre-
  evaluation generally, not to `.get` reuse.

## Spec sites

- `docs/spec/` §".get — the pure observer" (line
  627) — authoritative citation of Prop 8550.
- `docs/spec/` §"Theoretical framework §The
  semantics" (line 202) — the narrowed Hasegawa-Thielecke
  citation.
- `decision/codata_discipline_functions` — the design decision
  this anchor supports.

## Supersession note

Prior memo (pre-2026-04-11) framed CBV focusing as "Downen et
al.'s static focusing made operational" in the spec. The 2026-
04-11 roundtable identified two errors in that framing:

1. Downen static focusing is a syntactic rewrite pass, not a
   runtime mechanism. The name was applied to the wrong thing.
2. Even if the name were right, effectful `.get` bodies (the
   prior design) cannot be reused without appealing to a
   theorem psh cannot cite (the full Hasegawa-Thielecke, which
   requires dialogue-duploid structure).

Both errors are fixed by narrowing `.get` to pure (Option B)
and citing only the forward direction of thunkable ⇒ central.

## Status

Settled 2026-04-11. `.get` reuse is a theorem from Prop 8550;
cross-expression consistency is user-controlled via explicit
`.refresh` invocations, not a documented caveat.
