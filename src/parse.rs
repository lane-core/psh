//! Parser for psh — combine 4 rewrite.
//!
//! Six-layer combinator architecture:
//!   1. Lexical predicates and primitives
//!   2. Word atoms (quoted, var_ref, command_sub, etc.)
//!   3. Words with free carets
//!   4. Expression precedence tower
//!   5. Commands (keyword-initiated + simple/assign)
//!   6. Program (top-level sequencing)
//!
//! Grammar: docs/syntax.md (normative). This parser implements all
//! unmarked productions. [planned] productions get stub parsers
//! that return clear errors.
//!
//! Heritage: rc (Duff 1990) grammar with psh extensions (match
//! with =>, two-alphabet split, free carets, brace-delimited vars).

use anyhow::{bail, Result};

use crate::ast::*;

// ── Layer 1: Lexical predicates ───────────────────────────────

/// Variable-name alphabet — used after `$`, `$#`, `$"`.
/// rc's variable-name set: alphanumerics, `_`, and `*`.
/// Variable names terminate at the first character not in this set,
/// enabling `$home/bin` to parse as `$home` followed by `/bin`.
fn is_var_char(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '*')
}

/// Bare-word alphabet — for literals, function names, paths.
/// Includes `.` (discipline names), `/` (paths), `@` (user@host),
/// and other non-operator characters. `~` is NOT included —
/// it receives special handling (tilde expansion vs match builtin).
fn is_word_char(c: char) -> bool {
    matches!(c,
        'a'..='z' | 'A'..='Z' | '0'..='9' |
        '_' | '-' | '.' | '/' | ':' | '+' |
        ',' | '%' | '*' | '?' | '[' | ']' | '@'
    )
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "if" | "else" | "for" | "in" | "match" | "fn" | "while" | "ref" | "let" | "try" | "return"
    )
}

/// Parse the psh language. Public entry point.
pub struct PshParser;

