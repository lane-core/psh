---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [codata, discipline, get, set, observer, constructor, cbv-focusing, monadic-lens, kleisli, reentrancy, polarity-frame]
agents: [psh-optics-theorist, psh-sequent-calculus, vdc-theory, plan9-systems-engineer, psh-architect]
related: [decision/def_vs_lambda, decision/postfix_dot_accessors]
supersedes: [pre-7afc97d conservative model where .get was "return discarded, side effects only"]
verified_against: [git-log@7afc97d, git-log@6fbac31]
---

# Decision: codata discipline functions with CBV focusing as reentrancy semantics

## Decision

A psh variable with `.get` and `.set` disciplines is **codata** in the sequent-calculus sense:

- **`.get`** is the codata observer — the body **computes** the value seen by the accessor.
- **`.set`** is the codata constructor — the body receives the incoming value as `$1` and mediates the assignment (validate, transform, reject, propagate).

Both are `def` cells in Kl(Ψ) — the shell's effect monad. Together they form a **MonadicLens** (`def:monadiclens` from Clarke et al.).

**CBV focusing** is the reentrancy semantics: within a single expression, `.get` fires at most once per variable. Subsequent references reuse the produced value.

## Why

**Supersession note.** This decision supersedes the earlier **conservative model** (commit `6fbac31`): ".get as a def with 'return discarded, x free in body, side effects for logging only' constraints." The conservative model forced workarounds like `cursor.refresh` to actually compute values. Commit `7afc97d` tightened the design to let `.get` compute values directly — the codata observer view.

The codata framing comes from the sequent calculus / duploid semantics: data types are defined by constructors and eliminated by pattern matching; codata types are defined by destructors (observers) and eliminated by copattern matching. A variable with disciplines is genuinely codata — its behavior under observation is what the disciplines compute, not what a stored slot contains.

CBV focusing (Downen et al.'s static focusing, per `docs/vdc-framework.md` §6.2) is the correct reentrancy semantics. It's not memoization as optimization — it's the focusing discipline of the focused sequent calculus realized at the polarity boundary: once the `.get` shift lands a value in W, that value is used at each consumption site within the enclosing expression.

Alternative considered: aggressive memoization or no memoization. Rejected — CBV focusing is the theoretically correct answer that falls out of the calculus.

## Consequences

- `.get` bodies may have arbitrary effects: logging, coprocess queries, filesystem reads, metric emission.
- `.set` bodies may validate (reject via nonzero return), transform (clamp, normalize), reject, or propagate.
- `echo $cursor $cursor` fires `.get` **once** — the value computed for the first `$cursor` is reused for the second.
- Across **separate expressions** (e.g., on the next line), `.get` fires fresh — cross-expression consistency is not guaranteed.
- **Polarity frame** protects the surrounding expansion context from computation-mode intrusion. Within the body of `x.get`, a reference to `$x` returns the stored value directly, bypassing the discipline (reentrancy guard).
- The MonadicLens laws (PutGet, GetPut, PutPut) become **contracts the user must maintain**, not automatic consequences. Ordinary variables without disciplines get the laws for free.

Spec: `docs/specification.md` §"Discipline functions" (the codata model, CBV focusing as reentrancy, MonadicLens structure, reentrancy and the polarity frame, cross-variable consistency caveat). Ledger: `docs/deliberations.md` §"`.get` discipline: codata model (SUPERSEDED → APPLIED)".
