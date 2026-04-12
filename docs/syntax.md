# psh syntax

Formal grammar for psh. Starts from rc (Duff 1990) and names
each extension. Theoretical foundation: `docs/specification.md`.

rc reference: `refs/plan9/man/1/rc` and
`refs/plan9/papers/rc.ms`.


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

    binding     = assignment | let_binding | def_binding
                | struct_decl | enum_decl | ref_def

    assignment  = NAME '=' rhs
    let_binding = 'let' let_quals pat (':' type_ann)? '=' rhs
                  ('else' body)?       -- let-else for refutable patterns
    let_quals   = 'mut'? 'export'?
    def_binding = 'def' NAME def_params? (':' type_ann)? body

    struct_decl = 'struct' NAME type_params? '{' field_decl (';' field_decl)* ';'? '}'
    field_decl  = NAME ':' type_ann

    enum_decl   = 'enum' NAME type_params? '{' variant (';' variant)* ';'? '}'
    variant     = NAME '(' type_ann ')'    -- payload variant
                | NAME                     -- nullary variant

    type_params = '(' TYPE_NAME (',' TYPE_NAME)* ')'   -- uppercase type variables
    type_ann    = TYPE_NAME                             -- e.g. Int, Str, Pos
                | TYPE_NAME '(' type_ann (',' type_ann)* ')'  -- type application
                | '(' type_ann (',' type_ann)* ')'             -- bare tuple type

    ref_def     = 'ref' NAME '=' NAME

    rhs         = pipeline | value

    -- patterns: used in let, match, let-else
    pat         = NAME                      -- variable binding
                | '_'                       -- wildcard
                | '(' pat (',' pat)* ')'    -- tuple pattern
                | '{' field_pat (';' field_pat)* ';'? '}'   -- struct record pattern
                | NAME '(' pat ')'          -- enum payload variant pattern
                | NAME                      -- enum nullary variant pattern (same production as variable binding; disambiguated by scope)
                | literal                   -- literal pattern (Int, Str, Path, Bool)
    field_pat   = NAME '=' pat              -- named field pattern
                | NAME                      -- name-pun shorthand for NAME = NAME

`rhs` is either a computation (a pipeline, which includes
simple commands and builtin invocations) or a pure value. Both
forms are handled uniformly by the binding — see §let below
for the CBPV framing.

### Assignment

`x = val` walks the scope chain and updates the first matching
variable. If no variable exists, creates one in the current
scope. The RHS may be a value or a computation, just as with
`let`. rc heritage [1, §Variables and Assignment] extended to
admit computation RHS.

### let

`let` is CBPV's μ̃-binder: `let x = M` where `M` is a
computation that produces a value (in CBPV notation, `M : F(A)`).
The RHS may be a pure value, an effectful builtin call, a
pipeline, or a command substitution — all go through the same
binding mechanism. The computation is evaluated, the resulting
value is bound to `x`, execution continues.

    let x = 42                          # pure value (trivial computation)
    let files = ls *.txt                # builtin returning a list
    let tag = print -p myserver 'query' # effectful builtin returning an Int
    let count = wc -l < file            # pipeline returning an Int
    let out = `{ grep pattern $file }   # command substitution (forked subprocess)

Pure values are a special case: they are trivially thunkable
computations whose RHS is just the value itself. The "let is
always CBV" framing still holds — `let` evaluates the RHS
before binding — but CBV means "evaluate the computation to a
value first," not "RHS must be a pure term."

`let x = val` always creates in the current scope. Immutable
by default. Runs type inference from the computed value: `42`
→ Int, `true` → Bool, `/tmp` → Path, `hello` → Str. Optional
type annotation constrains the element type of the list.

Under the "every variable is a list" model (see
specification.md §Foundational commitment), a scalar binding
like `let x = 42` is sugar for `let x = (42)` — a list of one
Int. Type annotations refer to the element type, not the list
length.

### def

`def name { body }` defines a named computation — a
template in the command sort. This is rc's `fn` [1, §Functions], renamed. Duff chose `fn`
deliberately, but psh draws a distinction between commands
(cuts) and functions (morphisms) that rc did not make. `def`
names the sort.

Three forms for status-returning defs (rc-style, body is a
sequence of commands, return status is the last command's
status):

    def name { body }           # positional params: $1, $2, $*
    def name() { body }         # nullary: takes no arguments
    def name(a b c) { body }    # named params: $a, $b, $c

For **value-returning defs**, an optional return type
annotation after the parameter list specifies the value type.
The body still parses as a sequence of commands, but the final
item is the return value (either a bare expression of the
declared type, or an explicit `return expr` statement). The
bidirectional check flows the declared return type into the
final body position.

    def origin : Pos { { x = 0; y = 0 } }       # final expression is the return value
    def move : Pos -> Pos {                      # function type annotation
        |p| =>
        let new_x = $p .x + 1
        return { x = $new_x; y = $p .y }         # explicit return
    }
    def first_positive : List(Int) -> Option(Int) {
        |xs| =>
        for (x in $xs) {
            if ($x > 0) { return some($x) }      # early return
        }
        none                                     # implicit tail return
    }

The `return` keyword also works in status-returning defs to
set an explicit exit status (`return N` where N is an Int).
rc heritage: `return` in rc is the explicit status-setting
form.

Without parentheses, the command uses rc-style positional
parameters (`$1`, `$2`, `$*`, `$#*`). With parentheses, the
command declares its parameter interface — `()` means nullary,
`(a b c)` binds arguments to named variables. Named parameters
are bound in the command scope alongside positional `$1`, `$2`
etc. for compatibility.

