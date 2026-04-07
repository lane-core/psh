# psh syntax

Formal grammar for psh. Starts from rc (Duff 1990) and names
each extension. This grammar is the target language specification.
Productions marked **[planned]** are not yet implemented — the
current parser implements a subset (6 Val variants, `switch` not
`match`, no `try`, no Sum, no Tuple, no ExitCode, no free
carets, no two-alphabet split). Unmarked productions are
implemented and tested.

rc reference: `reference/plan9/man/1/rc` and `reference/plan9/papers/rc.ms`.
Theoretical foundation: `docs/specification.md`.


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

A comment starts with `#` and runs to end of line. Comments do
not nest.


## Commands

A command is one of: a binding (context extension), a control
flow construct, or an expression.

    command     = binding | control | return_cmd | expr_cmd
    return_cmd  = 'return' value?

### Bindings

Bindings extend the context Γ with a new name. They are
μ̃-binders in the sequent calculus reading (specification.md
§The four sorts).

    binding     = assignment | let_binding | fn_def | ref_def

    assignment  = NAME '=' value
    let_binding = 'let' let_quals NAME (':' type_ann)? '=' value
    let_quals   = 'mut'? 'export'?         -- order free, duplicates rejected
    fn_def      = 'fn' NAME body
    ref_def     = 'ref' NAME '=' NAME

    type_ann    -- see §Type annotations for the full grammar

**Assignment** (`x = val`) walks the scope chain and updates the
first matching variable. If no variable exists, creates one in
the current scope. Always produces `Val::Str` — no type
inference. rc heritage (Duff 1990, §Variables and Assignment).

**let** (`let x = val`) always creates in the current scope.
Immutable by default. Runs type inference: `42` → Int, `true` →
Bool, `/tmp` → Path, `hello` → Str. Quoted values (`'42'`) stay
Str. Leading-zero integers (`042`) stay Str. Optional type
annotation validates via Prism check. psh extension.

`let` is always CBV. The RHS is evaluated at binding time and
the result is stored. There is no call-by-name `let` form.

