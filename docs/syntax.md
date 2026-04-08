# psh syntax

Formal grammar for psh. Starts from rc (Duff 1990) and names
each extension. Theoretical foundation: `docs/specification.md`.

rc reference: `reference/plan9/man/1/rc` and
`reference/plan9/papers/rc.ms`.


## Design principle

rc's actual syntax is the baseline. Every convention from rc
is preserved unless explicitly departed from with justification.
Extensions are faithful to the spirit of rc: a keyword before
braces for new block constructs, operators where operators are
expected, no overloading of existing rc syntax for new purposes.

Duff's first principle — "input is never scanned more than
once" [1, §Design Principles] — governs all parsing decisions.
The parser does not consult the environment; parsing is
context-free.


## Notation

EBNF with these conventions:

    'literal'     keyword or punctuation
    NAME          a bare word (see §Word formation)
    WORD          a word expression (see §Words)
    *             zero or more
    +             one or more
    ?             zero or one
    |             alternation
    ( )           grouping


## Program structure

A program is a sequence of commands separated by terminators
(newlines, semicolons). Blank lines and comments are ignored.

    program     = terminator* (command terminator+)* command?
    terminator  = '\n' | ';'
    comment     = '#' (any char except '\n')*

A comment starts with `#` and runs to end of line. Comments
do not nest. rc heritage [1, §Simple commands].


## Commands

A command is one of: a binding (context extension), a control
flow construct, or an expression.

    command     = binding | control | return_cmd | expr_cmd
    return_cmd  = 'return' value?


## Bindings

Bindings extend the context Γ with a new name. They are
μ̃-binders in the sequent calculus reading (specification.md
§The three sorts).

    binding     = assignment | let_binding | cmd_def | ref_def

    assignment  = NAME '=' value
    let_binding = 'let' let_quals NAME (':' type_ann)? '=' value
    let_quals   = 'mut'? 'export'?
    cmd_def     = 'cmd' NAME cmd_params? body
    ref_def     = 'ref' NAME '=' NAME

    type_ann    -- see §Type annotations (future)

### Assignment

`x = val` walks the scope chain and updates the first matching
variable. If no variable exists, creates one in the current
scope. Always produces `Val::Str` — no type inference. rc
heritage [1, §Variables and Assignment].

### let

`let x = val` always creates in the current scope. Immutable
by default. Runs type inference: `42` → Int, `true` → Bool,
`/tmp` → Path, `hello` → Str. Quoted values (`'42'`) stay Str.
Leading-zero integers (`042`) stay Str. Optional type
annotation validates via Prism check. psh extension.

`let` is always CBV. The RHS is evaluated at binding time and
the result is stored. There is no call-by-name `let` form.

### cmd

`cmd name { body }` defines a named computation — a cut
template in the command sort. This is rc's `fn` [1, §Functions], renamed. Duff chose `fn`
deliberately, but psh draws a distinction between commands
(cuts) and functions (morphisms) that rc did not make. `cmd`
names the sort.

Three forms:

    cmd name { body }           # positional params: $1, $2, $*
    cmd name() { body }         # nullary: takes no arguments
    cmd name(a b c) { body }    # named params: $a, $b, $c

Without parentheses, the command uses rc-style positional
parameters (`$1`, `$2`, `$*`, `$#*`). With parentheses, the
command declares its parameter interface — `()` means nullary,
`(a b c)` binds arguments to named variables. Named parameters
are bound in the command scope alongside positional `$1`, `$2`
etc. for compatibility.

A `cmd` is not first-class; it is not a value; it cannot be
stored in a variable or passed as an argument. It is a named
entry in the computation context Θ.

Also handles discipline `.set` functions: `cmd x.set { body }`,
`cmd x.set(val) { body }`. ksh93 heritage. Note: `.get`
disciplines are pure and defined as lambdas, not `cmd` — see
§Discipline functions.

### ref

`ref name = target` creates a nameref — an alias that resolves
through `target` on every access. The target is a literal name,
not an expression. ksh93 heritage.


## Functions (values, not commands)

Functions are values in the value sort, created by `let`-binding
a lambda expression. They are first-class: storable, passable,
composable.

    lambda      = '\' params '=>' (body | command)
    params      = '(' NAME* ')' | NAME

    let double = \(x) => expr $x '*' 2
    let greet  = \(name) => { echo 'hello '$name; return 0 }
    let add    = \(x) => \(y) => expr $x + $y    # currying

