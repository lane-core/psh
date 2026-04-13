# AST — three-sort syntax tree

The AST has three sorts matching the λμμ̃ categories [CH00],
[BTMO23]. The sort boundary is enforced by the Rust type
system: `Term`, `Command`, and `Expr` are separate enums.
A `Command` cannot appear where a `Term` is expected. The
evaluator has three entry points that mirror the three sorts.

Spec correspondence: 02-calculus.md §The three sorts defines
the categorical structure. 04-syntax.md defines the grammar
productions that map to AST nodes.

### Common infrastructure

```rust
/// Source location for error reporting.
#[derive(Clone, Copy)]
struct Span {
    start: u32,
    end: u32,
}

/// Every AST node carries a span and an optional resolved type.
/// The type is None after parsing, filled in by the checker.
#[derive(Clone)]
struct Ann {
    span: Span,
    ty: Option<TypeRef>,
}

/// Interned name with span — used for variables, fields,
/// types, builtins.
#[derive(Clone)]
struct Ident {
    name: Name,
    span: Span,
}
```

### Term (producers — Γ)

Terms are values: evaluated eagerly (CBV) by `eval_term`,
producing a `Val`. They inhabit the context Γ.

```rust
/// Producer sort. Evaluated by eval_term → Val.
enum Term {
    /// 'hello', 42, true, /usr/bin
    Literal(Ann, Val),

    /// $x — project from Γ
    Var(Ann, Ident),

    /// $x[i] — bracket access (runtime index)
    Index(Ann, Box<Term>, Box<Term>),

    /// $x.name — dot access (static name)
    Access(Ann, Box<Term>, Ident),

    /// $x[a..b] — slice
    Slice(Ann, Box<Term>, Box<Term>, Box<Term>),

    /// `{cmd} — command substitution (shift ↓→↑)
    CmdSub(Ann, Box<Expr>),

    /// <{cmd} — process substitution (downshift into namespace)
    ProcSub(Ann, Box<Expr>),

    /// %N — job ID (expands to PID of background job N)
    JobId(Ann, u32),

    /// (a b c) — list construction
    List(Ann, Vec<Term>),

    /// (a, b) — tuple construction
    Tuple(Ann, Vec<Term>),

    /// {'k': v, ...} — map literal
    MapLit(Ann, Vec<(Term, Term)>),

    /// Type { f = v; ... } or Type { v, v } — struct construction
    StructLit(Ann, Ident, StructFields),

    /// ok(42), none, inl(v) — tagged construction
    Tagged(Ann, Option<Ident>, Ident, Option<Box<Term>>),
    //          ^type qualifier  ^variant   ^payload

    /// $a^$b — caret concatenation
    Concat(Ann, Vec<Term>),

    /// "hello $name" — interpolated string
    Interp(Ann, Vec<StringPart>),

    /// $((...)) — arithmetic expression
    Arith(Ann, Box<ArithExpr>),

    /// |x| => body or |x| { block } — lambda
    Lambda(Ann, Vec<Pat>, Box<LambdaBody>),

    /// $a ?? default — nil coalescing
    Coalesce(Ann, Box<Term>, Box<Term>),
}

enum StructFields {
    Named(Vec<(Ident, Term)>),              // { x = 10; y = 20 }
    Positional(Vec<Term>),                  // { 10, 20 }
}

enum StringPart {
    Lit(SmolStr),
    Var(Ident),
    Expr(Box<Term>),
}

enum LambdaBody {
    Expr(Expr),                             // => command
    Block(Program),                         // { program }
}
```

### Command (consumers — Δ)

Commands are consumers: they describe what expects to receive
values and what it does with them. Evaluated by `run_cmd`.

```rust
/// Consumer sort. Dispatched by run_cmd.
enum Command {
    /// name = value — assignment (μ̃-binder)
    Assign(Ann, Ident, Term),

    /// let pat = expr — binding (μ̃-binder with pattern)
    Let(Ann, LetFlags, Pat, Option<TypeAnn>, Term),

    /// let pat = expr else { diverge } — refutable binding
    LetElse(Ann, Pat, Option<TypeAnn>, Term, Box<Program>),

    /// def name(params) : type { body } — computation binding (Θ)
    Def(Ann, DefTarget, Vec<Ident>, Option<TypeAnn>, Program),

    /// ref name = target — nameref alias
    Ref(Ann, Ident, Ident),

    /// if(cond) { body } else { ... }
    If(Ann, Box<Expr>, Program, Option<ElseBranch>),

    /// if let pat = expr { body } else { body }
    IfLet(Ann, Pat, Term, Program, Option<Program>),

    /// for(x in list) { body }
    For(Ann, Ident, Term, Program),

    /// while(cond) { body }
    While(Ann, Box<Expr>, Program),

    /// loop { body }
    Loop(Ann, Program),

    /// match(val) { arms }
    Match(Ann, Term, Vec<MatchArm>),

    /// try { body } catch(e) { handler }
    Try(Ann, Program, Ident, Program),

    /// trap SIGNAL { handler } { body } | trap SIGNAL { handler } | trap SIGNAL
    Trap(Ann, Ident, Option<Program>, Option<Program>),

    /// return value?
    Return(Ann, Option<Term>),

    /// exit code? message?
    Exit(Ann, Option<Term>, Option<Term>),

    /// set -o name / +o name / -x / etc.
    Set(Ann, Vec<SetFlag>),

    /// A bare expression in command position
    Expr(Ann, Expr),
}

