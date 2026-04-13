# Invocation and startup


## Invocation modes

psh operates in one of three modes, determined at startup:

| Mode | Detection | Behavior |
|------|-----------|----------|
| **Login shell** | `argv[0]` starts with `-`, or `-l` flag | Sources system + user profile |
| **Interactive** | stdin is a terminal and no script argument | Prompt, job control, line editing |
| **Script** | Script filename argument, or `-c string` | Non-interactive, no prompt |

A login shell is also interactive (unless `-c` is given). An
interactive shell may or may not be a login shell.

### Login detection

Two conventions, both recognized:

1. `argv[0]` starts with `-` — the `login(1)` / `sshd` / `getty`
   convention. These programs exec the shell with `-psh` as
   argv[0].
2. `-l` flag — explicit request.

rc heritage [Duf90, rc(1) lines 940-949]: "If `-l` is given or
the first character of argument zero is `-`, rc reads commands
from `$home/lib/profile`."


## Startup file sequence

### Login shell

1. `/etc/psh/pshrc` — system-wide configuration
   (administrator-controlled). Sourced first if it exists.
2. `/etc/psh/pshrc.d/*.psh` — system drop-in fragments,
   lexicographic order.
3. `$HOME/.config/psh/profile` — user login profile. Sourced
   if it exists. XDG convention (`$XDG_CONFIG_HOME/psh/profile`
   when `$XDG_CONFIG_HOME` is set).
4. `$HOME/.config/psh/rc` — user interactive configuration
   (login shells are also interactive).
5. `$HOME/.config/psh/rc.d/*.psh` — user drop-in fragments,
   lexicographic order.

### Interactive (non-login)

1. `/etc/psh/pshrc` — system-wide configuration (same as login).
2. `/etc/psh/pshrc.d/*.psh` — system drop-in fragments.
3. `$HOME/.config/psh/rc` — user interactive configuration.
   Sourced if it exists.
4. `$HOME/.config/psh/rc.d/*.psh` — user drop-in fragments.

### Script

No startup files are sourced. The script is the input.

### `-c string`

No startup files are sourced. The string is the input. `-c` is
essential for ssh (`ssh host 'command'`), cron, and
programmatic shell invocation.

### Sourcing mechanism

Startup files are executed via the `.` (source) builtin in the
current scope. See §15-builtins.md for the `.` builtin
specification.

psh does NOT source `.bashrc`, `.bash_profile`, `.profile`,
`.zshrc`, or any other shell's configuration files. Each shell
has its own configuration namespace. Tools that write to
`.profile` (nix, cargo, rustup, pyenv) will need equivalent
entries in `$HOME/.config/psh/profile`.


## Invocation flags

    psh [-cilI] [-o option]... [-c string | file [arg...]]

### Invocation-only flags

These are meaningful only at startup and cannot be changed
at runtime via `set`:

| Flag | Meaning |
|------|---------|
| `-c string` | Execute `string` as input, then exit |
| `-l` | Login shell (source profile) |
| `-i` | Force interactive mode |
| `-I` | Force non-interactive mode |

### Flags settable at invocation and runtime

These can be passed on the command line (`psh -x script.sh`)
or toggled at runtime via `set -o name` / `set +o name`:

| Short | Long (`set -o`) | Default | Axis |
|-------|-----------------|---------|------|
| `-x` | `xtrace` | off | Print commands as executed |
| `-v` | `verbose` | off | Echo input as read |
| `-n` | `noexec` | off | Parse and check, don't execute |
| `-C` | `noclobber` | **on** | `>` won't truncate; `>\|` to override |
| `-m` | `monitor` | on (interactive) | Job control |
| `-b` | `notify` | off | Immediate job completion notification |
| | `pipefail` | **on** | `$status` = last nonzero of `$pipestatus` |
| | `ignoreeof` | off | Don't exit on EOF; require `exit` |
| | `linear` | off | Bindings default to linear zone |
| | `emacs` | (see below) | Emacs-style line editing |
| | `vi` | (see below) | Vi-style line editing |
| | `markdirs` | off | Append `/` to directories in glob |
| | `globstar` | off | Enable `**` recursive glob |

### Design principles

