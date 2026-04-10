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

    binding     = assignment | let_binding | def_binding | ref_def

    assignment  = NAME '=' value
    let_binding = 'let' let_quals NAME (':' type_ann)? '=' value
    let_quals   = 'mut'? 'export'?
    def_binding     = 'def' NAME def_params? body
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

### def

`def name { body }` defines a named computation — a
template in the command sort. This is rc's `fn` [1, §Functions], renamed. Duff chose `fn`
deliberately, but psh draws a distinction between commands
(cuts) and functions (morphisms) that rc did not make. `def`
names the sort.

Three forms:

    def name { body }           # positional params: $1, $2, $*
    def name() { body }         # nullary: takes no arguments
    def name(a b c) { body }    # named params: $a, $b, $c

Without parentheses, the command uses rc-style positional
parameters (`$1`, `$2`, `$*`, `$#*`). With parentheses, the
command declares its parameter interface — `()` means nullary,
`(a b c)` binds arguments to named variables. Named parameters
are bound in the command scope alongside positional `$1`, `$2`
etc. for compatibility.

A `def` is not first-class; it is not a value; it cannot be
stored in a variable or passed as an argument. It is a named
entry in the computation context Θ.

Also handles discipline `.set` functions: `def x.set { body }`,
`def x.set(val) { body }`. ksh93 heritage. Note: `.get`
disciplines are pure and defined as lambdas, not `def` — see
§Discipline functions.

### ref

`ref name = target` creates a nameref — an alias that resolves
through `target` on every access. The target is a literal name,
not an expression. ksh93 heritage.


## Functions (values, not commands)

Functions are values in the value sort, created by `let`-binding
a lambda expression. They are first-class: storable, passable,
composable.

    lambda      = '|' NAME* '|' lambda_body
    lambda_body = '=>' command      -- single-expression form
                | '{' program '}'   -- block form

    let double = |x| => $((x * 2))
    let greet  = |name| { echo 'hello '$name; return 0 }
    let add    = |x| => |y| => $((x + y))    # currying
    let thunk  = | | => echo 'no args'       # nullary

In CBPV terms [4], a lambda is `U(A₁ → ... → Aₙ → F(B))`
when impure, or `U(A₁ → ... → Aₙ → B)` when pure. The `U`
(thunk) wraps a computation as a value. The distinction between
`def` and `let` + lambda is the CBPV value/computation boundary
surfaced as syntax. See specification.md §Two kinds of callable.

The `|...|` parameter delimiter is unambiguous in value position:
`|` in command position is a shell pipe, but a lambda only appears
where a value is expected (RHS of `let`, inside a word, etc.),
and in those positions a leading `|` always opens a lambda.

**Capture semantics.** Lambdas capture free variables at
definition time — `Vec<(String, Val)>`, positive, Clone. This
is the closed-term property. Named `def` definitions use
dynamic resolution (read current scope at call time). The
distinction is the sort boundary: values (lambdas) are
self-contained; named computations (defs) live in a context.

**Purity inference.** The shell infers purity by conservative
AST analysis: if the body contains no assignments to variables
outside the lambda's scope, no fork/exec, no side-effecting
builtins, no I/O, no coprocess interaction — the lambda is
classified pure. Pure lambdas are thunkable/central in the
duploid [9, Table 1]. Impure lambdas work but degrade
to oblique maps. See specification.md §Two kinds of callable.

### Discipline .get functions

`.get` disciplines are defined as `def` — they are effectful
notification hooks that fire on every `$x` access:

    def x.get {
        # body runs on every access to $x
        # effects are permitted (logging, tracing, metrics)
    }

Constraints on `.get` bodies:

- The body's return value is discarded; `$x` always evaluates
  to the stored value.
- The body cannot modify the variable it is attached to (`x`
  is free in the body of `x.get`).
- Side effects are permitted.

See specification.md §Discipline functions for the full
rationale and the caveats around cross-variable consistency.


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
    structural_arm = NAME '(' NAME ')' '=>' lambda_body
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