In CBPV terms [4], a lambda is `U(A₁ → ... → Aₙ → F(B))`
when impure, or `U(A₁ → ... → Aₙ → B)` when pure. The `U`
(thunk) wraps a computation as a value. The distinction between
`cmd` and `let`+`\` is the CBPV value/computation boundary
surfaced as syntax. See specification.md §Two kinds of callable.

**Capture semantics.** Lambdas capture free variables at
definition time — `Vec<(String, Val)>`, positive, Clone. This
is the closed-term property. Named `cmd` definitions use
dynamic resolution (read current scope at call time). The
distinction is the sort boundary: values (lambdas) are
self-contained; computations (cmds) live in a context.

**Purity inference.** The shell infers purity by conservative
AST analysis: if the body contains no assignments to variables
outside the lambda's scope, no fork/exec, no side-effecting
builtins, no I/O, no coprocess interaction — the lambda is
classified pure. Pure lambdas are thunkable/central in the
duploid [9, Table 1]. Impure lambdas work but degrade
to oblique maps. See specification.md §Two kinds of callable.

### Discipline .get functions

`.get` disciplines are pure — they are Getters (specification.md
§Discipline functions). Defined as lambdas:

    let x.get = \() => { ... pure computation ... }

The body fires on every `$x` access as a notification. The
returned value is always the stored value, not the body's
output. `.get` bodies cannot perform I/O, cannot mutate the
variable they observe, cannot call external commands. See
specification.md §Discipline functions for the full rationale.


## Control flow

Control flow constructs branch or iterate. Each takes its
condition or value in rc-style parentheses and its body as
a braced block or `=>` single-line form.

    control     = if_cmd | for_cmd | while_cmd
                | match_cmd | try_cmd | trap_cmd

    if_cmd      = 'if' '(' pipeline ')' body ('else' (if_cmd | body))?
    for_cmd     = 'for' '(' NAME 'in' value ')' body
    while_cmd   = 'while' '(' pipeline ')' body
    match_cmd   = 'match' '(' value ')' '{' match_arm (';' match_arm)* ';'? '}'
    try_cmd     = 'try' body 'catch' NAME body
    trap_cmd    = 'trap' SIGNAL body body

    match_arm   = glob_arm | structural_arm
    glob_arm    = glob_pats '=>' lambda_body
    structural_arm = NAME NAME '=>' lambda_body
    glob_pats   = '(' NAME+ ')' | NAME

    body        = '{' program '}'
                | '=>' command

`=>` is a dependent keyword — a single-line body introducer.
After `=>`, the parser reads a single command (terminated by
`;`, newline, or `}`). `{` opens a multi-line body. Both forms
are interchangeable wherever `body` appears.

### if / else

    if(test -f $file) {
        echo 'exists'
    } else {
        echo 'missing'
    }

    if(test -d $dir) => echo 'is directory'

**rc parens for conditions.** `if(cond)` — rc's parentheses
around the condition [1, §Conditional execution]. psh preserves
this convention.

**`else` instead of `if not`.** Duff acknowledged: "The one
bit of large-scale syntax that Bourne unquestionably does
better than rc is the if statement with else clause" [1,
§Design Principles]. rc's `if not` was a separate command reading
`$status` implicitly — a state dependency. psh's `else` is a
syntactic clause of the `if` node.

**`else if` chaining.** `if(cond1) { A } else if(cond2) { B }
else { C }` — the parser checks for `if` after `else` and
recursively parses.

### for

    for(x in (a b c)) {
        echo $x
    }

    for(f in $files) => echo $f

`for(name in value) body` parses exactly one `value`: either
a parenthesized list `(a b c)` or a single word. To iterate
over multiple elements, use a list. rc heritage [1, §For
loops].

### while

    while(test -f /tmp/lock) {
        sleep 1
    }

rc's `while() echo y` (empty parens = always true) becomes
`while(true) { echo y }`. The `true` builtin is explicit.

### match

    match($filename) {
        *.txt => echo 'text file';
        *.rs  => echo 'rust source';
        * => echo 'other'
    }

    # Multi-pattern arm
    match($ext) {
        (c h) => echo 'C source';
        (rs toml) => echo 'Rust';
        * => echo 'unknown'
    }

**`match` instead of `switch`.** rc's `switch`/`case` used
`case` labels as top-level commands within a list body — the
`switch` body is syntactically a `{list}` with `case`
sub-commands [1, §Switch]. psh's `match` uses structured `=>`
arms with `;` separators — a genuinely different syntactic
form. Using `switch` would be a false cognate — it looks like
rc heritage but has a different syntactic role. `match` names
the operation honestly: pattern matching / coproduct
elimination.

Match arms use `=>` to introduce the body and `;` to
separate. Each arm is an independent branch. Multiple
patterns per arm use list syntax: `(*.txt *.md) => body`.

**Structural matching** on Sum values (future extension):

    match($result) {
        ok val  => echo 'success: '$val;
        err msg => echo 'error: '$msg
    }

Structural arms have the form `tag name =>` — the tag, a
binding name, then `=>`. The binding is a μ̃-binder scoped
to the arm body. The variable does not escape the arm. The
wildcard arm `* =>` does not bind a variable.

### try / catch

    try {
        let title = `{ cat /srv/window/title }
        let cursor = `{ cat /srv/window/cursor }
        echo $title' ['$cursor']'
    } catch e {
        echo 'unavailable: '$e
    }

Scoped error handling — ErrorT monad transformer over command
sequences. `try` changes the sequencing combinator from
unconditional `;` to monadic `;ₜ` that checks Status after
each command. On nonzero status, execution aborts to `catch`.
The `catch e` binding is a μ̃-binder on the error case.

This is categorically different from `if`:
- `if` = single coproduct elimination (check one command's
  status, branch)
- `try` = natural transformation on sequencing (change the
  semicolon's behavior for an entire block)

Boolean contexts are exempt: `if` conditions, `while`
conditions, `&&`/`||` LHS, `!` commands are not checked.

`try` blocks nest: inner catches before outer.

See specification.md §Error model for the full semantic
analysis.

### trap

    trap SIGINT { echo 'interrupted'; return 1 } {
        long_running_command
        another_command
    }

Lexically-scoped signal handler — the μ-binder of the sequent
calculus [5, §2.1]. The handler is installed for the duration
of the body. When the body exits, the handler is uninstalled.
Inner traps shadow outer traps for the same signal.

`try` and `trap` are distinct constructs:
- `try` = synchronous, checked at each `;`, status-only
- `trap` = asynchronous, signal-delivered, continuation-capturing

They compose freely: `try` inside `trap`, `trap` inside `try`.

See specification.md §Error model for the design rationale
(lexical vs dynamic scoping, sequent calculus faithfulness).


## Expressions

Expressions are the profunctor layer — commands with
redirections, pipelines, and operators.

    expr_cmd    = or_expr ('&')?
    or_expr     = and_expr ('||' and_expr)*
    and_expr    = match_expr ('&&' match_expr)*
    match_expr  = pipeline ('=~' value)?
    pipeline    = cmd_expr ('|' cmd_expr)*
                | cmd_expr '|&'
    cmd_expr    = '!' cmd_expr
                | body
                | '@' body
                | simple_cmd redirect*

    simple_cmd  = WORD+

**`|&` coprocess.** `cmd |&` starts a coprocess with a
9P-shaped bidirectional protocol. See specification.md
§Coprocesses for the full discipline.

**`@{ }` subshell.** Fork with a copy of the current scope.
rc's `@` operator [1, §Operators] — a subshell fork.
Classical contraction — continuation duplicated, each copy
independent.

**`!` negation.** Inverts exit status. rc heritage.

**`&` background.** Runs the command asynchronously. rc
heritage.


## Words

Words are positive (CBV) — evaluated eagerly before the
command that consumes them runs.

    word        = word_atom ('^' word_atom)*
    word_atom   = LITERAL | QUOTED
                | var_ref
                | '$#' VARNAME | '$"' VARNAME
                | '${' NAME '}'
                | '`{' program '}'
                | '<{' program '}'
                | '~' '/' LITERAL
                | '~'
                | lambda

    var_ref     = '$' VARNAME accessor*
    accessor    = '.' (NUM | NAME)

    value       = '(' word* ')'
                | lambda
                | word

### Accessor syntax (reserved)

`$x.0`, `$x.ok` are reserved for future use (tuples, sums).
In the base system, these are parse errors. This prevents
user-defined names from colliding with the future accessor
paths.

When activated (tuples/sums extension), accessors compose:
`$result.ok.name` = Prism then Lens (AffineTraversal). See
specification.md §Extension path.

### Free carets

rc's concatenation rule [1, §Free Carets]: when two word atoms
are adjacent with no intervening whitespace, an implicit `^`
(concatenation) is inserted between them.

    $stem.c           →  $stem ^ .c
    $home/bin         →  $home ^ /bin
    $user@$host       →  $user ^ @ ^ $host
    'hello'$name      →  'hello' ^ $name

Explicit `^` remains available and allows whitespace on either
side: `a ^ b` concatenates `a` and `b`. Free carets require
adjacency — whitespace between atoms produces separate
arguments.

Concatenation always goes through stringification — the
positive-to-positive Kleisli map. All values have a string
representation via Display.

### Two character sets

    var_char    = [a-zA-Z0-9_*]
    word_char   = [a-zA-Z0-9_\-./+:,%*?\[\]@]

    VARNAME     = var_char+
    NAME        = word_char+
    LITERAL     = word_char+
    QUOTED      = "'" (any | "''")* "'"

**`var_char`** is rc's variable-name alphabet. Used after `$`,
`$#`, and `$"`. Variable names terminate at the first character
not in this set: `$home/bin` parses as `$home` followed by
`/bin`.

**`word_char`** is the bare-word alphabet. Includes `.` (for
discipline function names: `cmd x.set { }`), `/` (for paths).

**Divergence from rc:** rc used a single word alphabet. psh
makes the split explicit and adds `.` to `word_char` (not
`var_char`) to support discipline function names.

### Quoting

Single quotes only. Inside quotes, `''` produces a literal
single quote. No double quotes. No backslash escaping inside
quotes. rc heritage [1, §Quotation].

    'hello world'      literal string with space
    'it''s'            produces: it's
    '$x'               literal $x, no expansion

### Tilde expansion

`~` is not a word character. It expands to `$home`:

- **`~/path`** — expands to `$home/path`. The `~` must be at
  word start, immediately followed by `/`.
- **Bare `~`** — expands to `$home`.

**Divergence from rc:** rc treated `~` as a keyword for pattern
matching and had no tilde expansion. psh separates the two:
`~` is tilde expansion (POSIX/ksh heritage), `=~` is pattern
matching.

### Pattern matching (`=~`)

Infix operator: value on the left, glob patterns on the right.
Returns success (exit 0) if any pattern matches, failure
otherwise.

    if($x =~ *.txt) { echo 'text file' }
    if($name =~ (foo bar baz)) { echo 'known' }
    $filename =~ (*.c *.h) && echo 'C source'

Patterns use fnmatch glob syntax (`*`, `?`, `[chars]`). rc
heritage for the glob semantics [1, §Simple commands].
Perl/Ruby heritage for the `=~` infix syntax.

### Brace-delimited variable names

`${name}` explicitly delimits a variable name. The name inside
braces uses `word_char`, not `var_char`. Escape hatch for edge
cases where the narrow `var_char` alphabet is insufficient.

    ${x.get}          looks up variable named x.get
    $x.get            reserved accessor syntax (future)

### Variable expansion

    $x                value of x (list)
    $x(n)             nth element of x (1-based)
    $#x               count of elements in x
    $"x               stringify: join elements with spaces
    ${name}           explicit variable name delimiting

rc heritage for all forms [1, §Variables, §Indexing].


## Redirections

    redirect    = '>' WORD | '>>' WORD
                | '<' WORD
                | '>[' NUM '=' NUM ']'
                | '<[' NUM '=' NUM ']'

Redirections are profunctor maps — transformations on the I/O
context. See specification.md §Profunctor structure for the
full analysis.

- `>file` — Output (rmap: post-compose on output continuation)
- `<file` — Input (lmap: pre-compose on input source)
- `>[n=m]` — Dup (contraction: two fds alias one resource)
- `>[n=]` — Close (weakening: discard a resource)

Redirections are evaluated left to right. The AST wraps
redirections as nesting (inner-to-outer = left-to-right
evaluation). The profunctor laws hold by construction.

rc heritage [1, §Advanced I/O Redirection].


## Coprocess syntax

    cmd_expr    = ... | cmd_expr '|&'

`cmd |&` starts a bidirectional coprocess with a 9P-shaped
protocol discipline. The shell holds both a write end and a
read end to the child.

    cmd |&
    print -p 'query'          # send request, get PendingReply
    read -p reply             # consume PendingReply, get response
    read -p -t $tag reply     # consume specific tag's response

See specification.md §Coprocesses for the full protocol
description (per-tag binary sessions, PendingReply handles,
wire format, star topology).


## Reserved words

Keywords: `cmd`, `let`, `mut`, `export`, `ref`, `if`, `else`,
`for`, `in`, `while`, `match`, `try`, `catch`, `trap`,
`return`.

Future: `type` (for type abbreviations when prenex polymorphism
arrives).

Operators: `=`, `|`, `|&`, `||`, `&&`, `&`, `!`, `=>`, `=~`,
`^`, `>`, `>>`, `<`, `>[`, `<[`.


## References

[1] Tom Duff. "Rc — The Plan 9 Shell." 1990.
    `reference/plan9/papers/rc.ms`

[3] Munch-Maccagnoni. "Syntax and Models of a Non-Associative
    Composition of Programs and Proofs." Thesis, 2013.

[4] Levy. *Call-by-Push-Value.* Springer, 2004.

[5] Curien, Herbelin. "The Duality of Computation." ICFP, 2000.

[9] Munch-Maccagnoni. "Models of a Non-Associative Composition."
    FoSSaCS, 2014.