**Each option is a behavioral axis.** An option changes how the
evaluator works along one dimension. No bundles, no cosmetic
knobs, no editor configuration.

**Safe by default.** `noclobber` and `pipefail` default to on.
psh does not silently truncate files or hide pipeline failures.
The user opts out explicitly (`set +o noclobber`, `>\|`).

**No `errexit`.** psh's `try`/`catch` (§12-errors.md) replaces
`set -e` with a lexically scoped, composable mechanism. The
POSIX `set -e` has well-documented composability defects that
`try`/`catch` eliminates structurally.

**No `nounset`.** psh's type checker catches unset variable
access at parse/check time. The Bourne-era runtime surprise
is eliminated structurally.

**Editor mode.** `set -o emacs` and `set -o vi` select the
line editing mode. They are mutually exclusive — enabling one
disables the other. The default is determined by `$EDITOR` or
`$VISUAL` at startup (ksh93 convention: if the value contains
`vi`, vi mode; otherwise emacs mode). Explicit `set -o emacs`
or `set -o vi` overrides the environment inference.


## The `set` builtin

    set -o option       # enable option
    set +o option       # disable option
    set -o              # print all options and their current state
    set -ShortFlag      # enable by short flag (e.g., set -x)
    set +ShortFlag      # disable by short flag

`set` with no arguments prints all shell variables (rc heritage:
rc.ms §Built-in commands, "With no arguments, `set` prints the
values of all variables").

`set` with `--` stops option processing; remaining arguments
become positional parameters:

    set -- a b c        # $1=a, $2=b, $3=c

### Scoped option changes

Options set with `set -o` are dynamic — they affect all
subsequent execution in the current scope. For scoped option
changes, use a subshell:

    @{ set -o linear; critical_section }

The subshell inherits the parent's options, applies the change,
and the change dies with the subshell. This is the composable
pattern for "linear mode in this section only."


## Environment inheritance

### Startup: environ(7) → psh namespace

On startup, psh scans the process environment (`environ(7)`)
and creates a shell variable for each `NAME=VALUE` pair:

- Each inherited variable has type `Str` (the environment is
  flat strings).
- Each inherited variable is marked `export` — it will
  propagate to child processes.
- The variable is placed in the classical zone (`!Str`) — the
  environment admits contraction and weakening.

### The PATH convention

psh uses `$PATH` (uppercase, POSIX convention), not `$path`
(lowercase, rc convention). rc could use lowercase `$path`
because Plan 9's `/env` filesystem backed it — the kernel
resolved the name. On Unix, `execvp(3)` reads `PATH` from the
process environment. Using `$path` would require a
synchronization discipline between the shell variable and the
environment entry. The honest adaptation to Unix is to use the
name the kernel reads: `$PATH`.

Other POSIX-convention variables follow the same rule: `$HOME`,
`$USER`, `$SHELL`, `$TERM`, `$LANG`, `$EDITOR`, `$VISUAL`.
psh does not introduce rc-style lowercase aliases for these.

### Special variables set at startup

| Variable | Type | Value |
|----------|------|-------|
| `$0` | Str | Shell name or script path |
| `$pid` | Int | Current process ID (rc heritage) |
| `$ppid` | Int | Parent process ID |
| `$apid` | Int | PID of last background process (rc heritage: rc(1) lines 47-49) |
| `$status` | ExitCode | Initially `ExitCode { code = 0; message = '' }` |
| `$pipestatus` | List(ExitCode) | Initially `($status)` |
| `$PWD` | Path | Current working directory |
| `$OLDPWD` | Path | Previous working directory (initially `$PWD`) |
| `$SHLVL` | Int | Shell nesting depth (incremented on each invocation) |
| `$COLUMNS` | Int | Terminal width (updated on SIGWINCH) |
| `$LINES` | Int | Terminal height (updated on SIGWINCH) |
| `$HOSTNAME` | Str | System hostname (from `gethostname(2)`) |
| `$CDPATH` | List(Path) | cd search path (empty by default; see `cd` in §15-builtins.md) |
| `$prompt` | List(Str) | `('% ' '  ')` — primary and continuation prompts |
| `$*` | List(Str) | Positional parameters (script args or `set --` args) |

`$apid` uses rc's name (rc(1) lines 47-49: "whenever a command
is followed by `&`, the variable `$apid` is set to its process
id"). ksh93 uses `$!` for the same purpose (sh.1 lines 5150-
5155). psh follows rc's naming — `$apid` is clearer than `$!`
and does not collide with the `!` negation operator.

