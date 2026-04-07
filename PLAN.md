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

A functional prototype (5940 lines, 157 tests) and a complete
target language specification (1103 + 801 lines). The spec
supersedes the prototype's grammar — this is a restart-worthy
respecification. The prototype validated feasibility; the spec
is the real design.

### What exists (prototype)

| Feature | Status |
|---------|--------|
| Typed Val (Unit, Bool, Int, Str, Path, List\<Val\>) | Complete (6 variants) |
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
| Discipline functions (.get notification, .set reentrancy) | Complete |
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

### What the spec adds (not yet implemented)

| Feature | Spec location |
|---------|---------------|
| `match` keyword (replaces `switch`) with `=>` arm syntax | §Control flow |
| `try` block (scoped ⅋, lexically-scoped `set -e`) | §Control flow |
| `try` in value position (returns `Result[T]`) | §`try` in value position |
| `=>` single-line body introducer (all control flow) | §Control flow |
| `;` arm separators in `match` blocks | §Control flow |
| `return` for value-producing blocks | §Control flow |
| Val::ExitCode(i32) — distinct from Int | §Type system |
| Val::Tuple(Vec\<Val\>) — products, comma-separated | §Type system |
| Val::Sum(String, Box\<Val\>) — coproducts | §Type system |
| Two-alphabet split (var\_char / word\_char) | §Two character sets |
| Free carets (implicit `^` at var/word boundary) | §Free carets |
| Accessor syntax ($x.0, $x.ok, $x.err, $x.code) | §Words |
| ${name} brace-delimited variable names | §Brace-delimited variable names |
| Shared `capture_subprocess` primitive | §Command substitution |
| Union type annotations (A \| B) | §Type annotations |
| Result[T] / Maybe[T] sugar | §Sugar |
| Heredoc variable expansion (unquoted delimiters) | §Redirections |
| `<>` read-write redirect | §Redirections |
| `>{cmd}` output process substitution | §Missing rc I/O features |
| `whatis` with type information | §`whatis` output format |
| Live re-evaluation via `.get` disciplines | §Live re-evaluation |

### Architecture

```
src/
  ast.rs       277 lines  four-sort AST (Word/Expr/Binding/Command)
  parse.rs    1622 lines  recursive descent — TO BE REPLACED with combine
  exec.rs     2470 lines  evaluator: rustix + libc, typed Val, disciplines
  env.rs       581 lines  scoped vars, type validation, readonly, namerefs
  value.rs     453 lines  Val enum (6 variants) — TO BE REWRITTEN (9 variants)
  job.rs       287 lines  JobTable, JobStatus, process group tracking
  signal.rs    129 lines  signals_receipts handlers, rc-style fn sig*
  main.rs      121 lines  bpaf CLI (-c, file, interactive)

docs/
  syntax.md   1103 lines  normative grammar (the target language spec)
  specification.md  801 lines  theoretical foundation (duploid, optics, CBPV)

tests/
  harness.rs         integration test harness (run psh as subprocess)
  integration.rs     test entry point
  *.rs               16 test modules (need rewrite against new spec)
```

### Dependencies

```
default:
  anyhow              error handling
  bpaf                 CLI argument parsing
  combine              monadic parser combinators — USE THIS for the rewrite
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
| Syntax specification (normative grammar) | `docs/syntax.md` (1103 lines) |
| Theoretical specification | `docs/specification.md` (801 lines) |
| Shell design vision | pane repo `docs/shell.md` |
| Style guide | `STYLEGUIDE.md` |

### Key design decisions

- **Val is a 9-variant enum** (Unit, Bool, Int, Str, Path, ExitCode,
  List, Tuple, Sum). ExitCode enters only through `try`.
  Tuple is product (Lens). Sum is coproduct (Prism).
- **let is always CBV.** Immutable by default, local by default.
  No call-by-name `let`. Live re-evaluation uses `.get` disciplines.
- **`match` with `=>` and `;`.** Arms use `=>` to introduce bodies,
  `;` to separate. Newlines inside `match { }` are trivia.
- **`try` is the ⊕→⅋ converter.** Scoped error handling (block form)
  and fallible capture (value-position form) sharing one
  `capture_subprocess` primitive with `` `{cmd}`` — siblings, not
  desugaring.
- **`return` for value-producing blocks.** CBPV's `return : A → F(A)`.
  Unambiguous polarity shift from command to value sort.
- **`=>` is a single-line body introducer.** Uniform across all
  control flow: match arms, if/else, for, while, try.
- **Two character predicates.** `var_char` for `$`-refs, `word_char`
  for bare words. Enables free carets and accessor syntax.
- **Profunctor AST.** Redirections wrap expressions. Structural.
- **⊕ error convention.** Status returns. `try` is scoped ⅋.
- **Tests require `--test-threads=1`** (fork interference).

---

## Now

### Implementation restart

The spec (`docs/syntax.md`) is the target. The prototype is a
reference, not the starting point. Implementation order:

- [ ] **1. Parser rewrite with `combine`.** Replace `parse.rs`
      entirely. The `combine` crate is in Cargo.toml, mandated.
      The new grammar (accessors, two-alphabet split, structural
      match patterns, `=>` arms, `try` in multiple positions,
      `return` as value-tail) warrants combinators.

- [ ] **2. Two-alphabet split.** `is_var_char` / `is_word_char`.
      Remove `~` from word chars. Add `@`. Enables free carets,
      tilde fix, and accessor parsing.

- [ ] **3. Val extension.** Rewrite `value.rs` with 9 variants.
      ExitCode (inverted truthiness), Tuple (0-based projection),
      Sum (payload-only display, always truthy).

- [ ] **4. `match` with `=>` and `;`.** Rename Switch → Match.
      Add structural patterns. `=>` introduces arm body. `;`
      separates arms. Newlines trivia inside `match { }`.

- [ ] **5. `try` block.** Scoped ⅋. `in_try` flag on Shell,
      checked after each command. Boolean contexts exempt.

- [ ] **6. `=>` single-line body.** Extend `body` production.
      Uniform across if/for/while/try/else/fn.

- [x] **7. Accessor syntax.** `$x.0`, `$x.ok`, `$x.err`, `$x.code`.
      Accessor takes priority over free carets after `$`-variables.

- [x] **8. `try` in value position + `capture_subprocess`.** Shared
      fork+pipe+capture+waitpid primitive. `try` wraps as
      Result[T]. `` `{cmd}`` projects stdout only.

- [x] **9. `return` in value-producing blocks.** `return word`
      injects a value from the command sort into the value sort.
      `Value::Compute(Vec<Command>)` for if/match/while/for/{ }
      in let RHS. `take` keyword for for-in-value collection
      (Raku gather/take heritage, Traversal introduction form).

- [x] **10. Type annotations.** ExitCode, `(A, B)` tuples,
      `A | B` unions, `A -> B` / `Fn[A, B]`, `Result[T]` / `Maybe[T]`.

- [x] **11. Heredoc expansion, missing I/O.** Unquoted `<<EOF`
      expands `$var`. `<>file`. `>{cmd}`.

### Test rewrite

- [ ] **Rewrite integration tests against `docs/syntax.md`.**
      Every example in the spec becomes a test. `[planned]`
      productions get `#[ignore]` tests.
- [ ] **ksh93-derived test suite.** Adapt ksh93u+m tests for
      psh's rc-heritage features. Skip POSIX-specific tests.

### Interactive features

- [ ] **Line editing.** reedline or rustyline. vi/emacs modes.
- [ ] **History.** Persistent (~/.psh_history), searchable (Ctrl-R).
- [ ] **Tab completion.** Filesystem paths + pane namespace.
- [ ] **Prompt.** Command substitution in PS1.

### Missing rc features

- [ ] **Assignment-before-command.** `x=local cmd`.
- [x] **`$0`** — script name.
- [x] **`shift`** — shift positional parameters.

### Infrastructure

- [ ] **FdTable.** Runtime fd tracking with save/restore stack.
- [ ] **CaptureBuffer.** Two-tier capture (memory + spill to file).

---

## Later

- [ ] **`${ cmd }` shared-state comsub** (no fork). ksh93 heritage.
      Deferred — `return` in value-producing blocks covers the
      primary use case. `${ }` is for in-process stdout capture
      when external commands need shared state.
- [ ] **KEYBD trap** for interactive pane-aware keybindings.
- [ ] **Coprocess protocol typing.** Opt-in typed channels.
- [ ] **psh as sysadmin interface.** Phase 2 interactive vision.

---

## Pane integration (feature-gated)

- [ ] **`get`/`set` for `/pane/` paths.** Connect through pane-session.
- [ ] **Tab completion for pane namespace.**
- [ ] **pane-terminal wrapper.** LooperCore<TerminalHandler>.
- [ ] **Live variables via `.get` disciplines.**
      `fn cursor.get { cursor = `{ get /pane/focused/attrs/cursor } }`

---

## Session log

### Session 1 (2026-04-06): Prototype

Lane and agent (Opus 4.6) built psh from scratch. 5940 lines,
15 commits. Theoretical work (duploid analysis, polarity tables,
shell design doc) then implementation (AST, parser, evaluator,
all features). See `docs/specification.md` for theoretical
foundation.

### Session 2 (2026-04-06): Syntax specification

Lane directed a complete syntax formalization through the
four-agent roundtable (Plan 9, Be, session type, optics). 25+
roundtable sessions, multiple refinement rounds. Produced
`docs/syntax.md` (1103 lines) — the normative target grammar.

Key decisions: `match` with `=>` and `;` (not `switch`/`case`);
9-variant Val (ExitCode, Tuple, Sum); two character predicates;
`try` as scoped ⅋ and value-position capture; `return` for
value-producing blocks; shared `capture_subprocess` primitive
(siblings, not desugaring); CBV-only `let`; `.get` disciplines
for live re-evaluation; accessor syntax; free carets.

The spec supersedes the prototype grammar. Implementation restart
required — parser rewrite with `combine`, `value.rs` rewrite,
new control flow constructs.
