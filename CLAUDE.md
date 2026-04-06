# psh project instructions

## What this is

psh is the pane system shell — rc's successor with ksh93 discipline
functions, typed values, and duploid-structured internals. It is a
standalone shell first, a pane client second.

Read PLAN.md for the roadmap and session log. Read
docs/specification.md for the theoretical foundation. Read
STYLEGUIDE.md for coding conventions.

## Quick start

```
cargo test -- --test-threads=1    # tests require single-threaded (fork)
cargo run -- -c 'echo hello'      # run a command
cargo run -- script.psh            # run a script
cargo run                          # interactive REPL (basic)
```

## Key design decisions

- **Val is a 6-variant enum** (Unit, Bool, Int, Str, Path, List<Val>).
  Type inference runs in `let` context only. Bare `x = val` produces
  Str (rc heritage). See value.rs.
- **let is the typed μ̃-binder.** Qualifiers: mut, export, : Type.
  Local scope by default. Bare `x = val` walks scope chain (rc heritage).
- **Profunctor AST.** Redirections wrap expressions. Left-to-right
  evaluation order is structural, not conventional.
- **⊕ error convention only.** Every function returns Status. No longjmp.
- **par is NOT a direct dependency.** Enters through pane-session only
  (feature-gated).
- **Tests need `--test-threads=1`.** Fork-based tests interfere in parallel.

## Committing

After completing a planned task where all tests pass, commit without
asking. Use a descriptive message summarizing the work.

### Commit message format

Two-paragraph body after the subject line. First paragraph describes
the user's provenance in third person, using their name: the decision
procedure, thought process, design direction. Second paragraph begins
with "Agent steps:" and describes what the agent did.

## Heritage

- **rc** (Duff 1990): grammar, value model (lists), quoting, philosophy
- **ksh93** (Korn): discipline functions, coprocesses, namerefs, let/typeset
- **ksh26 SPEC.md**: polarity/duploid analysis of ksh93 (at /Users/lane/src/ksh/ksh/SPEC.md)
- **pane**: duploid framework, session types, MonadicLens, namespace integration

## Agent workflow

Use the four design agents (Plan 9, Be, session type, optics) for
significant design decisions. Submit proposals, run deliberation
rounds. See serena `pane/agent_workflow` for the full process.

## References

| Source | Location |
|--------|----------|
| ksh93u+m | /Users/lane/src/ksh93 |
| ksh26 | /Users/lane/src/ksh/ksh |
| pane | /Users/lane/src/lin/pane |
| Plan 9 | /Users/lane/src/lin/pane/reference/plan9/ |