`$COLUMNS` and `$LINES` are set from the terminal size at
startup and updated on SIGWINCH. They are available to scripts
for layout calculations but are not authoritative — `stty size`
or `ioctl(TIOCGWINSZ)` give the canonical terminal size.

`$CDPATH` is a list of directories searched by `cd` when the
argument is a relative path. Empty by default. Inherited from
the environment if set (colon-delimited, decomposed into a
list). See `cd` in §15-builtins.md.


## Export semantics

`export` marks a shell variable for projection from Tier 1
(shell variables) to Tier 2 (process environment) on every
child exec.

### Mark-for-projection, not snapshot

`let export x = expr` marks `x` for automatic projection to the
child environment. On every `exec`, the shell materializes the
current value of `x` into the child's environment. It is not a
one-time copy — the child sees the value at exec-time.

ksh93 heritage (sh.1 lines 4050-4076): "On invocation, the
shell scans the environment and creates a variable for each name
found, giving it the corresponding value and attributes and
marking it export."

### Export invokes `.get`

When materializing an exported variable for the child
environment, the shell fires the `.get` discipline if one is
defined. The exported value is the observed value — the codata
observer's output, not the raw slot. This is consistent with
§08-discipline.md: `.get` is the interface through which all
variable access flows.

### Export requires classical zone

The process environment admits contraction and weakening
(§11-namespace.md). Only `!`-typed (classical) bindings are
structurally compatible with the environment. `let export` on a
linear or affine binding is a type error.

    let export x = 'hello'              # OK — Str is classical
    let export !fd = dup $log_fd        # OK — ! promotes to classical
    let export fd : Fd = open 'lock'    # ERROR — Fd is linear

### Serialization

The process environment is flat `name=value` strings. psh
serializes exported values per type:

| Type | Serialization |
|------|---------------|
| Str | identity |
| Int | decimal string |
| Bool | `'true'` / `'false'` |
| Path | components joined with `/` |
| ExitCode | decimal code string |
| List(Str) | elements joined with `\x01` (ctrl-A) — rc heritage [Duf90, rc(1) lines 795-799] |
| List(Path) | each path joined with `/`, paths joined with `\x01` |

Compound types (Tuple, Struct, Enum) are not directly
exportable. If a compound-typed variable has a `.get`
discipline returning Str, the discipline's output is exported.
Otherwise, `let export` on a compound type is a type error.

### Per-command local variables

`VAR=value cmd` scopes the assignment to the duration of a
single command. The variable is projected into the child's
environment for that command regardless of whether `VAR` is
marked `export`. This is environment-setup syntax — it always
affects the child's environment.

rc heritage [Duf90, rc.ms lines 1045-1066]. ksh93 heritage
(sh.1 lines 4050-4076).

    PATH='/custom/bin' make install   # child sees modified PATH

### Listing and removing exports

    set -o             # shows all options including export marks
    export             # list all exported variables (standalone form)
    unexport x         # remove export mark from x

`unexport` removes the export mark. The variable remains in
Tier 1 (shell variables) but no longer projects to Tier 2
(process environment) on child exec.


## Signal disposition

### Interactive shells

| Signal | Disposition |
|--------|-----------|
| SIGINT | Interrupt foreground job, return to prompt |
| SIGTSTP | Suspend foreground job (Ctrl-Z) |
| SIGQUIT | Ignored |
| SIGWINCH | Update `$COLUMNS` and `$LINES` |
| SIGCHLD | Reap background jobs, print notifications (if `notify`) |
| SIGHUP | Forward to jobs, then exit |
| SIGTERM | Cleanup, exit |
| SIGTTIN/SIGTTOU | Ignored (required for background process group management) |

### Non-interactive shells (scripts)

| Signal | Disposition |
|--------|-----------|
| SIGINT | Default (terminate) unless `trap` installed |
| SIGTSTP | Ignored |
| All others | Default unless `trap` installed |

