# Plan

Current roadmap. Living document — update when tasks complete,
priorities change, or new work is identified.

## Design position

psh is a new Plan 9 rc-derived system shell. It is a standalone
shell with no external infrastructure dependencies — usable as a
login shell on Linux, macOS, FreeBSD, and other Unix-likes. Its
design is grounded in the λμμ̃-calculus of sequent calculus,
duploid semantics, profunctor optics, session types, virtual double
categories (FVDblTT as the internal language), and the Plan 9 /
ksh93 operational heritage.

## Current state

**Design: active.** The resolved decisions are in
`docs/specification.md` and `docs/syntax.md`. The theoretical
framework is in `docs/vdc-framework.md`. The ksh93 operational
analysis is at `refs/ksh93/ksh93-analysis.md`. The dependency
rationale is in `docs/implementation.md`. All reference material
is vendored: rc paper and man page at `refs/plan9/`, ksh93
manpage, sfio-analysis, and ksh26 analysis at `refs/ksh93/`.

**Implementation: not yet started.** The source tree is a
deliberate stub:

| File | Lines | Role |
|---|---|---|
| `src/main.rs` | ~35 | Binary stub — reports retirement status and exits with 2 |
| `src/parse.rs` | ~130 | Combine boilerplate — character predicates, trivia, keyword/name primitives, PshParser shell |
| `src/signal.rs` | ~130 | Self-pipe signal handling (type-system neutral, preserved) |

The parser boilerplate is the expected starting point for the
grammar implementation. The signal self-pipe is preserved because
it's infrastructure, not type-system content.

**Dependencies** (unchanged, per `docs/implementation.md`):
`anyhow`, `bpaf`, `combine`, `fnmatch-regex`, `libc`, `rustix`,
`smallvec`, `signals_receipts`. No pane dependencies. No `par`
dependency.

## Implementation roadmap

The build sequence, in order. Each phase depends on the previous.

### Phase 1: parser

