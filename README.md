psh - a pleasant command shell
==============================

psh is a system shell descended from rc with typed values,
discipline functions, and first-class lambdas. It runs as a
login shell on any Unix without additional infrastructure.

    let greet = \name => echo hello $name
    $greet world

Requirements
------------

- Rust 1.70+
- A Unix-like operating system (Linux, macOS, *BSD)

Building
--------

    cargo build --release
    cp target/release/psh /usr/local/bin/

Running
-------

    psh                          # interactive
    psh -c 'echo hello'         # one command
    psh script.psh              # run a file

Testing
-------

    cargo test --bin psh -- --test-threads=1

Features
--------

- rc grammar: single quotes, free carets, `{cmd} substitution
- 10-variant typed values: Unit, Bool, Int, Str, Path,
  ExitCode, List, Tuple, Sum, Thunk
- match with glob and structural (coproduct) arms
- try blocks: scoped error handling, value-position capture
- first-class lambdas with capture-by-value (\x => body)
- discipline functions (.get/.set) as MonadicLens hooks
- profunctor redirections (left-to-right by nesting)
- coprocesses via socketpair
- job control (fg, bg, wait, Ctrl-Z)
- signal handlers as functions (fn sigint { })
- namerefs (ref x = target)
- pane namespace integration (feature-gated)

Design
------

Four connectives from linear logic map to shell constructs:

    tensor  Tuple     products      Lens
    plus    Sum       coproducts    Prism
    par     |&        coprocesses   session
    with    fn/.get   handlers      offer

The evaluator separates values (positive, CBV) from
computations (negative, CBN pipelines). fn defines named
functions in command position. \ creates lambdas in value
position. return marks the polarity shift. Pipes are cuts.

Documentation
-------------

    docs/syntax.md          normative grammar
    docs/specification.md   theoretical foundation
    docs/api.md             plugin and host API

Heritage
--------

rc (Duff 1990), ksh93 (discipline functions, coprocesses,
namerefs).

References
----------

- Tom Duff. "Rc — The Plan 9 Shell." 1990.
- Mangel, Melliès, Munch-Maccagnoni. "Classical notions of
  computation and the Hasegawa-Thielecke theorem." POPL, 2026.
- Munch-Maccagnoni. "Models of a Non-Associative Composition."
  FoSSaCS, 2014.
- Levy. *Call-by-Push-Value.* Springer, 2004.
- Curien, Herbelin. "The duality of computation." ICFP, 2000.
- Clarke et al. "Profunctor Optics, a Categorical Update."
  Compositionality, 2024.

License
-------

See LICENSE.