impl PshParser {
    pub fn parse(input: &str) -> Result<Program> {
        let mut p = RecParser::new(input);
        let cmds = p.program()?;
        p.skip_terminators();
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

// ── Recursive descent on &str with position tracking ──────────
//
// combine 4's parser combinator model provides the architectural
// thinking (layers, attempt, choice) but psh's grammar has enough
// context-sensitivity (heredocs, free carets, keyword detection)
// that a direct recursive descent on &str is cleaner than fighting
// combine's type system. The layered structure follows combine's
// six-layer pattern; the implementation uses direct &str indexing
// for the context-sensitive bits.

/// Internal parser state — recursive descent over &str.
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

    /// Look ahead at the remaining input from the current position.
    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
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

    // ── Layer 1: Whitespace and terminators ───────────────────

    /// Skip horizontal whitespace, comments, and line continuations.
    /// Does NOT consume newlines (those are terminators).
    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r') => {
                    self.advance();
                }
                // Line continuation: backslash immediately before newline
                Some('\\') if self.peek2() == Some('\n') => {
                    self.advance(); // consume backslash
                    self.advance(); // consume newline
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

    /// Skip terminators: newlines, semicolons, whitespace, comments,
    /// and line continuations.
    fn skip_terminators(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r' | '\n' | ';') => {
                    self.advance();
                }
                Some('\\') if self.peek2() == Some('\n') => {
                    self.advance();
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

    /// Can the current position start a word atom?
    fn at_word_start(&self) -> bool {
        match self.peek() {
            Some(c) if is_word_char(c) || c == '$' || c == '`' || c == '\'' => true,
            Some('~') => true,
            Some('<') if self.peek2() == Some('{') => true,
            _ => false,
        }
    }

    /// Is the current position at an operator that ends argument collection?
    fn at_operator(&self) -> bool {
        match self.peek() {
            Some('<') => self.peek2() != Some('{'),
            Some('|' | '>' | ';' | '\n' | '}' | ')') => true,
            Some('&') => true,
            None => true,
            _ => false,
        }
    }

    // ── Layer 1: Primitive readers ────────────────────────────

    /// Read a bare word (sequence of word_char characters).
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

    /// Read a variable name (sequence of var_char characters).
    fn read_var_name(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if is_var_char(c) {
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

    /// Read digits (for fd numbers in redirections).
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

    /// Read here-document content until a line matching the delimiter.
    fn read_heredoc(&mut self, delim: &str) -> Result<String> {
        // Skip the rest of the current line
        while let Some(c) = self.peek() {
            if c == '\n' {
                self.advance();
                break;
            }
            self.advance();
        }

        let mut content = String::new();
        loop {
            let mut line = String::new();
            loop {
                match self.peek() {
                    Some('\n') => {
                        self.advance();
                        break;
                    }
                    Some(c) => {
                        self.advance();
                        line.push(c);
                    }
                    None => {
                        if line.is_empty() {
                            bail!(
                                "{}: unterminated here-document (expected '{delim}')",
                                self.pos_str()
                            );
                        }
                        if line == delim {
                            return Ok(content);
                        }
                        bail!(
                            "{}: unterminated here-document (expected '{delim}')",
                            self.pos_str()
                        );
                    }
                }
            }

            if line == delim {
                return Ok(content);
            }
            content.push_str(&line);
            content.push('\n');
        }
    }

    /// Read a type name — alphabetic characters only.
    fn read_type_name(&mut self) -> Option<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() {
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

    /// Expect a bare-or-quoted word string.
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

    /// Check for and consume a specific keyword. Returns true if found.
    fn check_keyword(&mut self, kw: &str) -> bool {
        self.skip_ws();
        let saved = self.save();
        if let Some(w) = self.read_bare_word() {
            if w == kw {
                // Ensure the keyword is not a prefix of a longer word
                if let Some(c) = self.peek() {
                    if is_var_char(c) {
                        self.restore(saved);
                        return false;
                    }
                }
                return true;
            }
        }
        self.restore(saved);
        false
    }

    // ── Layer 2: Word atoms ──────────────────────────────────

    /// Parse a word atom — the indivisible unit before ^concat.
    /// Does NOT skip leading whitespace.
    fn word_atom_nows(&mut self) -> Result<Word> {
        match self.peek() {
            Some('\'') => {
                self.advance();
                let s = self.read_quoted_string()?;
                Ok(Word::Quoted(s))
            }
            Some('$') => {
                self.advance();
                match self.peek() {
                    Some('#') => {
                        self.advance();
                        let name = self.read_var_name().ok_or_else(|| {
                            anyhow::anyhow!("{}: expected variable name after $#", self.pos_str())
                        })?;
                        Ok(Word::Count(name))
                    }
                    Some('"') => {
                        // rc heritage: $"x joins list elements with spaces
                        self.advance();
                        let name = self.read_var_name().ok_or_else(|| {
                            anyhow::anyhow!("{}: expected variable name after $\"", self.pos_str())
                        })?;
                        Ok(Word::Stringify(name))
                    }
                    Some('{') => {
                        // ${name} — brace-delimited variable name
                        self.advance();
                        let mut name = String::new();
                        loop {
                            match self.peek() {
                                Some('}') => {
                                    self.advance();
                                    break;
                                }
                                Some(c) if is_word_char(c) => {
                                    self.advance();
                                    name.push(c);
                                }
                                other => {
                                    bail!(
                                        "{}: expected '}}' to close ${{name}}, got {}",
                                        self.pos_str(),
                                        match other {
                                            Some(c) => format!("'{c}'"),
                                            None => "end of input".into(),
                                        }
                                    );
                                }
                            }
                        }
                        if name.is_empty() {
                            bail!("{}: empty variable name in ${{}}", self.pos_str());
                        }
                        Ok(Word::Var(name))
                    }
                    _ => {
                        let name = self.read_var_name().ok_or_else(|| {
                            anyhow::anyhow!("{}: expected variable name after $", self.pos_str())
                        })?;
                        // Check for indexing: $var(idx)
                        if self.peek() == Some('(') {
                            self.advance();
                            let idx = self.word_inner()?;
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
            // <{ process substitution
            Some('<') if self.peek2() == Some('{') => {
                self.advance(); // <
                self.advance(); // {
                let body = self.command_list()?;
                self.skip_ws();
                if self.peek() != Some('}') {
                    bail!(
                        "{}: expected '}}' to close process substitution",
                        self.pos_str()
                    );
                }
                self.advance();
                Ok(Word::ProcessSub(body))
            }
            // ~ tilde expansion — not in word_char
            Some('~') => {
                self.advance();
                let rest = self.read_bare_word().unwrap_or_default();
                Ok(Word::Literal(format!("~{rest}")))
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

    // ── Layer 3: Words with free carets ──────────────────────

    /// Can the current character start a word atom (no ws consumed)?
    fn can_start_word_atom(&self) -> bool {
        match self.peek() {
            Some('$' | '\'' | '`' | '~') => true,
            Some('<') if self.peek2() == Some('{') => true,
            Some(c) if is_word_char(c) => true,
            _ => false,
        }
    }

    /// Parse a word — atoms joined by explicit `^` or implicit free carets.
    /// Skips leading whitespace before the first atom.
    fn word(&mut self) -> Result<Word> {
        self.skip_ws();
        self.word_inner()
    }

    /// Parse a word without skipping leading whitespace.
    fn word_inner(&mut self) -> Result<Word> {
        let base = self.word_atom_nows()?;
        self.concat_rest(base)
    }

    /// After parsing a word atom, check for explicit `^` or implicit
    /// free caret (adjacent word atoms with no whitespace).
    fn concat_rest(&mut self, base: Word) -> Result<Word> {
        let mut parts = vec![base];
        loop {
            if self.peek() == Some('^') {
                // Explicit caret — consume and parse next atom
                self.advance();
                // Allow whitespace after explicit ^
                if self.can_start_word_atom() {
                    parts.push(self.word_atom_nows()?);
                } else {
                    self.skip_ws();
                    parts.push(self.word_atom_nows()?);
                }
            } else if self.can_start_word_atom() {
                // Free caret — adjacent atom with no whitespace
                parts.push(self.word_atom_nows()?);
            } else {
                break;
            }
        }
        if parts.len() == 1 {
            Ok(parts.into_iter().next().unwrap())
        } else {
            Ok(Word::Concat(parts))
        }
    }

    /// Parse a word in argument position — skips ws, stops at
    /// operators, terminators, and command-starting keywords.
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
                    if matches!(
                        w.as_str(),
                        "if" | "for" | "while" | "match" | "fn" | "ref" | "let"
                    ) {
                        // Ensure it's actually a keyword, not a prefix
                        // of a longer word (e.g. `iffy`)
                        let is_keyword_boundary = match self.peek() {
                            None => true,
                            Some(c) => !is_var_char(c),
                        };
                        if is_keyword_boundary {
                            self.restore(saved);
                            return Ok(None);
                        }
                    }
                    // Not a stopping keyword — it's a literal
                    let base = Word::Literal(w);
                    return Ok(Some(self.concat_rest(base)?));
                }
                self.restore(saved);
            }
        }

        // Parse as a word atom
        if self.at_word_start() {
            let atom = self.word_atom_nows()?;
            Ok(Some(self.concat_rest(atom)?))
        } else {
            Ok(None)
        }
    }

    /// Parse a value: `( word* )` for lists, or a single word.
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

    // ── Layer 4: Expression precedence tower ─────────────────

    /// or_expr = and_expr ('||' and_expr)*
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

    /// and_expr = pipeline ('&&' pipeline)*
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

    /// pipeline = cmd_expr ('|' cmd_expr)* | cmd_expr '|&'
    fn pipeline(&mut self) -> Result<Expr> {
        let first = self.cmd_expr()?;

        self.skip_ws();
        // |& coprocess — must check before |
        if self.peek() == Some('|') && self.peek2() == Some('&') {
            self.advance();
            self.advance();
            return Ok(Expr::Coprocess(Box::new(first)));
        }

        // | pipe — not || (or)
        if self.peek() != Some('|') || self.peek2() == Some('|') {
            return Ok(first);
        }

        let mut stages = vec![first];
        while self.peek() == Some('|') && self.peek2() != Some('|') && self.peek2() != Some('&') {
            self.advance();
            stages.push(self.cmd_expr()?);
            self.skip_ws();
        }
        Ok(Expr::Pipeline(stages))
    }

    /// cmd_expr = '!' cmd_expr | body | '@' body | simple_cmd redirect*
    fn cmd_expr(&mut self) -> Result<Expr> {
        self.skip_ws();

        // ! negation
        if self.peek() == Some('!') {
            self.advance();
            let cmd = self.cmd_expr()?;
            return Ok(Expr::Not(Box::new(cmd)));
        }

        // @{ subshell
        if self.peek() == Some('@') && self.peek2() == Some('{') {
            self.advance();
            self.advance();
            let cmds = self.command_list()?;
            self.skip_ws();
            if self.peek() != Some('}') {
                bail!("{}: expected '}}' to close subshell", self.pos_str());
            }
            self.advance();
            let expr = Expr::Subshell(cmds);
            return self.parse_redirections(expr);
        }

        // { block
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

    // ── Layer 5: Commands ────────────────────────────────────

    /// Body: '{' program '}'
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

    /// Command list — zero or more commands separated by terminators.
    fn command_list(&mut self) -> Result<Vec<Command>> {
        let mut cmds = Vec::new();
        self.skip_terminators();
        while !self.at_end() && self.peek() != Some('}') && self.peek() != Some(')') {
            cmds.push(self.command()?);
            self.skip_terminators();
        }
        Ok(cmds)
    }

    /// Top-level command dispatch.
    fn command(&mut self) -> Result<Command> {
        self.skip_ws();
        let saved = self.save();
        if let Some(c) = self.peek() {
            if is_word_char(c) {
                if let Some(w) = self.read_bare_word() {
                    // Ensure this is a standalone keyword, not a prefix
                    let is_boundary = match self.peek() {
                        None => true,
                        Some(c) => !is_var_char(c),
                    };
                    if is_boundary {
                        match w.as_str() {
                            "if" => return self.if_cmd(),
                            "for" => return self.for_cmd(),
                            "while" => return self.while_cmd(),
                            "match" => return self.match_cmd(),
                            "fn" => return self.fn_cmd(),
                            "ref" => return self.ref_cmd(),
                            "let" => return self.let_cmd(),
                            "try" => {
                                bail!("{}: try blocks not yet implemented", self.pos_str());
                            }
                            "return" => return self.return_cmd(),
                            _ => {}
                        }
                    }
                    self.restore(saved);
                }
            }
        }
        self.simple_or_assign()
    }

    fn if_cmd(&mut self) -> Result<Command> {
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

    /// Parse `match value { arms }`.
    ///
    /// Supports two arm syntaxes for backwards compatibility:
    /// - New: `pat+ => command; ...` (spec syntax)
    /// - Old: `case pat+ { body }` (prototype syntax)
    ///
    /// Detection: if the first word inside the match body is `case`,
    /// parse old-style. Otherwise parse new-style with `=>` arms.
    fn match_cmd(&mut self) -> Result<Command> {
        let val = self.value()?;
        self.skip_ws();
        if self.peek() != Some('{') {
            bail!("{}: expected '{{' after match value", self.pos_str());
        }
        self.advance();

        // Detect old vs new syntax by peeking for `case` keyword
        let mut arms = Vec::new();
        self.skip_terminators();

        // Peek to detect which syntax
        let saved = self.save();
        let use_old_syntax = if let Some(w) = self.read_bare_word() {
            let is_old = w == "case";
            self.restore(saved);
            is_old
        } else {
            self.restore(saved);
            false
        };

        if use_old_syntax {
            // Old prototype syntax: case pat+ { body }
            while self.check_keyword("case") {
                let mut patterns = Vec::new();
                loop {
                    self.skip_ws();
                    let pat = self.expect_word_string("pattern")?;
                    patterns.push(self.classify_pattern(&pat));
                    self.skip_ws();
                    if self.peek() == Some('{') {
                        break;
                    }
                }
                let body = self.body()?;
                arms.push((patterns, body));
                self.skip_terminators();
            }
        } else {
            // New spec syntax: pat => body; ...
            // Newlines are trivia inside match { }
            while self.peek() != Some('}') && !self.at_end() {
                let (patterns, body) = self.match_arm()?;
                arms.push((patterns, body));

                // Skip ; separator and trivia (newlines are trivia in match)
                self.skip_match_trivia();
            }
        }

        self.skip_ws();
        if self.peek() != Some('}') {
            bail!("{}: expected '}}' to close match", self.pos_str());
        }
        self.advance();
        Ok(Command::Match { value: val, arms })
    }

    /// Skip whitespace, newlines, and semicolons inside match blocks
    /// (newlines are trivia in match).
    fn skip_match_trivia(&mut self) {
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

    /// Parse a single match arm: patterns => body
    ///
    /// Structural arm: `tag $binding => body`
    /// Glob arm: `pattern+ => body`
    fn match_arm(&mut self) -> Result<(Vec<Pattern>, Vec<Command>)> {
        self.skip_match_trivia();
        let mut patterns = Vec::new();

        // Read the first pattern word
        let first_pat = self.expect_word_string("pattern")?;

        // Check if this is a structural arm: tag $binding
        self.skip_ws();
        if self.peek() == Some('$') {
            // Structural arm: first_pat is the tag, next is $binding
            self.advance(); // consume $
            let binding = self.read_var_name().ok_or_else(|| {
                anyhow::anyhow!("{}: expected binding name after $", self.pos_str())
            })?;
            patterns.push(Pattern::Structural {
                tag: first_pat,
                binding,
            });
        } else {
            // Glob arm — collect patterns until =>
            patterns.push(self.classify_pattern(&first_pat));
            loop {
                self.skip_ws();
                if self.rest().starts_with("=>") {
                    break;
                }
                if self.at_end() || self.peek() == Some('}') {
                    bail!("{}: expected '=>' in match arm", self.pos_str());
                }
                let pat = self.expect_word_string("pattern")?;
                patterns.push(self.classify_pattern(&pat));
            }
        }

        // Consume =>
        self.skip_ws();
        if !self.rest().starts_with("=>") {
            bail!("{}: expected '=>' in match arm", self.pos_str());
        }
        self.advance(); // =
        self.advance(); // >
        self.skip_ws();

        // Parse the arm body: either { program } or a single command
        let body = if self.peek() == Some('{') {
            self.body()?
        } else {
            // Single command — terminated by ; or newline or }
            let cmd = self.command()?;
            vec![cmd]
        };

        Ok((patterns, body))
    }

    /// Classify a pattern string into the Pattern enum.
    fn classify_pattern(&self, pat: &str) -> Pattern {
        if pat == "*" {
            Pattern::Star
        } else if pat.contains('*') || pat.contains('?') || pat.contains('[') {
            Pattern::Glob(pat.to_string())
        } else {
            Pattern::Literal(pat.to_string())
        }
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
        let target = self.expect_word_string("reference target")?;
        Ok(Command::Bind(Binding::Ref { name, target }))
    }

    fn let_cmd(&mut self) -> Result<Command> {
        self.skip_ws();
        let mut mutable = false;
        let mut export = false;

        // Check for `mut` and `export` modifiers (order-free)
        loop {
            let saved = self.save();
            if let Some(w) = self.read_bare_word() {
                match w.as_str() {
                    "mut" if !mutable => {
                        mutable = true;
                        self.skip_ws();
                        continue;
                    }
                    "export" if !export => {
                        export = true;
                        self.skip_ws();
                        continue;
                    }
                    _ => {
                        self.restore(saved);
                        break;
                    }
                }
            } else {
                self.restore(saved);
                break;
            }
        }

        let name = self.expect_word_string("variable name")?;
        self.skip_ws();

        // Optional type annotation: `: Type`
        let type_ann = if self.peek() == Some(':') {
            self.advance();
            self.skip_ws();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        self.skip_ws();
        if self.peek() != Some('=') {
            bail!("{}: expected '=' in let binding", self.pos_str());
        }
        self.advance();
        self.skip_ws();

        let val = if self.at_terminator() {
            Value::List(vec![])
        } else {
            self.value()?
        };

        Ok(Command::Bind(Binding::Let {
            name,
            value: val,
            mutable,
            export,
            type_ann,
        }))
    }

    fn return_cmd(&mut self) -> Result<Command> {
        self.skip_ws();
        if self.at_terminator() {
            Ok(Command::Return(None))
        } else {
            let val = self.value()?;
            Ok(Command::Return(Some(val)))
        }
    }

    /// Parse type annotation after `:`.
    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation> {
        // [Type] sugar for List[Type]
        if self.peek() == Some('[') {
            self.advance();
            self.skip_ws();
            let inner = self.parse_type_annotation()?;
            self.skip_ws();
            if self.peek() != Some(']') {
                bail!("{}: expected ']' in type annotation", self.pos_str());
            }
            self.advance();
            return Ok(TypeAnnotation::List(Some(Box::new(inner))));
        }

        let type_name = self
            .read_type_name()
            .ok_or_else(|| anyhow::anyhow!("{}: expected type name", self.pos_str()))?;

        match type_name.as_str() {
            "Unit" => Ok(TypeAnnotation::Unit),
            "Bool" => Ok(TypeAnnotation::Bool),
            "Int" => Ok(TypeAnnotation::Int),
            "Str" => Ok(TypeAnnotation::Str),
            "Path" => Ok(TypeAnnotation::Path),
            "ExitCode" => Ok(TypeAnnotation::ExitCode),
            "List" => {
                if self.peek() == Some('[') {
                    self.advance();
                    self.skip_ws();
                    let inner = self.parse_type_annotation()?;
                    self.skip_ws();
                    if self.peek() != Some(']') {
                        bail!("{}: expected ']' after List element type", self.pos_str());
                    }
                    self.advance();
                    Ok(TypeAnnotation::List(Some(Box::new(inner))))
                } else {
                    Ok(TypeAnnotation::List(None))
                }
            }
            "Result" => {
                if self.peek() != Some('[') {
                    bail!("{}: expected '[' after Result", self.pos_str());
                }
                self.advance();
                self.skip_ws();
                let inner = self.parse_type_annotation()?;
                self.skip_ws();
                if self.peek() != Some(']') {
                    bail!(
                        "{}: expected ']' after Result type parameter",
                        self.pos_str()
                    );
                }
                self.advance();
                Ok(TypeAnnotation::Result(Box::new(inner)))
            }
            "Maybe" => {
                if self.peek() != Some('[') {
                    bail!("{}: expected '[' after Maybe", self.pos_str());
                }
                self.advance();
                self.skip_ws();
                let inner = self.parse_type_annotation()?;
                self.skip_ws();
                if self.peek() != Some(']') {
                    bail!(
                        "{}: expected ']' after Maybe type parameter",
                        self.pos_str()
                    );
                }
                self.advance();
                Ok(TypeAnnotation::Maybe(Box::new(inner)))
            }
            other => bail!("{}: unknown type '{other}'", self.pos_str()),
        }
    }

    /// Assignment or simple command.
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

    /// Parse a simple command: name + args.
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

    // ── Redirections ─────────────────────────────────────────

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
                    if self.peek() == Some('<') {
                        self.advance();
                        if self.peek() == Some('<') {
                            // <<< here-string
                            self.advance();
                            self.skip_ws();
                            let word = self.word()?;
                            ops.push(RedirectOp::Input {
                                fd: 0,
                                target: RedirectTarget::HereString(word),
                            });
                        } else {
                            // << here-document
                            self.skip_ws();
                            // Check for quoted delimiter: <<'EOF'
                            let (delim, _quoted) = if self.peek() == Some('\'') {
                                self.advance();
                                let d = self.read_quoted_string()?;
                                (d, true)
                            } else {
                                let d = self.read_bare_word().ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "{}: expected delimiter after <<",
                                        self.pos_str()
                                    )
                                })?;
                                (d, false)
                            };
                            let content = self.read_heredoc(&delim)?;
                            ops.push(RedirectOp::Input {
                                fd: 0,
                                target: RedirectTarget::HereDoc(content),
                            });
                        }
                    } else if self.peek() == Some('{') {
                        // <{ process substitution — not a redirect
                        bail!("{}: unexpected '<{{' in redirect position", self.pos_str());
                    } else {
                        self.skip_ws();
                        let target = self.word()?;
                        ops.push(RedirectOp::Input {
                            fd: 0,
                            target: RedirectTarget::File(target),
                        });
                    }
                }
                _ => break,
            }
        }
        // Wrap in reverse for left-to-right evaluation order
        let mut result = expr;
        for op in ops.into_iter().rev() {
            result = Expr::Redirect(Box::new(result), op);
        }
        Ok(result)
    }

    /// Parse what follows '>' — handles >>, >[fd], >[fd=fd], >[fd=], >target.
    fn parse_output_redirect(&mut self) -> Result<RedirectOp> {
        // >> append
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

        // >[fd...] — fd redirections
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
                // >[fd=] (close) or >[fd=src] (dup)
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

        // > target (fd defaults to 1)
        self.skip_ws();
        let target = self.word()?;
        Ok(RedirectOp::Output {
            fd: 1,
            target: RedirectTarget::File(target),
            append: false,
        })
    }

    // ── Layer 6: Program ─────────────────────────────────────

    /// program = terminator* (command terminator+)* command?
    fn program(&mut self) -> Result<Vec<Command>> {
        self.command_list()
    }
}

// ═══════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Program {
        PshParser::parse(input).expect("parse failed")
    }

    fn parse_err(input: &str) -> String {
        PshParser::parse(input).unwrap_err().to_string()
    }

    // ── Layer 1: lexical predicates ──────────────────────────

    #[test]
    fn var_char_alpha_digit_underscore_star() {
        assert!(is_var_char('a'));
        assert!(is_var_char('Z'));
        assert!(is_var_char('0'));
        assert!(is_var_char('_'));
        assert!(is_var_char('*'));
        assert!(!is_var_char('.'));
        assert!(!is_var_char('/'));
        assert!(!is_var_char('@'));
        assert!(!is_var_char('~'));
    }

    #[test]
    fn word_char_includes_dot_slash_at() {
        assert!(is_word_char('.'));
        assert!(is_word_char('/'));
        assert!(is_word_char('@'));
        assert!(is_word_char('-'));
        assert!(is_word_char(':'));
        assert!(is_word_char('+'));
        assert!(is_word_char(','));
        assert!(is_word_char('%'));
        assert!(is_word_char('?'));
        assert!(is_word_char('['));
        assert!(is_word_char(']'));
        assert!(!is_word_char('~'));
        assert!(!is_word_char('$'));
        assert!(!is_word_char('\''));
        assert!(!is_word_char('|'));
    }

    #[test]
    fn keyword_detection() {
        assert!(is_keyword("if"));
        assert!(is_keyword("for"));
        assert!(is_keyword("match"));
        assert!(is_keyword("try"));
        assert!(is_keyword("return"));
        assert!(!is_keyword("echo"));
        assert!(!is_keyword("case"));
    }

    // ── Layer 2: word atoms ──────────────────────────────────

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
    fn quoted_string() {
        let prog = parse("echo 'hello world'");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Quoted("hello world".into()));
            }
            other => panic!("expected quoted arg, got {other:?}"),
        }
    }

