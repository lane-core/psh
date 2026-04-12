# Plan

Current roadmap. Living document — update when tasks complete,
priorities change, or new work is identified.

## Current state

**Design: first draft complete.** The specification and grammar
are in `docs/specification.md` and `docs/syntax.md`. Both have
been through a six-agent review and verification pass. All typing
rules use classical sequent notation with derivable mode
annotations. All formal claims verified against cited references.

**Implementation: not yet started.** The source tree is a stub:

- `src/main.rs` — binary stub, exits with 2
- `src/parse.rs` — combine boilerplate (character predicates,
  trivia, keyword/name primitives)
- `src/signal.rs` — self-pipe signal handling (preserved,
  type-system neutral)

Dependencies per `docs/implementation.md`: `anyhow`, `bpaf`,
`combine`, `fnmatch-regex`, `libc`, `rustix`, `smallvec`,
`signals_receipts`.

## Implementation roadmap

Build sequence. Each phase depends on the previous.

### Phase 1: parser

Implement the grammar from `docs/syntax.md` using `combine`.

- [ ] Fix character predicates (`var_char`, `word_char`) against
      the spec
- [ ] Lexical: single-quoted and double-quoted strings (with
      `$var`, `$var[i]`, `` `{cmd} `` interpolation in double
      quotes; `${name.accessor}` for dot access in double
      quotes), backslash escapes, line continuation
- [ ] Words: literals, variable references with tight-binding
      dot and bracket accessors, `??` nil-coalescing, `$((...))`,
      command substitution `` `{} ``, process substitution `<{}`,
      tilde expansion, free caret `^` (explicit only — `.` is
      always accessor), lambdas `|x| => body`
- [ ] Values: tuple `(a, b)` (min 2), list `(a b c)`, type-
      prefixed struct `Pos { x = 10; y = 20 }` / `Pos { 10, 20 }`
      (min 2), map literal `{'key': v}`, enum variant `ok(42)` /
      bare `none`, here documents (`<<`, `<<'`, `<<-`, `<<[n]`),
      here-strings `<<<`
- [ ] Expressions: precedence tower (or `||`, and `&&`, pattern
      match `=~`, pipeline `|` / `|[n]` / `|[n=m]`, negation `!`,
      background `&`), per-command locals `VAR=val cmd`
- [ ] Control flow: `if(cond)`, `if let pat = rhs`, `while(cond)`,
      `for(x in list)`, `match(expr) { arms }` with guards
      `if(cond)` and `|` alternation, `try { } catch (e) { }`,
      `trap SIGNAL (body body?)?`
- [ ] Bindings: `let` with pattern LHS (tuple, struct, enum),
      `let-else` for refutable patterns, `def` with return type
      annotation, `struct`, `enum`, assignment `x = val`
- [ ] Patterns: enum variant (tagged + nullary), struct (named
      + positional, type-prefixed), tuple, wildcard `_`, literal
- [ ] Type annotations: `Type`, parametric `Result(Int, Str)`,
      function `Str -> Int`
- [ ] Redirections: `>`, `>>`, `<`, `>[n=m]`, `<[n=m]`

### Phase 2: AST, value model, type checker

- [ ] AST: three-sort structure — `Term` (producers/terms),
      `Command` (consumers/coterms — command shapes), `Expr`
      (cuts — pipelines, redirections, fork/exec).
- [ ] `Val` enum: `Str`, `Int`, `List`, `Tuple`, `Struct`,
      `Map`, `Sum` (enum values), `Lambda`, `Status`
- [ ] Every variable is a list at the outer level
- [ ] Bidirectional type checker (~500-900 lines):
  - [ ] Synth/check modes per the classical rules with mode
        annotations in the spec
  - [ ] Type parameter pinning for parametric types
  - [ ] Lambda parameter pinning (write-once slots from body)
  - [ ] Guard purity checking (reject commands in guard exprs)
  - [ ] Pattern exhaustiveness (conservative for guarded arms)
  - [ ] Error messages with source locations — invest here early

### Phase 3: evaluator

- [ ] Word expansion (Kleisli pipeline: tilde → parameter →
      command sub → arithmetic → glob)
- [ ] Glob no-match: non-matching glob stands for itself (rc)
- [ ] Command execution, pipeline forking, fd plumbing
- [ ] Polarity frames at ↓→↑ shift sites (command substitution,
      `$((...))`, `.refresh` and `.set` bodies)
- [ ] CBPV `let` — effectful RHS via μ̃-binder
- [ ] Per-command local variables (`VAR=val cmd`)
- [ ] Discipline functions: `.get` (pure, CBV focused), `.refresh`
      (effectful), `.set` (mutator with slot-write primitive)
- [ ] Redirections as profunctor wrapping (Adapter)
- [ ] fd table save/restore (Lens)
- [ ] `$status : Int`, `$pipestatus : List(Int)`

### Phase 4: control flow and error model

- [ ] `if`/`else`, `if let`, `while`, `for`
- [ ] `match` with pattern dispatch, guards, `|` alternation
- [ ] `let-else`, pattern lets
- [ ] `??` nil-coalescing (desugar to match on Option)
- [ ] `try`/`catch` as scoped ErrorT
- [ ] Unified `trap` (lexical / global / deletion)
- [ ] Signal delivery via self-pipe, EINTR retry

### Phase 5: types and methods

- [ ] Struct declaration: auto-generate named accessors (`.field`
      Lens), `.fields` (Getter), `.values` (Getter, homogeneous)
- [ ] Enum declaration: auto-generate Prism previews (`.ok`,
      `.err` returning `Option(Payload)`)
- [ ] Option Display: `some(v)` → `v`, `none` → empty string
- [ ] `Map(V)`: brace literal, `.insert` builder, `Map.from_list`,
      bracket access `$m['key']`, bracket assignment, `.keys`,
      `.values`
- [ ] String methods: `.length`, `.upper`, `.lower`, `.split`,
      `.strip_prefix`, `.strip_suffix`, `.replace`, `.contains`
- [ ] Sigil aliases: `$#x` = `.length`, `$"x` = `.join`

### Phase 6: coprocesses

- [ ] Named coprocess registry
- [ ] Per-tag binary session state machine (phantom types)
- [ ] Negotiate on tag 0, orderly teardown via close frame
- [ ] Cancellation: per-tag internal choice (⊕), Tflush/Rflush
- [ ] Wire format: length-prefixed, first-byte dispatch,
      MAX_FRAME_SIZE 16 MiB
- [ ] `print -p` returns Int tag, `read -p [-t tag]` reads
      response, PendingReply tracking, drop-as-cancel

### Phase 7: job control and interactive

- [ ] Job table, `fg`/`bg`/`jobs`/`wait -n`/`kill`, `%N` word
- [ ] REPL with line editing
- [ ] History, Ctrl-R search
- [ ] Tab completion (paths, variables, def names, struct fields
      after type-prefixed `{`)

### Phase 8: conformance and polish

- [ ] Integration tests from spec/syntax examples
- [ ] Property tests (no-rescan, sequence preservation, CBV
      focusing, bidirectional soundness)
- [ ] `ulimit`, `umask`, `export` (listing form)
- [ ] `printf`, `shift`, `$0`, positional parameters

## Non-goals

See specification.md §"Features and non-goals" for the full
list with reasoning. Key items: no parametric polymorphism on
`def` signatures, no typed session channels on pipes, no
pipeline fusion as user-visible feature.
