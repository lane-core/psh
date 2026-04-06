//! AST for psh.
//!
//! Three node families corresponding to the three syntactic sorts
//! of the linear classical L-calculus:
//!
//! - Values (positive, CBV): literals, variable references, command
//!   substitutions. Evaluated eagerly.
//! - Expressions (profunctor structure): commands with redirections.
//!   Redirections wrap inner expressions (lmap/rmap), not bolted on.
//! - Statements (cuts): bind values to expressions.
//!
//! Redirections compose by nesting. Each RedirectOp describes one fd
//! transformation. Linear fd tracking verifies the composition at
//! parse time.

use std::fmt;

/// A word in the shell — the smallest unit of value.
///
/// Words are positive (CBV): fully evaluated before the command
/// that consumes them runs.
#[derive(Debug, Clone, PartialEq)]
pub enum Word {
    /// Literal string: hello, 'quoted string'
    Literal(String),
    /// Variable reference: $x, $x(2)
    Var(String),
    /// Indexed variable: $x(n)
    Index(String, Box<Word>),
    /// Count: $#x
    Count(String),
    /// Command substitution: `{ cmd }
    /// Evaluated eagerly — the command runs and its stdout
    /// becomes the value.
    CommandSub(Vec<Statement>),
    /// Concatenation: a^b — juxtaposition of words
    Concat(Vec<Word>),
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
    /// Here-document with delimiter.
    HereDoc(String),
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
    Block(Vec<Statement>),
    /// Subshell (isolated namespace): @{ cmds }
    Subshell(Vec<Statement>),
    /// Coprocess: cmd |&
    /// Bidirectional cut — the shell holds both read and write
    /// endpoints to the child process.
    Coprocess(Box<Expr>),
}

/// Pattern for switch/case and ~ matching.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Literal pattern
    Literal(String),
    /// Glob pattern: * ? [chars]
    Glob(String),
    /// Match any
    Star,
}

/// A statement — binds values to expressions (the "cut" level).
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Variable assignment: x = value
    Assignment(String, Value),
    /// Execute an expression
    Exec(Expr),
    /// if(expr) { body } [else { body }]
    If {
        condition: Expr,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },
    /// for(x in list) { body }
    For {
        var: String,
        list: Value,
        body: Vec<Statement>,
    },
    /// switch(value) { case pat { body } ... }
    Switch {
        value: Value,
        cases: Vec<(Vec<Pattern>, Vec<Statement>)>,
    },
    /// fn name { body }
    /// Also handles discipline functions: fn x.get { body }
    Fn { name: String, body: Vec<Statement> },
    /// Return from a function (implicit via last status,
    /// but explicit return is useful).
    Return(Option<Value>),
}

/// A complete psh program — a sequence of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Word::Literal(s) => write!(f, "{s}"),
            Word::Var(name) => write!(f, "${name}"),
            Word::Index(name, idx) => write!(f, "${name}({idx})"),
            Word::Count(name) => write!(f, "$#{name}"),
            Word::CommandSub(_) => write!(f, "`{{...}}"),
            Word::Concat(parts) => {
                for part in parts {
                    write!(f, "{part}")?;
                }
                Ok(())
            }
        }
    }
}
