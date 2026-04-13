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
current directory to dir. The default argument is `$HOME`.
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

### `print`

    print [-p name] [arg...]

Write arguments to stdout (default) or to a named coprocess
channel (`-p name`). When `-p` is used, `print` sends a
request to the named coprocess and returns a `ReplyTag`
(affine resource) representing the pending response.

    print -p myproc 'request'           # send to coprocess
    let tag = print -p myproc 'query'   # capture ReplyTag
    read -p myproc -t $tag reply        # read response for tag

Without `-p`, `print` is equivalent to `echo` but accepts
flags for future extension (ksh93 heritage: sh.1 §print).

See 10-coprocesses.md for the full coprocess protocol.

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

    read [-u fd] [-n count] [-p name [-t tag]] name...

Read a line from stdin (or fd `fd` with `-u`) and split into
the named variables. The last variable receives the remainder
of the line. If there are fewer fields than names, the extra
names are set to empty strings.

- `-u fd`: read from file descriptor `fd` instead of stdin.
- `-n count`: read exactly `count` bytes (binary read).
- `-p name`: read from named coprocess channel. Consumes a
  response from the coprocess's reply queue.
- `-t tag`: with `-p`, read the response for a specific
  `ReplyTag`. The tag is an affine resource — using it
  consumes it. Without `-t`, reads the next available response.

Returns nonzero on EOF or error.

Without `-p`, `read` operates on standard file descriptors.
With `-p`, it routes to the coprocess protocol
(10-coprocesses.md).

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


## Interactive

### `menu`

    menu [-popup|-vertical|-transient] [-fuzzy] [-timeout N]
         [-prompt STR] $items

Structured interactive selection. Presents a list of items to
the user and returns their choice as a tagged enum. Interactive
only — in non-interactive mode (scripts, `-c`), `menu` returns
`err('not interactive')` immediately.

**Return type:**

    enum MenuResult(T) {
        selected(T);
        cancelled;
        err(Str)
    }

Three tags distinguish three outcomes: the user chose an item,
the user dismissed the menu (Escape, Ctrl-C), or the operation
failed (non-interactive shell, malformed request). This is NOT
a `Result` — `cancelled` is a deliberate user action, not an
error.

**Basic usage:**

    let choice = menu (stage commit push)
    match $choice {
        selected(v) => handle $v;
        cancelled   => ();
        err(e)      => echo "menu error: $e"
    }

**Style hints.** The shell suggests a presentation style. The
host renders however it can — a terminal may ignore `-popup`
and render a numbered list. Style hints degrade gracefully.

| Flag | Hint |
|------|------|
| `-popup` | Overlay popup (if host supports) |
| `-vertical` | Scrollable vertical list |
| `-transient` | Auto-dismiss after one selection |
| `-fuzzy` | Fuzzy filter as user types |
| `-timeout N` | Auto-cancel after N seconds |
| `-prompt STR` | Filter/input prompt string |

**Structured items.** Items can be simple strings or tuples
for richer presentation:

    # Simple: List(Str)
    menu -popup (stage commit push)

    # Structured: List with display labels
    menu -transient (
        ('s : stage', 'stage')
        ('c : commit', 'commit')
        ('p : push', 'push')
    )

For tuple items, the first element is the display label, the
second is the value returned in `selected(T)`.

**ksh93 heritage.** ksh93's `select` statement (sh.1 §Compound
Commands) is the closest ancestor — it prints a numbered menu,
reads a reply, sets a variable. `menu` supersedes `select`:
typed return instead of string-in-variable, cancellation as a
distinct tag instead of EOF, style hints for richer hosts.
psh does not provide `select` — `menu` covers all its use
cases with better types.

**VDC classification:** monadic (§8.5 clause 1). Positive
input (List(T)) → positive output (MenuResult(T)) through an
effect (user interaction). No polarity frame needed.

**Not an optic.** The old design doc classified `menu` as a
MonadicPrism. This is incorrect — `menu` is a Kleisli arrow
`List(T) → Ψ(MenuResult(T))`, not an optic. There is no
rebuild direction (you cannot inject a T back into a menu to
recover the original list). No prism laws apply. `menu` is a
unidirectional effectful selection, not a bidirectional
decomposition.



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


## Signal handling

### `trap`

    trap SIGNAL { handler } { body }   -- lexical (scoped)
    trap SIGNAL { handler }            -- global
    trap SIGNAL                        -- delete handler

Unified signal handling. Three forms distinguished by block
count. See 04-syntax.md §trap and 12-errors.md §trap for
the full grammar and semantics.

**Lexical** (two blocks): installs the handler for the
duration of the body — the μ-binder of the sequent calculus
[CH00, §2.1]. The handler captures a signal continuation
scoped to the body. Inner lexical traps shadow outer for
the same signal.

**Global** (one block): installs a persistent handler that
remains until overwritten or deleted.

**Delete** (no blocks): removes the handler for the named
signal, restoring the default disposition.

Precedence: innermost lexical > outer lexical > global >
OS default.

ksh93 heritage (sh.1 §Signals). rc had `{cmd}` blocks for
signal handlers; psh's three-form `trap` unifies rc's signal
handling with ksh93's `trap` command.


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


### `fc`

    fc [-l] [-e editor] [-s old=new] [first [last]]

Fix command — the programmatic history interface.

