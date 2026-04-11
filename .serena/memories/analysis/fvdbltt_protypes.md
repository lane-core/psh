---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [fvdbltt, hayashi, das, protypes, virtual-double-category, type-theory, classifying-vdc, channel-types, dependent-session-types, v2-typed-pipes, proterms, loose-arrows, biadjunction]
agents: [vdc-theory, psh-session-type-agent]
related: [reference/papers/logical_aspects_vdc, analysis/wire_format_horizontal_arrow, analysis/decision_procedure_8_5, reference/papers/fcmonads]
verified_against: [/Users/lane/gist/logical-aspects-of-vdc.gist.txt@HEAD §FVDTT/newproof.tex lines 1-152, audit/vdc-theory@2026-04-11]
---

# FVDblTT protypes (type theory of VDCs)

## Concept

**FVDblTT** (Fibrational Virtual Double Type Theory) is the type theory of virtual double categories developed by Hayashi, Das et al. in *Logical Aspects of Virtual Double Categories* (`~/gist/logical-aspects-of-vdc.gist.txt`). It gives a syntactic presentation of VDCs in which:

- **Tight arrows** (vertical arrows of the VDC) correspond to **terms** (term substitutions). A tight arrow `Γ → Δ` is a sequence of terms `Γ ⊢ s₁ : J₁, …, sₙ : Jₙ` modulo equality (gist lines 13-16).
- **Loose arrows** (horizontal arrows) correspond to **protypes**. A loose arrow `Γ ↛ Δ` is a protype judgment `Γ | Δ ⊢ α protype` modulo equality (gist lines 17-19).
- **Cells** correspond to **proterms**. A cell mediating between top horizontal arrows and a bottom one is a proterm `Γ̄ | a₁:α₁, …, aₙ:αₙ ⊢ μ : β[…]` modulo equality (gist lines 20-55).

The central construction is the **classifying virtual double category** `S(Σ, E)` for a specification `(Σ, E)` (gist line 11) — the syntactic VDC built from a signature of objects, terms, protypes, and equality judgments. The biadjunction between the 2-category of VDCs and the 2-category of FVDblTT specifications gives the standard "syntax/semantics" correspondence. The theorem at gist lines 146-152 establishes that the assignment from CFVDC to specification "extends to a functor `Sp : FibVDblCart^spl → Speci` which is a right adjoint to `nS`."

**Why psh cares (forward-looking, v2).** psh's coprocess channels are horizontal arrows of a VDC at the operational level (`analysis/wire_format_horizontal_arrow`). At the type level, the wire format and per-tag binary sessions are *informal* protype-like constructions — the actual session type lives in Rust phantom types, not in a typed-syntactic frame. **Future v2 work on typed pipes and dependent session types would benefit from FVDblTT as the type-theoretic ground**: protypes give a uniform syntactic account of "channel types parameterized by their endpoints" (the loose arrow), and proterms give a uniform account of "communication patterns between channels" (the cells). The biadjunction means anything provable in FVDblTT has a semantic interpretation in the VDC of psh's actual channels.

This anchor is **forward-looking** — nothing in v1 depends on FVDblTT. The reference exists so v2 design discussions can dispatch the session-type or vdc-theory agent and immediately have a starting point for "what type theory should typed pipes use."

## Foundational refs

- `reference/papers/logical_aspects_vdc` — Hayashi, Das et al. *Logical Aspects of Virtual Double Categories.* The classifying VDC construction is at `~/gist/logical-aspects-of-vdc.gist.txt:1-200`. Loose arrows = protypes is at lines 17-19; cells = proterms is at lines 20-55; the syntax/semantics biadjunction at lines 146-152.
- `reference/papers/fcmonads` — Cruttwell-Shulman §2 (line 2312) defines VDCs; FVDblTT is the type theory whose semantics live in those VDCs. The two papers are complementary: fcmonads gives the categorical structure, FVDblTT gives the syntactic presentation.

## Spec sites

The spec does not cite FVDblTT directly. v1 psh has no protypes — the typed-pipe extension is deferred per `PLAN.md` §"Open items from deliberations.md". This anchor is a **forward link** for v2 design work.

- `analysis/wire_format_horizontal_arrow` — v1 horizontal-arrow discipline that v2 protypes would generalize.
- `analysis/decision_procedure_8_5` — the §8.5 classifier whose treatment of new features would gain a syntactic frame under FVDblTT.

## Status

**Forward-looking / v2-pending.** Nothing in v1 depends on this anchor. Write-up exists so the next session that touches typed pipes or dependent session types has a starting point. If v2 starts with a different type-theoretic frame (e.g., a refinement-types approach via `reference/papers/practical_refinement_session`), supersede this anchor with the actual choice.