### Login shell exit

On exit, a login shell sends SIGHUP to all jobs that have not
been `disown`'d. ksh93 heritage (sh.1 §Signals).


## Exit codes

psh follows POSIX exit code conventions for ecosystem
compatibility:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1-125 | Command-specific failure |
| 126 | Command found but not executable |
| 127 | Command not found |
| 128+N | Killed by signal N |

The ExitCode type (§06-types.md) carries both the numeric code
and an optional message string.


## Prompt

Two mechanisms, in priority order:

**1. `def PROMPT` — computation (preferred).** If a `def`
named `PROMPT` exists, the shell calls it before each command
line. The def returns a list of two strings: primary prompt
and continuation prompt. This is a computation in Θ — full
psh is available (conditionals, variable access, command
substitution, string methods).

    def PROMPT {
        let branch = `{git branch --show-current 2>/dev/null}
        let dir = $PWD.name
        if $branch != '' {
            ("$dir ($branch) % " '  ')
        } else {
            ("$dir % " '  ')
        }
    }

`def PROMPT` fits psh's CBPV model: the prompt is a
computation, not a value. Arbitrary logic runs each time —
git status, exit code coloring, hostname for SSH detection.

**2. `$prompt` variable — simple case.** If no `def PROMPT`
exists, the shell uses `$prompt`, a list of two strings
(rc heritage):

- `$prompt[0]` — primary prompt
- `$prompt[1]` — continuation prompt

Default: `('% ' '  ')` — rc's `%` prompt, two-space
continuation. Prompt strings undergo variable expansion
(not command substitution). `$PWD`, `$USER`, `$SHLVL`
commonly used.

    prompt = ('$USER@$HOSTNAME:$PWD% ' '  ')

`def PROMPT` takes precedence when defined. Undefining it
(`unset -f PROMPT`) falls back to `$prompt`.


## Status line

Interactive shells may display a status line (the bottom line
of the terminal, below the prompt). The status line is
assembled from **segment defs** — functions matching the
naming convention `def status.NAME`.

### Segment convention

Any `def` whose name starts with `status.` is a status line
segment provider. The shell enumerates all `def status.*`
functions, calls each, and concatenates their outputs to form
the status line.

    def status.git {
        let branch = `{git branch --show-current 2>/dev/null}
        if $branch != '' { echo $branch }
    }

    def status.path {
        echo $PWD.name
    }

    def status.jobs {
        let n = $#jobs
        if $n > 0 { echo "$n jobs" }
    }

Segments return a string (their content) or empty string (to
hide). The shell renders them left-to-right in lexicographic
order of the segment name. Segments producing empty output
are omitted.

### Segment ordering

Segment order follows the `def` name lexicographically:
`status.a_git` renders before `status.b_path`. Numeric
prefixes are the convention for explicit ordering:

    def status.10_git { ... }
    def status.20_path { ... }
    def status.90_clock { ... }

### Refresh

The status line refreshes before each prompt (same timing as
`def PROMPT`). Segments should be fast — avoid expensive
operations or network calls. Expensive data (git status,
system load) should be cached in a variable and refreshed via
a `.refresh` discipline on a timer or directory-change hook.

### No status line

If no `def status.*` functions exist, no status line is
displayed. The feature is entirely opt-in. Hosts that do not
support a status line (dumb terminals) ignore segment output.

### Host rendering

The shell produces segment strings. The host decides visual
presentation — colors, separators, positioning. The shell
names content; the host names appearance. This follows the
same separation as `def PROMPT`: the shell computes what to
show, the host decides how to show it.


## History

Interactive shells maintain a command history for recall and
search. The history mechanism is available only in interactive
mode — scripts do not accumulate or search history.

### Variables

| Variable | Type | Default | Meaning |
|----------|------|---------|---------|
| `$HISTFILE` | Path | `$HOME/.config/psh/history` | Persistent history file |
| `$HISTSIZE` | Int | `8192` | Maximum history entries in memory |
| `$HISTFILESIZE` | Int | `$HISTSIZE` | Maximum entries in `$HISTFILE` |

