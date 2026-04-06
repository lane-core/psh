# Plan

Current implementation roadmap. Living document — update when tasks
complete, priorities change, or new work is identified.

## Design position

psh is an excellent standalone shell first. It must be usable as a
login shell on Linux, macOS, and other Unix-likes without pane
deployed. The pane namespace integration is a superpower, not a
requirement. Adoption comes from the shell being good — clean
syntax, typed values, discipline functions, principled design —
not from being tied to a specific system.

## Current state (2026-04-06)

14 commits, 5940 lines across 8 source files, 157 tests passing.
The shell is a functional rc successor with ksh93 extensions and a
typed value model grounded in sequent calculus / duploid theory.

### What exists

| Feature | Status |
|---------|--------|
| Typed Val (Unit, Bool, Int, Str, Path, List\<Val\>) | Complete |
| let bindings (mut, export, : Type, List[T]) | Complete |
| Type inference (let-only: 42→Int, /tmp→Path) | Complete |
| Prism validation on typed assignments | Complete |
| rc grammar (if/else/for/while/switch/fn) | Complete |
| First-class lists, pairwise/broadcast concat | Complete |
| Pipelines with process groups (setpgid) | Complete |
| Profunctor redirections (left-to-right, wrapped) | Complete |
| Command substitution (`` `{cmd} ``) | Complete |
| Process substitution (`<{cmd}`) | Complete |
| Here-documents (`<<EOF`) and here-strings (`<<<word`) | Complete |
| Globbing (fnmatch-regex, recursive) | Complete |
| Tilde expansion (~ and ~/path) | Complete |
| Discipline functions (.get purity, .set reentrancy) | Complete |
| Signal handlers as functions (fn sigint/sigexit) | Complete |
| Job control (fg/bg/jobs/wait, terminal control) | Complete |
| Coprocesses (socketpair, read -p, print -p) | Complete |
| ~ match operator | Complete |
| $" stringify | Complete |
| Namerefs (ref x = target) | Complete |
| whatis builtin | Complete |
| Builtins: cd echo exit get set builtin . wait jobs fg bg read print ~ true false whatis | Complete |
| rc-style $home/$path/$user | Complete |
| CLI (-c, file, interactive REPL stub) | Complete |

### Architecture

```
src/
  ast.rs       277 lines  four-sort AST (Word/Expr/Binding/Command)
                           Word::Quoted, TypeAnnotation, Binding::Let
  parse.rs    1622 lines  recursive descent on &str (fused lex+parse)
  exec.rs     2470 lines  evaluator: rustix + libc, typed Val, disciplines
  env.rs       581 lines  scoped vars, type validation, readonly, namerefs
  value.rs     453 lines  Val enum (6 variants), inference, Prism access
  job.rs       287 lines  JobTable, JobStatus, process group tracking
  signal.rs    129 lines  signals_receipts handlers, rc-style fn sig*
  main.rs      121 lines  bpaf CLI (-c, file, interactive)
```

### Dependencies

```
default:
  anyhow              error handling
  bpaf                 CLI argument parsing
  combine              monadic parser combinators (available, not yet used)
  fnmatch-regex        glob pattern matching
  libc                 fork/waitpid/setpgid/execvp/_exit
  rustix               safe Unix syscalls (pipe, open, write, dup2, socketpair)
  signals_receipts     atomic-counter signal handling

feature-gated (pane):
  pane-proto            protocol vocabulary
  pane-session          IPC client (uses par internally)
```

### Design documents

| Document | Location |
|----------|----------|
| Specification (theoretical foundation) | `docs/specification.md` (651 lines) |
| Shell design vision | pane repo `docs/shell.md` |
| Style guide | `STYLEGUIDE.md` |

### Key design decisions

- **Val is a 6-variant enum** (Unit, Bool, Int, Str, Path, List<Val>).
  Inference runs in let context only. Bare `x = val` stays Str (rc heritage).
- **let is the typed μ̃-binder.** `let [mut] [export] name [: Type] = val`.
  Local by default, immutable by default. Bare `x = val` walks scope chain (rc heritage).
- **par is NOT a direct dependency.** Enters through pane-session only.
- **Pipes carry bytes.** Types don't cross fork boundary. Concat coerces to Str.
- **Profunctor AST.** Redirections wrap expressions (not bolted on). Evaluation order is structural.
- **⊕ error convention only.** Status returns, no longjmp. ⅋ (traps) deferred.
- **Tests require `--test-threads=1`** due to fork-based tests interfering in parallel.

---

## Now

### Testing infrastructure

- [ ] **Robust test suite derived from ksh93.** Adapt ksh93u+m's test
      suite (`/Users/lane/src/ksh93/src/cmd/ksh93/tests/`) for psh.
      Focus on: variable expansion, quoting, pipelines, redirections,
      subshells, signal handling, job control. Port tests that exercise
      rc-compatible features. Skip POSIX-specific tests (psh is not POSIX).
      Reference: ksh26's test infrastructure at `/Users/lane/src/ksh/ksh/`.
- [ ] **Integration test harness.** Script-level tests that run psh as
      a subprocess and compare stdout/stderr/exit code. Separate from
      unit tests (which test internal APIs).
- [ ] **Fix the `combine` situation.** combine is a dependency but unused.
      Either use it for the parser (replace recursive descent) or remove it.

### Interactive features

- [ ] **Line editing.** reedline or rustyline. vi/emacs modes (ksh93 heritage).
- [ ] **History.** Persistent (~/.psh_history), searchable (Ctrl-R).
      Use atomicwrites for crash-safe persistence.
- [ ] **Tab completion.** Filesystem paths + pane namespace (when available).
      The AffineFold composition model from the spec drives completion.
- [ ] **Prompt.** Command substitution in PS1 for live system state.
      `fn prompt { echo `{ get /pane/focused/attrs/title }' $ ' }`

### Missing rc features

- [ ] **Assignment-before-command.** `x=local cmd` — temporary binding
      scoped to one command. Parsed into SimpleCommand.assignments but
      never evaluated.
- [ ] **`$0`** — name of the script being executed.
- [ ] **`shift`** — shift positional parameters.
- [ ] **`flag`** — rc's flag-parsing builtin.

### Infrastructure

- [ ] **FdTable.** Runtime fd tracking with save/restore stack.
      Spec describes it (specification.md §"fd tracking") but not implemented.
- [ ] **Parse-time fd bitset.** Static analysis for use-after-close.
- [ ] **CaptureBuffer.** Two-tier command substitution capture
      (memory with spill-to-file on overflow).
- [ ] **Tilde expansion refinement.** Bare `~` should not expand when
      it's the match operator (`whatis ~` currently expands to $home).

### Pane integration (feature-gated)

- [ ] **`get`/`set` for `/pane/` paths.** Connect through pane-session.
      The session type refinement tells the shell what type to expect.
- [ ] **Tab completion for pane namespace.** Query pane server for
      available pane IDs and attribute names.
- [ ] **pane-terminal wrapper.** LooperCore<TerminalHandler> hosting
      the interpreter. Separate binary.

---

## Later

- [ ] **`${ cmd }` shared-state comsub** (no fork). From ksh93.
- [ ] **KEYBD trap** for interactive pane-aware keybindings.
- [ ] **Coprocess protocol typing.** Opt-in typed coprocess channels.
- [ ] **psh as sysadmin interface.** The spec's Phase 2 interactive vision.

---

## Session log (2026-04-06)

### What was accomplished

Lane and the agent (Opus 4.6) built psh from scratch in one session,
starting from the theoretical design (eight agent consultations on
the shell concept, duploid analysis, polarity classifications) and
ending with a functional 5940-line implementation.

**Theoretical work (first half):**

1. Four-agent roundtable (Plan 9, Be, session type, optics) on whether
   an rc successor grounded in sequent calculus is compelling. Unanimous
   yes. The three-sort structure (values/stacks/commands) maps naturally
   to shell constructs.

2. Duploid analysis: Plan 9 is co-Kleisli, BeOS is already a duploid
   (not primarily Kleisli as initially hypothesized). pane is the duploid
   unifying both. BMessage's polarity confusion (positive data × negative
   continuation in one struct) identified as the design flaw psh avoids.

3. Polarity classification tables for Be, Plan 9, and pane abstractions.
   Stored in serena (`pane/polarity_classifications`).

4. Shell design doc written (`pane/docs/shell.md`): two-binary
   architecture (psh standalone + pane-terminal wrapper), rc grammar,
   ksh93 discipline functions/coprocesses/namerefs, `get`/`set` namespace
   builtins, system-first adoption path.

5. Dependency deliberation: par removed as direct dependency (enters
   through pane-session only). fp-library rejected. Final set: anyhow,
   bpaf, combine, fnmatch-regex, libc, rustix, signals_receipts.

**Implementation (second half):**

6. Initial prototype: AST, lexer, parser, evaluator, value model. 50 tests.

7. Code review against rc paper and ksh93 source. Fixed concat semantics
   (pairwise, not cross-product) and redirect evaluation order (left-to-right).

8. Specification document (`docs/specification.md`, 651 lines) written
   by four agents, cross-deliberated, with five resolutions from Lane
   (affine not linear, intuitionistic/classical boundary, AST refactor,
   MonadicLens scope, .get purity enforcement).

9. Rewrite with rustix (safe fd management), fused lex+parse, all bug
   fixes from code review.

10. Feature additions: globbing, tilde expansion, signal handlers, job
    control, coprocesses, here-documents, process substitution, ~ match,
    $" stringify, namerefs, whatis.

11. Four-agent roundtable on typed values. Initial rejection of heavy
    typing (nushell-style). Second round with five base types
    (Unit/Bool/Int/Str/Path) accepted unanimously. Val rewrite to
    6-variant enum with let bindings, type annotations, List[T] syntax,
    Prism validation, inference in let context.

12. sfio analysis (all 12 documents from ksh26) informed I/O architecture:
    FdTable design, CaptureBuffer, socketpair coprocesses, Plan 9
    directness principle ("if your I/O layer exceeds 200 lines, you've
    taken a wrong turn").

**Key references used:**
- ksh26 SPEC.md (polarity/duploid analysis of ksh93)
- ksh26 sfio-analysis suite (12 documents on I/O substrate)
- ksh93u+m source (nvdisc.c, xec.c, io.c, jobs.c, name.c)
- Plan 9 rc paper (Duff 1990)
- Mangel/Melliès/Munch-Maccagnoni 2025 (duploids, L-calculus)
- Ueno/Das ESOP 2025 (refinement session type inference)
- Clarke et al. (profunctor optics, MonadicLens)

**serena memories updated:**
- `pane/duploid_analysis` — refined with Be correction
- `pane/polarity_classifications` — Be, Plan 9, pane tables
- `pane/shell_sequent_calculus_analysis` — full deliberation history
- `pane/rustix_migration` — instructions for pane-session migration
