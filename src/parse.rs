//! Parser for psh.
//!
//! Recursive descent parser operating directly on `&str` — no
//! separate lexer or token stream. Handles rc's quoting rules
//! (single quotes only, '' for literal quote) and the context-
//! sensitive boundaries between words, operators, and whitespace.
//!
//! Grammar (informal):
//!
//!   program     = command*
//!   command     = assignment | if | for | while | switch | fn | ref | expr_cmd
//!   expr_cmd    = or_expr (& | ; | \n)?
//!   or_expr     = and_expr (|| and_expr)*
//!   and_expr    = pipeline (&& pipeline)*
//!   pipeline    = cmd_expr (| cmd_expr)*  (plus |& for coprocess)
//!   cmd_expr    = ! cmd_expr | { body } | @{ body } | simple_command redirection*
//!   simple_cmd  = word+
//!   word        = word_atom (^ word_atom)*
//!   word_atom   = literal | $var | $#var | $var(idx) | `{ program } | quoted
//!   value       = word | (word*)
//!   redirect    = > target | >> target | < target | >[fd] target | >[fd=fd] | >[fd=]

use anyhow::{bail, Result};

use crate::ast::*;

fn is_word_char(c: char) -> bool {
    matches!(c,
        'a'..='z' | 'A'..='Z' | '0'..='9' |
        '_' | '-' | '.' | '/' | ':' | '+' |
        ',' | '%' | '~' | '*' | '?' | '[' | ']'
    )
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "if" | "else" | "for" | "in" | "switch" | "case" | "fn" | "while" | "ref"
    )
}

/// Parse the psh language. Public entry point.
pub struct PshParser;

impl PshParser {
    pub fn parse(input: &str) -> Result<Program> {
        let mut p = RecParser::new(input);
        let cmds = p.command_list()?;
        p.skip_ws();
        if !p.at_end() {
            bail!(
                "{}:{}: unexpected character '{}'",
                p.line,
                p.col,
                p.peek().unwrap()
            );
        }
        Ok(Program { commands: cmds })
    }
}

pub use PshParser as Parser;

/// Recursive descent parser state.
struct RecParser<'a> {
    input: &'a str,
    pos: usize,
    line: u32,
    col: u32,
}

impl<'a> RecParser<'a> {
    fn new(input: &'a str) -> Self {
        RecParser {
            input,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut chars = self.input[self.pos..].chars();
        chars.next();
        chars.next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn pos_str(&self) -> String {
        format!("{}:{}", self.line, self.col)
    }

    fn save(&self) -> (usize, u32, u32) {
        (self.pos, self.line, self.col)
    }

    fn restore(&mut self, saved: (usize, u32, u32)) {
        self.pos = saved.0;
        self.line = saved.1;
        self.col = saved.2;
    }

    /// Skip horizontal whitespace and comments (NOT newlines).
    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r') => {
                    self.advance();
                }
                Some('#') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    /// Skip terminators: newlines, semicolons, whitespace, comments.
    fn skip_terminators(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r' | '\n' | ';') => {
                    self.advance();
                }
                Some('#') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn at_terminator(&self) -> bool {
        matches!(self.peek(), None | Some('\n' | ';' | '}' | ')'))
    }

    /// Is the current position at something that can start a word?
    fn at_word_start(&self) -> bool {
        match self.peek() {
            Some(c) => is_word_char(c) || c == '$' || c == '`' || c == '\'',
            None => false,
        }
    }

    /// Is the current position at an operator that ends argument collection?
    fn at_operator(&self) -> bool {
        matches!(
            self.peek(),
            Some('|' | '>' | '<' | ';' | '\n' | '}' | ')') | None
        ) || (self.peek() == Some('&'))
    }

    // ── Leaf parsers ───────────────────────────────────────

    /// Read a bare word (sequence of word characters).
    fn read_bare_word(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if is_word_char(c) {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos > start {
            Some(self.input[start..self.pos].to_string())
        } else {
            None
        }
    }

    /// Read a single-quoted string (opening quote already consumed).
    fn read_quoted_string(&mut self) -> Result<String> {
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('\'') => {
                    if self.peek() == Some('\'') {
                        self.advance();
                        s.push('\'');
                    } else {
                        return Ok(s);
                    }
                }
                Some(c) => s.push(c),
                None => bail!("{}:{}: unterminated quoted string", self.line, self.col),
            }
        }
    }

