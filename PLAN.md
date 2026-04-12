# Plan

Current roadmap. Living document — update when tasks complete,
priorities change, or new work is identified.

## Design position

psh is a new Plan 9 rc-derived system shell. It is a standalone
shell with no external infrastructure dependencies — usable as a
login shell on Linux, macOS, FreeBSD, and other Unix-likes. Its
design is grounded in the λμμ̃-calculus of sequent calculus,
duploid semantics, profunctor optics, session types, virtual double
categories, and the Plan 9 / ksh93 operational heritage.

## Current state

**Design: complete (pending VDC reframing pass).** The resolved
decisions are captured in `docs/specification.md`,
`docs/syntax.md`. The theoretical
framework is in `docs/vdc-framework.md`. The ksh93 operational
analysis is vendored as `refs/ksh93/ksh93-analysis.md`. The
dependency rationale is in `docs/implementation.md`. All reference
material is vendored: rc paper and man page at `refs/plan9/`,
ksh93 manpage, sfio-analysis, and ksh26 analysis at `refs/ksh93/`.

**Implementation: retired.** The prior implementation (~8800 lines
across ast.rs, parse.rs, exec.rs, env.rs, value.rs, job.rs)
encoded design decisions that have since moved. It was retired in
commit 76be317. The current source tree is a stub:

| File | Lines | Role |
|---|---|---|
| `src/main.rs` | ~35 | Binary stub — reports retirement status and exits with 2 |
| `src/parse.rs` | ~130 | Combine boilerplate — character predicates, trivia, keyword/name primitives, PshParser shell |
| `src/signal.rs` | ~130 | Self-pipe signal handling (type-system neutral, preserved) |

The parser boilerplate is the expected starting point for the
next grammar implementation. The signal self-pipe is preserved
because it's infrastructure, not type-system content.

**Dependencies** (unchanged, per `docs/implementation.md`):
`anyhow`, `bpaf`, `combine`, `fnmatch-regex`, `libc`, `rustix`,
`smallvec`, `signals_receipts`. No pane dependencies. No `par`
dependency.

## Next phase: VDC reframing (paused, ready to resume)

Before implementation can begin, the spec needs one final
restructure to make the Virtual Double Category framework the
top-level presentation. The current `specification.md` is
organized around the sequent calculus with duploids, CBPV, and
profunctor optics as supporting apparatus; the VDC framework
subsumes these. The VDC reframing preserves every resolved
decision but reorganizes the presentation.

The handoff memo for this work is in Lane's possession (not
committed). The reframing session uses `docs/vdc-framework.md`
as the primary source and produces a restructured
`specification.md` plus minor updates to `syntax.md`.

## Implementation roadmap (after VDC reframing)

The build sequence, in order:

### Phase 1: parser

- [ ] Fix `src/parse.rs` character predicates against the final
      spec (the boilerplate is mostly correct but may need minor
      adjustments).
