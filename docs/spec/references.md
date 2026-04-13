# psh Annotated Bibliography

Central bibliography for the psh project. Every citation key
used in code doc comments (`[Key]`) resolves to an entry here.
Citation rules: `STYLEGUIDE.md` §Theoretical Citations.
Full rationale: serena `policy/citation_workflow`.

## Index

| Key | Alias | Reference |
|-----|-------|-----------|
| `[BTMO23]` | | Binder et al. — Grokking the Sequent Calculus |
| `[Bur71]` | | Burroni — T-catégories |
| `[CBG24]` | `[Clarke]` | Clarke, Boisseau, Gibbons — Profunctor Optics |
| `[CH00]` | | Curien, Herbelin — The Duality of Computation |
| `[CMM10]` | | Curien, Munch-Maccagnoni — Duality Under Focus |
| `[CMS]` | | Carbone, Marin, Schürmann — Async Multiparty Compat. |
| `[CS10]` | | Cruttwell, Shulman — Generalized Multicategories |
| `[Duf90]` | | Duff — Rc, The Plan 9 Shell |
| `[HVK98]` | | Honda, Vasconcelos, Kubo — Session Types |
| `[Lei04]` | | Leinster — Higher Operads, Higher Categories |
| `[Lev04]` | | Levy — Call-by-Push-Value |
| `[MMM]` | `[Duploids]` | Mangel, Melliès, Munch-Maccagnoni — Duploids |
| `[Mun13]` | | Munch-Maccagnoni — Non-Assoc. Composition (thesis) |
| `[Mun14]` | | Munch-Maccagnoni — Models of Non-Assoc. Composition |
| `[Spi14]` | `[SPW]` | Spiwack — A Dissection of L |
| `[Wad14]` | | Wadler — Propositions as Sessions |

---

## Entries

### `[BTMO23]`

