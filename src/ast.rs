//! AST for psh.
//!
//! Four node families reflecting the sequent calculus structure
//! (λμμ̃-calculus, Curien-Herbelin 2000):
//!
//! - **Word/Value** (producers, positive, CBV): literals, variable
//!   references, command substitutions. Evaluated eagerly.
//! - **Expr** (the profunctor layer): commands with redirections.
//!   Redirections wrap inner expressions (lmap/rmap), not bolted on.
//! - **Binding** (μ̃-binders): extend the context Γ with a new name.
//!   Assignment and function definition.
//! - **Command** (cuts and control): connect producers to consumers
//!   (Exec), or branch/iterate over values (If, For, Match).
//!
//! Redirections compose by nesting. Each RedirectOp describes one fd
//! transformation. Linear fd tracking verifies the composition at
//! parse time.

use std::fmt;

/// Accessor — projects into structured values.
///
/// Accessors chain left-to-right after a variable reference:
/// `$x.0` (tuple projection), `$x.ok` (Prism preview into Sum),
/// `$x.code` (ExitCode extraction). Prism miss → Unit (absorbing
/// element). See syntax.md §Accessors.
#[derive(Debug, Clone, PartialEq)]
pub enum Accessor {
    /// Tuple projection: .0, .1, ... (0-based)
    Index(usize),
    /// Sum tag decomposition (Prism preview): .ok, .err, .name
    Tag(String),
    /// ExitCode extraction: .code → Int
    Code,
}

/// A word in the shell — the smallest unit of value.
///
/// Words are positive (CBV): fully evaluated before the command
/// that consumes them runs.
#[derive(Debug, Clone, PartialEq)]
pub enum Word {
    /// Literal string: hello, 'quoted string'
    Literal(String),
    /// Quoted string: 'text' — always produces Val::Str, never
    /// type-inferred. Distinguishes '42' (Str) from 42 (Int in
    /// let context).
    Quoted(String),
    /// Variable reference: $x
    Var(String),
    /// Variable with accessor chain: $x.0, $x.ok, $x.code, $x.ok.name
    /// Produced when the parser finds `.` immediately after a varname.
    /// Accessor takes priority over free carets.
    VarAccess(String, Vec<Accessor>),
    /// Indexed variable: $x(n)
    Index(String, Box<Word>),
    /// Count: $#x
    Count(String),
    /// Stringify: $"x — join list elements with spaces into a
    /// single string. rc heritage (Duff 1990, §Concatenation).
    Stringify(String),
    /// Brace-delimited variable: ${name} — uses word_char alphabet.
    BraceVar(String),
    /// Command substitution: `{ cmd }
    /// Evaluated eagerly — the command runs and its stdout
    /// becomes the value.
    CommandSub(Vec<Command>),
    /// Input process substitution: <{cmd}
    /// Evaluates to /dev/fd/N, where N is the read end of a
    /// pipe connected to the command's stdout.
    ProcessSub(Vec<Command>),
    /// Output process substitution: >{cmd}
    /// Evaluates to /dev/fd/N, where N is the write end of a
    /// pipe connected to the command's stdin.
    OutputProcessSub(Vec<Command>),
    /// Concatenation: a^b — juxtaposition of words
    Concat(Vec<Word>),
    /// Tilde: ~ expands to $home
    Tilde,
    /// Tilde path: ~/path expands to $home/path
    TildePath(String),
}

/// A list value — rc's first-class lists.
///
/// A scalar is a one-element list. The empty list is false.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Single word
    Word(Word),
    /// List literal: (a b c)
    List(Vec<Word>),
    /// Lambda: \params => body — value-level thunk literal.
    /// CBPV's U(A → F(B)). Named params, capture-by-value for
    /// free variables (enables currying).
    Lambda {
        params: Vec<String>,
        body: Vec<Command>,
    },
    /// try { body } in value position — fallible capture.
    /// Forks, captures stdout + exit status via capture_subprocess.
    /// Returns Sum("ok", captured_val) or Sum("err", ExitCode(n)).
    /// CBV: body evaluates immediately at binding time.
    Try(Vec<Command>),
    /// Value-producing block — U(F(A)) in CBPV. A computation that
    /// may produce a value via `return`. Runs commands, extracts
    /// RunOutcome::Value; defaults to Unit if no return issued.
    /// Used for if/match/while/{ } in value position (let RHS).
    Compute(Vec<Command>),
    /// Tagged value: tag payload — Sum construction in value position.
    Tagged(String, Box<Value>),
}

/// A simple command: name and arguments, all words.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleCommand {
    /// The command name (first word).
    pub name: Word,
    /// Arguments (remaining words).
    pub args: Vec<Word>,
    /// Assignments that precede the command: x=val cmd
    pub assignments: Vec<(String, Value)>,
}

