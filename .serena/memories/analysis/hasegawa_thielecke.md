---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [hasegawa-thielecke, thunkable, central, dialogue-duploid, fuhrmann, mellies, tabareau, theorem, attribution, classical-control, continuation-monad, idempotent, commutative]
agents: [vdc-theory, psh-sequent-calculus]
related: [reference/papers/duploids, analysis/polarity/duploid_composition, analysis/cbpv_f_u_separation, analysis/classical_control_mu_binder]
verified_against: [/Users/lane/gist/classical-notions-of-computation-duploids.gist.txt@HEAD lines 9512-9612, 6863, 7891-7906, 9608, audit/vdc-theory@2026-04-11]
---

# Hasegawa-Thielecke theorem

## Concept

The **Hasegawa-Thielecke theorem** (duploids paper §"The Hasegawa-Thielecke theorem", `~/gist/classical-notions-of-computation-duploids.gist.txt:9512`):

> "In a dialogue duploid, a morphism is central for ⊗ if and only if it is thunkable." (Theorem at gist lines 9520-9523.)

**Thunkable** means the map can be expressed as the thunk of a value computation — operationally, it can be "frozen" without observable effect on its surrounding context. **Central** means the map commutes with all other maps under the tensor product — operationally, it has no side-effects-on-context that interact with others. The theorem says these two notions coincide in a dialogue duploid (a duploid equipped with involutive negation, i.e., classical control).

The corresponding categorical-level corollary (gist lines 9610-9612): "The continuation monad of a dialogue category is commutative if and only if it is idempotent." This statement is **attributed in the duploids paper at gist line 9608 to Hasegawa via `melliestabareau`**.

**Attribution note.** The theorem was previously called "Führmann-Thielecke" in `docs/specification.md` line 211 — that was an upstream mis-attribution caught during the 2026-04-11 tier-1 audit. The duploids paper macro `\FH` at gist line 6863 expands unambiguously to "Hasegawa-Thielecke", and every use in the paper (lines 6917, 7891, 7896, 9514) follows that. **Führmann is credited at gist lines 7903-7906** for the related but distinct work distinguishing thunkability from centrality (`Fuhrmann2000PhD`, `fuhrmanndirectmodels`), but the theorem proper is Hasegawa-Thielecke. The spec was corrected to match.

**Why psh cares.** psh's coproduct of effectful and pure values relies on the thunkable=central characterization to know when a value can be safely treated as pure. Pure values (lambdas, builtin returns of pure data) are thunkable; their centrality means they can be substituted into any expansion context without interacting with surrounding effects. The theorem is the proof-theoretic justification for psh's CBPV F/U separation (`analysis/cbpv_f_u_separation`) being meaningful in the presence of classical control (`analysis/classical_control_mu_binder`) — which psh has via lexical `trap` as the μ-binder.

## Foundational refs

- `reference/papers/duploids` — Mangel, Melliès, Munch-Maccagnoni. The full statement and proof at `~/gist/classical-notions-of-computation-duploids.gist.txt` §"The Hasegawa-Thielecke theorem" lines 9512-9580. The "syntactic Hasegawa-Thielecke theorem" version is at line 9556-9580. Detailed proof via internal duality at lines 9527-9542.
- The theorem itself is attributed in the paper to Hasegawa via Mellies-Tabareau (line 9608). Führmann is credited for related work on the conceptual distinction (lines 7903-7906) but not for the theorem.
- `analysis/polarity/duploid_composition` — the surrounding duploid composition law machinery this theorem fits into.

## Spec sites

- `docs/specification.md` §"Theoretical framework §The semantics" line 211 — citation in psh's framework section. **Corrected from "Führmann-Thielecke" to "Hasegawa-Thielecke" on 2026-04-11** (audit pass).
- `docs/vdc-framework.md` — does not cite the theorem directly (verified by grep on 2026-04-11).
- `analysis/polarity/duploid_composition` — consumes the theorem as part of the duploid composition law treatment.

## Status

Settled. The corrected attribution stands. If a future contributor sees "Führmann-Thielecke" anywhere in psh materials, they should treat it as a stale reference and correct it to "Hasegawa-Thielecke." The standalone anchor exists for retrieval purposes when an agent needs the theorem statement and provenance directly without reading the surrounding `analysis/polarity/duploid_composition` context.