ksh93 heritage (sh.1 lines 5160-5177): `HISTFILE` names the
file for persistent history, `HISTSIZE` caps in-memory entries.
psh follows the same variables with XDG-compliant default path.

### Behavior

Commands are appended to the in-memory history after successful
parsing (even if execution fails — the history records what was
typed, not whether it worked). Duplicate consecutive commands
are stored once (deduplication of immediate repeats only, not
global).

On interactive shell exit, the in-memory history is written to
`$HISTFILE`. On startup, `$HISTFILE` is read into memory if it
exists. History is plain text, one entry per logical line
(multi-line commands joined with embedded newlines).

### Search

`Ctrl-R` initiates reverse incremental search in emacs mode.
Typing characters narrows the search through history entries
matching the typed substring. `Ctrl-R` again cycles to the next
match. `Enter` accepts the match; `Ctrl-C` or `Escape` cancels.

In vi mode, `/` in command mode initiates forward search, `?`
initiates backward search, and `n`/`N` repeat the search
direction. ksh93 heritage (sh.1 lines 5367-5395).

`fc` (fix command) is the programmatic history interface:

    fc -l              # list recent history
    fc -l -20          # list last 20 entries
    fc -l 100 110      # list entries 100-110
    fc -e editor       # edit and re-execute last command
    fc -s old=new      # substitute and re-execute

See §15-builtins.md for the full `fc` specification.

### History expansion

psh does NOT support `!`-style history expansion (`!!`, `!$`,
`!-2`). History expansion is a macro processor — it rescans
input, violating Duff's no-rescan principle [Duf90, §Design
Principles]. Use `Ctrl-R` search, `fc`, or up-arrow recall
instead. This matches rc's approach: rc had no history
expansion; history recall was handled by the `sam`-derived
terminal emulator.


## Completion

Interactive shells provide tab completion for common entities.
Completion is triggered by the `Tab` key (or `Ctrl-I`).

### Built-in completion targets

The shell completes the following without user configuration:

| Context | What completes | Source |
|---------|---------------|--------|
| Command position | Executable names | `$PATH` search, builtins, `def` names |
| Argument position | File paths | Filesystem traversal |
| After `$` | Variable names | Current scope |
| After `$name.` | Named accessors | Per-type accessor namespace (struct fields, enum previews, string methods) |
| After `def Name.` | Type method names | Per-type accessor namespace |
| Inside `match` arms | Enum variant names | Declared variants of the scrutinee's type |

### Path completion

File path completion follows the standard convention: `Tab`
completes the longest common prefix. If ambiguous, a second
`Tab` lists alternatives. Completion respects `$CDPATH` for
`cd` arguments. Hidden files (starting with `.`) are not
completed unless the user has typed the leading `.`.

### Programmable completion

The completion system is extensible. User-defined completion
functions can be registered per command:

    complete cmd_name { |word ctx| =>
        # word: the word being completed
        # ctx: completion context (command position, argument index)
        # return: List(Str) of candidates
    }

The completion function is a `def` cell receiving the partial
word and context, returning a list of candidate strings. This
is the ksh93 model (programmable completion via functions)
rather than bash's `complete`/`compgen` command-based model.

The details of the completion API — context structure, filtering
conventions, display formatting — are implementation-phase
decisions that will be specified when the line editor is built.


## Configuration layout

psh follows XDG Base Directory conventions. The configuration
root is `$XDG_CONFIG_HOME/psh/` (defaulting to
`$HOME/.config/psh/` when `$XDG_CONFIG_HOME` is unset).

### Directory structure

    ~/.config/psh/
    ├── profile          # login shell profile (sourced on login)
    ├── rc               # interactive rc (sourced on interactive start)
    ├── rc.d/            # drop-in directory (sourced after rc, *.psh files, sorted)
    ├── completions/     # user completion functions (autoloaded)
    └── history          # command history (managed by shell)

### File roles

**`profile`** — login-time initialization. Environment
variables, PATH modifications, and one-time setup. Sourced
only by login shells. Equivalent to rc's `$home/lib/profile`
[Duf90, rc(1) lines 940-949].

**`rc`** — interactive initialization. Aliases, prompt
customization, completion registrations, and interactive-only
configuration. Sourced by every interactive shell (including
login shells, after `profile`).

