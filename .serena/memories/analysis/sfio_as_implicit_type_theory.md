---
type: analysis
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [sfio, ksh93, implicit-type-theory, korn, fowler, buffer-polarity, mode-switch, dccache, filemap, lessons-for-psh, polarity-frame, structural-correspondence]
agents: [plan9-systems-engineer, vdc-theory, psh-architect]
related: [reference/sfio_analysis_suite, analysis/polarity/_hub, analysis/polarity/dccache_witness, analysis/polarity/frames, analysis/polarity/sh_prefix_critical_pair]
verified_against: [docs/spec/@HEAD §133-185, refs/ksh93/sfio-analysis/03-buffer-model.md, refs/ksh93/sfio-analysis/07-disciplines.md, refs/ksh93/sfio-analysis/10-ksh-integration.md, audit/plan9-systems-engineer@2026-04-11]
---

# sfio as ksh93's implicit type theory

## Concept

The framing claim itself: ksh93's I/O library sfio (Phong Vo, David Korn, Glenn Fowler at Bell Labs / AST toolkit; Vo is the primary author) implements a polarity-typed structure that the surrounding shell does not. The `refs/ksh93/sfio-analysis/` suite reads sfio through the polarized type theory lens and identifies three load-bearing patterns (`docs/spec/` §"The sfio insight" lines 133–185):

1. **Buffer polarity.** sfio's five-pointer buffer model (`_data`, `_next`, `_endr`, `_endw`, `_endb`) encodes polarity at the field level: `_endr` active = read mode (negative, consuming); `_endw` active = write mode (positive, producing). Mode switching via `_sfmode()` is a polarity shift with reconciliation cost — flush for write→read, seek-back for read→write. (Spec lines 139–145.)
2. **Discipline stacks as morphism chains.** Each `Sfdisc_t` in the discipline stack composes like an endomorphism between the buffer (value mode) and the kernel (computation mode). The stack as a whole mediates the value/computation boundary. (Spec lines 147–150.)
3. **Dccache as non-associativity witness.** When a discipline is pushed onto a stream with buffered data, the bracketings yield different results — see `analysis/polarity/dccache_witness` for the full treatment. The pattern is **structurally analogous** to the duploid (+,−) failure; per the spec's caveat at lines 159–161 ("the pattern matches; the full duploid composition laws have not been formally verified for sfio's discipline stack") it is a pattern-match, not a formal verification. (See `policy/memory_discipline` §10 for the epistemic-strength rule.)

**The lesson for psh** (`docs/spec/` lines 163–169): "ksh93's authors built correct polarity discipline in sfio and then failed to propagate it to the shell proper. The `sh.prefix` bugs (SPEC.md bugs 001–003b) are exactly the same non-associativity that Dccache handles correctly — a computation (DEBUG trap) intruding into a value context (compound assignment) with no mediator. sfio had the mediator; the shell didn't."

psh's response (spec lines 171–184):

- **Typed fd roles** (`Pipe`, `File`, `Tty`, `Coproc`, `Session`) replace sfio's universal `Sfio_t` with runtime mode bits — explicit types over runtime flags.
- **Wrapped redirections** make evaluation order structural via AST nesting — the profunctor composition prevents non-associative bracketing by construction. No Dccache problem possible.
- **Save/restore as lens roundtrips** — the `filemap[]` / `sh.topfd` pattern from `refs/ksh93/sfio-analysis/10-ksh-integration.md` translated to typed Rust as monomorphic Lenses with PutGet / GetPut / PutPut.

## Foundational refs

- `reference/sfio_analysis_suite` — the 14-file analysis. Load-bearing files: `03-buffer-model.md` (buffer polarity), `07-disciplines.md` (discipline stacks + Dccache), `10-ksh-integration.md` (filemap/sh.topfd as Lens pattern). The reference memo correctly hedges the duploid correspondence as "structural analogy, not formal verification."
- `analysis/polarity/_hub` — the polarity discipline cluster, where the dccache_witness and sh_prefix_critical_pair spokes consume this lesson.
- `analysis/polarity/dccache_witness` — the specific structural correspondence and its caveats.
- `analysis/polarity/sh_prefix_critical_pair` — the manifestation in interpreter code that motivates the lesson.

## Spec sites

- `docs/spec/` §"The sfio insight" lines 133–185 — the framing claim itself, including the lessons-for-psh section.
- `docs/spec/` §"Polarity discipline" line 343 — the operational discipline psh inherits.

## Status

Settled framing. The framing claim is "sfio is the structure ksh93 built right at I/O and failed to propagate to the shell proper" — the **lesson**, not a category-theoretic theorem. Treat it as motivating, not as proof. Cite `analysis/polarity/dccache_witness` for the specific structural correspondence and its caveats.
