# psh plugin and host API

The plugin API is the shell itself. Three existing mechanisms —
discipline functions, the unified namespace, and function conventions
— constitute the extensibility surface. No plugin framework.

This document specifies the plugin model, the host interface, and
the visual protocol. It is the output of a four-agent roundtable
(Plan 9, Be, session type, optics) synthesized and refined by Lane.


## Thesis

Discipline functions ARE the plugin interface. The namespace IS the
plugin data layer. Function conventions ARE the visual protocol.
`menu` IS the interactive primitive. A plugin is a psh script that
defines functions and disciplines.

No plugin system needs to be designed.


## The three mechanisms

### 1. Discipline functions

`.get` fires on variable read as a notification hook. The body
runs in a readonly scope — mutations are rejected except for the
discipline's own variable (the "self-update" exception, see
§Readonly resolution below). `.set` fires on write with a
reentrancy guard that prevents `fn x.set { x = $1 }` from
recursing infinitely.

Function wrapping (`fn cd { builtin cd $*; ... }`) intercepts
builtins. Signal functions (`fn sigint { }`) handle signals.

### 2. Unified namespace

`get`/`set` resolve against three tiers:

| Tier | Scope | Structural rules |
|------|-------|------------------|
| Shell variables | `$x` — scope chain lookup | Weakening, contraction, exchange (classical) |
| Process environment | `$PATH` — flat key-value | Weakening, contraction, exchange (classical) |
| Pane namespace | `/pane/editor/attrs/cursor` | Weakening, exchange. No contraction (affine) |

Live re-evaluation uses `.get` disciplines:

    let mut cursor : Int = 0
    fn cursor.get { cursor = `{ get /pane/editor/attrs/cursor } }

Every `$cursor` access fires the discipline, refreshing the
stored value. For error-tracked live variables:

    let mut cursor : Result[Int] = try { get /pane/editor/attrs/cursor }
    fn cursor.get { cursor = try { get /pane/editor/attrs/cursor } }

### 3. Function conventions

Plugins register behavior by defining functions with conventional
names. The shell and host discover capabilities by checking
whether functions exist.

    fn status.git { ... }       # status line segment
    fn status.path { ... }      # status line segment
    fn complete.history { ... }  # completion source
    fn keymap.leader { ... }    # key binding handler
    fn prompt { ... }           # prompt renderer


## The `menu` protocol

`menu` is a builtin that sends a structured request to the host
and returns the user's selection. It is a MonadicPrism — effectful
partial decomposition. The user interaction (rendering, input) is
the effect; the selection (or its absence) is the prism result.

### Return type: three-tag Sum

    menu returns: selected $v | cancelled () | err $e

- `selected $v` — the user chose item `v`. Type of `v`
  inherits the input list's element type.
- `cancelled ()` — the user dismissed the menu (Escape,
  click-away, timeout). Not a failure — a deliberate absence.
- `err $e` — the host is unavailable or the request is
  malformed. An actual error.

The three-tag model lets plugins distinguish "user said no"
from "host unavailable":

    let choice = menu -popup (a b c)
    match $choice {
        selected $v => handle $v;
        cancelled   => ();          # user changed their mind
        err $e      => echo 'host error: '$e
    }

### Menu styles (hints, not commands)

The shell sends a style hint. The host renders however it can.
A terminal host may ignore `popup` and render as a numbered
list. A pane-terminal host may render a compositor popup.

    menu -popup $items           # popup overlay
    menu -vertical $items        # scrollable vertical list
    menu -vertical -fuzzy $items # vertical with fuzzy filter
    menu -transient $items       # auto-dismiss after one key
    menu -prompt 'search: ' -filter  # input-driven filter

### Menu request as Val

The menu request IS a Val — no special wire format. Items are
a List. Richer items are Tuples or Sums:

    # Simple: List[Str]
    menu -popup (stage commit push)

    # Structured: List[Tuple]
    menu -transient (
        (label 'Stage'  key s)
        (label 'Commit' key c)
        (label 'Push'   key p enabled false)
    )


## Visual plugin examples

All examples use the full syntax.md grammar.

### Which-key popup

    fn keymap.leader {
        let choice = menu -popup -timeout 2 ('g : git' 'f : files' 'p : pane')
        match $choice {
            selected $k => match $k {
                g* => git_menu;
                f* => file_menu;
                p* => pane_menu
            };
            cancelled => ();
            err $e    => ()
        }
    }

### Vertical fuzzy completion (ivy/vertico style)

    fn complete.history {
        menu -vertical -fuzzy `{ history -list }
    }

### Transient command menu (magit/transient style)

    fn git_menu {
        let choice = menu -transient ('s : stage' 'c : commit' 'p : push')
        match $choice {
            selected $action => match $action {
                s* => git add -p;
                c* => git commit;
                p* => git push
            };
            cancelled => ();
            err $e    => ()
        }
    }