    /// Read a sequence of digits (for fd numbers in redirections).
    fn read_digits(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos > start {
            Some(self.input[start..self.pos].to_string())
        } else {
            None
        }
    }

    /// Expect a bare-or-quoted word string (used for names in keyword positions).
    fn expect_word_string(&mut self, what: &str) -> Result<String> {
        self.skip_ws();
        if self.peek() == Some('\'') {
            self.advance();
            return self.read_quoted_string();
        }
        match self.read_bare_word() {
            Some(s) => Ok(s),
            None => bail!("{}: expected {what}", self.pos_str()),
        }
    }

    // ── Word parsing ───────────────────────────────────────

    /// Parse a word atom — the indivisible unit before ^concat.
    /// Skips leading whitespace.
    fn word_atom(&mut self) -> Result<Word> {
        self.skip_ws();
        self.word_atom_nows()
    }

    /// Parse a word atom without skipping leading whitespace.
    fn word_atom_nows(&mut self) -> Result<Word> {
        match self.peek() {
            Some('\'') => {
                self.advance();
                let s = self.read_quoted_string()?;
                Ok(Word::Literal(s))
            }
            Some('$') => {
                self.advance();
                if self.peek() == Some('#') {
                    self.advance();
                    let name = self.read_bare_word().ok_or_else(|| {
                        anyhow::anyhow!("{}: expected variable name after $#", self.pos_str())
                    })?;
                    Ok(Word::Count(name))
                } else {
                    let name = self.read_bare_word().ok_or_else(|| {
                        anyhow::anyhow!("{}: expected variable name after $", self.pos_str())
                    })?;
                    if self.peek() == Some('(') {
                        self.advance();
                        let idx = self.word()?;
                        if self.peek() != Some(')') {
                            bail!("{}: expected ')' after index", self.pos_str());
                        }
                        self.advance();
                        Ok(Word::Index(name, Box::new(idx)))
                    } else {
                        Ok(Word::Var(name))
                    }
                }
            }
            Some('`') => {
                self.advance();
                self.skip_ws();
                if self.peek() != Some('{') {
                    bail!("{}: expected '{{' after `", self.pos_str());
                }
                self.advance();
                let body = self.command_list()?;
                self.skip_ws();
                if self.peek() != Some('}') {
                    bail!(
                        "{}: expected '}}' to close command substitution",
                        self.pos_str()
                    );
                }
                self.advance();
                Ok(Word::CommandSub(body))
            }
            Some(c) if is_word_char(c) => {
                let s = self.read_bare_word().unwrap();
                Ok(Word::Literal(s))
            }
            other => bail!(
                "{}: expected word, got {}",
                self.pos_str(),
                match other {
                    Some(c) => format!("'{c}'"),
                    None => "end of input".into(),
                }
            ),
        }
    }

    /// Parse a word — atoms joined by ^ (concatenation).
    fn word(&mut self) -> Result<Word> {
        let base = self.word_atom()?;
        self.concat_rest(base)
    }

    /// After parsing a word atom, check for ^concat continuation.
    fn concat_rest(&mut self, base: Word) -> Result<Word> {
        if self.peek() == Some('^') {
            let mut parts = vec![base];
            while self.peek() == Some('^') {
                self.advance();
                parts.push(self.word_atom_nows()?);
            }
            Ok(Word::Concat(parts))
        } else {
            Ok(base)
        }
    }

