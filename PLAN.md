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

## Current state (2026-04-07)

Rebuilt from spec. ~5700 lines, 249 tests, combine 4 parser.
The shell runs as `psh -c 'cmd'`, `psh file.psh`, or
interactive. Live at github.com/lane-core/psh.

### What exists

| Feature | Status |
|---------|--------|
| 10-variant Val (Unit, Bool, Int, Str, Path, ExitCode, List, Tuple, Sum, Thunk) | Complete |
| let bindings (mut, export, : Type) | Complete |
| Type inference (let-only: 42→Int, /tmp→Path) | Complete |
| Prism validation on typed assignments | Complete |
| rc grammar (if/else/for/while/fn) with `=>` single-line bodies | Complete |
| `match` with `=>` arms, `;` separators, glob + structural arms | Complete |
| `try` block (scoped ⅋) with else handler | Complete |
| `return` for value-producing blocks (CBPV return : A → F(A)) | Complete |
| RunOutcome { Status, Value } evaluator | Complete |
| First-class lambdas (`\x => body`) with capture-by-value | Complete |
| `fn name { }` / `fn name(params) { }` (positional + named) | Complete |
| `=~` infix pattern matching (replaces ~ builtin) | Complete |
| First-class lists, pairwise/broadcast concat | Complete |
| Pipelines with process groups (setpgid) | Complete |
| Profunctor redirections (left-to-right, wrapped) | Complete |
| Command substitution (`` `{cmd} ``) | Complete |
| Process substitution (`<{cmd}`) | Complete |
| Here-documents (`<<EOF`) and here-strings (`<<<word`) | Complete |
| Globbing (fnmatch-regex, recursive) | Complete |
| Tilde expansion (~ always $home, ~/path) | Complete |
| Discipline functions (.get notification, .set reentrancy) | Complete |
| Signal handlers as functions (fn sigint/sigexit) | Complete |
| Job control (fg/bg/jobs/wait, terminal control) | Complete |
| Coprocesses (socketpair, read -p, print -p) | Complete |
| `$"` stringify | Complete |
| Namerefs (ref x = target) | Complete |
| `${name}` brace-delimited variables | Complete |
| Free carets (implicit `^` on adjacency) | Complete |
| Two-alphabet split (var\_char / word\_char) | Complete |
| `capture_subprocess` shared primitive | Complete |
| whatis builtin | Complete |
| Builtins: cd echo exit get set builtin . wait jobs fg bg read print true false whatis | Complete |
| rc-style $home/$path/$user | Complete |
| CLI (-c, file, interactive REPL stub) | Complete |

### Not yet implemented

| Feature | Spec location |
|---------|---------------|
| Accessor syntax ($x.0, $x.ok, $x.code) | §Words |
| `try` in value position (returns Result[T]) | §`try` in value position |
| Type annotation parsing (Tuple, Union, Fn, Result, Maybe, (T)) | §Type annotations |
| Heredoc variable expansion (unquoted delimiters) | §Redirections |
| `<>` read-write redirect | §Redirections |
| `>{cmd}` output process substitution | §Missing rc I/O features |
| `whatis` with type information | §`whatis` output format |
| `shift` builtin, `$0`, list slicing | §Missing rc features |

### Architecture

```
src/
  ast.rs        ~350 lines  four-sort AST (Word/Expr/Binding/Command)
  parse.rs     ~2365 lines  combine 4 parser (6-layer architecture)
  exec.rs      ~1906 lines  evaluator: RunOutcome, try, thunks, disciplines
  env.rs        ~580 lines  scoped vars, type validation, readonly, namerefs
  value.rs      ~500 lines  Val enum (10 variants)
  job.rs        ~290 lines  JobTable, JobStatus, process group tracking
  signal.rs     ~130 lines  signals_receipts handlers, rc-style fn sig*
  main.rs       ~120 lines  bpaf CLI (-c, file, interactive)

docs/
  syntax.md              normative grammar
  specification.md       theoretical foundation (duploid, optics, CBPV)
  api.md                 plugin and host API
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

- **Val is a 10-variant enum** (Unit, Bool, Int, Str, Path, ExitCode,
  List, Tuple, Sum, Thunk). ExitCode enters only through `try`.
  Tuple is product (Lens). Sum is coproduct (Prism). Thunk is
  first-class function (CBPV's U(C), optic leaf).
- **`fn` for command bindings, `\` for value lambdas.** Sort
  boundary visible in syntax. `fn name { }` or `fn name(a b) { }`.
  `\x => body` produces Val::Thunk with capture-by-value.
- **`=~` for pattern matching.** Infix: `$x =~ *.txt`. Primitive
  (not sugar for match), shares `glob_match()` with match glob arms.
  `~` is purely tilde expansion.
- **RunOutcome { Status, Value }.** Replaces Status as return type.
  `return` produces Value (CBPV's `return : A → F(A)`).
- **`match` with `=>` and `;`.** Structural arms: `tag name =>`.
  Glob arms: `pattern =>` or `(pat ...) =>`. Unmatched → Unit +
  nonzero Status.
- **`try` is the ⊕→⅋ converter.** Scoped error handling (block form)
  and fallible capture (value-position form) sharing one
  `capture_subprocess` primitive with `` `{cmd}`` — siblings, not
  desugaring.
- **All binders are bare names.** `for x`, `let x`, `fn name`,
  `else e`, `\x`, `ok v`. `$` means reference, always.
- **Two character predicates.** `var_char` for `$`-refs, `word_char`
  for bare words. `\` + newline = continuation, `\` + name = lambda.
- **Profunctor AST.** Redirections wrap expressions. Structural.
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
