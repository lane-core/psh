# Parser — grammar as code

The parser is a transliteration of 04-syntax.md into combine
combinators. Each grammar production maps to a named parser
function. When the grammar changes, the parser changes to
match. The parser produces an unannotated AST (`ty: None` on
all nodes); the checker fills in types.

Spec correspondence: 04-syntax.md is the implementer's
contract. The parser is correct when it accepts exactly the
language defined there.

### Existing infrastructure (parse.rs)

The current `parse.rs` retains from the prior implementation:

- **Character predicates:** `is_var_char`, `is_word_char`,
  `can_start_atom` — these are correct per the spec's §Two
  character sets and do not need rewriting.
- **Trivia handling:** `hspace`, `comment`, `line_cont`,
  `trivia`, `full_trivia` — whitespace and comment consumption.
- **Keyword/name primitives:** `keyword`, `varname`, `wname` —
  lexical building blocks.

The grammar productions were retired. The next implementation
rebuilds them on top of this lexical layer.

### Six-layer architecture

The parser is organized in six layers matching the grammar's
precedence and nesting structure. Each layer is a module or
section within parse.rs, calling down to lower layers.

```
Layer 6: program     — command sequences (top-level)
Layer 5: command     — bindings, control flow, expr_cmd
Layer 4: expr        — pipelines, &&/||, &, redirections
Layer 3: simple_cmd  — WORD+ with cmd_prefix
Layer 2: word        — coalesce, concat, atoms
Layer 1: lexical     — char predicates, trivia, keywords (exists)
```

**Layer 1 (lexical)** exists. Layers 2-6 are rebuilt in
dependency order: 2 first (words are leaves), then 3
(simple commands consume words), then 4 (expressions compose
commands), then 5 (commands include control flow with nested
expressions), then 6 (programs sequence commands).

### Layer 2: Words

Words are the term-level parser — they produce `Term` nodes.
This is the most complex layer because terms have the richest
syntax (variables, accessors, interpolation, substitution,
literals, lambdas, tagged construction, arithmetic).

```rust
fn word<I>(sigma: &Sigma) -> impl Parser<I, Output = Term>
where I: Stream<Token = char>
{
    // word = coalesce_expr ('^' coalesce_expr)*
    // coalesce_expr = word_atom ('??' word_atom)*
    // word_atom = var_ref | literal | quoted | cmd_sub | proc_sub
    //           | list | tuple | map_lit | struct_lit
    //           | tagged | lambda | arith | path_literal
}
```

Key parsing decisions at this layer:

- **Free caret rule.** Adjacent atoms with no whitespace
  concatenate: `$stem^.c` produces `Concat`. Whitespace
  between atoms produces separate arguments. The `can_start_atom`
  predicate determines adjacency.
- **Tagged construction.** `NAME(` with no space commits to
  `Tagged`. The parser checks `sigma.constructors` for arity
  to determine if arguments are expected.
- **Struct vs map literal.** Non-empty braces: first key-value
  pair's delimiter disambiguates (`:` = map, `=` = struct).
  Empty `{}` produces an ambiguous node resolved by the checker.
- **Lambda.** `|params| => body` or `|params| { block }`. The
  `|` is unambiguous in value position (in command position
  it's a pipe).

### Layer 3: Simple commands

```rust
fn simple_cmd<I>(sigma: &Sigma) -> impl Parser<I, Output = Command>
where I: Stream<Token = char>
{
    // simple_cmd = cmd_prefix* WORD+
    // cmd_prefix = NAME '=' value
}
```

A simple command is one or more words. The first word becomes
the command name; the rest are arguments. Per-command variable
assignments (`NAME=value`) precede the command words.

### Layer 4: Expressions

```rust
fn expr_cmd<I>(sigma: &Sigma) -> impl Parser<I, Output = Expr>
where I: Stream<Token = char>
{
    // expr_cmd = or_expr ('&' '!'?)?
    // or_expr  = and_expr ('||' and_expr)*
    // and_expr = match_expr ('&&' match_expr)*
    // match_expr = pipeline ('=~' value)?
    // pipeline = cmd_expr (pipe_op cmd_expr)*
    // cmd_expr = '!' cmd_expr | body | '@' body | simple_cmd redirect*
}
```

Operator precedence is encoded in the call chain: `expr_cmd`
calls `or_expr`, which calls `and_expr`, which calls
`match_expr`, which calls `pipeline`, which calls `cmd_expr`.
Each level handles one precedence tier.

Redirections attach at the `cmd_expr` level — they bind
tighter than pipes but looser than word-level syntax.

### Layer 5: Commands

```rust
fn command<I>(sigma: &Sigma) -> impl Parser<I, Output = Command>
where I: Stream<Token = char>
{
    // command = binding | control | return_cmd | exit_cmd
    //        | set_cmd | expr_cmd
}
```

The command parser tries each alternative in order. Bindings
and control flow are recognized by leading keyword (`let`,
`def`, `if`, `for`, `while`, `loop`, `match`, `try`, `trap`).
Anything else falls through to `expr_cmd`.

Control flow productions contain nested `body` (a brace-
delimited program) and `expr` nodes — mutual recursion between
layers 4 and 5. combine handles this via `parser!` macro or
explicit `lazy` combinators.

### Layer 6: Program

```rust
fn program<I>(sigma: &Sigma) -> impl Parser<I, Output = Program>
where I: Stream<Token = char>
{
    // program = terminator* (command terminator+)* command?
    // terminator = '\n' | ';'
}
```

Top-level: a sequence of commands separated by terminators.
`full_trivia` (whitespace + comments + line continuations)
consumed between tokens.

### Parser ↔ Σ interface

The parser takes `&Sigma` as a parameter threaded through all
layers. It queries Σ for exactly two things (as specified in
Layer 1):

1. **Constructor arity.** When the parser sees `NAME(`, it
   checks `sigma.constructors` to determine if this is a
   tagged construction and whether the constructor is nullary.
   This is the only point where parsing is not fully context-
   free — it depends on declared types. The alternative
   (context-free parsing with post-hoc resolution) would
   produce ambiguous AST nodes that the checker must
   disambiguate, which is viable but defers work.

2. **Name classification.** Uppercase bare name — is it a type
   name (for `Type::name()` or struct literals) or a
   constructor (for bare nullary variants)? Query:
   `sigma.is_type(name)`.

### Error recovery

combine's error handling produces error messages with spans.
The parser does not attempt error recovery in the initial
implementation — a parse error aborts with a diagnostic. Error
recovery (inserting missing delimiters, skipping to the next
statement) is a future ergonomic improvement that does not
affect correctness.

### Testing strategy

Parser tests are golden-file tests: input source → expected
AST (serialized as S-expressions or debug format). Each
grammar production has at least one positive test and one
negative test. The test corpus grows alongside the grammar.

```
tests/
    parse/
        word.rs          — word-level parsing
        command.rs       — command-level parsing
        expr.rs          — expression-level parsing
        program.rs       — full program parsing
        golden/          — golden file fixtures
```


