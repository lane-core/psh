---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [cut, execution, command-formation, t-cut-e, mu-tilde, sequent-calculus, command-template, pipe-as-cut, four-sorts]
agents: [psh-sequent-calculus, vdc-theory, psh-architect]
related: [analysis/three_sorts, analysis/oblique_maps, analysis/cbpv_f_u_separation, decision/let_is_mu_tilde_binder_cbpv]
verified_against: [docs/specification.md@HEAD §302-340, docs/vdc-framework.md@HEAD §3.2 lines 207-232, audit/psh-sequent-calculus@2026-04-11]
---

# Cut as execution (⟨t | e⟩)

## Concept

In the λμμ̃-calculus, a **command** is formed by cutting a producer (term `t`) against a consumer (coterm `e`): `⟨t | e⟩`. The cut is the execution rule of the calculus — running a command means **reducing the cut**.

The typing rule (`docs/vdc-framework.md` §3.2 lines 209–213):

```
Γ ⊢ t : A       Γ, x : A ⊢ c
────────────────────────────────
       Γ ⊢ ⟨t | μ̃x.c⟩
```

In the call-by-value reading, evaluate `t` first, bind its result to `x`, then run the continuation `c` with the binding (`docs/vdc-framework.md` line 215).

In psh, every executable construct is a cut. The spec gives the table at `docs/specification.md` §"Commands (cuts) — ⟨t | e⟩" lines 309–315:

| psh construct | Cut structure |
|---|---|
| `echo hello` | ⟨hello \| stdout + continuation⟩ |
| `cmd1 \| cmd2` | ⟨cmd1-stdout \| cmd2-stdin⟩ (pipe = cut) |
| `x = val` | ⟨val \| μ̃x.rest⟩ (assignment as μ̃-binder) |
| `if(cond) { A } else { B }` | ⟨status \| case(A, B)⟩ |
| `match(v) { arms }` | ⟨v \| case(arm₁, ..., armₙ)⟩ |

The pipe operator `|` is **literally the cut** at the operational level. Variable binding is the cut against a μ̃-binder. Conditionals are cuts against case eliminators. The sequent calculus gives a uniform structural account of every form of execution in the shell.

In psh's AST (`docs/specification.md` §"The AST's four sorts" line 321), the `Command` sort handles cuts and control flow (Exec, If, For, Match, Try, Trap), and the `Binding` sort handles μ̃-binders (Assignment, Cmd, Let). Together they form the cut-and-bind layer of the calculus. The `Word`/`Value` sort is the producer side (terms Γ), and the `Expr` sort is the engineering coterm layer (pipelines, redirections, profunctor maps) — `Expr` is "an engineering choice, not a logical one" per spec line 333.

## Foundational refs

- `docs/vdc-framework.md` §3.2 "Cut as Execution" lines 207–232 — psh's framing of cut as the execution rule, with the λμμ̃ typing rule and the rc cut examples (`who | wc` as cut, variable substitution as cut).
- `reference/papers/grokking_sequent_calculus` — Binder et al. introduce cut as the execution rule of Core (the sequent-calculus IL) in the Fun→Core compilation.
- `reference/papers/dissection_of_l` — Spiwack treats cut structurally in System L.

## Spec sites

- `docs/specification.md` §"Commands (cuts) — ⟨t | e⟩" line 302 — authoritative for psh's cut catalog.
- `docs/specification.md` §"The AST's four sorts" line 321 — `Command` and `Binding` sorts realize cut and μ̃ respectively.
- `analysis/three_sorts` — the surrounding three-sort structure cut lives in.
- `analysis/cbpv_f_u_separation` — `let` as μ̃-binder on `F(A)` is a cut against an effectful producer.
- `analysis/oblique_maps` — every shell command has the structure of an oblique map, which is what cuts produce.

## Status

Settled. Foundational. When explaining "what does psh actually do when it runs a command", the answer is "reduces a cut".