For live re-evaluation (tier-3, pane namespace queries), use a
`.get` discipline on the variable:

    let mut cursor : Int = 0
    fn cursor.get { cursor = `{ get /pane/editor/attrs/cursor } }

The `.get` fires on every `$cursor` access (notification hook),
and the assignment inside updates the stored value. See
§Discipline functions above.

For capturing fallible computation results as typed values, use
`try` in expression position:

    let result : Result[Int] = try { get /pane/editor/attrs/cursor }

`try` in value position forks, captures stdout + exit status,
and returns `Result[T]`. See §`try` in value position below.

**fn** (`fn name { body }`) defines a function. Also handles
discipline functions: `fn x.get { body }`, `fn x.set { body }`.
rc heritage for plain functions; ksh93 heritage for disciplines.

**Discipline functions.** `fn x.get { body }` fires as a
notification on `$x` access — a side-effect hook, not a view
transformer. The `.get` body runs in a readonly scope (mutations
rejected). The returned value is always the stored value, not
the body's output. The `.get` body cannot influence what `$x`
evaluates to. `fn x.set { body }` fires on assignment to `x`,
with `$1` bound to the new value. Reentrancy guard prevents
`fn x.set { x = $1 }` from recursing. ksh93 heritage.

**ref** (`ref name = target`) creates a nameref — an alias that
resolves through `target` on every access. The target is a
literal name, not an expression. Disciplines are keyed to the
syntactic name, not the resolved target: `$y` where `ref y = x`
does NOT fire `fn x.get`. ksh93 heritage.

### Control flow

Control flow constructs branch or iterate. Each takes its
condition or value as a bare expression and its body as a braced
block.

    control     = if_cmd | for_cmd | while_cmd
                | match_cmd | try_cmd

    if_cmd      = 'if' pipeline body ('else' (if_cmd | body))?
    for_cmd     = 'for' NAME 'in' value body
    while_cmd   = 'while' pipeline body
    match_cmd   = 'match' value '{' match_arm (';' match_arm)* ';'? '}'  -- [planned]
    try_cmd     = 'try' body ('else' NAME body)?        -- [planned]

    match_arm   = match_pat+ '=>' command+              -- [planned]
    match_pat   = NAME                          -- glob pattern
                | NAME '$' NAME                 -- structural: tag + binding [planned]

    body        = '{' program '}'
                | '=>' command                          -- single-line [planned]

`=>` is a dependent keyword — a single-line body introducer.
It can appear after any keyword-initiated clause: `if`, `for`,
`while`, `try`, `else`, `fn`, or a `match` pattern. After `=>`,
the parser reads a single command (terminated by `;`, newline,
or `}`). `{` opens a multi-line body as before. Both forms are
interchangeable wherever `body` appears in the grammar.

Inside `match { }`, arms are separated by `;` with an optional
trailing `;` before `}`. Newlines between arms are trivia —
the parser ignores them. This means multi-line and single-line
match blocks use the same grammar:

    # Multi-line
    match $type {
        editor   => return '📝';
        terminal => return '💻';
        * => return '?'
    }

    # Single-line (same grammar, just horizontal)
    match $type { editor => return '📝'; terminal => return '💻'; * => return '?' }

Note: `ok => { }` (no `$binding`) is a glob match on the
literal string "ok", not structural decomposition. Structural
arms always require `$binding`: `ok $v => { }`. The `$` after
the tag name distinguishes structural from glob arms.

**Divergence from rc:** rc wraps conditions in parentheses:
`if(list) command`, `for(name in list) command`,
`while(list) command`, `switch(word){ ... }`. psh does not. Four
related changes:

1. **No parentheses around conditions.** The opening `{` of the
   body terminates the condition. rc used `()` for this, but `()`
   also means list literal — an overloading psh avoids. The `{`
   token is unambiguous: it cannot appear as a bare word.

2. **`else` instead of `if not`.** rc's `if not` is a separate
   command that reads `$status` left by the preceding `if` — an
   implicit state dependency. Duff acknowledged: "The one bit of
   large-scale syntax that Bourne unquestionably does better than
   rc is the if statement with else clause" (rc.ms §Why not).
   psh's `else` is a syntactic clause of the `if`
   node. The AST represents both branches as fields of one
   `Command::If` — a proper coproduct elimination. No state
   dependency, no dangling-else ambiguity.

3. **Bodies use `{` or `=>`.** rc allows a single command as
   body: `if(cond) cmd`. psh uses two body forms: `{ program }`
   for multi-line bodies and `=> command` for single-line bodies.
   Without parentheses to delimit the condition, the body needs
   an unambiguous introducer — `{` or `=>` serves that role.
   This enables `else` (the parser knows the then-body is
   complete at `}` or `;`) and resolves the dangling-else by
   making structure explicit.

4. **`match` arms use `=>`, separated by `;`.** rc delimits
   case bodies implicitly — execution runs from one `case`
   keyword to the next. psh uses `=>` to introduce each arm's
   body and `;` to separate arms. Each arm is an independent
   branch. Multiple patterns per arm are supported:
   `a b => body` matches either `a` or `b`. Fall-through is not
   available — `;` terminates each arm.

These four changes form a coherent package. Removing parentheses
requires explicit body introducers (`{` or `=>`), which enables
structural `else`, which makes `=>` + `;` the natural arm syntax.

**`match` instead of `switch`.** rc used `switch` as the outer
keyword with `case` for arms inside. psh uses `match` — a new
keyword for a new construct. rc's `case` was an arm-introducer
(sub-keyword inside `switch`). Reusing `case` as a block-
introducer would create a false cognate — it looks like rc
heritage but has a different syntactic role. `match` is honest:
it names the operation (pattern matching / coproduct elimination)
without pretending to be rc's `case`.

psh's `match` does two kinds of dispatch: glob pattern matching
on string values (rc heritage) and structural coproduct
elimination on Sum values (psh extension). Arms use `=>`
to introduce the body and `;` to separate. No sub-keyword
needed.

    # Glob matching on strings (rc heritage)
    match $filename {
        *.txt => echo 'text file';
        *.rs  => echo 'rust source';
        * => echo 'other'
    }

    # Structural matching on Sum values (psh extension)
    match $result {
        ok $v  => echo 'success: '$v;
        err $e => echo 'error: '$e
    }

Structural arms have the form `tag $binding =>` — a tag name
followed by a `$`-prefixed binding variable, then `=>`. Glob
arms have the form `pattern =>`. The `$` after the tag name
distinguishes structural from glob arms.

**Value-producing blocks.** When a body appears in value
position (RHS of `let`, etc.), it may end with `return value`
to produce a typed value. Commands in the body run for effects;
`return` injects a value from the command sort into the value
sort (CBPV's `return : A → F(A)`). Without `return`, the
body produces `Unit`.

    let icon = match $type {
        editor   => return '📝';
        terminal => return '💻';
        * => return '?'
    }

    let greeting = if ~ $lang fr => return 'bonjour'; else => return 'hello'

    # Multi-line with effects before return
    let result = match $code {
        200 => {
            log 'success'
            return ok $body
        };
        * => return err 'unexpected'
    }

`return` is unambiguous — the keyword marks the polarity shift.
Bare words in a body are always commands. `return` followed by
a word is always a value injection. No disambiguation needed.

**`try` block.** Scoped error handling — the ⊕→⅋ converter.
Inside `try`, any command with nonzero Status aborts the block
and transfers control to the `else` clause. The `else` variable
receives the Status string of the failing command.

    try {
        let title = `{ get /pane/focused/attrs/title }
        let cursor = `{ get /pane/focused/attrs/cursor }
        echo $title' ['$cursor']'
    } else $e {
        echo 'pane unavailable: '$e
    }

Semantics:
- After each command inside `try`, if `$status` is nonzero,
  execution jumps to `else`.
- Boolean contexts are exempt: `if` conditions, `while`
  conditions, `&&`/`||` LHS, `!` commands.
- `try` without `else` is legal — errors abort the block
  silently and Status propagates to the enclosing scope.
- `try` blocks nest: inner catches before outer.
- No longjmp, no continuation stack. Implementation: `in_try`
  flag, checked after each command in `run_cmds`.

psh extension. Equivalent to lexically-scoped `set -e` without
the composability defects of POSIX `set -e`. The specification
identifies this as the ⊕→⅋ converter from ksh26 SPEC.md, with
proper scoping.

**`for` list termination.** `for name in value { body }` parses
exactly one `value`: either a parenthesized list `(a b c)` or a
single word. To iterate over multiple elements, use a list:
`for x in (a b c) { ... }`. A variable reference
`for x in $list { ... }` expands at runtime to the variable's
elements. This differs from rc's `for(x in a b c)` where the
parentheses group multiple bare words into the list. The grammar
is LL(1) — after `in`, the parser reads one value, then
expects `{`.

**`while` with empty condition.** rc's `while() echo y` (empty
parens = always true) becomes `while true { echo y }` in psh.
The `true` builtin is explicit.

**`else if` chaining.** psh supports `else if` as syntactic
sugar for a nested `if` in the else branch:

    if cond1 { A } else if cond2 { B } else { C }

The parser handles this — after `else`, it checks for `if` and
recursively parses an `if_cmd`.


## Expressions

Expressions are the profunctor layer — commands with
redirections, pipelines, and operators.

    expr_cmd    = or_expr ('&')?
    or_expr     = and_expr ('||' and_expr)*
    and_expr    = pipeline ('&&' pipeline)*
    pipeline    = cmd_expr ('|' cmd_expr)*
                | cmd_expr '|&'
    cmd_expr    = '!' cmd_expr
                | body
                | '@' body
                | simple_cmd redirect*

    simple_cmd  = WORD+
    redirect    -- see §Redirections for the full grammar


## Words

Words are positive (CBV) — evaluated eagerly before the command
that consumes them runs. A word is one or more word atoms joined
by concatenation.

    word        = word_atom ('^' word_atom)*
    word_atom   = LITERAL | QUOTED
                | var_ref
                | '$#' VARNAME | '$"' VARNAME
                | '${' NAME '}'
                | '`{' program '}'
                | '<{' program '}'
                | '~' '/' LITERAL
                | '~'

    var_ref     = '$' VARNAME accessor* ('(' word ')')?
    accessor    = '.' (NUM | NAME)                  -- [planned]

    value       = '(' word* ')'
                | tagged_val                        -- [planned]
                | lambda
                | word
    tagged_val  = NAME value                        -- [planned]
    lambda      = '\' (NAME+ | '(' ')') '=>' ('{' program '}' | command)

**Accessors** **[planned]** project into structured values. Since
`.` is NOT in `var_char`, after `$pos` the parser sees `.0` as
the start of a new token. The `accessor` production captures this:
`$pos.0` is tuple projection (π₀), `$result.ok` is tagged
decomposition (Prism preview), `$e.code` is ExitCode extraction.
The accessor chains compose: `$result.ok.name` is Prism then
Lens (AffineTraversal). Without the accessor production, the free
caret rule would produce `$pos ^ .0` (string concatenation), not
structural access. The accessor takes priority over free carets
when the token immediately following a `var_ref` is `.` followed
by a digit or `NAME`.

**Sum construction** **[planned]** is context-sensitive. In
`let` RHS, `match` arm body, and `value` position, a bare word
followed by a value is Sum construction: `ok 42` →
`Sum("ok", Int(42))`. In command position, it is a simple
command: `ok 42` runs the command `ok` with argument `42`. The
parser context determines interpretation — `value` attempts the
`tagged_val` production before falling through to `word`.

### Two character sets

**[planned]** The current implementation uses a single
`is_word_char` predicate. The two-alphabet split described here
is the target design and the first implementation task for the
new grammar. The current parser includes `~` in word characters
and excludes `@`; the target grammar reverses both.

psh uses two character predicates for different parsing contexts.
This is the key mechanism that enables free carets and discipline
function names to coexist.

    var_char    = [a-zA-Z0-9_*]
    word_char   = [a-zA-Z0-9_\-./+:,%*?\[\]@]

    VARNAME     = var_char+
    NAME        = word_char+
    LITERAL     = word_char+
    QUOTED      = "'" (any | "''")* "'"

**`var_char`** is rc's variable-name alphabet. Used after `$`,
`$#`, and `$"`. Variable names terminate at the first character
not in this set. This means `$home/bin` parses as `$home`
followed by `/bin` — the `/` is not part of the variable name.

**`word_char`** is the bare-word alphabet. Used for literals,
function names (after `fn`), and other name positions. Includes
`.` (for discipline function names: `fn x.get { }`), `/` (for
paths), and other characters that are not shell operators.

`~` is not in either set. It receives special handling (see
§Tilde below).

**Divergence from rc:** rc used a single word alphabet for both
contexts, but rc's variable names were implicitly restricted to
alphanumerics and `_` because all other characters triggered
free caret insertion. psh makes the split explicit and adds `.`
to `word_char` (not `var_char`) to support discipline function
names — a construct rc did not have.

### Free carets

When two word atoms are adjacent with no intervening whitespace,
an implicit `^` (concatenation) is inserted between them. This
is rc's free caret rule (Duff 1990, §Free Carets).

The rule fires when: after parsing a word atom, the next
character (with no whitespace) can start another word atom —
that is, it is `$`, `'`, `` ` ``, `<{`, `~`, or any `word_char`.

Examples:

    $stem.c           →  $stem ^ .c        (var ends at .)
    $home/bin         →  $home ^ /bin      (var ends at /)
    $user@$host       →  $user ^ @ ^ $host
    'hello'$name      →  'hello' ^ $name
    $file.$ext        →  $file ^ . ^ $ext

**Interaction with accessors [planned]:** When the accessor
production is implemented, `$pos.0` is parsed as a projection
(accessor), not a free caret. The accessor production takes
priority over free carets after a `var_ref`. This means
`$stem.c` would also parse as an accessor — a breaking change
from rc's `$stem ^ .c` idiom. The resolution: rc's pattern
uses explicit caret (`$stem^.c`) or brace delimiting
(`${stem}.c`) when the accessor production is active. This
tradeoff is acceptable because the accessor enables the entire
optic composition story (Prism, Lens, AffineTraversal), which
is more valuable than the implicit concat convenience.

Explicit `^` remains available and allows whitespace on either
side: `a ^ b` concatenates `a` and `b`. Free carets require
adjacency — whitespace between atoms produces separate arguments.

**Divergence from rc:** rc states: "Whenever one of `$ ' `` ` ``
follows a quoted or unquoted word or an unquoted word follows a
quoted word with no intervening blanks or tabs, a `^` is inserted
between the two" (rc(1) §Free Carets). psh's rule is equivalent,
adapted to the two-alphabet split: the boundary between `var_char`
and `word_char` is where free carets fire after `$`-references.

### Quoting

Single quotes only. Inside quotes, `''` produces a literal
single quote. No double quotes. No backslash escaping inside
quotes.

    'hello world'      literal string with space
    'it''s'            produces: it's
    '$x'               literal $x, no expansion

**`Word::Quoted` vs `Word::Literal`.** The AST distinguishes
quoted from unquoted words. In `let` context, this controls type
inference: `let x = 42` infers Int, `let x = '42'` stays Str.
Outside `let`, both evaluate to `Val::Str`. The distinction is
consumed at evaluation time and does not propagate into the
runtime value model. psh extension — rc had no types.

### Tilde expansion

`~` is not a word character. It receives special handling:

- **`~/path`** — expands to `$home/path`. The `~` must be at
  word start, immediately followed by `/`.
- **Bare `~`** — expands to `$home` when in argument position.
- **`~ value pattern...`** — in command position, dispatches to
  the match builtin (rc heritage).

The parser resolves the ambiguity by position: in command-name
position, `~` is the match operator. In argument position, `~`
is tilde expansion.

**Divergence from rc:** rc treated `~` as a keyword and had no
tilde expansion (Plan 9 used `$home` exclusively). psh adds
tilde expansion from the POSIX/ksh tradition and resolves the
conflict by parse-position dispatch. The match builtin retains
rc's semantics: `~ $x *.c` succeeds if `$x` matches `*.c`.

### Brace-delimited variable names

`${name}` explicitly delimits a variable name. The name inside
braces uses `word_char` (the broad alphabet), not `var_char`.
This is the escape hatch for edge cases where the narrow
`var_char` alphabet is insufficient.

    ${x.get}          looks up variable named x.get
    $x.get            accessor: project .get from $x [planned]

Without the accessor production (current implementation),
`$x.get` is a free caret: `$x` concatenated with `.get`. With
the accessor production, `$x.get` becomes structural access.
`${x.get}` is always an explicit variable name lookup regardless
of the accessor rule. Discipline functions dispatch through the
evaluator, not through `$`-expansion.

### Variable expansion

    $x                value of x (list)
    $x(n)             nth element of x (1-based)
    $x.0              tuple projection (0-based) [planned]
    $#x               count of elements in x
    $"x               stringify: join elements with spaces
    ${name}           explicit variable name delimiting

**Indexing conventions.** List indexing `$x(n)` is 1-based (rc
heritage — lists are ordinal: first, second, third). Tuple
projection `$pos.0` is 0-based (structural — π₀, π₁ from type
theory: field 0, field 1). The conventions differ because lists
are sequences with positional identity and tuples are products
with structural identity.

### Command substitution

    `{program}        run program, capture stdout, split on
                      newlines into a list

**Shared capture primitive.** `` `{cmd}`` and `try { cmd }` both
fork a child, pipe stdout, and call waitpid. They share one
`capture_subprocess` implementation that returns `(stdout, exit_code)`.
The two operations project different components of this product:

- `` `{cmd}`` takes stdout only (π₁). Exit status discarded.
  On failure, returns `Unit` (empty).
- `try { cmd }` takes both. Returns `Sum("ok", val)` or
  `Sum("err", ExitCode(n))`.

They are siblings — parallel consumers of the same primitive —
not parent-child. Neither desugars into the other.

    let val = `{ cmd }              # stdout only, status discarded
    let result = try { cmd }        # Result[T]: ok val | err ExitCode

rc heritage. psh splits on newlines only — no `$ifs`. The
captured output has its trailing newline stripped.

### Process substitution

    <{program}        run program, connect stdout to a pipe,
                      evaluate to /dev/fd/N

rc heritage (Duff 1990, §Pipeline branching). Enables
non-linear pipeline topologies: `cmp <{old} <{new}`.

### Line continuation and `\`

Backslash has two roles, disambiguated by one character of
lookahead:

- `\` + newline → line continuation (consumed as whitespace)
- `\` + NAME or `(` → lambda expression (see §Thunks)

    echo $very_long_variable \
         $another_variable \
         $a_third_one

    let inc = \x => expr $x + 1

Backslash has no escape semantics — `\n` in a lambda is a
parameter named `n`, not a newline character. `\` cannot
appear in bare words (it is not in `var_char` or `word_char`).

rc heritage for line continuation (rc(1): "A long command line
may be continued on subsequent lines by typing a backslash
followed by a newline. This sequence is treated as though it
were a blank."). Lambda syntax is a psh extension.

### Concatenation

The `^` operator concatenates two words. If both operands are
lists of equal length, concatenation is pairwise. If one is
scalar and the other is a non-empty list, the scalar broadcasts.

    a = (hello good)
    b = (world bye)
    echo $a^$b              # helloworld goodbye

    echo pre-^$list         # pre-a pre-b pre-c

rc heritage (Duff 1990, §Concatenation).


## Type system

psh values are typed. The type system has seven atomic types,
three type constructors (products, coproducts, thunks), and
sugar for common patterns.

### Atoms and List

    Unit        the empty value. False. Unset variables produce Unit.
    Bool        true | false
    Int         64-bit signed integer (i64)
    Str         string (the universal type, rc heritage)
    Path        filesystem path (starts with / ./ ../ ~/)
    ExitCode    computation outcome (i32). Distinct from Int.
    List[T]     homogeneous sequence (rc's first-class lists)

**ExitCode** is not a data integer. `ExitCode(0)` is "success."
`Int(0)` is "the number zero." ExitCode enters the value world
only through `try` (the ↑ shift that reifies a computation
outcome into value position). ExitCode is not constructible
by literal — `Val::infer` never produces it. This preserves
the computation/value sort boundary.

### Products (tuples)

    (A, B)      binary product. Lens decomposes.
    (A, B, C)   sugar for (A, (B, C))

Comma-separated values in parentheses. Lists use spaces:
`(a b c)`. Tuples use commas: `(a, b, c)`. The comma
disambiguates.

    let pos : (Int, Int) = (42, 7)
    echo $pos.0                      # 42 (projection, 0-based)
    let (x, y) = $pos                # destructuring

Access via `.0`, `.1` etc. (Lens projection, 0-based).
Destructuring in `let` bindings: `let (x, y) = expr`.

### Coproducts (tagged values)

    A | B       coproduct (union). Prism decomposes.

Sum values carry a string tag and a payload. The tag is the
coproduct injection label. The payload is any Val.

    let x = ok 42                    # Sum("ok", Int(42))
    let y = err 1                    # Sum("err", ExitCode(1))
    let e = KeyEvent (97, 0)         # Sum("KeyEvent", Tuple(..))

Construction: `tag payload` — a bare word (the tag) followed by
a value (the payload). `try { body }` implicitly produces
`ok val` or `err ExitCode(n)`.

Elimination: `match` with structural arms.

    match $result {
        ok $v  => echo $v;
        err $e => echo 'error: '$e
    }

The tag is an open string namespace. Any script can define new
tags. Pane protocol enum variants map to tag strings when
messages cross from Rust into the shell.

### Thunks (first-class functions)

    Thunk       suspended computation. CBPV's U(A → F(B)).

A thunk is a first-class function — a computation suspended as
a value. It carries parameter names and a body (AST), but no
captured environment. Free variables resolve dynamically at
force time, consistent with rc/ksh function semantics.

    let greet = \name => echo hello $name
    $greet world                    # forces the thunk: "hello world"

    let inc = \x => expr $x + 1
    let result = `{ $inc 41 }      # "42"

    let multi = \x y => {
        echo $x
        echo $y
    }

**`\` syntax and `fn` sort split.** `\` introduces a lambda
(thunk literal) in value position. `fn` introduces a named
function definition in command position. They are different
syntax for different sorts:

- `fn name { body }` — command-level binding. μ̃-binder that
  extends Γ with a named function. Only valid in command
  position (inside blocks). Uses positional parameters (`$1`,
  `$2`, `$*`). rc heritage.
- `\params => body` — value-level lambda. Produces
  `Val::Thunk`. Only valid in value position (RHS of let,
  argument to a command). Uses named parameters.

The sort boundary is visible in the syntax: `fn` is always a
binding (left of the turnstile), `\` is always a term (right
of the turnstile). `let f = fn { body }` is illegal — use
`let f = \() => body`.

**Lambda grammar:**

    lambda        = '\' lambda_params '=>' lambda_body
    lambda_params = NAME+ | '(' ')'
    lambda_body   = '{' program '}' | command

The `=>` is mandatory — it separates parameters from body,
consistent with match arms where `=>` separates pattern from
body. The body after `=>` is either a braced block or a single
command (terminated by `;`, newline, or `}`).

    \x => echo $x               # single command
    \x => { echo $x; echo done }  # braced block
    \x y => expr $x + $y        # multi-param
    \() => curl -f $url          # nullary

**`\` and line continuation.** `\` followed by a newline is
line continuation (rc heritage, consumed as whitespace). `\`
followed by a NAME or `(` is a lambda. The disambiguation is
LL(1) — the character sets are disjoint. `\` has no other
role; it is not an escape character.

**Forcing.** `$f args` — when a command name evaluates to
`Val::Thunk`, the evaluator forces the thunk by pushing a scope,
binding named parameters, and running the body. This is CBPV's
`force : U(C) → C` — the polarity crossing from value to
computation.

The thunk is positive (inert, Clone, PartialEq). The body is
data (AST). No live resources, no continuations, no captured
mutable state. Negativity (computation, side effects) appears
only at force time. This is CBPV's U operator — the thunk wraps
a negative computation in a positive envelope.

**Dynamic resolution, not closures.** Free variables in the body
resolve against the calling scope at force time, not the defining
scope at creation time. This matches how `fn name { body }`
already works. Closures (capture-by-value at creation time) are
a possible future extension but not part of the initial design.
Currying (`\x => \y => expr $x + $y`) does not work — the
inner thunk does not close over `$x`.

**Thunk as optic leaf.** A thunk has no internal optic structure
for the user — no `.params` or `.body` accessor. It is atomic
from the accessor perspective, like ExitCode. A Tuple containing
a thunk is Lens-accessible at the thunk's position, but the
thunk itself is opaque. PartialEq on thunks is structural (same
params + same AST = equal), which preserves the Lens laws.

Display: `fn(x y){...}` — diagnostic. Not round-trippable.
Truthiness: always true (a thunk exists).
Concat: coerces to display string.

### Sugar

    Result[T]   = T | ExitCode       implied tags: ok, err
    Maybe[T]    = T | Unit          implied tags: ok, none

`Result[T]` is the type of `try { body }` — either the
computation succeeded (producing `Sum("ok", T)`) or failed
(producing `Sum("err", ExitCode(n))`). The tag names `ok`
and `err` are conventional — `Result[T]` is sugar for a
coproduct with these specific tags. `Maybe[T]` uses `ok` and
`none` analogously.

### Type annotations

    type_ann    = 'Unit' | 'Bool' | 'Int' | 'Str' | 'Path'
                | 'ExitCode'
                | 'List' '[' type_ann ']'
                | '(' type_ann ')'                     -- sugar for List[T]
                | '(' type_ann (',' type_ann)+ ')'     -- Tuple
                | type_ann '|' type_ann
                | 'Fn' '(' type_ann* ')'               -- Thunk (arity check)
                | 'Result' '[' type_ann ']'
                | 'Maybe' '[' type_ann ']'

Type annotations validate via Prism check at the binding site.
Union annotations (`A | B`) constrain which Sum variants are
legal. Tuple annotations validate componentwise.

### Val representation

```rust
pub enum Val {
    Unit,
    Bool(bool),
    Int(i64),
    Str(String),
    Path(PathBuf),
    ExitCode(i32),
    List(Vec<Val>),
    Tuple(Vec<Val>),
    Sum(String, Box<Val>),
    Thunk(Thunk),
}

pub struct Thunk {
    pub params: Vec<String>,
    pub body: Vec<Command>,
}
```

Ten variants. Seven atoms, one product constructor (Tuple),
one coproduct constructor (Sum), one thunk constructor (Thunk).
Products give users Lenses. Coproducts give users Prisms.
Thunks are optic leaves (atomic, like ExitCode). The optic
hierarchy is fully inhabited.

### Var metadata

Each variable binding carries metadata alongside its value:

```rust
struct Var {
    value: Val,
    error: Option<String>,       // from last failed try evaluation
    exported: bool,
    readonly: bool,
    mutable: bool,
    type_ann: Option<TypeAnnotation>,
}
```

For stored variables (tiers 1-2), `error` is always `None`.
For computed variables (`let x = try { }`), the value IS the
Sum result — `Sum("ok", T)` or `Sum("err",
ExitCode(n))`. The `$x.err` accessor is a Prism preview into
the err branch of that Sum result, not a separate metadata
channel. The `error` field preserves the error *string* (the
human-readable message from stderr) alongside the ExitCode for
diagnostic use. Val stays inert — pure positive data, no
embedded error signals.

### Display

What `echo $x` prints for each type. Display is the path to
pipes, `execvp` argv, and `$x` in word position. Types are
erased at this boundary — pipes carry bytes.

    Unit          (empty string)
    Bool          true | false
    Int           42 | -5
    Str           the string itself
    Path          the path string
    ExitCode      the numeric code: 0, 1, 42
    List          space-separated elements
    Tuple         space-separated elements (same as List)
    Sum           payload only (tag stripped)
    Thunk         fn(params){...} (diagnostic, not round-trippable)

**Sum display is payload-only.** `echo $result` where
result = `Sum("ok", Int(42))` prints `42`, not `ok 42`.
The tag is control-flow metadata for `match` decomposition
(the Prism's `match` function), not data for display. Prism
laws hold within Val (in-process); they do not extend across
pipe boundaries. Pipes already break type preservation by
design (`Int(42)` → `"42"` → `Str("42")`). Sum is no
different. For inspection, `whatis` shows tag + type + payload.

### Truthiness

What `is_true` returns. Determines `if`/`while`/`&&`/`||`
behavior for values used in boolean contexts.

    Unit          false
    Bool(b)       b
    Int(0)        false
    Int(n)        true
    Str("")       false
    Str(s)        true
    Path          true (always non-empty by construction)
    ExitCode(0)   true  (success)
    ExitCode(n)   false (failure)
    List([])      false
    List(_)       true
    Tuple         true  (always ≥2 elements)
    Sum           true  (always has content)
    Thunk         true  (a suspended computation exists)

**ExitCode truthiness inverts Int truthiness.** `ExitCode(0)`
is true (success). `Int(0)` is false (zero). These are
different sorts: ExitCode is a reified computation outcome
(shell convention: 0 = success = true), Int is data (numeric
convention: 0 = false). This inversion is why ExitCode→Int
coercion is prohibited — it would reverse the meaning of
`if $x`.

**Sum is always true.** A tagged value exists — it has a
tag and a payload. Use `match` to dispatch on the tag, not
`if`. This prevents conflating error handling with truthiness.

### Type inference

Inference runs in `let` context only. Bare `x = val` always
produces `Val::Str`. In `let x = val`, unquoted literals
are inferred:

    42            Int (positive integer, no leading zero)
    -5            Int (negative)
    042           Str (leading zero — not octal)
    true          Bool
    false         Bool
    /tmp          Path (starts with /)
    ./foo         Path (starts with ./)
    ../bar        Path (starts with ../)
    ~/path        Path (after tilde expansion)
    '42'          Str (quoted — inference suppressed)
    hello         Str (default)
    (42, 7)       Tuple — inferred componentwise: (Int, Int)
    ok 42         Sum — tag is "ok", payload inferred: Int
    (a b c)       List — elements inferred individually

ExitCode is NEVER inferred from literals. It enters the
value world only through `try`. Thunk is produced by the
`fn(params) body` literal syntax, not by inference.

### Coercion

Widening (total, information-preserving) is allowed. Narrowing
(partial, may fail) is attempted at reassignment to typed
`let mut` variables.

**Widening (always succeeds):**

    Bool → Str        true → "true"
    Int → Str         42 → "42"
    Path → Str        /tmp → "/tmp"
    ExitCode → Str    0 → "0"

**Narrowing (attempted at reassignment, may fail):**

    Str → Int         "42" → Int(42), "hello" → error
    Str → Bool        "true" → Bool(true), "x" → error
    Str → Path        always succeeds (any string is a path)

**Prohibited:**

    ExitCode → Int    different sorts. ExitCode(0) is true
                      (success), Int(0) is false (zero). The
                      coercion reverses truthiness. Use the
                      explicit `.code` accessor instead.
    Int → ExitCode    require explicit construction via `try`.


## `try` in value position

`try { body }` in value position (RHS of `let`, argument to a
command) forks, captures stdout + exit status, and returns
`Result[T]` = `Sum("ok", T) | Sum("err", ExitCode(n))`.
This is CBV — the body evaluates immediately at binding time.

    let cursor : Result[Int] = try { get /pane/editor/attrs/cursor }
    let seconds : Result[Int] = try { date +%s }

Semantics:
- `try` forks and captures (shares `capture_subprocess` with
  `` `{ } ``). Returns the full Result, not just stdout.
- `$cursor.ok` — Prism preview into the ok branch. Returns
  the captured value if the computation succeeded, Unit
  otherwise.
- `$cursor.err` — Prism preview into the err branch. Returns
  the ExitCode if the computation failed, Unit otherwise.

The `try` keyword carries consistent meaning across contexts:
- `try { } else { }` — scoped ⅋ block (abort on first error)
- `try { }` in value position — captures Result[T], CBV

Both mean "this is a fallible computation." The difference is
what happens with the result: the `else` form handles it
inline; the value form stores it for later inspection.

## Live re-evaluation via `.get` disciplines

For tier-3 variables that should re-query on every access
(pane namespace, computed values), use a `.get` discipline:

    let mut cursor : Int = 0
    fn cursor.get { cursor = `{ get /pane/editor/attrs/cursor } }

Every `$cursor` access fires `cursor.get` as a notification
hook. The discipline body updates the stored value. Subsequent
accesses see the fresh value. This is the mechanism for live
computed variables — the variable stores the latest value, and
the discipline refreshes it on demand.

For error-tracked live variables, combine with `try`:

    let mut cursor : Result[Int] = try { get /pane/editor/attrs/cursor }
    fn cursor.get { cursor = try { get /pane/editor/attrs/cursor } }

The `.get` discipline re-queries and stores a fresh `Result[T]`
on every access. `$cursor.ok` gives the latest value.
`$cursor.err` gives the latest error.


## Subshell and concurrency

Three constructs for concurrent and isolated execution.

### Subshell

    '@' body

`@{ cmds }` forks with an isolated scope. The child receives
a copy of the current context Γ; mutations in the child do not
affect the parent. This is classical contraction — the
continuation is duplicated, each copy evolves independently.

    @{ x = local; echo $x }    # child sees local, parent doesn't
    echo $x                     # unchanged

**Divergence from rc:** rc had `@ command` (bare `@` as a
prefix operator) and `rfork [flags]` for fine-grained fork
control. psh replaces both with `@{ body }` — mandatory
braces, consistent with the grammar. The fine-grained flags
are lost, but so is the kernel dependency.

### Coprocess

    cmd_expr '|&'

`cmd |&` starts `cmd` with stdin/stdout connected to a
bidirectional socketpair. The shell holds one end; the child
holds the other. `print -p` writes to the coprocess.
`read -p` reads from it. Only one coprocess at a time.

This is the ⅋ (par) connective — the negative dual of tuples.
Both channels (read and write) exist concurrently, demand-
driven. The pane project's session type library is named `par`
after this connective.

    cat |&
    print -p hello
    read -p line
    echo $line               # hello

ksh93 heritage. Plan 9 pipes were bidirectional by default
(both ends of `pipe(2)` could read and write). psh uses
`socketpair(AF_UNIX, SOCK_STREAM)` for the same effect.

Coprocesses are byte-stream only. Typed coprocess channels
(session-typed when the child is a pane service) are deferred
to the pane integration phase.

### Here-string

    '<<<' word

Feed a word to stdin without a heredoc or a fork. `cat <<<hello`
is equivalent to `echo hello | cat` but avoids the fork.

    cat <<<$var
    cmd <<<'literal text'

psh extension (ksh93 heritage). rc had no equivalent.


## Redirections

    redirect    = output_redir | input_redir | dup_redir
                | close_redir | heredoc | herestring

    output_redir = '>' target
                 | '>>' target
                 | '>[' NUM ']' target
                 | '>>[' NUM ']' target

    input_redir  = '<' target
                 | '<>' target
                 | '<[' NUM ']' target

    dup_redir    = '>[' NUM '=' NUM ']'
                 | '<[' NUM '=' NUM ']'

    close_redir  = '>[' NUM '=' ']'
                 | '<[' NUM '=' ']'

    heredoc      = '<<' WORD                    -- expanding
                 | '<<' QUOTED                  -- literal (no expansion)

    herestring   = '<<<' word

    target       = word

Redirections are profunctor transformations wrapping inner
expressions. Left-to-right evaluation order is structural
(inner-to-outer nesting in the AST). The save/restore
discipline preserves fds across redirections.

rc reference: rc(1) §I/O Redirections.

**`>` `>>` `<`** — standard redirections. `>` truncates,
`>>` appends, `<` reads. Default fd is 1 for output, 0 for
input.

**`>[fd]`** — redirect a specific fd. `echo error >[2] /tmp/log`
redirects fd 2 to a file. rc heritage.

**`>[fd=fd]`** — dup. `cmd >[2=1]` dups fd 1 onto fd 2
(stderr → stdout). Evaluation order matters:
`cmd >file >[2=1]` redirects stdout to file, then stderr to
the now-redirected stdout. rc heritage.

**`>[fd=]`** — close. `cmd >[2=]` closes fd 2. rc heritage.

**`<>` — read-write open.** `cmd <> file` opens `file` for
both reading and writing on fd 0. Useful for lock files and
bidirectional device access. rc heritage (Plan 9's
`open(file, ORDWR)`).

**`<<` — here-document.** Unquoted delimiter: variable
expansion (`$var`) in the body. Quoted delimiter (`<<'EOF'`):
no expansion, literal text. Content is everything between the
delimiter line and the terminating delimiter.

    cat <<EOF
    hello $user
    EOF

    cat <<'EOF'
    literal $user (not expanded)
    EOF

rc heritage. Unquoted heredocs expand `$var` references in
the body at evaluation time. Quoted heredocs pass content
through unchanged.

**`<<<` — here-string.** See §Here-string above.

### Missing rc I/O features

**`|[fd]` (decorated pipes):** Omitted. psh's profunctor
wrapping model handles this compositionally:
`cmd >[2=1] | cmd2` pipes stderr through stdout. Decorated
pipes are a shorthand that rc needed because it lacked the
wrapping model.

**`>{cmd}` (output process substitution):** Planned. The
dual of `<{cmd}`. `<{cmd}` gives a fd to read from;
`>{cmd}` gives a fd to write to. Without it, patterns like
`tee >(grep err > errors.log)` require named pipes.


## `whatis` output format

`whatis` reports what a name is — variable, function,
builtin, or external command. With the typed value model,
`whatis` shows type information for `let`-bound variables.

    whatis x           x : Int = 42
    whatis y           y : List[Str] = (foo bar baz)
    whatis z           z = hello        # bare assignment, no type shown
    whatis result      result : Result[Int] = ok 42
    whatis cursor      cursor : try Result[Int]   # computed, no stored value
    whatis cd          builtin cd
    whatis ls          /usr/bin/ls
    whatis greet       fn greet { ... }
    whatis x.get       fn x.get { ... }

The rule: `let`-bound variables show `: Type`. Bare
assignments (rc heritage) show just the value — the user
chose the untyped path. Computed variables (`let x = try {}`)
show the type signature but do not force evaluation.
Functions, builtins, and external commands follow rc's
`whatis` convention.


## Keywords

    if else for in while match fn ref let try return mut export

rc's keywords: `for in while if not switch fn ~ ! @`.

psh drops `not` (replaced by `else`), `switch` (replaced by
`match`), and `case` (arms in `match` are bare patterns, no
sub-keyword needed). Replaces bare `@` with `@{`. Adds `let`,
`ref`, `else`, `match`, `try`, `return`, `mut`, `export`.
Treats `~` as a builtin command with parse-position dispatch,
not as a keyword (see §Tilde expansion).

`!` and `@` are prefix operators, not keywords. `!` negates
a command's exit status (`! cmd` succeeds if `cmd` fails).
`@` introduces a subshell (`@{ cmds }`). rc listed both as
keywords; psh classifies them as operators because they do not
introduce new binding or control forms — they modify an
existing command expression.


## References

1. Tom Duff. "Rc — The Plan 9 Shell." 1990.
   `reference/plan9/papers/rc.ms`

2. rc(1) man page, Plan 9 4th edition.
   `reference/plan9/man/1/rc`

3. psh specification. `docs/specification.md`

4. Clarke, Elgot, Gibbons, Sherwood-Taylor, Wu. "Profunctor
   Optics, a Categorical Update." Compositionality, 2024.

5. Levy, Paul Blain. *Call-by-Push-Value.* Springer, 2004.

6. Curien, Pierre-Louis and Herbelin, Hugo. "The duality of
   computation." ICFP, 2000.