    #[test]
    fn quoted_with_escape() {
        let prog = parse("echo 'it''s'");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Quoted("it's".into()));
            }
            other => panic!("expected quoted with escape, got {other:?}"),
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
    fn stringify_parse() {
        let prog = parse("echo $\"x");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Stringify("x".into()));
            }
            other => panic!("expected stringify word, got {other:?}"),
        }
    }

    #[test]
    fn brace_variable() {
        let prog = parse("echo ${x.get}");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Var("x.get".into()));
            }
            other => panic!("expected brace var, got {other:?}"),
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
    fn process_sub_parse() {
        let prog = parse("cat <{echo hello}");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.name, Word::Literal("cat".into()));
                assert_eq!(cmd.args.len(), 1);
                assert!(matches!(&cmd.args[0], Word::ProcessSub(_)));
            }
            other => panic!("expected simple command with process sub, got {other:?}"),
        }
    }

    #[test]
    fn tilde_bare() {
        let prog = parse("echo ~");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Literal("~".into()));
            }
            other => panic!("expected tilde literal, got {other:?}"),
        }
    }

    #[test]
    fn tilde_with_path() {
        let prog = parse("echo ~/bin");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Literal("~/bin".into()));
            }
            other => panic!("expected tilde path, got {other:?}"),
        }
    }

    // ── Layer 3: words with free carets ──────────────────────

    #[test]
    fn explicit_caret() {
        let prog = parse("echo foo^bar");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert!(matches!(&cmd.args[0], Word::Concat(_)));
            }
            other => panic!("expected concat, got {other:?}"),
        }
    }

    #[test]
    fn free_caret_var_then_literal() {
        // $home/bin → $home ^ /bin (free caret at var_char boundary)
        let prog = parse("echo $home/bin");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => match &cmd.args[0] {
                Word::Concat(parts) => {
                    assert_eq!(parts.len(), 2);
                    assert_eq!(parts[0], Word::Var("home".into()));
                    assert_eq!(parts[1], Word::Literal("/bin".into()));
                }
                other => panic!("expected concat, got {other:?}"),
            },
            other => panic!("expected simple cmd, got {other:?}"),
        }
    }

    #[test]
    fn free_caret_var_dot_ext() {
        // $file.c → $file ^ .c
        let prog = parse("echo $file.c");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => match &cmd.args[0] {
                Word::Concat(parts) => {
                    assert_eq!(parts.len(), 2);
                    assert_eq!(parts[0], Word::Var("file".into()));
                    assert_eq!(parts[1], Word::Literal(".c".into()));
                }
                other => panic!("expected concat, got {other:?}"),
            },
            other => panic!("expected simple cmd, got {other:?}"),
        }
    }

    #[test]
    fn free_caret_quoted_then_var() {
        // 'hello'$name → 'hello' ^ $name
        let prog = parse("echo 'hello'$name");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => match &cmd.args[0] {
                Word::Concat(parts) => {
                    assert_eq!(parts.len(), 2);
                    assert_eq!(parts[0], Word::Quoted("hello".into()));
                    assert_eq!(parts[1], Word::Var("name".into()));
                }
                other => panic!("expected concat, got {other:?}"),
            },
            other => panic!("expected simple cmd, got {other:?}"),
        }
    }

    #[test]
    fn free_caret_user_at_host() {
        // $user@$host → $user ^ @ ^ $host
        let prog = parse("echo $user@$host");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => match &cmd.args[0] {
                Word::Concat(parts) => {
                    assert_eq!(parts.len(), 3);
                    assert_eq!(parts[0], Word::Var("user".into()));
                    assert_eq!(parts[1], Word::Literal("@".into()));
                    assert_eq!(parts[2], Word::Var("host".into()));
                }
                other => panic!("expected concat, got {other:?}"),
            },
            other => panic!("expected simple cmd, got {other:?}"),
        }
    }

    #[test]
    fn list_value() {
        let prog = parse("x = (a b c)");
        match &prog.commands[0] {
            Command::Bind(Binding::Assignment(_, Value::List(items))) => {
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected list assignment, got {other:?}"),
        }
    }

    #[test]
    fn empty_list() {
        let prog = parse("x = ()");
        match &prog.commands[0] {
            Command::Bind(Binding::Assignment(_, Value::List(items))) => {
                assert_eq!(items.len(), 0);
            }
            other => panic!("expected empty list, got {other:?}"),
        }
    }

    // ── Layer 4: expression precedence ───────────────────────

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
    fn and_or() {
        let prog = parse("test -f foo && echo yes || echo no");
        match &prog.commands[0] {
            Command::Exec(Expr::Or(_, _)) => {}
            other => panic!("expected or, got {other:?}"),
        }
    }

    #[test]
    fn negation() {
        let prog = parse("! test -f foo");
        match &prog.commands[0] {
            Command::Exec(Expr::Not(_)) => {}
            other => panic!("expected not, got {other:?}"),
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
    fn coprocess() {
        let prog = parse("cmd |&");
        match &prog.commands[0] {
            Command::Exec(Expr::Coprocess(_)) => {}
            other => panic!("expected coprocess, got {other:?}"),
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

    // ── Layer 5: commands ────────────────────────────────────

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
    fn if_else_if() {
        let prog = parse("if cond1 { A } else if cond2 { B } else { C }");
        match &prog.commands[0] {
            Command::If {
                else_body: Some(else_cmds),
                ..
            } => {
                assert_eq!(else_cmds.len(), 1);
                assert!(matches!(&else_cmds[0], Command::If { .. }));
            }
            other => panic!("expected if/else if, got {other:?}"),
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
    fn ref_parse() {
        let prog = parse("ref y = x");
        match &prog.commands[0] {
            Command::Bind(Binding::Ref { name, target }) => {
                assert_eq!(name, "y");
                assert_eq!(target, "x");
            }
            other => panic!("expected ref binding, got {other:?}"),
        }
    }

    #[test]
    fn ref_parse_path() {
        let prog = parse("ref cursor = /pane/editor/attrs/cursor");
        match &prog.commands[0] {
            Command::Bind(Binding::Ref { name, target }) => {
                assert_eq!(name, "cursor");
                assert_eq!(target, "/pane/editor/attrs/cursor");
            }
            other => panic!("expected ref binding with path, got {other:?}"),
        }
    }

    // ── Let bindings ─────────────────────────────────────────

    #[test]
    fn let_basic() {
        let prog = parse("let x = 42");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                name,
                mutable,
                export,
                type_ann,
                ..
            }) => {
                assert_eq!(name, "x");
                assert!(!mutable);
                assert!(!export);
                assert!(type_ann.is_none());
            }
            other => panic!("expected let binding, got {other:?}"),
        }
    }

    #[test]
    fn let_mut() {
        let prog = parse("let mut x = hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                name,
                mutable,
                export,
                ..
            }) => {
                assert_eq!(name, "x");
                assert!(mutable);
                assert!(!export);
            }
            other => panic!("expected let mut binding, got {other:?}"),
        }
    }

    #[test]
    fn let_export() {
        let prog = parse("let export x = hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                name,
                export,
                mutable,
                ..
            }) => {
                assert_eq!(name, "x");
                assert!(export);
                assert!(!mutable);
            }
            other => panic!("expected let export binding, got {other:?}"),
        }
    }

    #[test]
    fn let_mut_export() {
        let prog = parse("let mut export x = hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                mutable, export, ..
            }) => {
                assert!(mutable);
                assert!(export);
            }
            other => panic!("expected let mut export binding, got {other:?}"),
        }
    }

    #[test]
    fn let_typed() {
        let prog = parse("let x : Int = 42");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { name, type_ann, .. }) => {
                assert_eq!(name, "x");
                assert_eq!(type_ann, &Some(TypeAnnotation::Int));
            }
            other => panic!("expected typed let binding, got {other:?}"),
        }
    }

    #[test]
    fn let_list_typed() {
        let prog = parse("let x : List[Int] = (1 2 3)");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { type_ann, .. }) => {
                assert_eq!(
                    type_ann,
                    &Some(TypeAnnotation::List(Some(Box::new(TypeAnnotation::Int))))
                );
            }
            other => panic!("expected List[Int] typed let, got {other:?}"),
        }
    }

    #[test]
    fn let_bracket_sugar() {
        let prog = parse("let x : [Int] = (1 2)");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { type_ann, .. }) => {
                assert_eq!(
                    type_ann,
                    &Some(TypeAnnotation::List(Some(Box::new(TypeAnnotation::Int))))
                );
            }
            other => panic!("expected [Int] sugar typed let, got {other:?}"),
        }
    }

    #[test]
    fn let_quoted_word() {
        let prog = parse("let x = '42'");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                value: Value::Word(Word::Quoted(s)),
                ..
            }) => {
                assert_eq!(s, "42");
            }
            other => panic!("expected let with quoted word, got {other:?}"),
        }
    }

    #[test]
    fn quoted_stays_quoted() {
        let prog = parse("echo '42'");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.args[0], Word::Quoted("42".into()));
            }
            other => panic!("expected quoted arg, got {other:?}"),
        }
    }

    // ── Match command ────────────────────────────────────────

    #[test]
    fn match_cmd() {
        // Old prototype syntax (case keyword + braced bodies)
        let prog = parse("match $x { case foo { echo foo } case * { echo other } }");
        match &prog.commands[0] {
            Command::Match { value: _, arms } => {
                assert_eq!(arms.len(), 2);
                assert!(matches!(&arms[1].0[0], Pattern::Star));
            }
            other => panic!("expected match, got {other:?}"),
        }
    }

    #[test]
    fn match_new_syntax_glob() {
        let prog = parse("match $x { foo => echo foo; * => echo other }");
        match &prog.commands[0] {
            Command::Match { value: _, arms } => {
                assert_eq!(arms.len(), 2);
                assert_eq!(arms[0].0[0], Pattern::Literal("foo".into()));
                assert!(matches!(&arms[1].0[0], Pattern::Star));
            }
            other => panic!("expected match with new syntax, got {other:?}"),
        }
    }

    #[test]
    fn match_new_syntax_structural() {
        let prog = parse("match $result { ok $v => echo $v; err $e => echo $e }");
        match &prog.commands[0] {
            Command::Match { value: _, arms } => {
                assert_eq!(arms.len(), 2);
                assert!(matches!(&arms[0].0[0], Pattern::Structural { tag, binding }
                    if tag == "ok" && binding == "v"));
                assert!(matches!(&arms[1].0[0], Pattern::Structural { tag, binding }
                    if tag == "err" && binding == "e"));
            }
            other => panic!("expected structural match, got {other:?}"),
        }
    }

    #[test]
    fn match_multi_pattern_glob() {
        let prog = parse("match $f { *.txt *.md => echo text; * => echo other }");
        match &prog.commands[0] {
            Command::Match { value: _, arms } => {
                assert_eq!(arms[0].0.len(), 2);
                assert_eq!(arms[0].0[0], Pattern::Glob("*.txt".into()));
                assert_eq!(arms[0].0[1], Pattern::Glob("*.md".into()));
            }
            other => panic!("expected multi-pattern match, got {other:?}"),
        }
    }

    #[test]
    fn match_with_braced_bodies() {
        let prog = parse("match $x {\n  foo => { echo foo };\n  * => { echo other }\n}");
        match &prog.commands[0] {
            Command::Match { value: _, arms } => {
                assert_eq!(arms.len(), 2);
            }
            other => panic!("expected match with braced bodies, got {other:?}"),
        }
    }

    #[test]
    fn match_trailing_semicolon() {
        let prog = parse("match $x { foo => echo foo; }");
        match &prog.commands[0] {
            Command::Match { value: _, arms } => {
                assert_eq!(arms.len(), 1);
            }
            other => panic!("expected match with trailing semicolon, got {other:?}"),
        }
    }

    #[test]
    fn match_newlines_trivia() {
        let prog = parse("match $x {\n  foo => echo foo\n  bar => echo bar\n}");
        match &prog.commands[0] {
            Command::Match { value: _, arms } => {
                assert_eq!(arms.len(), 2);
            }
            other => panic!("expected match with newline trivia, got {other:?}"),
        }
    }

    // ── Return ───────────────────────────────────────────────

    #[test]
    fn return_with_value() {
        let prog = parse("return hello");
        match &prog.commands[0] {
            Command::Return(Some(Value::Word(Word::Literal(s)))) => {
                assert_eq!(s, "hello");
            }
            other => panic!("expected return with value, got {other:?}"),
        }
    }

    #[test]
    fn return_without_value() {
        let prog = parse("return");
        match &prog.commands[0] {
            Command::Return(None) => {}
            other => panic!("expected return without value, got {other:?}"),
        }
    }

    // ── Planned stubs ────────────────────────────────────────

    #[test]
    fn try_not_yet_implemented() {
        let err = parse_err("try { echo hello }");
        assert!(err.contains("try blocks not yet implemented"));
    }

    // ── Redirections ─────────────────────────────────────────

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
    fn redirect_append() {
        let prog = parse("echo hello >> file");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Output {
                    fd: 1,
                    append: true,
                    ..
                },
            )) => {}
            other => panic!("expected append redirect, got {other:?}"),
        }
    }

    #[test]
    fn redirect_input() {
        let prog = parse("cmd < file");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Input {
                    fd: 0,
                    target: RedirectTarget::File(_),
                },
            )) => {}
            other => panic!("expected input redirect, got {other:?}"),
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
    fn heredoc_parse() {
        let prog = parse("cat <<EOF\nhello\nworld\nEOF\n");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Input {
                    fd: 0,
                    target: RedirectTarget::HereDoc(content),
                },
            )) => {
                assert_eq!(content, "hello\nworld\n");
            }
            other => panic!("expected heredoc redirect, got {other:?}"),
        }
    }

    #[test]
    fn heredoc_quoted_delimiter() {
        let prog = parse("cat <<'EOF'\nhello $x\nEOF\n");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Input {
                    fd: 0,
                    target: RedirectTarget::HereDoc(content),
                },
            )) => {
                assert_eq!(content, "hello $x\n");
            }
            other => panic!("expected heredoc with quoted delimiter, got {other:?}"),
        }
    }

    #[test]
    fn herestring_parse() {
        let prog = parse("cat <<<hello");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Input {
                    fd: 0,
                    target: RedirectTarget::HereString(Word::Literal(s)),
                },
            )) => {
                assert_eq!(s, "hello");
            }
            other => panic!("expected herestring redirect, got {other:?}"),
        }
    }

    // ── Layer 6: program structure ───────────────────────────

    #[test]
    fn multiple_commands() {
        let prog = parse("echo a; echo b\necho c");
        assert_eq!(prog.commands.len(), 3);
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
    fn line_continuation() {
        let prog = parse("echo \\\nhello");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.name, Word::Literal("echo".into()));
                assert_eq!(cmd.args[0], Word::Literal("hello".into()));
            }
            other => panic!("expected simple cmd with line continuation, got {other:?}"),
        }
    }

    // ── Type annotations ─────────────────────────────────────

    #[test]
    fn type_ann_exit_code() {
        let prog = parse("let x : ExitCode = 0");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { type_ann, .. }) => {
                assert_eq!(type_ann, &Some(TypeAnnotation::ExitCode));
            }
            other => panic!("expected ExitCode type, got {other:?}"),
        }
    }

    #[test]
    fn type_ann_result() {
        let prog = parse("let x : Result[Int] = 42");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { type_ann, .. }) => {
                assert_eq!(
                    type_ann,
                    &Some(TypeAnnotation::Result(Box::new(TypeAnnotation::Int)))
                );
            }
            other => panic!("expected Result[Int] type, got {other:?}"),
        }
    }

    #[test]
    fn type_ann_maybe() {
        let prog = parse("let x : Maybe[Str] = hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { type_ann, .. }) => {
                assert_eq!(
                    type_ann,
                    &Some(TypeAnnotation::Maybe(Box::new(TypeAnnotation::Str)))
                );
            }
            other => panic!("expected Maybe[Str] type, got {other:?}"),
        }
    }

    // ── Edge cases ───────────────────────────────────────────

    #[test]
    fn keyword_prefix_not_keyword() {
        // "iffy" should parse as a command, not "if" + "fy"
        let prog = parse("iffy");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.name, Word::Literal("iffy".into()));
            }
            other => panic!("expected simple command 'iffy', got {other:?}"),
        }
    }

    #[test]
    fn keyword_prefix_for_like() {
        // "fortune" should parse as a command, not "for" + error
        let prog = parse("fortune");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.name, Word::Literal("fortune".into()));
            }
            other => panic!("expected simple command 'fortune', got {other:?}"),
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
    fn empty_assignment() {
        let prog = parse("x =");
        match &prog.commands[0] {
            Command::Bind(Binding::Assignment(_, Value::List(items))) => {
                assert!(items.is_empty());
            }
            other => panic!("expected empty assignment, got {other:?}"),
        }
    }

    #[test]
    fn inline_comment() {
        let prog = parse("echo hello # this is a comment");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(cmd)) => {
                assert_eq!(cmd.name, Word::Literal("echo".into()));
                assert_eq!(cmd.args.len(), 1);
                assert_eq!(cmd.args[0], Word::Literal("hello".into()));
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }
}