A `def` is not first-class; it is not a value; it cannot be
stored in a variable or passed as an argument. It is a named
entry in the computation context Θ.

Also handles discipline functions: `def x.get { body }`,
`def x.refresh { body }`, and `def x.set { body }`. All three
are `def` cells, all three are **destructors** of a codata
cocase. `.get` is the pure observer; `.refresh` is the effectful
updater, invoked imperatively as `x.refresh`; `.set` is the
mutator, fired on assignment. Dotted-name convention is ksh93
heritage; the three-discipline codata model is psh's
formalization. See specification.md §Discipline functions for
the full semantics.

Type name vs variable name in `def` is disambiguated by
**capitalization**: `def x.set { }` (lowercase `x`) is a
discipline function on variable `x`; `def List.length { }`
(uppercase `List`) is a method on the `List` type. Parser
inspects the first character before the dot.

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

### Discipline functions

A variable may have three discipline cells — `.get`, `.refresh`,
and `.set` — that together make it **codata**: its behavior is
defined by the cells, not by a naive stored slot.

    def x.get {
        # pure observer: reads the stored slot and returns a value.
        # No effects — no subshells, no coprocess queries, no file reads.
    }

    def x.refresh {
        # effectful updater: may invoke any shell machinery and
        # writes the refreshed value into the slot via `x = v`.
        # Invoked imperatively at a step boundary.
    }

    def x.set {
        # mutator: receives incoming value as $1, mediates the
        # assignment, writes the slot via `x = v`. May have effects.
    }

All three are **destructors** in the codata cocase; the cocase
itself is the sole constructor. `.get` is invoked implicitly on
every `$x` reference and must remain pure. `.refresh` is invoked
explicitly as `x.refresh` — a command-position name lookup in Θ,
runs at a step boundary, produces a status. `.set` is invoked on
every assignment to `x`.

Within a single expression, `.get` fires once per variable and
its value is shared at every consumption site — this is a
theorem via Duploids Proposition 8550 ("thunkable ⇒ central"),
not an operational convention. Pure maps into positive values
are thunkable, and thunkable maps are central.

`.refresh` and `.set` bodies run inside polarity frames that
guard reentrancy. Inside the frame, `x = v` is the primitive
slot write (it bypasses the cocase). Pure `.get` needs no
polarity frame — there is no polarity crossing and nothing to
reenter.

See specification.md §Discipline functions for the full
semantics, the mixed monadic lens structure, and the heritage
rationale for the observation/refresh split.


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
    try_cmd     = 'try' body 'catch' '(' NAME ')' body
    trap_cmd    = 'trap' SIGNAL (body body?)?

    match_arm   = pattern ('|' pattern)* '=>' lambda_body
    pattern     = glob_pat | structural_pat | literal_pat | wildcard_pat

    glob_pat    = GLOB              -- e.g. *.txt, [a-z]*
    literal_pat = NUM | QUOTED
    wildcard_pat = '_'
    structural_pat = tagged_pat | tuple_pat | list_pat
    tagged_pat  = NAME '(' pattern* ')'           -- sum or struct destructure
    tuple_pat   = '(' pattern (',' pattern)+ ','? ')'
    list_pat    = '(' ')'                          -- nil
                | '(' pattern pattern ')'          -- cons (head, tail)

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
    } catch (e) {
        echo 'unavailable: '$e
    }

Scoped error handling — ErrorT monad transformer over command
sequences. `try` changes the sequencing combinator from
unconditional `;` to monadic `;ₜ` that checks Status after
each command. On nonzero status, execution aborts to `catch`.
The `catch (e)` binding is a μ̃-binder on the error case.

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

`trap` has three forms, distinguished by the number of block
bodies:

**Lexical** (two blocks): `trap SIGNAL { handler } { body }`

    trap SIGINT { echo 'interrupted'; return 1 } {
        long_running_command
        another_command
    }

