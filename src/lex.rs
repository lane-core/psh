//! Lexer for psh.
//!
//! Tokenizes input into a stream for the parser. Handles rc's
//! quoting rules (single quotes only, '' to escape) and the
//! context-sensitive boundaries between words, operators, and
//! whitespace.

use std::fmt;

/// Source position for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// A token in the psh grammar.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Words and values
    Word(String),          // bare word or quoted string
    Dollar,                // $
    DollarHash,            // $# (count prefix)
    Backtick,              // ` (command substitution start)
    Caret,                 // ^ (concatenation)

    // Operators
    Pipe,                  // |
    PipeAnd,               // |& (coprocess)
    And,                   // &&
    Or,                    // ||
    Bang,                  // !
    Amp,                   // & (background)

    // Redirections
    Less,                  // <
    Great,                 // >
    GreatGreat,            // >> (append)

    // Grouping
    LBrace,                // {
    RBrace,                // }
    LParen,                // (
    RParen,                // )
    AtLBrace,              // @{ (subshell)

    // Assignment
    Equals,                // =

    // Separators
    Semi,                  // ;
    Newline,               // \n (significant in shell grammar)

    // Keywords
    If,
    Else,
    For,
    In,
    Switch,
    Case,
    Fn,
    While,
    Ref,                   // name references

    // Special
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Word(s) => write!(f, "'{s}'"),
            Token::Dollar => write!(f, "$"),
            Token::DollarHash => write!(f, "$#"),
            Token::Backtick => write!(f, "`"),
            Token::Caret => write!(f, "^"),
            Token::Pipe => write!(f, "|"),
            Token::PipeAnd => write!(f, "|&"),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Bang => write!(f, "!"),
            Token::Amp => write!(f, "&"),
            Token::Less => write!(f, "<"),
            Token::Great => write!(f, ">"),
            Token::GreatGreat => write!(f, ">>"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::AtLBrace => write!(f, "@{{"),
            Token::Equals => write!(f, "="),
            Token::Semi => write!(f, ";"),
            Token::Newline => write!(f, "\\n"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::For => write!(f, "for"),
            Token::In => write!(f, "in"),
            Token::Switch => write!(f, "switch"),
            Token::Case => write!(f, "case"),
            Token::Fn => write!(f, "fn"),
            Token::While => write!(f, "while"),
            Token::Ref => write!(f, "ref"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

/// A positioned token.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub token: Token,
    pub pos: Pos,
}

/// Lexer state.
pub struct Lexer<'a> {
    input: &'a [u8],
    offset: usize,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input: input.as_bytes(),
            offset: 0,
            line: 1,
            col: 1,
        }
    }

    fn pos(&self) -> Pos {
        Pos {
            line: self.line,
            col: self.col,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.offset).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.input.get(self.offset + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.input.get(self.offset).copied()?;
        self.offset += 1;
        if ch == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r') => {
                    self.advance();
                }
                Some(b'#') => {
                    // Comment: skip to end of line
                    while let Some(ch) = self.peek() {
                        if ch == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn is_word_char(ch: u8) -> bool {
        matches!(ch,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' |
            b'_' | b'-' | b'.' | b'/' | b':' | b'+' |
            b',' | b'@' | b'%' | b'~' | b'*' | b'?'  |
            b'[' | b']'
        )
    }

    fn read_word(&mut self) -> String {
        let mut word = String::new();
        while let Some(ch) = self.peek() {
            if Self::is_word_char(ch) {
                word.push(ch as char);
                self.advance();
            } else {
                break;
            }
        }
        word
    }

    fn read_quoted(&mut self) -> anyhow::Result<String> {
        // Opening quote already consumed
        let mut s = String::new();
        loop {
            match self.advance() {
                Some(b'\'') => {
                    // '' inside quotes = literal quote
                    if self.peek() == Some(b'\'') {
                        self.advance();
                        s.push('\'');
                    } else {
                        return Ok(s);
                    }
                }
                Some(ch) => s.push(ch as char),
                None => anyhow::bail!("unterminated quoted string"),
            }
        }
    }

    fn classify_word(word: &str) -> Token {
        match word {
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "switch" => Token::Switch,
            "case" => Token::Case,
            "fn" => Token::Fn,
            "while" => Token::While,
            "ref" => Token::Ref,
            _ => Token::Word(word.to_string()),
        }
    }

    /// Tokenize the next token.
    pub fn next_token(&mut self) -> anyhow::Result<Spanned> {
        self.skip_whitespace_and_comments();

        let pos = self.pos();

        let ch = match self.peek() {
            Some(ch) => ch,
            None => {
                return Ok(Spanned {
                    token: Token::Eof,
                    pos,
                })
            }
        };

        let token = match ch {
            b'\n' => {
                self.advance();
                Token::Newline
            }
            b'\'' => {
                self.advance();
                let s = self.read_quoted()?;
                Token::Word(s)
            }
            b'$' => {
                self.advance();
                if self.peek() == Some(b'#') {
                    self.advance();
                    Token::DollarHash
                } else {
                    Token::Dollar
                }
            }
            b'`' => {
                self.advance();
                Token::Backtick
            }
            b'^' => {
                self.advance();
                Token::Caret
            }
            b'|' => {
                self.advance();
                match self.peek() {
                    Some(b'|') => {
                        self.advance();
                        Token::Or
                    }
                    Some(b'&') => {
                        self.advance();
                        Token::PipeAnd
                    }
                    _ => Token::Pipe,
                }
            }
            b'&' => {
                self.advance();
                match self.peek() {
                    Some(b'&') => {
                        self.advance();
                        Token::And
                    }
                    _ => Token::Amp,
                }
            }
            b'!' => {
                self.advance();
                Token::Bang
            }
            b'<' => {
                self.advance();
                Token::Less
            }
            b'>' => {
                self.advance();
                match self.peek() {
                    Some(b'>') => {
                        self.advance();
                        Token::GreatGreat
                    }
                    _ => Token::Great,
                }
            }
            b'{' => {
                self.advance();
                Token::LBrace
            }
            b'}' => {
                self.advance();
                Token::RBrace
            }
            b'(' => {
                self.advance();
                Token::LParen
            }
            b')' => {
                self.advance();
                Token::RParen
            }
            b'@' if self.peek2() == Some(b'{') => {
                self.advance();
                self.advance();
                Token::AtLBrace
            }
            b'=' => {
                self.advance();
                Token::Equals
            }
            b';' => {
                self.advance();
                Token::Semi
            }
            _ if Self::is_word_char(ch) => {
                let word = self.read_word();
                Self::classify_word(&word)
            }
            _ => {
                self.advance();
                anyhow::bail!("unexpected character '{}' at {pos}", ch as char);
            }
        };

        Ok(Spanned { token, pos })
    }

    /// Tokenize all remaining input.
    pub fn tokenize_all(&mut self) -> anyhow::Result<Vec<Spanned>> {
        let mut tokens = Vec::new();
        loop {
            let spanned = self.next_token()?;
            if spanned.token == Token::Eof {
                tokens.push(spanned);
                break;
            }
            tokens.push(spanned);
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: &str) -> Vec<Token> {
        Lexer::new(input)
            .tokenize_all()
            .unwrap()
            .into_iter()
            .map(|s| s.token)
            .filter(|t| !matches!(t, Token::Eof))
            .collect()
    }

    #[test]
    fn simple_command() {
        assert_eq!(
            tokens("echo hello world"),
            vec![
                Token::Word("echo".into()),
                Token::Word("hello".into()),
                Token::Word("world".into()),
            ]
        );
    }

    #[test]
    fn pipeline() {
        assert_eq!(
            tokens("ls | grep foo"),
            vec![
                Token::Word("ls".into()),
                Token::Pipe,
                Token::Word("grep".into()),
                Token::Word("foo".into()),
            ]
        );
    }

    #[test]
    fn variable() {
        assert_eq!(
            tokens("echo $x"),
            vec![
                Token::Word("echo".into()),
                Token::Dollar,
                Token::Word("x".into()),
            ]
        );
    }

    #[test]
    fn quoted_string() {
        assert_eq!(
            tokens("echo 'hello world'"),
            vec![
                Token::Word("echo".into()),
                Token::Word("hello world".into()),
            ]
        );
    }

    #[test]
    fn escaped_quote() {
        assert_eq!(
            tokens("echo 'it''s'"),
            vec![
                Token::Word("echo".into()),
                Token::Word("it's".into()),
            ]
        );
    }

    #[test]
    fn assignment() {
        assert_eq!(
            tokens("x = hello"),
            vec![
                Token::Word("x".into()),
                Token::Equals,
                Token::Word("hello".into()),
            ]
        );
    }

    #[test]
    fn list_assignment() {
        assert_eq!(
            tokens("x = (a b c)"),
            vec![
                Token::Word("x".into()),
                Token::Equals,
                Token::LParen,
                Token::Word("a".into()),
                Token::Word("b".into()),
                Token::Word("c".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn redirection() {
        assert_eq!(
            tokens("cmd > file"),
            vec![
                Token::Word("cmd".into()),
                Token::Great,
                Token::Word("file".into()),
            ]
        );
    }

    #[test]
    fn append_redirection() {
        assert_eq!(
            tokens("cmd >> file"),
            vec![
                Token::Word("cmd".into()),
                Token::GreatGreat,
                Token::Word("file".into()),
            ]
        );
    }

    #[test]
    fn background() {
        assert_eq!(
            tokens("cmd &"),
            vec![Token::Word("cmd".into()), Token::Amp]
        );
    }

    #[test]
    fn coprocess() {
        assert_eq!(
            tokens("cmd |&"),
            vec![Token::Word("cmd".into()), Token::PipeAnd]
        );
    }

    #[test]
    fn keywords() {
        assert_eq!(
            tokens("if fn for in switch case"),
            vec![
                Token::If,
                Token::Fn,
                Token::For,
                Token::In,
                Token::Switch,
                Token::Case,
            ]
        );
    }

    #[test]
    fn comments() {
        assert_eq!(
            tokens("echo hello # this is a comment\necho world"),
            vec![
                Token::Word("echo".into()),
                Token::Word("hello".into()),
                Token::Newline,
                Token::Word("echo".into()),
                Token::Word("world".into()),
            ]
        );
    }

    #[test]
    fn subshell() {
        assert_eq!(
            tokens("@{ cmd }"),
            vec![
                Token::AtLBrace,
                Token::Word("cmd".into()),
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn fn_with_discipline() {
        assert_eq!(
            tokens("fn x.get { echo hello }"),
            vec![
                Token::Fn,
                Token::Word("x.get".into()),
                Token::LBrace,
                Token::Word("echo".into()),
                Token::Word("hello".into()),
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn complex_pipeline() {
        assert_eq!(
            tokens("cat file | grep pattern | sort -u > output"),
            vec![
                Token::Word("cat".into()),
                Token::Word("file".into()),
                Token::Pipe,
                Token::Word("grep".into()),
                Token::Word("pattern".into()),
                Token::Pipe,
                Token::Word("sort".into()),
                Token::Word("-u".into()),
                Token::Great,
                Token::Word("output".into()),
            ]
        );
    }
}