/// File descriptor redirection target.
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectTarget {
    /// Redirect to/from a file path.
    File(Word),
    /// Here-document: body text + whether to expand $var references.
    /// expand=true for unquoted delimiter (<<EOF), false for quoted (<<'EOF').
    HereDoc { body: String, expand: bool },
    /// Here-string.
    HereString(Word),
}

/// A single fd operation.
///
/// In the profunctor reading:
/// - Output = rmap (post-compose on the output continuation)
/// - Input = lmap (pre-compose on the input source)
/// - Dup = contraction (merge two continuations)
/// - Close = weakening (discard a continuation)
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectOp {
    /// >[fd] target  or  >target (fd defaults to 1)
    Output {
        fd: u32,
        target: RedirectTarget,
        append: bool,
    },
    /// <[fd] target  or  <target (fd defaults to 0)
    Input { fd: u32, target: RedirectTarget },
    /// <>target — read-write open on fd 0
    ReadWrite { fd: u32, target: RedirectTarget },
    /// >[dst=src] — dup src onto dst
    Dup { dst: u32, src: u32 },
    /// >[fd=] — close fd
    Close { fd: u32 },
}

/// An expression — a command or composition of commands.
///
/// Redirections are not properties of commands. They are
/// transformations that wrap inner expressions, making the
/// profunctor structure explicit in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A simple command: name args...
    Simple(SimpleCommand),
    /// A redirection wrapping an inner expression.
    /// Applied inner-to-outer (left-to-right in source).
    Redirect(Box<Expr>, RedirectOp),
    /// Pipeline: cmd1 | cmd2 | cmd3
    /// A sequence of cuts — each | connects stdout of the left
    /// to stdin of the right. Pipe endpoints are linear.
    Pipeline(Vec<Expr>),
    /// Short-circuit and: cmd1 && cmd2
    And(Box<Expr>, Box<Expr>),
    /// Short-circuit or: cmd1 || cmd2
    Or(Box<Expr>, Box<Expr>),
    /// Negation: ! cmd
    Not(Box<Expr>),
    /// Background: cmd &
    /// Concurrent fork — the cut runs in parallel with the
    /// shell's continuation.
    Background(Box<Expr>),
    /// Block: { cmds }
    Block(Vec<Command>),
    /// Subshell (isolated namespace): @{ cmds }
    Subshell(Vec<Command>),
    /// Coprocess: cmd |&
    /// Bidirectional cut — the shell holds both read and write
    /// endpoints to the child process.
    Coprocess(Box<Expr>),
    /// Pattern match: expr =~ value
    /// Infix glob match — returns success if the expr's string
    /// representation matches any pattern in the value. Shares
    /// glob_match() with match arm evaluation.
    PatternMatch { expr: Box<Expr>, patterns: Value },
}

/// Pattern for match arms and ~ matching.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Literal pattern
    Literal(String),
    /// Glob pattern: * ? [chars]
    Glob(String),
    /// Match any
    Star,
    /// Structural pattern: tag name — coproduct elimination.
    /// The tag is matched against Sum's tag; the binding receives
    /// the payload. Distinguished by word count: two bare words
    /// before => is structural, one word or parenthesized list is glob.
    Structural { tag: String, binding: String },
}

/// Type annotation for let bindings.
///
/// Validates incoming values at the binding site (Prism check).
/// Coercion policy: widening (Int→Str, Bool→Str, Path→Str) is
/// allowed; narrowing (Str→Int) is rejected.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Unit,
    Bool,
    Int,
    Str,
    Path,
    /// Reified computation outcome. Matches Val::ExitCode.
    ExitCode,
    /// List or List[ElementType]. None means untyped elements.
    List(Option<Box<TypeAnnotation>>),
    /// Product type — componentwise validation.
    Tuple(Vec<TypeAnnotation>),
    /// Union type — value matches if it passes any branch.
    /// Used for Sum validation: constrains which tags are legal.
    Union(Vec<TypeAnnotation>),
    /// Sugar for `T | ExitCode` with implied tags ok/err.
    /// Result[T] matches Sum("ok", T) or Sum("err", ExitCode).
    Result(Box<TypeAnnotation>),
    /// Sugar for `T | Unit` with implied tags ok/none.
    /// Maybe[T] matches Sum("ok", T) or Sum("none", Unit).
    Maybe(Box<TypeAnnotation>),
    /// Function type — A -> B. Param is the input type (or Tuple
    /// for multi-param), ret is the output type. Right-associative:
    /// A -> B -> C = A -> (B -> C).
    Fn(Box<TypeAnnotation>, Box<TypeAnnotation>),
}