**Structural matching** on Sum values:

    match($result) {
        ok(val)  => echo 'success: '$val;
        err(msg) => echo 'error: '$msg
    }

Structural arms use `tag(binding) =>` — the same parens
syntax as sum construction. The binding is a μ̃-binder scoped
to the arm body. The variable does not escape the arm. The
wildcard arm `* =>` does not bind a variable.

The presence of `NAME(` (no space before paren) distinguishes
structural from glob arms. Glob patterns never have parens
immediately after a word.

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
                | '${' NAME accessor* '}'
    accessor    = '.' (NUM | NAME)
                | '`{' program '}'
                | '<{' program '}'
                | '$((' arith_expr '))'
                | '~' '/' LITERAL
                | '~'
                | lambda

    arith_expr  = arith_term (arith_op arith_term)*
    arith_term  = NUM | VARNAME | '(' arith_expr ')'
    arith_op    = '+' | '-' | '*' | '/' | '%'
                | '>' | '<' | '>=' | '<=' | '==' | '!='

    var_ref     = '$' VARNAME

    value       = '(' word* ')'
                | tuple
                | sum_val
                | lambda
                | word
    tuple       = '(' word ',' (word ',')* word? ')'
    sum_val     = NAME '(' value ')'

**Tuples.** Comma-separated values in parentheses. Lists are
space-separated (rc heritage). The comma disambiguates.

    (a b c)              # list — space-separated
    (10, 20)             # tuple — comma-separated
    ('lane', '/home/lane', 1000)

A trailing comma is permitted: `(10, 20,)` = `(10, 20)`.
A single-element tuple requires a trailing comma: `(42,)`.
Without the comma, `(42)` is a one-element list.

psh extension — rc had no tuples.

### Accessor syntax

Accessors project into structured values. They live inside
`${ }` braces — ksh93's `${x.field}` convention. Without
braces, `.` is always a free caret boundary (rc heritage).

    ${pos.0}             # tuple projection (0-based)
    ${pos.1}             # second element
    ${record.2}          # third element

    $pos.c               # free caret: $pos ^ .c (NOT accessor)
    ${pos}.c             # explicit: value of pos, then ^ .c

Accessors compose: `${nested.0.1}` = projection into element
0, then projection into element 1 of that (Lens . Lens = Lens).
`${result.ok.0}` = Prism then Lens (AffineTraversal).

ksh93 heritage for the brace-delimited convention. See
specification.md §Tuples for the typing rules.

### Arithmetic (`$((...))`)

Arithmetic expressions in `$((...))` are evaluated in-process
and return a typed `Val::Int`. Inside the double-parens,
variable names refer to their integer values without `$`, and
arithmetic operators are not shell syntax (no quoting needed).

    let x = 42
    let y = $((x + 1))          # 43
    let z = $((x * 2))          # 84
    let ok = $((x > 10))        # 1 (true)
    let sum = $((x + y * z))    # standard precedence

Variables inside `$((...))` are looked up by name and coerced
to integer. Non-integer values are an error. Nested parens
for grouping: `$((x * (y + z)))`.

This is a polarity shift: computation (arithmetic) → value
(Int). Like command substitution `` `{} `` but the computation
is arithmetic rather than a subprocess — no fork, evaluated
in-process.

ksh93/POSIX heritage. rc had no arithmetic (used external
`expr` or `bc`). psh adds `$((...))` because quoting
arithmetic operators (`'*'`, `'>'`) in a builtin `expr` defeats
the purpose of avoiding a fork.

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
discipline function names: `def x.set { }`), `/` (for paths).

**Divergence from rc:** rc used a single word alphabet. psh
makes the split explicit and adds `.` to `word_char` (not
`var_char`) to support discipline function names.

### Quoting

Single quotes only. No double quotes. rc heritage [1,
§Quotation] for the single-quote convention. psh adds limited
backslash escape support (see §Backslash escapes).

    'hello world'      literal string with space
    'it''s'            produces: it's (rc-compatible doubling)
    'it\'s'            produces: it's (psh extension)
    '$x'               literal $x, no expansion

### Backslash escapes

psh allows backslash escapes in limited form, both inside and
outside single-quoted strings.

    \<non-whitespace>    literal escape — produces the character
    \<newline>           line continuation (rc heritage)
    \<space>, \<tab>     trivia (the whitespace character, backslash stripped)

Specifically:

- `\\` produces a literal backslash.
- `\'` inside a single-quoted string produces a literal single
  quote. This is in addition to rc's `''` convention, which also
  works.