- [ ] Implement the grammar from `docs/syntax.md` in layers:
  - [ ] Lexical primitives (already in the boilerplate)
  - [ ] Word atoms: literals, quoted strings with backslash
        escapes, variable references with postfix dot accessors,
        `$((...))`, command substitution `` `{} ``, process
        substitution `<{}`, tilde expansion, lambdas `|x| => body`
  - [ ] Tagged construction `NAME(args)` for sums, structs, maps
  - [ ] Tuple literals `(a, b, c)` and list literals `(a b c)`
  - [ ] Expression precedence tower (or, and, pipeline, cmd_expr)
  - [ ] Commands: `if(cond)`, `while(cond)`, `for(x in list)`,
        `match(expr) { arms }`, `try { } catch (e) { }`,
        `trap SIGNAL (body body?)?`
  - [ ] Bindings: `let`, `def`, `struct`, `ref`, assignment
  - [ ] Patterns for match arms: constructor patterns with
        alternation via `|`
  - [ ] Program structure (terminator-separated command sequence)
- [ ] Parse-time AST validation (arity checks, type/variable
      capitalization disambiguation for dotted def names)

### Phase 2: AST and value model

- [ ] AST matching the three-sort structure: `Word` (producers),
      `Expr` (profunctor layer), `Command` (cuts/control),
      `Binding` (μ̃-binders)
- [ ] Val enum with element types: Str, Int, Bool, Path, List,
      Tuple, Sum, Struct, Map, Lambda. Every variable is a list
      at the outer level.
- [ ] Type inference for `let` bindings (no annotations needed
      for the common case; annotations available for clarity).

### Phase 3: evaluator

- [ ] Word expansion pipeline (Kleisli composition of Word → Val
      stages)
- [ ] Command execution with three-composition-pattern discipline
      (Kleisli pipeline, co-Kleisli sequencing, cut)
- [ ] Polarity frames at value/computation boundaries
- [ ] CBPV `let` — accepts effectful RHS, binds the result
- [ ] Codata discipline functions with CBV focusing as reentrancy
      semantics
- [ ] Redirections as profunctor wrapping (Adapter composition)
- [ ] fd table with save/restore as Lens

### Phase 4: control flow and error model

- [ ] `if`/`else`, `while`, `for` with rc parens
- [ ] `match` with constructor patterns, `|` alternation, no
      guards (deferred)
- [ ] `try { } catch (e) { }` as scoped ErrorT monad transformer
- [ ] Unified `trap` (lexical / global / deletion forms)
- [ ] Signal delivery via self-pipe with wake-from-block handling

### Phase 5: coprocesses

- [ ] Named coprocess registry (`HashMap<String, Coproc>`)
- [ ] Per-tag binary session state machine
- [ ] Wire format: length-prefixed frames with `MAX_FRAME_SIZE`
      of 16 MiB
- [ ] Negotiate handshake on tag 0 (protocol version only)
- [ ] `print -p` returns Int tag; `read -p [-t tag]` reads response
- [ ] Shell-internal PendingReply tracking with drop-as-cancel

### Phase 6: primitive type methods

- [ ] String operations as `def Str.name { }` methods:
      `.length`, `.upper`, `.lower`, `.split`, `.strip_prefix`,
      `.strip_suffix`, `.replace`, `.contains`
- [ ] Sigil aliases: `$#x` for `.length`, `$"x` for `.join` on
      List (rc heritage)

### Phase 7: Map type

- [ ] `Map` as a first-class type with tagged construction
      (`Map(('k' 'v') ...)`)
- [ ] `.get`, `.set`, `.keys`, `.values` methods (AffineTraversal
      semantics)

### Phase 8: job control and interactive features

- [ ] Job table with `JobStatus` tracking
- [ ] Builtins: `fg`, `bg`, `jobs`, `wait` (with `-n`), `kill`
- [ ] `%N` word form expanding to job PIDs
- [ ] REPL with line editing (reedline or rustyline)
- [ ] History (persistent `~/.psh_history`, Ctrl-R search)
- [ ] Tab completion (filesystem paths, variable names, def names)

### Phase 9: spec conformance tests

- [ ] Integration tests with every example from `docs/syntax.md`
      as a test case
- [ ] Property tests for structural invariants (no-rescan,
      sequence preservation, CBV focusing)
- [ ] Adapted ksh93u+m test suite for rc-heritage features
- [ ] Fork-based tests must run single-threaded
      (`--test-threads=1`)

### Phase 10: polish

- [ ] Error messages with source locations
- [ ] `$((...))` arithmetic
- [ ] Here-strings (`<<<`)
- [ ] Heredocs (`<<EOF`)
- [ ] `ulimit`, `umask`, `export` builtin (for listing;
      `let export` is the binding form)
- [ ] `printf` (if fork cost matters for format strings)
- [ ] `shift`, `$0`, positional parameter handling

## Non-goals (with reasoning in specification.md)

See specification.md §"Features and non-goals" for the full
list with reasoning. Key items: no parametric polymorphism on
`def` signatures, no typed session channels on pipes, no
pipeline fusion as user-visible feature.

## Session log

### Session N (current): VDC reframing preparation

The design underwent substantial deliberation resulting in a
VDC-grounded foundation. Key resolved decisions: every variable
is a list (CBPV model with type annotations on element types),
unified `trap` grammar, codata discipline functions with CBV
focusing, uniform tagged construction `NAME(args)`, postfix dot
accessors with required space, structs with positional-only
construction, coprocess protocol with Int tag interface,
`let` as μ̃-binder on `F(A)` accepting effectful RHS, rejection
of anonymous records and named struct construction.

Prior implementation retired in commit 76be317. Source tree is
now a stub pending the VDC reframing pass and subsequent
implementation restart.

All reference material vendored: rc paper and man page, ksh93u+m
manpage, sfio-analysis suite, ksh26 theoretical foundation. Six
specialized agents defined in `.claude/agents/` for future design
work (plan9 systems engineer, session type, optics, VDC theory,
sequent calculus, psh architect).

Next: Serena memory bootstrap (separate prompt), then VDC
reframing pass, then implementation Phase 1.
