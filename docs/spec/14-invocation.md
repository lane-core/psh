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
2. `$HOME/.config/psh/profile` — user login profile. Sourced
   second if it exists. XDG convention
   (`$XDG_CONFIG_HOME/psh/profile` when `$XDG_CONFIG_HOME` is
   set).

### Interactive (non-login)

1. `/etc/psh/pshrc` — system-wide configuration (same as login).
2. `$HOME/.config/psh/rc` — user interactive configuration.
   Sourced if it exists.

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
| `$status` | ExitCode | Initially `ExitCode { code = 0; message = '' }` |
| `$pipestatus` | List(ExitCode) | Initially `($status)` |
| `$PWD` | Path | Current working directory |
| `$OLDPWD` | Path | Previous working directory (initially `$PWD`) |
| `$SHLVL` | Int | Shell nesting depth (incremented on each invocation) |
| `$prompt` | List(Str) | `('% ' '  ')` — primary and continuation prompts |
| `$*` | List(Str) | Positional parameters (script args or `set --` args) |


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

`$prompt` is a list of two strings (rc heritage):

- `$prompt[0]` — primary prompt (displayed before each command)
- `$prompt[1]` — continuation prompt (displayed when more input
  needed)

Default: `('% ' '  ')` — the `%` prompt from rc, with a
two-space continuation indent.

Prompt strings undergo variable expansion (but not command
substitution — prompts should be fast). `$PWD`, `$USER`,
`$SHLVL` are commonly used in prompt customization.

    prompt = ('$USER@$HOSTNAME:$PWD% ' '  ')