    /// Parse a word in argument position — skips ws first and
    /// stops at operators, terminators, and command-starting keywords.
    fn arg_word(&mut self) -> Result<Option<Word>> {
        self.skip_ws();
        if self.at_terminator() || self.at_operator() || !self.at_word_start() {
            return Ok(None);
        }

        // Check for unquoted keyword that starts a command — stop
        if let Some(c) = self.peek() {
            if is_word_char(c) && c != '\'' {
                let saved = self.save();
                if let Some(w) = self.read_bare_word() {
                    if matches!(w.as_str(), "if" | "for" | "while" | "switch" | "fn" | "ref") {
                        self.restore(saved);
                        return Ok(None);
                    }
                    // Not a stopping keyword — it's a literal
                    let base = Word::Literal(w);
                    return Ok(Some(self.concat_rest(base)?));
                }
                self.restore(saved);
            }
        }

        // Parse as a word atom (handles $, `, ')
        if self.at_word_start() {
            let atom = self.word_atom_nows()?;
            Ok(Some(self.concat_rest(atom)?))
        } else {
            Ok(None)
        }
    }

    fn value(&mut self) -> Result<Value> {
        self.skip_ws();
        if self.peek() == Some('(') {
            self.advance();
            let mut items = Vec::new();
            loop {
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.advance();
                    break;
                }
                if self.at_end() {
                    bail!("{}: expected ')' to close list", self.pos_str());
                }
                items.push(self.word()?);
            }
            Ok(Value::List(items))
        } else {
            Ok(Value::Word(self.word()?))
        }
    }

    // ── Body and command list ──────────────────────────────

    fn body(&mut self) -> Result<Vec<Command>> {
        self.skip_ws();
        if self.peek() != Some('{') {
            bail!("{}: expected '{{' to open body", self.pos_str());
        }
        self.advance();
        let cmds = self.command_list()?;
        self.skip_ws();
        if self.peek() != Some('}') {
            bail!("{}: expected '}}' to close body", self.pos_str());
        }
        self.advance();
        Ok(cmds)
    }

    fn command_list(&mut self) -> Result<Vec<Command>> {
        let mut cmds = Vec::new();
        self.skip_terminators();
        while !self.at_end() && self.peek() != Some('}') && self.peek() != Some(')') {
            cmds.push(self.command()?);
            self.skip_terminators();
        }
        Ok(cmds)
    }

    // ── Commands ───────────────────────────────────────────

    fn command(&mut self) -> Result<Command> {
        self.skip_ws();
        let saved = self.save();
        if let Some(c) = self.peek() {
            if is_word_char(c) {
                if let Some(w) = self.read_bare_word() {
                    match w.as_str() {
                        "if" => return self.if_cmd(),
                        "for" => return self.for_cmd(),
                        "while" => return self.while_cmd(),
                        "switch" => return self.switch_cmd(),
                        "fn" => return self.fn_cmd(),
                        "ref" => return self.ref_cmd(),
                        _ => {
                            self.restore(saved);
                        }
                    }
                } else {
                    self.restore(saved);
                }
            }
        }
        self.simple_or_assign()
    }

    fn if_cmd(&mut self) -> Result<Command> {
        // "if" already consumed
        let condition = self.pipeline()?;
        let then_body = self.body()?;
        self.skip_ws();
        let else_body = if self.check_keyword("else") {
            self.skip_ws();
            if self.check_keyword("if") {
                let nested = self.if_cmd()?;
                Some(vec![nested])
            } else {
                Some(self.body()?)
            }
        } else {
            None
        };
        Ok(Command::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn for_cmd(&mut self) -> Result<Command> {
        let var = self.expect_word_string("variable name")?;
        self.skip_ws();
        if !self.check_keyword("in") {
            bail!("{}: expected 'in' after for variable", self.pos_str());
        }
        let list = self.value()?;
        let body = self.body()?;
        Ok(Command::For { var, list, body })
    }

    fn while_cmd(&mut self) -> Result<Command> {
        let condition = self.pipeline()?;
        let body = self.body()?;
        Ok(Command::While { condition, body })
    }

    fn switch_cmd(&mut self) -> Result<Command> {
        let val = self.value()?;
        self.skip_ws();
        if self.peek() != Some('{') {
            bail!("{}: expected '{{' after switch value", self.pos_str());
        }
        self.advance();
        let mut cases = Vec::new();
        self.skip_terminators();
        while self.check_keyword("case") {
            let mut patterns = Vec::new();
            loop {
                self.skip_ws();
                let pat = self.expect_word_string("pattern")?;
                patterns.push(if pat == "*" {
                    Pattern::Star
                } else if pat.contains('*') || pat.contains('?') || pat.contains('[') {
                    Pattern::Glob(pat)
                } else {
                    Pattern::Literal(pat)
                });
                self.skip_ws();
                if self.peek() == Some('{') {
                    break;
                }
            }
            let body = self.body()?;
            cases.push((patterns, body));
            self.skip_terminators();
        }
        self.skip_ws();
        if self.peek() != Some('}') {
            bail!("{}: expected '}}' to close switch", self.pos_str());
        }
        self.advance();
        Ok(Command::Switch { value: val, cases })
    }

    fn fn_cmd(&mut self) -> Result<Command> {
        let name = self.expect_word_string("function name")?;
        let body = self.body()?;
        Ok(Command::Bind(Binding::Fn { name, body }))
    }

    fn ref_cmd(&mut self) -> Result<Command> {
        let name = self.expect_word_string("reference name")?;
        self.skip_ws();
        if self.peek() != Some('=') {
            bail!("{}: expected '=' after ref name", self.pos_str());
        }
        self.advance();
        let target = self.value()?;
        Ok(Command::Bind(Binding::Assignment(name, target)))
    }

    /// Check for and consume a specific keyword.
    fn check_keyword(&mut self, kw: &str) -> bool {
        self.skip_ws();
        let saved = self.save();
        if let Some(w) = self.read_bare_word() {
            if w == kw {
                return true;
            }
        }
        self.restore(saved);
        false
    }

    fn simple_or_assign(&mut self) -> Result<Command> {
        self.skip_ws();

        // Look ahead for assignment: word = value
        let saved = self.save();
        if let Some(c) = self.peek() {
            if is_word_char(c) || c == '\'' {
                if let Ok(Word::Literal(ref name)) = self.word_atom_nows() {
                    if !is_keyword(name) {
                        self.skip_ws();
                        if self.peek() == Some('=') {
                            self.advance();
                            self.skip_ws();
                            let val = if self.at_terminator() {
                                Value::List(vec![])
                            } else {
                                self.value()?
                            };
                            return Ok(Command::Bind(Binding::Assignment(name.clone(), val)));
                        }
                    }
                }
                self.restore(saved);
            }
        }

        let expr = self.or_expr()?;

        self.skip_ws();
        if self.peek() == Some('&') && self.peek2() != Some('&') {
            self.advance();
            return Ok(Command::Exec(Expr::Background(Box::new(expr))));
        }

        Ok(Command::Exec(expr))
    }

    // ── Expression precedence ──────────────────────────────

    fn or_expr(&mut self) -> Result<Expr> {
        let mut left = self.and_expr()?;
        loop {
            self.skip_ws();
            if self.peek() == Some('|') && self.peek2() == Some('|') {
                self.advance();
                self.advance();
                let right = self.and_expr()?;
                left = Expr::Or(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expr> {
        let mut left = self.pipeline()?;
        loop {
            self.skip_ws();
            if self.peek() == Some('&') && self.peek2() == Some('&') {
                self.advance();
                self.advance();
                let right = self.pipeline()?;
                left = Expr::And(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn pipeline(&mut self) -> Result<Expr> {
        let first = self.cmd_expr()?;

        self.skip_ws();
        // Check for |& (coprocess)
        if self.peek() == Some('|') && self.peek2() == Some('&') {
            self.advance();
            self.advance();
            return Ok(Expr::Coprocess(Box::new(first)));
        }

        // Check for | (pipe) — not || (or)
        if self.peek() != Some('|') || self.peek2() == Some('|') {
            return Ok(first);
        }

        let mut stages = vec![first];
        while self.peek() == Some('|') && self.peek2() != Some('|') && self.peek2() != Some('&') {
            self.advance(); // consume |
            stages.push(self.cmd_expr()?);
            self.skip_ws();
        }
        Ok(Expr::Pipeline(stages))
    }

    fn cmd_expr(&mut self) -> Result<Expr> {
        self.skip_ws();

        // Negation
        if self.peek() == Some('!') {
            self.advance();
            let cmd = self.cmd_expr()?;
            return Ok(Expr::Not(Box::new(cmd)));
        }

        // Subshell: @{
        if self.peek() == Some('@') && self.peek2() == Some('{') {
            self.advance(); // @
            self.advance(); // {
            let cmds = self.command_list()?;
            self.skip_ws();
            if self.peek() != Some('}') {
                bail!("{}: expected '}}' to close subshell", self.pos_str());
            }
            self.advance();
            let expr = Expr::Subshell(cmds);
            return self.parse_redirections(expr);
        }

        // Block: { ... }
        if self.peek() == Some('{') {
            let body = self.body()?;
            let expr = Expr::Block(body);
            return self.parse_redirections(expr);
        }

        // Simple command
        if !self.at_word_start() {
            bail!(
                "{}: expected command, got {}",
                self.pos_str(),
                match self.peek() {
                    Some(c) => format!("'{c}'"),
                    None => "end of input".into(),
                }
            );
        }

        let cmd = self.parse_simple_command()?;
        self.parse_redirections(Expr::Simple(cmd))
    }

    fn parse_simple_command(&mut self) -> Result<SimpleCommand> {
        let name = self.word()?;
        let mut args = Vec::new();
        while let Some(w) = self.arg_word()? {
            args.push(w);
        }
        Ok(SimpleCommand {
            name,
            args,
            assignments: vec![],
        })
    }

    /// Parse zero or more redirections, wrapping the expression.
    fn parse_redirections(&mut self, expr: Expr) -> Result<Expr> {
        let mut ops = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('>') => {
                    self.advance();
                    let op = self.parse_output_redirect()?;
                    ops.push(op);
                }
                Some('<') => {
                    self.advance();
                    self.skip_ws();
                    let target = self.word()?;
                    ops.push(RedirectOp::Input {
                        fd: 0,
                        target: RedirectTarget::File(target),
                    });
                }
                _ => break,
            }
        }
        // Wrap in reverse for left-to-right evaluation
        let mut result = expr;
        for op in ops.into_iter().rev() {
            result = Expr::Redirect(Box::new(result), op);
        }
        Ok(result)
    }

    /// Parse what follows '>' — handles >>, >[fd], >[fd=fd], >[fd=], >target.
    fn parse_output_redirect(&mut self) -> Result<RedirectOp> {
        // >> (append)
        if self.peek() == Some('>') {
            self.advance();
            self.skip_ws();
            let target = self.word()?;
            return Ok(RedirectOp::Output {
                fd: 1,
                target: RedirectTarget::File(target),
                append: true,
            });
        }

        // >[fd...] — fd redirections use []
        if self.peek() == Some('[') {
            self.advance();
            let fd_str = self
                .read_digits()
                .ok_or_else(|| anyhow::anyhow!("{}: expected fd number", self.pos_str()))?;
            let fd: u32 = fd_str
                .parse()
                .map_err(|_| anyhow::anyhow!("{}: invalid fd number '{fd_str}'", self.pos_str()))?;

            if self.peek() == Some('=') {
                self.advance();
                // >[fd=] (close) or >[fd=src]
                if self.peek() == Some(']') {
                    self.advance();
                    return Ok(RedirectOp::Close { fd });
                }
                let src_str = self
                    .read_digits()
                    .ok_or_else(|| anyhow::anyhow!("{}: expected source fd", self.pos_str()))?;
                let src: u32 = src_str.parse().map_err(|_| {
                    anyhow::anyhow!("{}: invalid source fd '{src_str}'", self.pos_str())
                })?;
                if self.peek() != Some(']') {
                    bail!("{}: expected ']'", self.pos_str());
                }
                self.advance();
                return Ok(RedirectOp::Dup { dst: fd, src });
            }

            if self.peek() != Some(']') {
                bail!("{}: expected ']' or '='", self.pos_str());
            }
            self.advance();
            self.skip_ws();
            let target = self.word()?;
            return Ok(RedirectOp::Output {
                fd,
                target: RedirectTarget::File(target),
                append: false,
            });
        }

        // > target (simple, fd defaults to 1)
        self.skip_ws();
        let target = self.word()?;
        Ok(RedirectOp::Output {
            fd: 1,
            target: RedirectTarget::File(target),
            append: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Program {
        Parser::parse(input).expect("parse failed")
    }

    #[test]
    fn simple_command() {
        let prog = parse("echo hello world");
        assert_eq!(prog.commands.len(), 1);
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.name, Word::Literal("echo".into()));
                assert_eq!(cmd.args.len(), 2);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn assignment() {
        let prog = parse("x = hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Assignment(name, Value::Word(Word::Literal(val)))) => {
                assert_eq!(name, "x");
                assert_eq!(val, "hello");
            }
            other => panic!("expected assignment, got {other:?}"),
        }
    }

    #[test]
    fn list_assignment() {
        let prog = parse("x = (a b c)");
        match &prog.commands[0] {
            Command::Bind(Binding::Assignment(name, Value::List(items))) => {
                assert_eq!(name, "x");
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected list assignment, got {other:?}"),
        }
    }

    #[test]
    fn pipeline() {
        let prog = parse("ls | grep foo | sort");
        match &prog.commands[0] {
            Command::Exec(Expr::Pipeline(stages)) => {
                assert_eq!(stages.len(), 3);
            }
            other => panic!("expected pipeline, got {other:?}"),
        }
    }

    #[test]
    fn redirect_output() {
        let prog = parse("echo hello > file");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
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
        match &prog.commands[0] {
            Command::Exec(Expr::Background(inner)) => {
                assert!(matches!(inner.as_ref(), Expr::Simple(_)));
            }
            other => panic!("expected background, got {other:?}"),
        }
    }

    #[test]
    fn if_else() {
        let prog = parse("if test -f foo { echo yes } else { echo no }");
        match &prog.commands[0] {
            Command::If {
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
        match &prog.commands[0] {
            Command::For { var, list, body } => {
                assert_eq!(var, "x");
                assert!(matches!(list, Value::List(_)));
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected for, got {other:?}"),
        }
    }

    #[test]
    fn while_loop() {
        let prog = parse("while true { echo looping }");
        match &prog.commands[0] {
            Command::While { condition, body } => {
                assert!(matches!(condition, Expr::Simple(_)));
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected while, got {other:?}"),
        }
    }

    #[test]
    fn fn_definition() {
        let prog = parse("fn greet { echo hello }");
        match &prog.commands[0] {
            Command::Bind(Binding::Fn { name, body }) => {
                assert_eq!(name, "greet");
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected fn, got {other:?}"),
        }
    }

    #[test]
    fn discipline_fn() {
        let prog = parse("fn x.get { echo computed }");
        match &prog.commands[0] {
            Command::Bind(Binding::Fn { name, body }) => {
                assert_eq!(name, "x.get");
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected fn, got {other:?}"),
        }
    }

    #[test]
    fn variable_expansion() {
        let prog = parse("echo $x");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Var("x".into()));
            }
            other => panic!("expected command with var, got {other:?}"),
        }
    }

    #[test]
    fn variable_index() {
        let prog = parse("echo $x(2)");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert!(matches!(&cmd.args[0], Word::Index(name, _) if name == "x"));
            }
            other => panic!("expected indexed var, got {other:?}"),
        }
    }

    #[test]
    fn variable_count() {
        let prog = parse("echo $#x");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Count("x".into()));
            }
            other => panic!("expected count, got {other:?}"),
        }
    }

    #[test]
    fn command_substitution() {
        let prog = parse("echo `{ date }");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert!(matches!(&cmd.args[0], Word::CommandSub(_)));
            }
            other => panic!("expected command sub, got {other:?}"),
        }
    }

    #[test]
    fn concatenation() {
        let prog = parse("echo foo^bar");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert!(matches!(&cmd.args[0], Word::Concat(_)));
            }
            other => panic!("expected concat, got {other:?}"),
        }
    }

    #[test]
    fn coprocess() {
        let prog = parse("cmd |&");
        match &prog.commands[0] {
            Command::Exec(Expr::Coprocess(_)) => {}
            other => panic!("expected coprocess, got {other:?}"),
        }
    }

    #[test]
    fn and_or() {
        let prog = parse("test -f foo && echo yes || echo no");
        match &prog.commands[0] {
            Command::Exec(Expr::Or(_, _)) => {}
            other => panic!("expected or, got {other:?}"),
        }
    }

    #[test]
    fn block() {
        let prog = parse("{ echo a; echo b }");
        match &prog.commands[0] {
            Command::Exec(Expr::Block(cmds)) => {
                assert_eq!(cmds.len(), 2);
            }
            other => panic!("expected block, got {other:?}"),
        }
    }

    #[test]
    fn subshell() {
        let prog = parse("@{ echo isolated }");
        match &prog.commands[0] {
            Command::Exec(Expr::Subshell(cmds)) => {
                assert_eq!(cmds.len(), 1);
            }
            other => panic!("expected subshell, got {other:?}"),
        }
    }

    #[test]
    fn multiple_commands() {
        let prog = parse("echo a; echo b\necho c");
        assert_eq!(prog.commands.len(), 3);
    }

    #[test]
    fn nested_redirect_left_to_right() {
        let prog = parse("cmd > out >> log");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                inner,
                RedirectOp::Output {
                    fd: 1,
                    append: false,
                    ..
                },
            )) => {
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
        assert_eq!(prog.commands.len(), 0);
    }

    #[test]
    fn comments_only() {
        let prog = parse("# just a comment\n# another one\n");
        assert_eq!(prog.commands.len(), 0);
    }

    #[test]
    fn switch_cmd() {
        let prog = parse("switch $x { case foo { echo foo } case * { echo other } }");
        match &prog.commands[0] {
            Command::Switch { value: _, cases } => {
                assert_eq!(cases.len(), 2);
                assert!(matches!(&cases[1].0[0], Pattern::Star));
            }
            other => panic!("expected switch, got {other:?}"),
        }
    }

    #[test]
    fn fd_redirect_bracket() {
        let prog = parse("cmd >[2=1]");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(_, RedirectOp::Dup { dst: 2, src: 1 })) => {}
            other => panic!("expected dup redirect, got {other:?}"),
        }
    }

    #[test]
    fn fd_close_bracket() {
        let prog = parse("cmd >[2=]");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(_, RedirectOp::Close { fd: 2 })) => {}
            other => panic!("expected close redirect, got {other:?}"),
        }
    }

    #[test]
    fn fd_output_bracket() {
        let prog = parse("cmd >[2] file");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Output {
                    fd: 2,
                    append: false,
                    ..
                },
            )) => {}
            other => panic!("expected fd output redirect, got {other:?}"),
        }
    }
}