- `\$`, `\#`, `\"`, etc. outside quotes produce the literal
  character, bypassing any interpretation.
- `\<newline>` at the end of a line continues the line without
  a terminator. Standard line continuation.
- `\<space>` and `\<tab>` collapse to the whitespace character
  alone — the backslash is stripped. This is trivia and has no
  semantic effect, but is not an error.

psh does NOT do C-style escape sequences. `\n` is literal `n`
(the character), not a newline. `\t` is literal `t`. If you need
a real newline in a string, use a multi-line quoted string.

**Divergence from rc:** rc had no backslash escaping. psh adds
it as a cleaner alternative to `''` for literal quotes inside
strings, and for escaping shell syntax characters in general.
The additions are strict — only the explicit cases above are
recognized; everything else is a parse error.

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

`${name}` explicitly delimits a variable name and provides
accessor syntax. The name inside braces uses `word_char`
(including `.`), not `var_char`. Accessor `.N` or `.name`
inside braces projects into structured values.

    ${x.0}            tuple projection (accessor)
    ${x.get}          looks up variable named x.get (no accessor — .get is not a number)
    $x.get            free caret: $x ^ .get (NOT accessor — no braces)

### Variable expansion

    $x                value of x (list)
    $x(n)             nth element of x (1-based)
    $#x               count of elements in x — list length destructor
    $"x               stringify: join elements with spaces — list join destructor
    ${name}           explicit variable name delimiting
    ${name.N}         tuple projection (Lens)
    ${name.tag}       sum projection (Prism)

rc heritage for `$x`, `$x(n)`, `$#x`, `$"x`, and `${name}` [1,
§Variables, §Indexing].

**Parameter expansion as destructors.** `$#x` and `$"x` are
prefix-sigil parameter expansion operators in the ksh93/Bourne
sense — they are eliminators for the List type:

    $#x : List → Int         -- length destructor
    $"x : List → Str         -- join destructor

psh uses a prefix-sigil convention (inherited from rc) rather
than ksh93's suffix operators (`${var#pat}`, `${var%pat}`). The
operators available are exactly those rc provides. Additional
type-specific operations are expressed as named cells in the
per-type namespace rather than as new sigils or suffix forms.


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

    cmd_expr    = ... | cmd_expr '|&' NAME?

`cmd |& name` starts a named bidirectional coprocess with a
9P-shaped protocol discipline. `cmd |&` without a name targets
the default coprocess.

    server |& myserver                # start named coprocess
    print -p myserver 'query'         # send to myserver
    read -p myserver reply            # read from myserver
    read -p myserver -t $tag reply    # read specific tag

    worker |& bg                      # second coprocess
    print -p bg 'task'                # independent channel

    cmd |&                            # anonymous (default name)
    print -p 'query'                  # targets default
    read -p reply                     # targets default

See specification.md §Coprocesses for the full protocol
description (per-tag binary sessions, PendingReply handles,
wire format, named coprocesses).


## Reserved words

Keywords: `def`, `let`, `mut`, `export`, `ref`, `if`, `else`,
`for`, `in`, `while`, `match`, `try`, `catch`, `trap`,
`return`.

Reserved for future use: `type` (type aliases), `struct`
(named product types generalizing tuples), `enum` (named
coproduct types generalizing sums).

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
