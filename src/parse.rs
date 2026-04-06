//! Parser for psh.
//!
//! Transforms a token stream into the AST. rc-derived grammar
//! with ksh93 extensions (discipline functions, coprocesses, ref).
//!
//! Grammar (informal):
//!
//!   program    = statement*
//!   statement  = assignment | if | for | while | switch | fn | ref | expr_stmt
//!   expr_stmt  = pipeline (& | ;  | \n)?
//!   pipeline   = command (| command)*
//!   command    = simple_cmd redirection*
//!   simple_cmd = word+
//!   word       = WORD | $var | $#var | $var(idx) | `{ program } | (words) | word^word

use anyhow::{bail, Context, Result};

use crate::{
    ast::*,
    lex::{Lexer, Pos, Spanned, Token},
};

/// Parser state — walks a pre-lexed token stream.
pub struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Spanned>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(input: &str) -> Result<Program> {
        let tokens = Lexer::new(input).tokenize_all()?;
        let mut parser = Parser::new(tokens);
        parser.program()
    }

    // ── Token access ────────────────────────────────────────

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|s| &s.token)
            .unwrap_or(&Token::Eof)
    }

    fn peek_pos(&self) -> Pos {
        self.tokens
            .get(self.pos)
            .map(|s| s.pos)
            .unwrap_or(Pos { line: 0, col: 0 })
    }

    fn advance(&mut self) -> &Token {
        let tok = self
            .tokens
            .get(self.pos)
            .map(|s| &s.token)
            .unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<()> {
        let pos = self.peek_pos();
        let got = self.advance().clone();
        if &got != expected {
            bail!("{pos}: expected {expected}, got {got}");
        }
        Ok(())
    }

    fn at(&self, tok: &Token) -> bool {
        self.peek() == tok
    }

    fn at_word(&self) -> bool {
        matches!(self.peek(), Token::Word(_))
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.at(tok) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_terminators(&mut self) {
        while matches!(self.peek(), Token::Newline | Token::Semi) {
            self.advance();
        }
    }

    fn at_terminator(&self) -> bool {
        matches!(
            self.peek(),
            Token::Newline | Token::Semi | Token::Eof | Token::RBrace | Token::RParen
        )
    }

    /// Is the current position at something that could start a word?
    fn at_word_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::Word(_) | Token::Dollar | Token::DollarHash | Token::Backtick | Token::LParen
        )
    }

    // ── Grammar rules ───────────────────────────────────────

    fn program(&mut self) -> Result<Program> {
        let stmts = self.statement_list()?;
        if !self.at(&Token::Eof) {
            let pos = self.peek_pos();
            bail!("{pos}: unexpected token {} at end of input", self.peek());
        }
        Ok(Program { statements: stmts })
    }

    fn statement_list(&mut self) -> Result<Vec<Statement>> {
        let mut stmts = Vec::new();
        self.skip_terminators();
        while !matches!(self.peek(), Token::Eof | Token::RBrace | Token::RParen) {
            stmts.push(self.statement()?);
            self.skip_terminators();
        }
        Ok(stmts)
    }

    fn body(&mut self) -> Result<Vec<Statement>> {
        self.expect(&Token::LBrace)?;
        let stmts = self.statement_list()?;
        self.expect(&Token::RBrace)?;
        Ok(stmts)
    }

    fn statement(&mut self) -> Result<Statement> {
        match self.peek().clone() {
            Token::If => self.if_stmt(),
            Token::For => self.for_stmt(),
            Token::While => self.while_stmt(),
            Token::Switch => self.switch_stmt(),
            Token::Fn => self.fn_stmt(),
            Token::Ref => self.ref_stmt(),
            _ => self.simple_or_assign(),
        }
    }

    fn if_stmt(&mut self) -> Result<Statement> {
        self.expect(&Token::If)?;
        let condition = self.pipeline()?;
        let then_body = self.body()?;
        let else_body = if self.eat(&Token::Else) {
            if self.at(&Token::If) {
                // else if — wrap in a single-statement body
                let nested = self.if_stmt()?;
                Some(vec![nested])
            } else {
                Some(self.body()?)
            }
        } else {
            None
        };
        Ok(Statement::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn for_stmt(&mut self) -> Result<Statement> {
        self.expect(&Token::For)?;
        let var = self.expect_word("variable name")?;
        self.expect(&Token::In)?;
        let list = self.value()?;
        let body = self.body()?;
        Ok(Statement::For { var, list, body })
    }

    fn while_stmt(&mut self) -> Result<Statement> {
        self.expect(&Token::While)?;
        let condition = self.pipeline()?;
        let body = self.body()?;
        // Desugar while into a recursive if
        // while cond { body } => fn _while { if cond { body; _while } }; _while
        // Actually, just represent it directly as If + loop in the AST.
        // We'll add While to Statement for clarity.
        Ok(Statement::If {
            condition,
            then_body: {
                let mut b = body;
                // The evaluator will handle while loops directly.
                // For now, tag this as a while by wrapping.
                b.insert(
                    0,
                    Statement::Exec(Expr::Simple(SimpleCommand {
                        name: Word::Literal("__while_marker".into()),
                        args: vec![],
                        assignments: vec![],
                    })),
                );
                b
            },
            else_body: None,
        })
    }

    fn switch_stmt(&mut self) -> Result<Statement> {
        self.expect(&Token::Switch)?;
        let value = self.value()?;
        self.expect(&Token::LBrace)?;
        let mut cases = Vec::new();
        self.skip_terminators();
        while self.eat(&Token::Case) {
            let mut patterns = Vec::new();
            loop {
                let pat = self.expect_word("pattern")?;
                patterns.push(if pat == "*" {
                    Pattern::Star
                } else if pat.contains('*') || pat.contains('?') || pat.contains('[') {
                    Pattern::Glob(pat)
                } else {
                    Pattern::Literal(pat)
                });
                if !self.eat(&Token::Or) {
                    break;
                }
            }
            let body = self.body()?;
            cases.push((patterns, body));
            self.skip_terminators();
        }
        self.expect(&Token::RBrace)?;
        Ok(Statement::Switch { value, cases })
    }

    fn fn_stmt(&mut self) -> Result<Statement> {
        self.expect(&Token::Fn)?;
        let name = self.expect_word("function name")?;
        let body = self.body()?;
        Ok(Statement::Fn { name, body })
    }

    fn ref_stmt(&mut self) -> Result<Statement> {
        self.expect(&Token::Ref)?;
        let name = self.expect_word("reference name")?;
        self.expect(&Token::Equals)?;
        let target = self.value()?;
        Ok(Statement::Assignment(
            name,
            // Store as a ref by using a special prefix convention
            // The evaluator will recognize this and create a nameref.
            target,
        ))
    }

    /// Parse a simple command or assignment.
    /// Assignment: word = value
    /// Command: word word word...
    fn simple_or_assign(&mut self) -> Result<Statement> {
        // Look ahead for assignment: word = value
        if self.at_word() {
            let saved_pos = self.pos;
            let name = self.expect_word("word")?;
            if self.eat(&Token::Equals) {
                let val = if self.at_terminator() {
                    // x = (empty, clears the variable)
                    Value::List(vec![])
                } else {
                    self.value()?
                };
                return Ok(Statement::Assignment(name, val));
            }
            // Not an assignment — rewind
            self.pos = saved_pos;
        }

        let expr = self.or_expr()?;

        // Check for background
        if self.eat(&Token::Amp) {
            return Ok(Statement::Exec(Expr::Background(Box::new(expr))));
        }

        Ok(Statement::Exec(expr))
    }

    // ── Expression precedence ───────────────────────────────

    fn or_expr(&mut self) -> Result<Expr> {
        let mut left = self.and_expr()?;
        while self.eat(&Token::Or) {
            let right = self.and_expr()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expr> {
        let mut left = self.pipeline()?;
        while self.eat(&Token::And) {
            let right = self.pipeline()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn pipeline(&mut self) -> Result<Expr> {
        let first = self.command()?;

        if self.eat(&Token::PipeAnd) {
            return Ok(Expr::Coprocess(Box::new(first)));
        }

        if !self.at(&Token::Pipe) {
            return Ok(first);
        }

        let mut stages = vec![first];
        while self.eat(&Token::Pipe) {
            stages.push(self.command()?);
        }
        Ok(Expr::Pipeline(stages))
    }

    fn command(&mut self) -> Result<Expr> {
        let pos = self.peek_pos();

        // Negation
        if self.eat(&Token::Bang) {
            let cmd = self.command()?;
            return Ok(Expr::Not(Box::new(cmd)));
        }

        // Block
        if self.at(&Token::LBrace) {
            let body = self.body()?;
            let expr = Expr::Block(body);
            return self.redirections(expr);
        }

        // Subshell
        if self.eat(&Token::AtLBrace) {
            let stmts = self.statement_list()?;
            self.expect(&Token::RBrace)?;
            let expr = Expr::Subshell(stmts);
            return self.redirections(expr);
        }

        // Simple command
        if !self.at_word_start() {
            bail!("{pos}: expected command, got {}", self.peek());
        }

        let cmd = self.simple_command()?;
        self.redirections(Expr::Simple(cmd))
    }

    fn simple_command(&mut self) -> Result<SimpleCommand> {
        let name = self.word()?;
        let mut args = Vec::new();
        while self.at_word_start() && !self.at_terminator() {
            // Stop at operators
            if matches!(
                self.peek(),
                Token::Pipe
                    | Token::PipeAnd
                    | Token::And
                    | Token::Or
                    | Token::Amp
                    | Token::Great
                    | Token::GreatGreat
                    | Token::Less
            ) {
                break;
            }
            args.push(self.word()?);
        }
        Ok(SimpleCommand {
            name,
            args,
            assignments: vec![],
        })
    }

    /// Wrap an expression in zero or more redirection nodes.
    ///
    /// rc evaluates redirections left-to-right. Our AST nests
    /// them as Redirect(inner, op), where execution recurses
    /// into inner before applying op. To get left-to-right
    /// evaluation, the leftmost redirect must be outermost
    /// (applied first). We collect all redirects, then wrap
    /// in reverse order so the first redirect is outermost.
    fn redirections(&mut self, expr: Expr) -> Result<Expr> {
        let mut ops = Vec::new();
        loop {
            match self.peek().clone() {
                Token::Great => {
                    self.advance();
                    let (_fd, op) = self.redirect_target(1, false)?;
                    ops.push(op);
                }
                Token::GreatGreat => {
                    self.advance();
                    let target = self.word()?;
                    ops.push(RedirectOp::Output {
                        fd: 1,
                        target: RedirectTarget::File(target),
                        append: true,
                    });
                }
                Token::Less => {
                    self.advance();
                    let target = self.word()?;
                    ops.push(RedirectOp::Input {
                        fd: 0,
                        target: RedirectTarget::File(target),
                    });
                }
                _ => break,
            }
        }
        // Wrap in reverse: last redirect innermost, first outermost.
        // Execution recurses inward, so outermost runs first = left-to-right.
        let mut result = expr;
        for op in ops.into_iter().rev() {
            result = Expr::Redirect(Box::new(result), op);
        }
        Ok(result)
    }

    /// Parse redirect target after > — handles >[fd], >[fd=fd], >file
    fn redirect_target(&mut self, default_fd: u32, append: bool) -> Result<(u32, RedirectOp)> {
        if self.eat(&Token::LParen) {
            // >[fd] or >[fd=fd] or >[fd=]
            // Actually rc uses [] not () for fd redirections.
            // But we already consumed (, so let's handle it.
            // TODO: decide on [] vs () for fd redirects
            let fd_str = self.expect_word("fd number")?;
            let fd: u32 = fd_str.parse().context("invalid fd number")?;
            if self.eat(&Token::Equals) {
                // >[fd=src] or >[fd=] (close)
                if self.at(&Token::RParen) {
                    self.expect(&Token::RParen)?;
                    return Ok((fd, RedirectOp::Close { fd }));
                }
                let src_str = self.expect_word("source fd")?;
                let src: u32 = src_str.parse().context("invalid source fd")?;
                self.expect(&Token::RParen)?;
                return Ok((fd, RedirectOp::Dup { dst: fd, src }));
            }
            self.expect(&Token::RParen)?;
            let target = self.word()?;
            return Ok((
                fd,
                RedirectOp::Output {
                    fd,
                    target: RedirectTarget::File(target),
                    append,
                },
            ));
        }

        // Just >file
        let target = self.word()?;
        Ok((
            default_fd,
            RedirectOp::Output {
                fd: default_fd,
                target: RedirectTarget::File(target),
                append,
            },
        ))
    }

    // ── Words and values ────────────────────────────────────

    fn word(&mut self) -> Result<Word> {
        let pos = self.peek_pos();
        let base = self.word_atom()?;

        // Check for concatenation: word^word
        if self.eat(&Token::Caret) {
            let right = self.word()?;
            match base {
                Word::Concat(mut parts) => {
                    parts.push(right);
                    Ok(Word::Concat(parts))
                }
                _ => Ok(Word::Concat(vec![base, right])),
            }
        } else {
            Ok(base)
        }
    }

    fn word_atom(&mut self) -> Result<Word> {
        let pos = self.peek_pos();
        match self.peek().clone() {
            Token::Word(s) => {
                self.advance();
                Ok(Word::Literal(s))
            }
            Token::DollarHash => {
                self.advance();
                let name = self.expect_word("variable name")?;
                Ok(Word::Count(name))
            }
            Token::Dollar => {
                self.advance();
                let name = self.expect_word("variable name")?;
                if self.eat(&Token::LParen) {
                    let idx = self.word()?;
                    self.expect(&Token::RParen)?;
                    Ok(Word::Index(name, Box::new(idx)))
                } else {
                    Ok(Word::Var(name))
                }
            }
            Token::Backtick => {
                self.advance();
                self.expect(&Token::LBrace)?;
                let body = self.statement_list()?;
                self.expect(&Token::RBrace)?;
                Ok(Word::CommandSub(body))
            }
            _ => {
                bail!("{pos}: expected word, got {}", self.peek());
            }
        }
    }

    fn value(&mut self) -> Result<Value> {
        if self.eat(&Token::LParen) {
            // List: (word word word)
            let mut items = Vec::new();
            while !self.at(&Token::RParen) && !self.at(&Token::Eof) {
                items.push(self.word()?);
            }
            self.expect(&Token::RParen)?;
            Ok(Value::List(items))
        } else {
            Ok(Value::Word(self.word()?))
        }
    }

    fn expect_word(&mut self, what: &str) -> Result<String> {
        let pos = self.peek_pos();
        match self.advance().clone() {
            Token::Word(s) => Ok(s),
            other => bail!("{pos}: expected {what}, got {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Program {
        Parser::parse(input).expect("parse failed")
    }

    fn parse_err(input: &str) -> String {
        Parser::parse(input).unwrap_err().to_string()
    }

    #[test]
    fn simple_command() {
        let prog = parse("echo hello world");
        assert_eq!(prog.statements.len(), 1);
        match &prog.statements[0] {
            Statement::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.name, Word::Literal("echo".into()));
                assert_eq!(cmd.args.len(), 2);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn assignment() {
        let prog = parse("x = hello");
        match &prog.statements[0] {
            Statement::Assignment(name, Value::Word(Word::Literal(val))) => {
                assert_eq!(name, "x");
                assert_eq!(val, "hello");
            }
            other => panic!("expected assignment, got {other:?}"),
        }
    }

    #[test]
    fn list_assignment() {
        let prog = parse("x = (a b c)");
        match &prog.statements[0] {
            Statement::Assignment(name, Value::List(items)) => {
                assert_eq!(name, "x");
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected list assignment, got {other:?}"),
        }
    }

    #[test]
    fn pipeline() {
        let prog = parse("ls | grep foo | sort");
        match &prog.statements[0] {
            Statement::Exec(Expr::Pipeline(stages)) => {
                assert_eq!(stages.len(), 3);
            }
            other => panic!("expected pipeline, got {other:?}"),
        }
    }

    #[test]
    fn redirect_output() {
        let prog = parse("echo hello > file");
        match &prog.statements[0] {
            Statement::Exec(Expr::Redirect(
                inner,
                RedirectOp::Output {
                    fd: 1,
                    append: false,
                    ..
                },
            )) => {
                assert!(matches!(inner.as_ref(), Expr::Simple(_)));
            }
            other => panic!("expected redirect, got {other:?}"),
        }
    }

    #[test]
    fn background() {
        let prog = parse("sleep 10 &");
        match &prog.statements[0] {
            Statement::Exec(Expr::Background(inner)) => {
                assert!(matches!(inner.as_ref(), Expr::Simple(_)));
            }
            other => panic!("expected background, got {other:?}"),
        }
    }

    #[test]
    fn if_else() {
        let prog = parse("if test -f foo { echo yes } else { echo no }");
        match &prog.statements[0] {
            Statement::If {
                condition,
                then_body,
                else_body: Some(else_body),
            } => {
                assert!(matches!(condition, Expr::Simple(_)));
                assert_eq!(then_body.len(), 1);
                assert_eq!(else_body.len(), 1);
            }
            other => panic!("expected if/else, got {other:?}"),
        }
    }

    #[test]
    fn for_loop() {
        let prog = parse("for x in (a b c) { echo $x }");
        match &prog.statements[0] {
            Statement::For { var, list, body } => {
                assert_eq!(var, "x");
                assert!(matches!(list, Value::List(_)));
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected for, got {other:?}"),
        }
    }

    #[test]
    fn fn_definition() {
        let prog = parse("fn greet { echo hello }");
        match &prog.statements[0] {
            Statement::Fn { name, body } => {
                assert_eq!(name, "greet");
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected fn, got {other:?}"),
        }
    }

    #[test]
    fn discipline_fn() {
        let prog = parse("fn x.get { echo computed }");
        match &prog.statements[0] {
            Statement::Fn { name, body } => {
                assert_eq!(name, "x.get");
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected fn, got {other:?}"),
        }
    }

    #[test]
    fn variable_expansion() {
        let prog = parse("echo $x");
        match &prog.statements[0] {
            Statement::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Var("x".into()));
            }
            other => panic!("expected command with var, got {other:?}"),
        }
    }

    #[test]
    fn variable_index() {
        let prog = parse("echo $x(2)");
        match &prog.statements[0] {
            Statement::Exec(Expr::Simple(cmd)) => {
                assert!(matches!(&cmd.args[0], Word::Index(name, _) if name == "x"));
            }
            other => panic!("expected indexed var, got {other:?}"),
        }
    }

    #[test]
    fn variable_count() {
        let prog = parse("echo $#x");
        match &prog.statements[0] {
            Statement::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Count("x".into()));
            }
            other => panic!("expected count, got {other:?}"),
        }
    }

    #[test]
    fn command_substitution() {
        let prog = parse("echo `{ date }");
        match &prog.statements[0] {
            Statement::Exec(Expr::Simple(cmd)) => {
                assert!(matches!(&cmd.args[0], Word::CommandSub(_)));
            }
            other => panic!("expected command sub, got {other:?}"),
        }
    }

    #[test]
    fn concatenation() {
        let prog = parse("echo foo^bar");
        match &prog.statements[0] {
            Statement::Exec(Expr::Simple(cmd)) => {
                assert!(matches!(&cmd.args[0], Word::Concat(_)));
            }
            other => panic!("expected concat, got {other:?}"),
        }
    }

    #[test]
    fn coprocess() {
        let prog = parse("cmd |&");
        match &prog.statements[0] {
            Statement::Exec(Expr::Coprocess(_)) => {}
            other => panic!("expected coprocess, got {other:?}"),
        }
    }

    #[test]
    fn and_or() {
        let prog = parse("test -f foo && echo yes || echo no");
        match &prog.statements[0] {
            Statement::Exec(Expr::Or(_, _)) => {}
            other => panic!("expected or, got {other:?}"),
        }
    }

    #[test]
    fn block() {
        let prog = parse("{ echo a; echo b }");
        match &prog.statements[0] {
            Statement::Exec(Expr::Block(stmts)) => {
                assert_eq!(stmts.len(), 2);
            }
            other => panic!("expected block, got {other:?}"),
        }
    }

    #[test]
    fn subshell() {
        let prog = parse("@{ echo isolated }");
        match &prog.statements[0] {
            Statement::Exec(Expr::Subshell(stmts)) => {
                assert_eq!(stmts.len(), 1);
            }
            other => panic!("expected subshell, got {other:?}"),
        }
    }

    #[test]
    fn multiple_statements() {
        let prog = parse("echo a; echo b\necho c");
        assert_eq!(prog.statements.len(), 3);
    }

    #[test]
    fn nested_redirect_left_to_right() {
        // cmd > out >> log — leftmost redirect is outermost (runs first)
        let prog = parse("cmd > out >> log");
        match &prog.statements[0] {
            Statement::Exec(Expr::Redirect(
                inner,
                RedirectOp::Output {
                    fd: 1,
                    append: false,
                    ..
                },
            )) => {
                // Outermost is > out (first redirect, runs first)
                // Inner is >> log (second redirect, runs second)
                assert!(matches!(
                    inner.as_ref(),
                    Expr::Redirect(
                        _,
                        RedirectOp::Output {
                            fd: 1,
                            append: true,
                            ..
                        }
                    )
                ));
            }
            other => panic!("expected nested redirects (left-to-right), got {other:?}"),
        }
    }

    #[test]
    fn empty_program() {
        let prog = parse("");
        assert_eq!(prog.statements.len(), 0);
    }

    #[test]
    fn comments_only() {
        let prog = parse("# just a comment\n# another one\n");
        assert_eq!(prog.statements.len(), 0);
    }

    #[test]
    fn switch_stmt() {
        let prog = parse("switch $x { case foo { echo foo } case * { echo other } }");
        match &prog.statements[0] {
            Statement::Switch { value, cases } => {
                assert_eq!(cases.len(), 2);
                assert!(matches!(&cases[1].0[0], Pattern::Star));
            }
            other => panic!("expected switch, got {other:?}"),
        }
    }
}