Installs the handler for the duration of the body. When the
body exits, the handler is uninstalled. This is the μ-binder
of the sequent calculus [5, §2.1] — the handler captures a
signal continuation, scoped to the body. Inner lexical traps
shadow outer ones for the same signal. The handler may `return
N` to abort the body with status N.

**Global** (one block): `trap SIGNAL { handler }`

    trap SIGINT { echo 'interrupted'; return 1 }
    long_running_command
    another_command

Registers the handler at the top-level object's signal
interface. The handler persists until overridden by another
`trap` for the same signal, or removed (see deletion form
below).

**Deletion** (no block): `trap SIGNAL`

    trap SIGINT    # removes any global handler for SIGINT

Removes a previously-installed global handler. This is the
`fn name` (no body) convention from rc applied to traps.

**Precedence:** at signal delivery, innermost lexical > outer
lexical > global > OS default.

**Signal masking:** an empty handler `trap SIGNAL { } { body }`
(lexical) or `trap SIGNAL { }` (global) silently discards the
signal for the relevant scope.

`try` and `trap` are distinct constructs:
- `try` = synchronous, checked at each `;`, status-only
- `trap` = asynchronous, signal-delivered, continuation-capturing

They compose freely: `try` inside `trap`, `trap` inside `try`.

See specification.md §Error model for the full operational
model, including signal delivery at interpreter step
boundaries (wake-from-block during child waits) and the four
cases of signal interaction with try blocks.


## Expressions

Expressions are the profunctor layer — commands with
redirections, pipelines, and operators.

    expr_cmd    = or_expr ('&')?
    or_expr     = and_expr ('||' and_expr)*
    and_expr    = match_expr ('&&' match_expr)*
    match_expr  = pipeline ('=~' value)?
    pipeline    = cmd_expr ('|' cmd_expr)*
                | cmd_expr '|&' NAME?
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
                | '$((' arith_expr '))'
                | '~' '/' LITERAL
                | '~'
                | lambda
                | tagged_val
                | tuple

    tagged_val  = NAME '(' word* ')'
    tuple       = '(' word (',' word)+ ','? ')'

    arith_expr  = arith_term (arith_op arith_term)*
    arith_term  = NUM | VARNAME | '(' arith_expr ')'
    arith_op    = '+' | '-' | '*' | '/' | '%'
                | '>' | '<' | '>=' | '<=' | '==' | '!='

    var_ref     = '$' VARNAME bracket_access* (ws dot_accessor)*
    bracket_access = '[' expr ']'         -- no space between VARNAME and '['
    dot_accessor   = '.' NAME
    ws          = (' ' | '\t')+

    value       = '(' word* ')'            -- list (homogeneous, runtime arity)
                | tuple                   -- anonymous product (comma-delim, static arity)
                | record_lit              -- struct construction (named fields, check-mode)
                | map_lit                 -- map construction (string keys, synth-capable)
                | variant_val             -- enum construction (tagged)
                | nullary_variant         -- enum nullary variant (bare name)
                | lambda
                | word
    tuple       = '(' word ',' (word ',')* word? ')'
    record_lit  = '{' field_init (';' field_init)* ';'? '}'
    field_init  = NAME '=' value          -- named field
                | NAME                    -- name-pun shorthand for NAME = NAME
    map_lit     = '{' map_entry (',' map_entry)* ','? '}'
    map_entry   = expr ':' expr           -- key (Str) : value
    variant_val = NAME '(' value ')'      -- enum construction with payload
    nullary_variant = NAME                -- bare variant name (context-determined)

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

psh has **two accessor forms** for projecting into structured
values:

**Bracket `$a[i]`** — projection by runtime value. Tuples,
lists, maps. The bracket binds immediately after `$name` with
no space (the no-space rule mirrors rc's `$path(2)` subscript
convention, but with `[` instead of `(`).

**Dot `$a .name`** — named field/method/discipline access with
a required leading space. The space disambiguates from free
caret concatenation.

Grammar:

    accessed_word = '$' VARNAME bracket_access* (ws dot_accessor)*
    bracket_access = '[' expr ']'
    dot_accessor   = '.' NAME
    ws             = (' ' | '\t')+

Examples:

    $pos[0]              # tuple projection (0-based, Lens)
    $pos[1]              # second element
    $pos[-1]             # last element (negative indexing)
    $list[n]             # list element by index (AffineTraversal)
    $list[1..3]          # slice (AffineFold, returns List)
    $m['key']            # map lookup (AffineTraversal, returns Option)
    $name .upper         # string method
    $items .length       # list length
    $s .x                # struct field (Lens)

Parsing rules:

- `$x[0]` (no space before `[`) is a bracket accessor.
- `$x [0]` (space before `[`) is a separate argument.
- `$x .name` (space before `.`) is a dot accessor.
- `$x.name` (no space before `.`) is free caret: `$x ^ .name`.

Inside brackets is **expression context** — never glob.
`$a[0-9]` is arithmetic (evaluates to -9), not a character
class. The bracket content is a full psh expression
type-checked at the bracket site: `Int` for lists/tuples,
`Str` for maps. Tuple bracket access requires a **literal**
integer index (the return type depends on which position is
accessed).

All bracket accesses return `Option(T)` — `some(val)` or
`none` on miss (out-of-bounds for lists/tuples, missing key
for maps). Users pattern-match on the option or use `let-else`.

Bracket and dot compose freely: `$t[0] .name` is Lens ∘ Lens
= Lens. `$m['key'] .name` is AffineTraversal ∘ Lens =
AffineTraversal. `$s .field[0]` is Lens ∘ Lens = Lens.

The dot accessor namespace is per-type. `def Type.name { }`
registers a new accessor on a type. Capitalization
disambiguates type methods (`def List.length { }`) from
discipline functions (`def x.set { }`).

**No `[*]` or `[@]` inside brackets.** psh does not adopt
ksh93's all-elements subscript forms. `$a` already gives the
whole list (every variable is a list); iteration uses
`for x in $list { ... }`.

See specification.md §Tuples, §Structs, §Map type, and
§Optics activation for the full typing rules.

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

`${name}` explicitly delimits a variable name. The name inside
braces uses `word_char` (including `.`), not `var_char`. This
is the escape hatch for variable names containing characters
that would otherwise terminate a bare `$name` reference.

    ${x.get}          looks up variable named literally "x.get"
    $x.get            free caret: $x ^ .get (rc heritage)
    $x .get           dot accessor: call the .get method on $x
    $x[0]             bracket accessor: first element of $x

Braces are NOT the accessor form. Dot accessors are postfix
with required space; bracket accessors bind immediately after
`$name` with no space (see §Accessor syntax above).

### Variable expansion

    $x                value of x (list)
    $#x               count of elements in x — list length destructor
    $"x               stringify: join elements with spaces — list join destructor
    ${name}           explicit variable name delimiting (escape hatch only)

    $x[0]             first element (bracket projection, Lens on tuples)
    $x[n]             list/tuple element by index (AffineTraversal)
    $x['key']         map lookup by key (AffineTraversal)
    $x .name          named accessor (struct field or type method)

rc heritage for `$x`, `$#x`, `$"x`, and `${name}` [1,
§Variables, §Indexing]. Bracket accessor is psh's addition,
following ksh93's `${a[n]}` convention with simplified syntax.
rc's `$x(n)` 1-based list indexing is replaced by `$x[0]`
(0-based). Dot accessor is modeled on Agda copatterns for
named field/method/discipline access.

**Parameter expansion sigils are sugar aliases.** `$#x` and
`$"x` are prefix-sigil parameter expansion operators inherited
from rc:

    $#x : List → Int         -- length destructor
    $"x : List → Str         -- join destructor

Under the per-type accessor namespace model, these are sugar
aliases for the canonical postfix forms:

    $#x  ≡  $x .length
    $"x  ≡  $x .join

The canonical form is the accessor; the sigil form is rc-faithful
ergonomic shorthand. These are the only two inherited from rc
— no new prefix-sigil destructors are added. Additional
type-specific operations (`.upper`, `.split`, `.replace`, etc.)
are expressed as accessor methods in the per-type namespace, not
as new sigils.


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
description (per-tag binary sessions, Int tag interface,
wire format, named coprocesses).


## Reserved words

Keywords: `def`, `let`, `mut`, `export`, `ref`, `if`, `else`,
`for`, `in`, `while`, `match`, `try`, `catch`, `trap`,
`return`, `struct`.

Reserved for future use: `type` (type aliases), `enum` (named
coproduct types generalizing sums — built-in sum tags like
`ok`/`err`/`some`/`none` suffice for v1).

Operators: `=`, `|`, `|&`, `||`, `&&`, `&`, `!`, `=>`, `=~`,
`^`, `>`, `>>`, `<`, `>[`, `<[`.


## References

[1] Tom Duff. "Rc — The Plan 9 Shell." 1990.
    `refs/plan9/papers/rc.ms`

[3] Munch-Maccagnoni. "Syntax and Models of a Non-Associative
    Composition of Programs and Proofs." Thesis, 2013.

[4] Levy. *Call-by-Push-Value.* Springer, 2004.

[5] Curien, Herbelin. "The Duality of Computation." ICFP, 2000.

[9] Munch-Maccagnoni. "Models of a Non-Associative Composition."
    FoSSaCS, 2014.