| Form | Behavior |
|------|----------|
| `fc -l` | List recent history entries (default: last 16) |
| `fc -l first last` | List entries in range |
| `fc -l -N` | List last N entries |
| `fc -e editor` | Open last command in editor, execute on save |
| `fc -e -` | Re-execute last command without editing |
| `fc -s old=new` | Substitute and re-execute last command |
| `fc first last` | Open range in editor, execute on save |

Without `-l`, `fc` opens the selected command(s) in `$FCEDIT`
(defaulting to `$EDITOR`, defaulting to `ed`). On editor exit,
the edited text is executed as a command.

ksh93 heritage (sh.1 lines 6480-6555). Also a POSIX utility.

### `complete`

    complete cmd_name { |word ctx| => body }
    complete -d cmd_name
    complete -l

Register, remove, or list programmable completion functions.
When `Tab` is pressed after `cmd_name`, the registered function
is called with the partial word (`Str`) and a
`CompletionContext` struct (§14-invocation.md §Completion).

The function returns `List(Str)` (simple — auto-wrapped) or
`List(Candidate)` (structured — with display, description, tag,
nospace metadata). The shell filters results against the
current prefix.

    # Simple
    complete brew { |word ctx| =>
        (install uninstall update upgrade search info list)
    }

    # Structured
    complete brew { |word ctx| =>
        match $ctx.word_index {
            1 => (
                Candidate { value = 'install'; description = 'Install a formula or cask' }
                Candidate { value = 'search';  description = 'Search for formulae' }
            );
            _ => ()   # fall through to default
        }
    }

`complete -d cmd_name` deregisters the completion function.
`complete -l` lists all registered completion commands.

See §14-invocation.md §Completion for the full completion
framework, context structure, candidate format, and external
provider protocol.


## Generic list combinators

Generic combinators (`map`, `filter`, `each`) are shell
builtins whose types live at the Rust implementation layer.
They are not polymorphic `def` signatures (§16-features.md
§Non-goals) — the Rust code handles dispatch on the element
type internally.

**Two input modes.** Each combinator accepts a list argument
or reads from stdin. With an explicit `$list` argument, the
element type is known from the list's type. From stdin, the
combinator reads **one element per line** (`List(Str)`) — this
is an implicit ↑ (upshift from byte stream to typed list) at
the pipe boundary. The line-splitting convention matches rc's
text-stream heritage: lines are the natural unit of shell data.

### `filter`

    filter { |x| => condition } $list
    filter { |x| => condition }              # reads from stdin

Remove elements that don't satisfy the condition. Returns a
new list. The body is a lambda — its exit status determines
inclusion (0 = keep, nonzero = discard).

    let evens = filter { |n| => test $(( n % 2 )) -eq 0 } $numbers
    ls | filter { |f| => test -d $f }

**List removal shorthand.** `filter` with negation replaces
Ion's `\\=` (remove-by-value):

    let cleaned = filter { |x| => test $x != 'unwanted' } $list

### `map`

    map { |x| => transform } $list
    map { |x| => transform }                 # reads from stdin

Apply a transformation to each element, returning a new list.
The body is a lambda whose stdout becomes the output element
— this is a shell convention (capture-by-stdout), not a pure
return. The lambda is oblique in the duploid sense: it
interacts with the pipe fd and is not thunkable. Each lambda
invocation runs in its own polarity frame scope.

    let upper = map { |s| => echo $s.upper } $names
    seq 1 10 | map { |n| => echo $(( n * n )) }

### `each`

    each { |x| => body } $list
    each { |x| => body }                     # reads from stdin

Execute the body for each element. Unlike `map`, output is not
collected — `each` is for side effects. Returns the status of
the last iteration.

    each { |f| => rm $f } $tempfiles
    ls *.log | each { |f| => gzip $f }

`for x in $list { body }` is the control-flow equivalent.
`each` exists for the pipeline case where the list arrives
on stdin.

### `fold`

    fold init { |acc x| => body } $list
    fold init { |acc x| => body }            # reads from stdin

Accumulate across elements. `init` is the initial accumulator
value. The body is a lambda that receives the current
accumulator and the next element; its stdout becomes the new
accumulator value (capture-by-stdout, same convention as `map`).
Returns the final accumulator.

    let total = fold 0 { |acc n| => echo $(( acc + n )) } $numbers
    let csv = fold '' { |acc s| =>
        if(test -z $acc) { echo $s } else { echo "$acc,$s" }
    } $fields

`fold` is the general form; `each` is side-effect-only (no
accumulator), `map` is one-to-one (no cross-element state).


## Summary by priority

### Essential (blocks login shell use)

`.`, `cd`, `echo`, `print`, `read`, `true`, `false`, `exec`,
`export`, `set` (§14-invocation.md), `exit` (§04-syntax.md),
`trap`.

### Important (daily interactive use)

`printf`, `eval`, `shift`, `wait`, `kill`, `fg`, `bg`, `jobs`,
`break`, `continue`, `command`, `builtin`, `whatis`, `pwd`,
`unset`, `unexport`, `fc`, `filter`, `map`, `each`, `fold`,
`menu`.

### Operational (robustness)

`disown`, `umask`, `ulimit`, `hash`, `times`, `complete`.

### Planned (used in spec examples, not yet fully specified)

`open`, `close`, `dup`, `write` — fd-level I/O operations
(used in 03-polarity.md examples). `str`, `path`, `path.join`
— type conversion builtins (used in 06-types.md §Path).

`test` is an external command (`/usr/bin/test`), not a builtin.
Spec examples that use `test` in `filter` bodies assume it is
available on `$PATH`.
