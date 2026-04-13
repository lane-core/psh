---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-11
importance: high
keywords: [codata, discipline, get, refresh, set, observer, updater, mutator, pure-view, cbv-focusing, thunkability, monadic-lens, mixed-optic, kleisli, reentrancy, polarity-frame]
agents: [psh-optics-theorist, psh-sequent-calculus, vdc-theory, psh-session-type-agent, plan9-systems-engineer, psh-architect]
related: [decision/def_vs_lambda, decision/postfix_dot_accessors, analysis/polarity/cbv_focusing, analysis/polarity/frames, analysis/monadic_lens, analysis/polarity/_hub, analysis/data_vs_codata]
supersedes: [decision/codata_discipline_functions@pre-2026-04-11 (the "effectful .get as codata observer" design from commit 7afc97d)]
verified_against: [docs/spec/@HEAD §"Discipline functions" line 562-806, git-log@7afc97d, git-log@6fbac31, roundtable-2026-04-11]
---

# Decision: three-discipline codata model with pure `.get`, effectful `.refresh`, mutating `.set`

## Decision

A psh variable may be equipped with **three discipline cells**
that together make it **codata** in the sequent-calculus sense:

- **`.get`** — the **pure observer**. Body of type `W(S, A)`
  that reads the stored slot and returns a value. No effects
  allowed. Default: identity slot reader. User-defined `.get`
  must remain pure.
- **`.refresh`** — the **effectful updater**. Body in `Kl(Ψ)`
  that may invoke arbitrary shell machinery (subshells,
  coprocess queries, filesystem reads) and writes the updated
  value to the stored slot. Invoked as an imperative command
  at a step boundary: `cursor.refresh; echo $cursor`.
- **`.set`** — the **mutator**. Body that receives the incoming
  value as `$1`, mediates the assignment, and writes the slot
  via the primitive `x = v` inside its polarity frame.

Together `.get` and `.set` form a **mixed monadic lens** per
Clarke `def:monadiclens`: pure view in `W(S, A)`, effectful
update in `W(S × B, ΨT)`. `.refresh` is orthogonal — an
imperative write from outside the lens framework.

The cocase presentation:

    cocase{ get(α)     ⇒ ⟨.get-body | α⟩,
            refresh(α) ⇒ ⟨.refresh-body | α⟩,
            set(v; α)  ⇒ ⟨.set-body[v] | α⟩ }

All three cells are **destructors** of the codata type per
Grokking §6.3. The cocase is the sole constructor — the
variable *is* its cocase. Earlier drafts calling `.set` a
"constructor" were terminologically incorrect.

## Why

**Heritage.** The observation/refresh split is a return to rc's
filesystem-backed observation philosophy (`rc.ms` §Environment):
observation is a read, mutation is an imperative step, and the
shell's reference model never hides work behind a variable
reference. Plan 9 realized this via `/env`; psh realizes the
same philosophy on contemporary unix-likes using whatever
filesystem or IPC mechanism the user chooses. ksh93 collapsed
observation and refresh by making `get` effectful; psh declines
that collapse.

**Theory.** Pure `.get` is thunkable by construction. Thunkable
maps are central (Duploids Prop 8550, the forward direction of
Hasegawa-Thielecke). Central maps may be reused at every
consumption site inside an expression without disturbing
composition order — so CBV argument expansion shares the result
of `.get` across all occurrences of the variable in the same
expression **as a theorem**, not as operational memoization.
The reverse direction (central ⇒ thunkable) requires dialogue-
duploid structure that psh does not commit to; we only need the
forward direction.

**Soundness.** Effectful `.get` querying a coprocess races with
drop-as-cancel on polarity frame unwinding: a signal unwinding
the frame while `read -p` is blocked can leave the coprocess
with a stale request whose tag is later reused, violating the
per-tag binary session `Send<Req, Recv<Resp, End>>`. Option B
(`.refresh` + pure `.get`) eliminates this failure mode because
pure `.get` cannot open sessions.

**Composition laws.** Mixed monadic lenses compose cleanly via
the Tambara-module framework and the central-preserving
inclusion `ι : P_t ↪ Kl(Ψ)`. Non-mixed Kleisli lenses (where
both sides live in `Kl(Ψ)`) compose only premonoidally and
require commutative Ψ for law preservation — psh's Ψ is
non-commutative (fork order, coprocess message order).

**Operational simplicity.** With `.get` pure:

- No polarity frame around `.get` — nothing to reenter.
- CBV focusing is a theorem (thunkability), not an ad-hoc
  operational rule.
- The §"Known caveat: cross-variable consistency" section from
  the prior spec disappears — it was necessary only because the
  prior effectful-`.get` design had inconsistencies the type
  system could not rule out.

## Supersession chain

1. **Pre-7afc97d conservative model** (commit 6fbac31): `.get`
   as a `def` with "return discarded, x free in body, side
   effects for logging only" constraints. Required a separate
   `.refresh` for value computation. Retired for ergonomic
   reasons.
2. **7afc97d effectful-.get model**: `.get` body computes the
   value directly, may have arbitrary effects. Matched ksh93
   ergonomics at the cost of an optic-class contradiction with
   Clarke's `def:monadiclens` and the soundness hole session-
   type agent later identified.
3. **Current (2026-04-11 Option B)**: Three cells. `.get` pure,
   `.refresh` effectful updater, `.set` mutator. Correctly
   instantiates Clarke's mixed monadic lens. The `.refresh`
   cell is reinstated from the conservative model but paired
   with a genuinely useful pure `.get` (not "return discarded")
   and with the Prop 8550 justification for CBV reuse.

Each supersession resolves a different problem with its
predecessor. The roundtable at 2026-04-11 resolved it by
consensus of four theory agents (optics, sequent-calculus, vdc,
session-type) plus plan9 heritage check.

## Consequences

- **`.get`** bodies are pure. Default is identity slot reader;
  custom `.get` is a pure derived view.
- **`.refresh`** is invoked explicitly as `varname.refresh` at
  a step boundary. Failure propagates as `$status` and composes
  with `try`/`catch` and `trap` normally.
- **`.set`** bodies may have effects; they own the slot write
  via the primitive `x = v` inside the polarity frame.
- **Polarity frames** narrow to `.refresh` and `.set`. `.get`
  has no frame — there is nothing to reenter.
- **The monadic lens laws** (PutGet, GetPut, PutPut) are user
  contracts stated up to `Kl(Ψ)`-equality. Ordinary variables
  without discipline cells satisfy them unconditionally.
- **Cross-variable consistency** across expressions is under
  user control via explicit `.refresh` invocations, not a
  caveat emergent from CBV focusing.
- **ksh93 migration.** Existing ksh93 `get`/`set` discipline
  functions port to psh by splitting effects out of `.get` into
  `.refresh`. The rewritten call site reads `varname.refresh;
  $varname` or wraps the pair in a user function.

## Spec

- `docs/spec/` §"Discipline functions" (line 562)
- `docs/spec/` §"Polarity frames" (line 414)
- Decision history is in git.
  (SUPERSEDED → APPLIED)" — ledger entry; may need a new
  supersession note for the 2026-04-11 rewrite.