### Status line segments

    fn status.git {
        let branch = try { `{ git branch --show-current } }
        match $branch {
            ok $b  => return $b;
            err $e => return ''
        }
    }
    fn status.path { return `{ basename `{ pwd } } }
    fn status.jobs { return $#jobs }

### Prompt rendering

    fn prompt {
        let git = `{ status.git }
        let dir = `{ status.path }
        echo $dir' '$git'$ '
    }

### Directory hooks

    fn cd {
        builtin cd $*
        # Auto-activate virtualenv
        if test -f .venv/bin/activate {
            . .venv/bin/activate
        }
    }


## The host interface

Two layers, matching the two-binary architecture:

    psh (interpreter)       produces structured requests
    host (rendering)        consumes requests, renders, returns results

The host is a Rust trait. Implementations exist for:
- **reedline** — standalone terminal (compiled into psh binary)
- **pane-terminal** — LooperCore, compositor rendering
- **dumb** — graceful degradation (no interactive features)

### The Host trait

```rust
pub trait Host {
    /// Present a selection and return the chosen item.
    fn menu(&mut self, req: MenuRequest) -> Result<Val, HostError>;

    /// Request a line of input with a prompt.
    fn prompt(&mut self, req: PromptRequest) -> Result<String, HostError>;

    /// Update the status line.
    fn status(&mut self, segments: Option<&[StyledSegment]>) -> Result<(), HostError>;

    /// Request tab completion candidates.
    fn complete(&mut self, req: CompleteRequest) -> Result<Option<String>, HostError>;

    /// Query host capabilities (once at startup).
    fn capabilities(&self) -> HostCapabilities;

    /// Deliver a non-blocking notification.
    fn notify(&mut self, note: Notification) -> Result<(), HostError>;
}
```

The shell never describes how to render — only what to present.
Each method takes structured data and returns a result. The host
decides the visual form.

### Capability negotiation

    pub struct HostCapabilities {
        pub color: ColorSupport,
        pub popup_menu: bool,
        pub inline_completion: bool,
        pub status_line: bool,
        pub unicode: bool,
        pub mouse: bool,
    }

    pub enum ColorSupport { None, Ansi16, Ansi256, TrueColor }

Capabilities are queried once at startup. The shell adapts its
requests — a dumb host gets no menu requests; a terminal host
gets vertical lists instead of popups. No progressive enhancement
at runtime.

### Supporting types

```rust
pub struct MenuRequest {
    pub title: Option<String>,
    pub items: Vec<MenuItem>,
    pub style: MenuStyle,
    pub default: Option<usize>,
}

pub struct MenuItem {
    pub label: String,
    pub value: Val,
    pub key: Option<char>,
    pub enabled: bool,
    pub group: Option<String>,
}

pub enum MenuStyle {
    Popup, Vertical, Inline, Transient,
}

pub struct PromptRequest {
    pub segments: Vec<StyledSegment>,
    pub history_key: Option<String>,
    pub multiline: bool,
}

pub struct StyledSegment {
    pub text: String,
    pub role: SemanticRole,
}

pub struct CompleteRequest {
    pub input: String,
    pub cursor: usize,
    pub candidates: Vec<Candidate>,
}
```


## Syntax highlighting

Syntax highlighting is NOT an optic — it is a stateful transducer.
A separate fast lexer (`highlight.rs`) classifies tokens. The host
maps semantic roles to colors.

The shell produces semantic markup:

```rust
pub enum SemanticRole {
    Keyword, Builtin, Command, Argument,
    Variable, Substitution, String, Comment,
    Operator, Redirect, Path, Number,
    Error, Plain,
}
```

The host renders:

```rust
impl TerminalHost {
    fn role_to_color(&self, role: SemanticRole) -> Color {
        match role {
            SemanticRole::Keyword => self.theme.keyword,
            SemanticRole::Error => self.theme.error,
            // ...
        }
    }
}
```

The lexer runs on the visible line only, in under a millisecond.
It handles incomplete input (unclosed quotes, partial commands)
gracefully. It is NOT the full parser — it is a separate, simpler
state machine optimized for speed over correctness at boundaries.


## Semantic colors

The shell names meaning. The host names appearance. No raw ANSI
codes cross the shell-to-host protocol.

```rust
pub enum SemanticColor {
    Error, Warning, Success,
    Path, Keyword, Builtin, Variable, String, Comment, Operator,
    PromptPrimary, PromptSecondary,
    StatusText, StatusBackground,
    SelectionBackground, SelectionText,
    MatchHighlight,
}
```

This is psh's equivalent of Be's `ui_color()` / `color_which`.
Plugins that want "error = red" use `SemanticColor::Error`. The
host decides what error looks like. A dark theme, a light theme,
and a no-color mode all work with the same plugin code.


## Plugin composition

### Function composition: last wins

rc model. The last `fn cd` definition replaces earlier ones.
Explicit chaining via thunk capture:

    let _prev_cd = \args => { builtin cd $args }
    fn cd {
        $_prev_cd $*
        echo 'now in '`{ pwd }
    }

No handler chains. No union semantics. No framework. The thunk
sort is the mechanism for capturing a function as a value — the
`\` lambda snapshots behavior that `fn` can then wrap.

### Plugin loading: explicit sourcing

    ~/.psh/
        env             # sourced on every shell startup
        profile         # sourced on interactive startup (after env)
        lib/            # user's function libraries

Loading order:

1. `/etc/psh/env` (system-wide, all instances)
2. `~/.psh/env` (user, all instances)
3. `/etc/psh/profile` (system-wide, interactive only)
4. `~/.psh/profile` (user, interactive only)

Plugins are sourced explicitly from profile:

    # ~/.psh/profile
    . ~/.psh/lib/git-helpers
    . ~/.psh/lib/pane-shortcuts

No auto-discovery. No directory scanning. No dependency
resolution. If a library depends on another, it sources it.

### Manifest convention (optional)

For lazy discovery without sourcing entire plugins:

    fn git.provides {
        return (git.prompt git.complete git.menu)
    }


## Optic structure of plugin patterns

| Pattern | Optic | Notes |
|---------|-------|-------|
| menu | MonadicPrism | Effectful decomposition, not AffineTraversal |
| Keymap dispatch | Prism per trie level | Standard coproduct elimination |
| Completion | Fold + MonadicPrism | Plugin provides Fold, host resolves |
| Status line | Fold over Getters | fn status.* enumerated, read-only |
| Accessor composition | AffineFold | $result.ok.line — Prism then Lens, cross-plugin sound |

Optics break at: syntax highlighting (stateful transducer),
event streams (session types), geometry-dependent layout (Glass
optic — exotic, Val doesn't provide).


## Open design decisions

### 1. Function disciplines (.before/.after)

Should psh support `fn cd.before { }` and `fn cd.after { }`
for composable hooks? Session type analysis confirms the
protocols compose (sequential). Plan 9 and Be prefer explicit
wrapping via thunks (simpler, debuggable).

Current position: deferred. Explicit wrapping is sufficient
for v1. Function disciplines can be added if real-world plugin
composition demands them.

### 2. .get readonly resolution

The live re-evaluation pattern (`fn cursor.get { cursor = ... }`)
contradicts the readonly scope claim. Resolution: `.get` is
readonly except for the discipline's own variable. The
reentrancy bound is depth-2: `.get` fires → body assigns to
own variable → `.set` fires (if defined) → `.set` guarded
against re-triggering `.get`. The bound is not obvious; it
must be documented.

### 3. Namespace discoverability

Plan 9 let you `ls` the namespace to discover what was
available. psh's functions aren't in the namespace — `get /fn/cd`
doesn't work. `whatis` provides the information through a
separate mechanism. Whether to unify this (make functions
discoverable through the namespace) is a design question for
later.


## The acid test: package manager

A package manager validates the plugin API using the full
syntax.md grammar:

    fn pkg {
        match $1 {
            install => {
                let results = `{ curl -s `{get pkg.registry}^/search?q=$2 }
                let choice = menu -vertical -fuzzy $results
                match $choice {
                    selected $name => {
                        curl -s `{get pkg.registry}^/$name/latest > ~/.psh/lib/$name^.psh
                        . ~/.psh/lib/$name^.psh
                        echo 'pkg: installed '$name
                    };
                    cancelled => echo cancelled;
                    err $e    => echo 'pkg: '$e
                }
            };
            search => {
                let results = `{ curl -s `{get pkg.registry}^/search?q=$2 }
                echo $results
            };
            list => {
                for f in ~/.psh/lib/*.psh => echo `{ basename $f .psh }
            };
            remove => {
                try {
                    rm ~/.psh/lib/$2^.psh
                    echo 'pkg: removed '$2
                } else $e {
                    echo 'pkg: '$2' not installed'
                }
            };
            * => echo 'usage: pkg install|search|list|remove'
        }
    }


## References

1. Tom Duff. "Rc — The Plan 9 Shell." 1990.
2. Be/Haiku: BTranslatorRoster, BAlert, BMenu, BHandler chains,
   ui_color(), BTextView text_run_array.
3. Clarke et al. "Profunctor Optics, a Categorical Update."
   Compositionality, 2024.
4. Levy. *Call-by-Push-Value.* Springer, 2004.
5. psh specification: docs/syntax.md, docs/specification.md.