/// def target: variable discipline or type method
enum DefTarget {
    Plain(Ident),                           // def name { }
    Discipline(Ident, Ident),               // def var.method { }
    TypeMethod(Ident, Ident),               // def Type::method { }
}

enum ElseBranch {
    ElseIf(Box<Command>),                   // else if ...
    Else(Program),                          // else { ... }
}

struct MatchArm {
    patterns: Vec<Pattern>,                 // space-separated multi-pattern
    guard: Option<Box<Expr>>,               // if(expr)
    body: LambdaBody,
}

struct LetFlags {
    export: bool,
    mutable: bool,                          // let mut
    classical: bool,                        // let !x (explicit ! promotion)
}
```

### Expr (cuts — ⟨t | e⟩)

Expressions are where producers meet consumers: pipelines,
redirections, backgrounding, boolean operators. The profunctor
layer. Evaluated by `run_expr`.

```rust
/// Cut sort. Executed by run_expr → Status.
enum Expr {
    /// command — a bare command (the trivial cut)
    Cmd(Ann, Box<Command>),

    /// cmd1 | cmd2 — pipeline (cut via pipe fd)
    Pipe(Ann, Vec<PipeStage>),

    /// cmd1 && cmd2
    And(Ann, Box<Expr>, Box<Expr>),

    /// cmd1 || cmd2
    Or(Ann, Box<Expr>, Box<Expr>),

    /// !cmd — negation
    Not(Ann, Box<Expr>),

    /// cmd =~ pattern — pattern match test
    MatchTest(Ann, Box<Expr>, Term),

    /// cmd & — background
    Background(Ann, Box<Expr>),

    /// cmd &! — background + disown
    BackgroundDisown(Ann, Box<Expr>),

    /// @{ program } — subshell
    Subshell(Ann, Program),

    /// cmd with redirections
    Redirected(Ann, Box<Expr>, Vec<Redirect>),
}

struct PipeStage {
    expr: Expr,
    pipe_op: PipeOp,
}

enum PipeOp {
    Stdout,                                 // |
    Fd(u32),                                // |[N]
    FdToFd(u32, u32),                       // |[N=M]
    Typed(TypeRef),                         // |[T]
    FdTyped(u32, TypeRef),                  // |[N:T]
    Coproc(Option<Ident>),                  // |& or |& name
}

enum Redirect {
    Output(Ann, Term),                      // >file
    Append(Ann, Term),                      // >>file
    NoClobber(Ann, Term),                   // >|file
    Input(Ann, Term),                       // <file
    FdOutput(Ann, u32, Term),               // >[N]file
    FdInput(Ann, u32, Term),                // <[N]file
    HereDoc(Ann, HereDoc),                  // <<MARKER
    HereStr(Ann, Term),                     // <<<word
    Dup(Ann, u32, u32),                     // >[N=M]
    Close(Ann, u32),                        // >[N=]
}

struct HereDoc {
    marker: SmolStr,
    body: Vec<StringPart>,                  // expanded or literal
    strip_tabs: bool,                       // <<- form
    target_fd: Option<u32>,                 // <<[N] form
}
```

### Pattern

Patterns appear in `let`, `let-else`, `if let`, `match`, and
`for`. Two pattern grammars exist in the spec (04-syntax.md):
`pat` for bindings (no globs) and `pattern` for match arms
(includes globs). Both map to one Pattern enum with a
context flag.

```rust
enum Pattern {
    /// _ — wildcard
    Wildcard(Span),

    /// x — variable binding
    Bind(Span, Ident),

    /// 42, 'hello' — literal match
    Literal(Span, Val),

    /// *.txt, [a-z]* — glob (match arms only)
    Glob(Span, SmolStr),

    /// ok(pat), inl(pat) — tagged destructure
    Tagged(Span, Option<Ident>, Ident, Vec<Pattern>),

    /// (pat, pat) — tuple destructure
    TuplePat(Span, Vec<Pattern>),

    /// () — nil list, (head tail) — cons
    ListPat(Span, Option<(Box<Pattern>, Box<Pattern>)>),

    /// Type { field = pat; ... } — struct destructure
    StructPat(Span, Ident, Vec<(Ident, Pattern)>),
}
```

### Program

The top-level sequence — a list of commands separated by
terminators.

```rust
struct Program {
    commands: Vec<Command>,
    span: Span,
}
```

### AST traits

```rust
/// Every AST node exposes its annotation.
trait AstNode {
    fn ann(&self) -> &Ann;
    fn span(&self) -> Span { self.ann().span }
    fn ty(&self) -> Option<&TypeRef> { self.ann().ty.as_ref() }
}

/// Sort-tagged trait for the three evaluator entry points.
trait Eval {
    type Output;
}

impl Eval for Term    { type Output = Val; }
impl Eval for Command { type Output = (); }
impl Eval for Expr    { type Output = Status; }
```

The `Eval` trait does not define an `eval` method — that lives
on the evaluator (Layer 6), which holds the mutable state
(environment, fds, job table). The trait exists to document the
sort→output mapping at the type level.

### Parser ↔ AST interface

The parser produces `Program` (a `Vec<Command>`). Every node
carries `Ann` with span but `ty: None`. The checker walks the
tree, fills in type annotations, and reports errors. The
evaluator consumes the annotated tree.

### Checker ↔ AST interface

The checker reads the AST and Σ. For each node it:
1. Determines the mode (synth or check) from context
2. Queries Σ for type information
3. Fills in `ann.ty` on the node
4. Reports type errors with spans

The checker does not transform the AST — it annotates in
place. No separate "typed AST" representation. This keeps the
data structure count low and avoids a translation pass.