- [ ] Fix `src/parse.rs` character predicates against the spec.
- [ ] Implement the grammar from `docs/syntax.md` in layers:
  - [ ] Lexical primitives (already in the boilerplate)
  - [ ] Word atoms: literals, quoted strings with backslash
        escapes, variable references with postfix dot accessors,
        bracket accessors, `$((...))`, command substitution
        `` `{} ``, process substitution `<{}`, tilde expansion,
        lambdas `|x| => body`
  - [ ] Tuple literals `(a, b, c)` and list literals `(a b c)`
  - [ ] Brace record literal `{ field = value; field = value }`
        for struct construction; name-pun shorthand `{ x; y }`
  - [ ] Tagged construction `NAME(args)` for enum variants and
        Map; bare nullary variant names
  - [ ] Type annotations with parametric constructors:
        `Result(Int, Str)`, `List(Int)`, `Option(Path)`
  - [ ] Expression precedence tower (or, and, pipeline, cmd_expr)
  - [ ] Commands: `if(cond)`, `while(cond)`, `for(x in list)`,
        `match(expr) { arms }`, `try { } catch (e) { }`,
        `trap SIGNAL (body body?)?`
  - [ ] Bindings: `let` with pattern LHS (wildcard, tuple
        destructuring, struct destructuring), `let-else` for
        refutable patterns, `if let` for refutable pattern
        branches, `def` with optional return type annotation
        and `return` keyword, `struct`, `enum`, `ref`,
        assignment
  - [ ] Patterns for match arms: enum variant patterns (tagged
        and nullary), struct record patterns (symmetric with
        construction), tuple patterns, wildcard, literal
        patterns
- [ ] Parse-time AST validation (arity checks, type/variable
      capitalization disambiguation for dotted def names)

### Phase 2: AST and value model

- [ ] AST matching the three-sort structure from the spec:
      `Word` (producers), `Expr` (engineering layer for
      pipelines and redirections), `Command` (statements /
      cuts). Consumers are synthesized implicitly from the
      statement's shape.
- [ ] `Val` enum with element types: `Str`, `Int`, `Bool`,
      `Path`, `List`, `Tuple`, `Sum`, `Struct`, `Map`,
      `Lambda`, typed-fd roles (`Pipe`, `File`, `Tty`,
      `Coproc`, `Session`). Every variable is a list at the
      outer level.
- [ ] `Status(String)` type with `is_success` checking
      emptiness (rc heritage).
- [ ] Bidirectional type checker: synth mode (bottom-up from
      literals and typed constructors) + check mode (top-down
      from annotations, return types, parameter types). No
      unification, no let-polymorphism. Under-determined
      bindings are type errors at the binding site.

### Phase 3: evaluator

- [ ] Word expansion pipeline (Kleisli composition of
      `Word → Val` stages)
- [ ] Command execution with three-composition-pattern
      discipline (Kleisli pipeline, co-Kleisli sequencing, cut)
- [ ] Polarity frames at `↓→↑` shift sites: command
      substitution, `$((...))` (trivial frame), `.refresh` and
      `.set` discipline bodies
- [ ] CBPV `let` — accepts effectful RHS, binds the result
      via the μ̃-binder
- [ ] Codata discipline cells `.get` (pure), `.refresh`
      (effectful, invoked imperatively), `.set` (mutator with
      slot-write primitive); CBV focusing of `.get` via
      thunkability
- [ ] Redirections as profunctor wrapping (Adapter composition)
- [ ] fd table with save/restore as Lens (PutGet/GetPut/PutPut)

### Phase 4: control flow and error model

- [ ] `if`/`else`, `while`, `for` with rc parens
- [ ] `match` with enum variant patterns, struct record
      patterns, tuple patterns, literal patterns, wildcards,
      `|` alternation between patterns within an arm
- [ ] Pattern lets (wildcard, tuple, struct) and `let-else`
      for refutable patterns; `if let` for refutable branches
- [ ] `try { } catch (e) { }` as scoped ErrorT monad transformer
- [ ] Unified `trap` (lexical / global / deletion forms)
- [ ] Signal delivery via self-pipe with wake-from-block
      handling, EINTR retry policy in builtins

### Phase 5: coprocesses

- [ ] Named coprocess registry (`HashMap<String, Coproc>`)
- [ ] Per-tag binary session state machine with phantom session
      types
- [ ] Admin session for Tflush / Rflush cancellation
- [ ] Wire format: length-prefixed frames with first-byte
      dispatch; `MAX_FRAME_SIZE` of 16 MiB
- [ ] Negotiate handshake on tag 0 (protocol version only)
- [ ] `print -p` returns Int tag; `read -p [-t tag]` reads
      response
- [ ] Shell-internal `PendingReply` tracking with compile-time
      use-site affinity and runtime drop-as-cancel; tag reuse
      gated on session termination

### Phase 6: primitive type methods

- [ ] String operations as `def Str.name { }` methods:
      `.length`, `.upper`, `.lower`, `.split`, `.strip_prefix`,
      `.strip_suffix`, `.replace`, `.contains`
- [ ] Sigil aliases: `$#x` for `.length`, `$"x` for `.join` on
      List (rc heritage)
- [ ] Struct accessors auto-generated from declarations
      (named `.field` and positional `.N`)
- [ ] Enum Prism preview accessors `$v .variant` returning
      `Option(Payload)`

### Phase 7: Map type

- [ ] `Map(K, V)` as a parametric type constructor
- [ ] Construction form (under design discussion)
- [ ] `.get`, `.set`, `.keys`, `.values` methods (key-indexed
      view is affine traversal; iterate-all-values is traversal)

### Phase 8: job control and interactive features

- [ ] Job table with `JobStatus` tracking
- [ ] Builtins: `fg`, `bg`, `jobs`, `wait` (with `-n`), `kill`
- [ ] `%N` word form expanding to job PIDs
- [ ] REPL with line editing (reedline or rustyline)
- [ ] History (persistent `~/.psh_history`, Ctrl-R search)
- [ ] Tab completion (filesystem paths, variable names, def
      names)

### Phase 9: spec conformance tests

- [ ] Integration tests with every example from
      `docs/specification.md` and `docs/syntax.md`
- [ ] Property tests for structural invariants (no-rescan,
      sequence preservation, CBV focusing, bidirectional check
      soundness)
- [ ] Adapted ksh93u+m test suite for rc-heritage features
- [ ] Fork-based tests must run single-threaded
      (`--test-threads=1`)

### Phase 10: polish

- [ ] Error messages with source locations
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
