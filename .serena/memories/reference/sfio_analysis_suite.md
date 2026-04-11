---
type: reference
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [sfio, ksh93, stream, buffer, discipline, dccache, non-associativity, polarity, mode-switch, type-theory, sfdisc, endr, endw, filemap]
agents: [plan9-systems-engineer, vdc-theory, psh-architect]
---

# Reference: sfio analysis suite (14 files)

**Path.** `refs/ksh93/sfio-analysis/`

**Origin.** Lane's systematic analysis of sfio as ksh93's implicit type theory. Vendored into psh as the empirical precedent for the polarity frame discipline and the typed fd model.

## What's in the suite

14 markdown files documenting sfio as the structure ksh93 built correctly at the I/O layer and failed to propagate to the shell layer.

- **README.md** — entry point and overview.
- **01-data-structures.md** — `Sfio_t`, `Sfdisc_t`, `Mac_t`, and the core types.
- **02-lifecycle.md** — stream creation, discipline push/pop, destruction.
- **03-buffer-model.md** — **load-bearing**. The five-pointer buffer model (`_data`, `_next`, `_endr`, `_endw`, `_endb`) encodes polarity: `_endr` active = read mode (negative, consuming); `_endw` active = write mode (positive, producing). Mode switching via `_sfmode()` is a **polarity shift with reconciliation cost** — seek-back for read→write, flush for write→read.
- **04-io.md** — read/write primitives.
- **05-pool.md** — stream pooling and sharing.
- **06-mode.md** — explicit mode switching semantics.
- **07-disciplines.md** — **load-bearing**. `Sfdisc_t` as an endomorphism chain. **Dccache** (discipline cache) is the structural precedent for psh's polarity frame discipline: when a new discipline is pushed onto a stream with buffered data, the two possible bracketings yield different results because data already in value mode (buffered) cannot be re-processed through a new computation discipline. **This is the non-associativity failure of the duploid (+,−) equation, manifest in a real I/O library.**
- **08-error-handling.md** — error propagation through discipline stacks.
- **09-streams.md** — file, string, and memory streams.
- **10-ksh-integration.md** — **load-bearing**. `filemap[]` / `sh.topfd` save/restore as Lens pattern. ksh93's fd discipline is a value-level Lens (PutGet/GetPut/PutPut); psh preserves this as the typed fd table.
- **11-implementation.md** — Rust / C implementation notes.
- **12-testing.md** — test discipline for the I/O layer.
- **13-security.md** — security considerations.
- **14-performance.md** — benchmarks and throughput characteristics.

## Concepts it informs in psh

- **Polarity frame discipline** — Dccache (`07-disciplines.md`) is the structural precedent. psh generalizes it from "I/O library mechanism" to "shell-level discipline at every polarity boundary."
- **Typed fd model** — `filemap[]`/`sh.topfd` (`10-ksh-integration.md`) is the untyped ancestor of psh's typed fd roles (`Pipe`, `File`, `Tty`, `Coproc`, `Session`).
- **Wrapped redirections as Lens** — `decision/postfix_dot_accessors`, `docs/specification.md` §"Profunctor structure". The save/restore pattern is a monomorphic Lens with PutGet/GetPut/PutPut laws.
- **Non-associativity witness** — Dccache is where duploid (+,−) non-associativity shows up concretely in a real codebase. `docs/vdc-framework.md` §8.4 cites this.
- **Buffer polarity as shift** — the five-pointer model is a runtime realization of the polarity shift from focused type theory.

## Who consults it

- **plan9 agent** (primary, canonical): sfio discipline is core plan9 agent territory. Cite files by name.
- **vdc-theory agent**: Dccache as the (+,−) non-associativity witness; the polarity frame discipline generalization.
- **psh-architect**: for the typed fd model and the fd save/restore Lens implementation.

## Note

sfio is ksh93's I/O library — authored by David Korn and Glenn Fowler, separate from ksh93 proper. The analysis is Lane's reading of sfio through the polarized type theory lens; the mapping claims in the files are structural analogies, and the claim "Dccache ≈ duploid (+,−) failure" is made explicitly as a pattern-match, not a formal verification. Treat the correspondence as motivating, not as mathematical proof.
