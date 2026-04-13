# Builtin reference

Builtins are commands implemented inside the shell process.
They execute without forking. Some must be builtins (they modify
shell state that a child process cannot access); others are
builtins for performance (avoiding fork/exec overhead for
trivial operations).

Builtins follow the same ExitCode convention as external
commands (§06-types.md §ExitCode and Status): success returns
`ExitCode { code = 0; message = '' }`, failure returns a
nonzero code with an optional message.


## Sourcing

### `.` (source)

    . file [arg...]

Execute `file` in the current scope. The file is read, parsed,
and executed as if its contents replaced the `.` command. All
variable bindings, function definitions, and option changes
persist after the file completes. Positional parameters `$*`
are set to `arg...` for the duration of the file, then restored.

If `file` does not contain a `/`, it is searched in `$PATH`.

rc heritage [Duf90, rc.ms §Built-in commands]: "Execute commands
from file. `$*` is set for the duration to the remainder of the
argument list."

`source` is accepted as an alias for `.` — it has no semantic
difference in psh (ksh93 distinguishes them via special-builtin
discipline, which psh does not adopt). The alias aids migration
from bash/ksh scripts.

This is the mechanism for startup file sourcing
(§14-invocation.md): `/etc/psh/pshrc`, user profile, and
interactive rc are all executed via `.`.


## Navigation

### `cd`

    cd [dir]
    cd -
    cd old new

Change the current working directory.

- No argument: `cd $HOME`.
- `-` argument: `cd $OLDPWD` (previous directory).
- Two arguments: substitute `old` with `new` in `$PWD` and cd
  to the result. ksh93 heritage — useful for deep paths:
  `cd src test` in `/project/src/lib` → `/project/test/lib`.
- With `dir`: change to `dir`, searching `$CDPATH` if `dir` is
  relative and not found in the current directory.