David Binder, Marco Tzschentke, Marius Müller, and Klaus
Ostermann. "Grokking the Sequent Calculus (Functional Pearl)."
*Proc. ACM Program. Lang.* 8, ICFP, pp. 395–425, 2024.
DOI: [10.1145/3674639](https://doi.org/10.1145/3674639).

**Annotation.** Accessible introduction to the sequent calculus
and the λμμ̃-calculus with a programming-languages audience in
mind. psh uses BTMO23's presentation as the bridge between the
Curien-Herbelin formalism and the implementation-facing sort
classification (producers, consumers, commands). The "three
sorts" framing in psh's spec draws directly on this paper's
exposition.

---

### `[Bur71]`

Albert Burroni. "T-catégories (catégories dans un triple)."
*Cahiers de Topologie et Géométrie Différentielle
Catégoriques* 12(3), 1971, pp. 215–321.

**Annotation.** Original source for the structure later named
"virtual double category." Burroni's "multicatégorie" is the
same structure Leinster calls "fc-multicategory" and
Cruttwell-Shulman call "virtual double category." Cited for
historical priority and to make the terminological lineage
explicit.

---

### `[CBG24]`

**Alias:** `[Clarke]`

Bryce Clarke, Derek Elkins, Jeremy Gibbons, Fosco Loregian,
Bartosz Milewski, Emily Pillmore, and Mario Román.
"Profunctor Optics, a Categorical Update."
*Compositionality* 6, 2024.
DOI: [10.32408/compositionality-6-1](https://doi.org/10.32408/compositionality-6-1).

**Annotation.** Primary reference for psh's profunctor optics
framework. Provides the formal definitions that psh's accessor
system realizes: MonadicLens (`def:monadiclens`), mixed optic
structure, the profunctor hierarchy (Lens, Prism,
AffineTraversal, Traversal), and the power-series action for
traversals (`def:traversal`). The spec's §Mixed-optic structure
and §Redirections as profunctor maps cite this paper at the
level of specific definitions. Clarke's taxonomy of mixed optics
(where decomposition and reconstruction categories differ) is
load-bearing for psh's discipline function semantics: `.get` is
pure (in W), `.set` is effectful (in Kl(Ψ)), making the
disciplined variable a monadic lens by Clarke's
`prop:monadiclens`.

---

### `[CH00]`

Pierre-Louis Curien and Hugo Herbelin. "The Duality of
Computation." In *Proceedings of the Fifth ACM SIGPLAN
International Conference on Functional Programming (ICFP)*,
2000, pp. 233–243.
DOI: [10.1145/351240.351262](https://doi.org/10.1145/351240.351262).

**Annotation.** Foundational paper for psh's type theory. The
λμμ̃-calculus introduced here provides psh's three-sorted
structure: producers (μ̃-terms / values / words), consumers
(μ-terms / continuations / redirect targets), and commands
(cuts ⟨t | e⟩ / pipelines and simple commands). The spec's
§rc's execution model as sequent calculus maps rc constructs
directly onto the Curien-Herbelin sorts.

---

### `[CMM10]`

Pierre-Louis Curien and Guillaume Munch-Maccagnoni. "The
Duality of Computation Under Focus." In *IFIP TCS 2010:
Theoretical Computer Science*, IFIP AICT vol 323,
pp. 165–181, 2010.
DOI: [10.1007/978-3-642-15240-5_13](https://doi.org/10.1007/978-3-642-15240-5_13).

**Annotation.** Extends CH00 with focusing. psh's CBV focusing
discipline for discipline function reentrancy (within one
expression, `.get` fires once per variable) draws from the
focused evaluation strategy described here. The interaction
between μ and μ̃ binders under focusing informs the critical
pair analysis in the spec.

---

### `[CMS]`

Marco Carbone, Sonia Marin, and Carsten Schürmann. "A Logical
Interpretation of Asynchronous Multiparty Compatibility."
In *LOPSTR 2023*, LNCS 14330, pp. 99–117, 2023.
DOI: [10.1007/978-3-031-45784-5_7](https://doi.org/10.1007/978-3-031-45784-5_7).

**Annotation.** Proves the MCutF admissibility theorem:
forwarders subsume classical coherence and capture all
multiparty compatible compositions. This is the load-bearing
justification for psh's star topology for coprocesses — the
shell as hub, each coprocess as a spoke, with the forwarder
result guaranteeing that the star topology is deadlock-free
under the conditions the spec establishes (per-tag binary
sessions, no inter-coprocess channels).

---

### `[CS10]`

Geoffrey S. H. Cruttwell and Michael A. Shulman. "A Unified
Framework for Generalized Multicategories." *Theory and
Applications of Categories* 24(21), 2010, pp. 580–655.

**Annotation.** Introduces virtual double categories under their
current name, unifying Burroni's multicatégories, Leinster's
fc-multicategories, and related structures. psh's VDC framework
(`docs/vdc-framework.md`) uses Cruttwell-Shulman's formulation
as the categorical backbone: objects are shell contexts, vertical
morphisms are scope operations, horizontal morphisms are I/O
channels, and cells are shell commands. The composition-only-
when-earned discipline (virtual = composition not always defined)
is the key architectural principle.

---

### `[Duf90]`

Tom Duff. "Rc — The Plan 9 Shell." *Plan 9 Programmer's
Manual*, Bell Laboratories, 1990.

**Annotation.** psh's direct ancestor. Duff's no-rescan
invariant ("input is never scanned more than once") is the
founding principle of psh's type theory: substitution is
structure-preserving, the list is the fundamental data type,
and quoting is syntax rather than semantics. The spec's §rc's
execution model as sequent calculus begins with Duff's design
and discovers the three-sorted structure latent in rc. Every
psh design decision that touches variables, substitution, or
quoting traces back to this paper.

---

### `[HVK98]`

Kohei Honda, Vasco T. Vasconcelos, and Makoto Kubo. "Language
Primitives and Type Discipline for Structured Communication-
Based Programming." In *ESOP '98: European Symposium on
Programming*, LNCS 1381, pp. 122–138, 1998.
DOI: [10.1007/BFb0053567](https://doi.org/10.1007/BFb0053567).

**Annotation.** Foundational paper for binary session types.
psh's coprocess protocol uses per-tag binary sessions
multiplexed over a single socketpair — Honda et al.'s binary
session discipline applied at the granularity of individual
request-response tags. The spec cites HVK98 for the session-
type framing of the coprocess wire protocol.

---

### `[Lei04]`

Tom Leinster. *Higher Operads, Higher Categories.* London
Mathematical Society Lecture Note Series 298, Cambridge
University Press, 2004.

**Annotation.** Introduces fc-multicategories, the structure
Cruttwell-Shulman later named "virtual double category."
Leinster's perspective on composition-as-structure (rather
than composition-as-given) is the conceptual ancestor of
psh's "composition only when earned" principle. Cited for
terminological lineage alongside Burroni and
Cruttwell-Shulman.

---

### `[Lev04]`

Paul Blain Levy. *Call-by-Push-Value: A Functional/Imperative
Synthesis.* Semantics Structures in Computation, vol 2,
Springer, 2004.
DOI: [10.1007/978-94-007-0954-6](https://doi.org/10.1007/978-94-007-0954-6).

**Annotation.** psh's `let` binding is the CBPV thunk-force
discipline: `let x = M` where `M : F(A)` is the μ̃-binder on
monadic bind. Pure values are a trivial special case. The
value/computation distinction (Γ for values, Θ for named
computations via `def`) is Levy's CBPV stratification realized
in shell syntax.

---

### `[MMM]`

**Alias:** `[Duploids]`

Éléonore Mangel, Paul-André Melliès, and Guillaume
Munch-Maccagnoni. "Classical Notions of Computation and the
Hasegawa-Thielecke Theorem." *Proc. ACM Program. Lang.* 10,
POPL, pp. 2112–2141, 2026.
DOI: [10.1145/3776715](https://doi.org/10.1145/3776715).

**Annotation.** The most load-bearing reference in the project.
Defines duploids (non-associative categories capturing mixed-
polarity composition) and dialogue duploids (duploids with
involutive negation via a strong monoidal duality functor).
psh commits to dialogue-duploid structure (Definition 9.4),
which gives the linear classical L-calculus (§9.3) as the
internal type theory and the full Hasegawa-Thielecke theorem
(§9.6): **thunkable ⇔ central** in any dialogue duploid.
Proposition 8550 (thunkable ⇒ central, forward direction only)
was cited throughout the spec before the dialogue commitment;
the commitment licenses the reverse. The three composition
patterns ((+,+), (−,−), and the failing (+,−) equation from
`docs/vdc-framework.md` §8) are duploid composition laws. The
non-associativity of (+,−) is the theoretical basis for polarity
frames. The exponentials `!`/`?` from the L-calculus partition
psh's typing context into classical and linear zones (§Linear
resources).

---

### `[Mun13]`

Guillaume Munch-Maccagnoni. "Syntax and Models of a
Non-Associative Composition of Programs and Proofs." PhD
thesis, Université Paris Diderot, 2013.

**Annotation.** Thesis developing the non-associative
composition framework that duploids formalize. Provides the
detailed treatment of polarity, focusing, and the interaction
between CBV and CBN evaluation that informs psh's evaluation
strategy. The spec's analysis of rc's implicit three-sorted
structure and the focusing discipline for discipline functions
draws on this thesis alongside [CMM10].

---

### `[Mun14]`

Guillaume Munch-Maccagnoni. "Models of a Non-Associative
Composition." In *FoSSaCS 2014: Foundations of Software
Science and Computation Structures*, LNCS 8412,
pp. 396–410, 2014.
DOI: [10.1007/978-3-642-54830-7_26](https://doi.org/10.1007/978-3-642-54830-7_26).

**Annotation.** Conference paper distilling the thesis [Mun13]
into the non-associativity result. Cited alongside [Mun13]
for the published, peer-reviewed statement of the composition
laws psh draws on.

---

### `[Spi14]`

**Alias:** `[SPW]`

Arnaud Spiwack. "A Dissection of L." Unpublished preprint,
2014. URL: https://assert-false.science/arnaud/papers/A%20dissection%20of%20L.pdf

**Annotation.** Analysis of the L-calculus (Levy's
call-by-push-value in sequent form) that psh's spec cites for
the shift placement discipline. The "dissection" of the
interaction between ↓ (thunk) and ↑ (force) shifts informs
psh's sort classification and the boundary between the value
layer (words) and the computation layer (commands).

---

### `[Wad14]`

Philip Wadler. "Propositions as Sessions." *Journal of
Functional Programming* 24(2-3), 2014, pp. 384–418.

**Annotation.** Establishes the correspondence between
propositions of classical linear logic and session types for
concurrent computation. The key observation: the cut rule of
linear sequent calculus *is* the typing rule for composing
two processes over a channel, and involutive negation *is*
session duality. psh's typed pipes use this correspondence
directly — the `|[T]` annotation places a session type on the
cut formula at the pipe site, the consumer receives the dual
type `¬Stream(T)`, and the L-calculus cut rule (§Calculus)
verifies compatibility. Wadler builds on the
Caires-Pfenning [2010] correspondence between intuitionistic
linear logic and session types, extending it to the classical
setting — which is the setting psh inhabits (multiple
conclusions in Δ, classical control via trap/μ-binders).

---

## Correspondence to spec references

The spec (prior monolithic `docs/specification.md`) used its own numbering
scheme. This table maps spec reference numbers to bibliography
keys for cross-referencing during the transition period.

| Spec ref | Bibliography key | Notes |
|----------|-----------------|-------|
| `[1]` | `[Duf90]` | |
| `[2]` | `[MMM]` | |
| `[3]` | `[Mun13]` | |
| `[4]` | `[Lev04]` | |
| `[5]` | `[CH00]` | |
| `[6]` | *(removed — uncited)* | |
| `[7]` | `[BTMO23]` | |
| `[8]` | `[CMM10]` | |
| `[9]` | `[Mun14]` | |
| `[Honda98]` | `[HVK98]` | |
| `[CMS]` | `[CMS]` | Key unchanged |
| `[Clarke]` | `[CBG24]` | `[Clarke]` retained as alias |
| `[SPW]` | `[Spi14]` | `[SPW]` retained as alias |
| `[CS]` | `[CS10]` | |
| `[Lei]` | `[Lei04]` | |
| `[Bur]` | `[Bur71]` | |

The following spec references are **project-internal documents**,
not published works. They do not get bibliography entries. Code
cites them by repo-relative path:

- `[9P]` → `refs/plan9/man/5/intro` (vendored) + `refs/plan9/rfc9p2000.txt`
- `[Be]` → Haiku/BeOS source (heritage annotation)
- `[SPEC]` → `refs/ksh93/ksh93-analysis.md`
- `[SFIO]` → `refs/ksh93/sfio-analysis/README.md`
- `[SFIO-3]` → `refs/ksh93/sfio-analysis/03-buffer-model.md`
- `[SFIO-7]` → `refs/ksh93/sfio-analysis/07-disciplines.md`
- `[VDC]` → `docs/vdc-framework.md`

---

## Backfill status

All entries resolved (2026-04-13). Corrections applied:

- `[BTMO23]` — venue corrected from "Haskell Symposium, 2023"
  to "Proc. ACM Program. Lang. 8, ICFP, 2024"
- `[CBG24]` — authors corrected (7 authors, not 3)
- `[CMM10]` — venue clarified as IFIP TCS conference, not
  Elsevier journal
- `[Spi14]` — confirmed unpublished preprint, no DOI exists