/// A binding — extends the context Γ with a new name.
///
/// μ̃-binders in the sequent calculus: `x = val` binds a value
/// to a variable name (let-binding). `fn name { body }` binds
/// a computation to a function name (including discipline
/// functions like x.get, x.set).
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    /// Variable assignment: x = value
    /// μ̃-binding: evaluate value (CBV), then extend Γ.
    /// rc heritage: walks scope chain, mutable, no type check.
    Assignment(String, Value),
    /// let binding: let [mut] [export] name [: Type] = value
    /// Always creates in current scope, never walks up.
    /// Immutable by default (use `mut` for mutable).
    Let {
        name: String,
        value: Value,
        mutable: bool,
        export: bool,
        type_ann: Option<TypeAnnotation>,
    },
    /// Function definition: fn name { body } or fn name(params) { body }
    /// Without params: positional ($1, $2, $*). With params: named binding.
    /// Also handles discipline functions: fn x.get { }, fn x.set(val) { }
    Fn {
        name: String,
        params: Vec<String>,
        body: Vec<Command>,
    },
    /// Nameref: ref name = target
    /// ksh93 heritage: creates an alias that resolves through the
    /// target variable on every access.
    Ref { name: String, target: String },
}

/// A command — the top-level sequencing unit.
///
/// Commands are either cuts (connecting a producer to a consumer),
/// bindings (extending Γ), or control flow (case analysis on values).
/// The separation from Binding makes the sort boundaries visible:
/// bindings extend the context, cuts consume it.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// A binding — context extension (μ̃).
    Bind(Binding),
    /// Execute an expression — the cut ⟨t | e⟩.
    /// Connects a producer (the command's words) to a consumer
    /// (stdout, a pipe, a redirect target).
    Exec(Expr),
    /// if(expr) { body } [else { body }]
    /// Case elimination on exit status.
    If {
        condition: Expr,
        then_body: Vec<Command>,
        else_body: Option<Vec<Command>>,
    },
    /// for(x in list) { body }
    /// Structural recursion over a list value.
    For {
        var: String,
        list: Value,
        body: Vec<Command>,
    },
    /// match value { pat => body; ... }
    /// Multi-way case elimination — glob patterns and structural
    /// coproduct decomposition. Arms use `=>` to introduce the body
    /// and `;` to separate.
    Match {
        value: Value,
        arms: Vec<(Vec<Pattern>, Vec<Command>)>,
    },
    /// while(expr) { body }
    /// Iterative looping — re-evaluates condition after each body.
    While { condition: Expr, body: Vec<Command> },
    /// try { body } [else name { handler }]
    /// Scoped ⅋ — the ⊕→⅋ converter. Inside try, nonzero Status
    /// aborts to the else clause. Boolean contexts are exempt.
    Try {
        body: Vec<Command>,
        else_var: Option<String>,
        else_body: Option<Vec<Command>>,
    },
    /// return value — injects a value from command sort into value sort.
    /// CBPV's return : A → F(A).
    Return(Option<Value>),
    /// take value — contribute a value to the enclosing for-in-value
    /// accumulator. Raku heritage (gather/take). Does not terminate
    /// the computation — the loop continues. Only valid inside
    /// for-in-value-position; error elsewhere.
    Take(Value),
}

/// A complete psh program — a sequence of commands.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub commands: Vec<Command>,
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Word::Literal(s) => write!(f, "{s}"),
            Word::Quoted(s) => write!(f, "'{s}'"),
            Word::Var(name) => write!(f, "${name}"),
            Word::VarAccess(name, accessors) => {
                write!(f, "${name}")?;
                for acc in accessors {
                    match acc {
                        Accessor::Index(i) => write!(f, ".{i}")?,
                        Accessor::Tag(t) => write!(f, ".{t}")?,
                        Accessor::Code => write!(f, ".code")?,
                    }
                }
                Ok(())
            }
            Word::Index(name, idx) => write!(f, "${name}({idx})"),
            Word::Count(name) => write!(f, "$#{name}"),
            Word::Stringify(name) => write!(f, "$\"{name}"),
            Word::BraceVar(name) => write!(f, "${{{name}}}"),
            Word::CommandSub(_) => write!(f, "`{{...}}"),
            Word::ProcessSub(_) => write!(f, "<{{...}}"),
            Word::OutputProcessSub(_) => write!(f, ">{{...}}"),
            Word::Concat(parts) => {
                for part in parts {
                    write!(f, "{part}")?;
                }
                Ok(())
            }
            Word::Tilde => write!(f, "~"),
            Word::TildePath(path) => write!(f, "~/{path}"),
        }
    }
}