**`rc.d/`** — drop-in directory for modular configuration.
Files matching `*.psh` are sourced in lexicographic order after
`rc`. This supports package managers and tools that install
shell configuration fragments (e.g., `rc.d/50-cargo.psh`,
`rc.d/90-nix.psh`). The `.psh` extension prevents stray files
(`.swp`, `.bak`) from being sourced.

**`completions/`** — completion function directory. Files in
this directory are autoloaded when the completion system
initializes. Each file defines `complete` registrations for one
or more commands. Naming convention: `cmd_name.psh`.

**`history`** — command history file (see §History). Managed
by the shell, not user-edited.

### System configuration

    /etc/psh/
    ├── pshrc            # system-wide rc (sourced first, all modes)
    └── pshrc.d/         # system drop-in directory (*.psh, sorted)

System configuration is sourced before user configuration.
`pshrc.d/` follows the same drop-in pattern as the user `rc.d/`.

### Data directory

    $XDG_DATA_HOME/psh/     # defaults to ~/.local/share/psh/
    └── history              # alternative history location if preferred

The spec places `history` in the config directory for
simplicity. Implementations may respect `$XDG_DATA_HOME` for
history if strict XDG compliance is desired — history is
arguably data, not configuration. The default `$HISTFILE` path
in the config directory follows ksh93 convention (history
adjacent to configuration).


## IFS removal

psh does not have `$IFS`. Word splitting on unquoted variable
expansion — the mechanism IFS controls in POSIX shells — does
not exist in psh. Variables are lists; substitution splices
list elements into argument positions. There is no field
splitting pass.

### External tool compatibility

Some external tools expect IFS behavior indirectly:

- `xargs` splits stdin on whitespace by default. This works
  unchanged — psh pipes byte streams to external commands, and
  `xargs` reads them as bytes. The IFS variable is irrelevant
  because `xargs` does its own splitting.
- `read` in POSIX shells splits input on IFS. psh's `read`
  builtin reads whole lines — splitting is done explicitly
  by the user via `.split` or pattern matching.
- `for x in $var` in POSIX shells depends on IFS for word
  splitting. psh's `for x in $var` iterates over list elements
  — no splitting occurs because `$var` is already a list.

Tools that explicitly read `$IFS` from the environment (rare)
will see it unset. If a tool requires IFS in its environment,
the per-command local variable form works:

    IFS=$'\n' some_tool


## Idle timeout

`$TMOUT` is an integer variable specifying the idle timeout in
seconds for interactive shells. If set and greater than zero,
the shell exits after `$TMOUT` seconds without input. Before
exiting, the shell prints a warning and waits 60 seconds for a
keypress — if input arrives, the timer resets.

ksh93 heritage (sh.1 lines 2484-2491): `TMOUT` causes the
shell to terminate if a command is not entered within the
prescribed number of seconds.

Default: unset (no timeout). Primarily useful for security on
shared systems (terminals left unattended). Scripts ignore
`$TMOUT`.


## Rejected features

The following login/interactive shell features from other shells
are explicitly not adopted:

- **Restricted mode (`-r`).** ksh93 supports a restricted shell
  mode that disables `cd`, PATH modification, and output
  redirection. psh does not implement restricted mode. It is a
  weak security mechanism that is easily circumvented and
  provides a false sense of isolation. Containerization and
  sandboxing are the modern equivalents.

- **`$ENV` variable.** ksh93 uses `$ENV` to name a file sourced
  for each interactive shell (sh.1 lines 2545ff). psh uses the
  fixed path `~/.config/psh/rc` for this purpose (see
  §Configuration layout). The `$ENV` variable is not recognized
  — if set in the inherited environment, it is ignored. This
  avoids the confusion of having two mechanisms for the same
  function.

- **Logout file.** ksh93 sources `$HOME/.sh_logout` on login
  shell exit. psh does not have a logout file. The `trap`
  builtin with `EXIT` (or `sigexit`) provides the same
  functionality without a separate configuration file:

      trap EXIT { cleanup_commands }

  This is consistent with rc, which had `fn sigexit` but no
  logout file.

- **`!`-style history expansion.** See §History above.