Updates `$PWD` and `$OLDPWD` on success. Failure (directory
doesn't exist, permission denied) returns nonzero ExitCode.

rc heritage [Duf90, rc(1) §Built-in commands]: "Change the
current directory to dir. The default argument is `$home`.
`$cdpath` is searched."

### `pwd`

    pwd

Print the current working directory (the value of `$PWD`).
Equivalent to `echo $PWD` but guaranteed to be the
canonical path.


## Output

### `echo`

    echo [arg...]

Write arguments to stdout, separated by spaces, followed by a
newline. No flag processing — `echo` is deliberately simple.
If flag-like arguments are needed, use `printf`.

rc heritage [Duf90, rc.ms §Built-in commands]. psh's `echo`
does not interpret escape sequences (unlike bash's `echo -e`).
For formatted output, use `printf`.

### `printf`

    printf format [arg...]

Formatted output following POSIX printf conventions. The
`format` string supports `%s`, `%d`, `%x`, `%o`, `%f`, `%%`,
and `\n`, `\t`, `\\` escape sequences. If there are more
arguments than format specifiers, the format string is reused.

The `-v name` flag assigns the formatted result directly to a
variable instead of writing to stdout — avoids command
substitution overhead and trailing-newline stripping (ksh93
heritage: sh.1 §printf).

This is the reliable output mechanism — `echo` for simple
cases, `printf` when formatting matters.


## Input

### `read`

    read [-u fd] [-n count] name...

Read a line from stdin (or fd `fd` with `-u`) and split into
the named variables. The last variable receives the remainder
of the line. If there are fewer fields than names, the extra
names are set to empty strings.

- `-u fd`: read from file descriptor `fd` instead of stdin.
- `-n count`: read exactly `count` bytes (binary read).

Returns nonzero on EOF or error.

This is distinct from `read -p` (coprocess read, §10-coprocesses.md).
The `-p` flag routes to the coprocess protocol; without `-p`,
`read` operates on standard file descriptors.

Splitting: psh splits on whitespace (spaces and tabs), not on
`$IFS` (which psh does not have). Fields are assigned left to
right. The split produces a list; each name receives one element
except the last, which receives the rest.


## Status

### `true`

    true

Do nothing, return success (ExitCode 0). Used in `while(true)`
loops and boolean contexts.

### `false`

    false

Do nothing, return failure (ExitCode 1).


## Metaprogramming

### `eval`

    eval arg...

Concatenate arguments with spaces, then parse and execute the
resulting string as a command. This is the one construct that
deliberately violates Duff's principle — structure is flattened
to a string and re-parsed.

rc heritage [Duf90, rc.ms §Built-in commands]: "The arguments
are concatenated (separated by spaces) into a string, which is
then parsed and executed as a command. The raison d'être is to
break the rule."

The VDC framework (docs/vdc-framework.md §5.12) characterizes
`eval` as forcing the Segal condition — taking a sequence of
horizontal arrows, collapsing them to a composite string, and
re-parsing. Information loss happens at concatenation (boundaries
erased), not at the re-parse.

Use sparingly. Every `eval` is a design smell indicating the
type system should have provided a structural solution (name
references, accessor notation, function tables).


## Process control

### `exec`

    exec [cmd [arg...]]
    exec redirect...

Without a command: apply redirections to the current shell
process. With a command: replace the shell process with the
named command (no fork, no return).

rc heritage [Duf90, rc.ms §Built-in commands]: "Exec does a
Unix exec (2) — without forking."

Essential for wrapper scripts and the `exec $name` pattern in
init system supervision scripts.

### `wait`

    wait [-n] [pid | %job]...

Wait for background jobs to complete.

- No arguments: wait for all background jobs.
- `pid` or `%job`: wait for specific process/job.
- `-n`: wait for any one background job to complete (ksh93
  heritage: sh.1 §wait).

Sets `$status` to the ExitCode of the waited-for process.

### `kill`

    kill [-signal] pid | %job...
    kill -l

Send a signal to processes or jobs. Default signal is SIGTERM.

- `-l`: list available signal names.
- `%job`: job ID expansion (§14-invocation.md).

### `fg`

    fg [%job]

Move a stopped or background job to the foreground. Default:
the most recent background job. Requires `set -o monitor`
(on by default for interactive shells).

### `bg`

    bg [%job]

Resume a stopped job in the background. Default: the most
recently stopped job.

### `jobs`

    jobs [-l]

List background and stopped jobs. `-l` includes PIDs.

### `disown`

    disown [%job]

Remove a job from the shell's job table. The job continues
running but will not receive SIGHUP when the shell exits.
Essential for login shells — prevents background work from
being killed on logout.


## Positional parameters

### `shift`

    shift [n]

Shift positional parameters left by `n` positions (default 1).
`$1` becomes what was `$2`, etc. The first `n` parameters are
discarded. `$#*` decreases by `n`.

rc heritage: rc kept `shift` — Duff's deletion list (rc.ms
§Design Principles) does not include it, and both rc.ms and
rc(1) list `shift` among the builtins. ksh93 heritage
(sh.1 §shift) adds arithmetic expressions for the count.


## Environment

### `export`

    export name...
    export

Standalone form complementing `let export`. Marks named
variables for projection to the process environment. Without
arguments, lists all exported variables.

See §14-invocation.md §Export semantics for the full model:
mark-for-projection (not snapshot), invokes `.get`, requires
classical zone, serialization rules.

### `unexport`

    unexport name...

Remove the export mark from named variables. The variable
remains in the shell scope but no longer projects to child
environments.

### `unset`

    unset name...

Remove variables from the current scope. If the variable has
discipline functions, those are also removed. If the variable
is exported, the export mark and the environment entry are
removed.

For functions: `unset -f name` removes a `def` binding.

rc heritage: Duff deleted `unset` (rc.ms §Design Principles)
— in rc, `x=()` (assign empty list) was sufficient. psh
restores it because the three-tier namespace makes "remove
entirely" semantically distinct from "set to empty."


## Resource limits

### `umask`

    umask [mask]

Get or set the file creation mask. Without arguments, prints
the current mask (octal). With an argument, sets the mask.

### `ulimit`

    ulimit [-HSa] [-c | -d | -f | -l | -m | -n | -s | -t | -v] [limit]

Get or set resource limits. `-H` for hard limit, `-S` for soft
limit, `-a` for all limits. Without `limit`, prints the current
value.


## Name resolution

### `command`

    command name [arg...]

Execute `name` as a command, bypassing any `def` function with
the same name. Searches for external commands in `$PATH`.

POSIX heritage. Essential for writing a `def cd` wrapper that
calls the real `cd`:

    def cd {
        command cd $*
        update_prompt
    }

### `builtin`

    builtin name [arg...]

Execute `name` as a builtin, bypassing both `def` functions
and external commands. If `name` is not a builtin, returns
nonzero.

rc heritage [Duf90, rc(1) §Built-in commands]: "Execute command
as a built-in, even if a function by that name exists."

### `whatis`

    whatis name...

Print the definition of each name. For builtins, prints
"builtin". For `def` functions, prints the body. For external
commands, prints the path. For variables, prints the value.

rc heritage [Duf90, rc(1) §Built-in commands]: "Print what name
would be if used as a command — the source of a function, the
path of an external command, or the name of a built-in."


## Loop control

### `break`

    break [n]

Exit the innermost `n` enclosing loops (default 1). Valid
inside `for` and `while` bodies.

rc heritage: Duff deleted `break` (rc.ms §Design Principles).
psh restores it — the absence of `break` and `continue` is the
most-noticed gap when rc-derived shells are used on Unix.

Categorically, `break` is a μ-binding that jumps to the loop's
post-continuation. It is a named control operator in the
sequent calculus, not an ad-hoc escape.

### `continue`

    continue [n]

Skip to the next iteration of the innermost `n` enclosing loops
(default 1). Valid inside `for` and `while` bodies.

Same heritage and categorical justification as `break`.
`continue` jumps to the loop's iteration-continuation rather
than the post-continuation.


## Path cache

### `hash`

    hash [-r] [name...]

Manage the command path cache. Without arguments, lists cached
paths. With names, resolves and caches them. `-r` clears the
cache entirely.

ksh93 heritage (sh.1 §hash). The cache avoids repeated `$PATH`
search for frequently-used commands. The cache is invalidated
when `$PATH` changes.


## Timing

### `times`

    times

Print accumulated user and system times for the shell and its
children. Minimal utility but required by POSIX for shell
conformance contexts.


## Summary by priority

### Essential (blocks login shell use)

`.`, `cd`, `echo`, `read`, `true`, `false`, `exec`, `export`,
`set` (§14-invocation.md), `exit` (§04-syntax.md).

### Important (daily interactive use)

`printf`, `eval`, `shift`, `wait`, `kill`, `fg`, `bg`, `jobs`,
`break`, `continue`, `command`, `builtin`, `whatis`, `pwd`,
`unset`, `unexport`.

### Operational (robustness)

`disown`, `umask`, `ulimit`, `hash`, `times`.
