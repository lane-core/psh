//! Parser for psh — combine 4 implementation.
//!
//! Six-layer architecture per docs/syntax.md:
//!   L1: Lexical primitives (var_char, word_char, whitespace, terminators)
//!   L2: Word atoms (quoted, variables, command sub, process sub, tilde, lambda)
//!   L3: Words with free carets (implicit ^ on adjacency)
//!   L4: Expression precedence tower (or, and, pipeline, cmd_expr)
//!   L5: Commands (if, for, while, match, try, fn, let, ref, return)
//!   L6: Program (terminator-separated command sequence)
//!
//! rc heritage for the grammar. combine for the combinator substrate.

use combine::{
    attempt, choice,
    error::ParseError,
    many, many1, not_followed_by,
    parser::{
        char::{char as ch, string},
        token::satisfy,
    },
    skip_many, Parser as CombineParser, Stream,
};

use crate::ast::*;

// ── Layer 1: Lexical primitives ────────────────────────────────

/// var_char = [a-zA-Z0-9_*]
pub(crate) fn is_var_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '*'
}

/// word_char = [a-zA-Z0-9_\-./+:,%*?\[\]@]
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '_' | '-' | '.' | '/' | '+' | ':' | ',' | '%' | '*' | '?' | '[' | ']' | '@'
        )
}

/// Can the character start a new word atom? (for free caret rule)
fn can_start_atom(c: char) -> bool {
    c == '$' || c == '\'' || c == '`' || is_word_char(c)
}

/// Parse one var_char.
fn var_char_<I: Stream<Token = char>>() -> impl CombineParser<I, Output = char> {
    satisfy(is_var_char)
}

/// Horizontal whitespace only (space, tab, CR). Not newlines.
fn hspace<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    satisfy(|c: char| c == ' ' || c == '\t' || c == '\r').map(|_| ())
}

/// Comment: # to end of line (doesn't consume the newline).
fn comment<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    ch('#').with(skip_many(satisfy(|c: char| c != '\n')))
}

/// Line continuation: backslash + newline consumed as whitespace.
/// Uses attempt() because \ alone is a lambda introducer — we must
/// not commit on the \ until we've confirmed the \n follows.
fn line_cont<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    attempt(ch('\\').with(ch('\n'))).map(|_| ())
}

/// Skip trivia: horizontal whitespace, comments, line continuations.
/// Does NOT skip newlines (those are terminators).
fn trivia<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    skip_many(choice!(hspace(), comment(), line_cont()))
}

/// Skip trivia including newlines — used inside match { } and braces.
fn full_trivia<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    skip_many(choice!(
        hspace(),
        comment(),
        line_cont(),
        ch('\n').map(|_| ())
    ))
}

/// A keyword followed by boundary (not followed by var_char).
fn keyword<I: Stream<Token = char>>(
    kw: &'static str,
) -> impl CombineParser<I, Output = &'static str> {
    attempt(string(kw).skip(not_followed_by(var_char_())))
}

/// VARNAME = var_char+
fn varname<I: Stream<Token = char>>() -> impl CombineParser<I, Output = String> {
    many1(var_char_())
}

/// NAME for word positions = word_char+
fn wname<I: Stream<Token = char>>() -> impl CombineParser<I, Output = String> {
    many1(satisfy(is_word_char))
}

// ── Layer 2: Word atoms ────────────────────────────────────────

/// Quoted string: 'text' with '' escape.
fn quoted<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    // Use a manual parser because '' escape requires stateful lookahead
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        // Consume opening quote
        let _ = ch('\'').parse_stream(input).into_result()?;
        let mut text = String::new();
        loop {
            match input.uncons() {
                Ok('\'') => {
                    // Check for '' escape
                    let cp = input.checkpoint();
                    match input.uncons() {
                        Ok('\'') => text.push('\''),
                        Ok(_) => {
                            // Not '' — put back and return
                            input.reset(cp).ok();
                            return Ok((Word::Quoted(text), Commit::Commit(())));
                        }
                        Err(_) => {
                            return Ok((Word::Quoted(text), Commit::Commit(())));
                        }
                    }
                }
                Ok(c) => text.push(c),
                Err(_) => {
                    // Unterminated string — use combine's error mechanism
                    return Err(Commit::Commit(
                        <I::Error as ParseError<I::Token, I::Range, I::Position>>::empty(
                            input.position(),
                        )
                        .into(),
                    ));
                }
            }
        }
    })
}

/// Parse a single accessor: '.' followed by digit(s) or a name.
/// .0 → Index(0), .code → Code, .ok/.err/.anything → Tag.
fn accessor<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Accessor> {
    ch('.').with(choice!(
        // Digits → tuple index (0-based)
        many1::<String, _, _>(satisfy(|c: char| c.is_ascii_digit()))
            .map(|s| { Accessor::Index(s.parse::<usize>().unwrap_or(0)) }),
        // Name → code or tag
        many1::<String, _, _>(satisfy(is_var_char)).map(|s| {
            if s == "code" {
                Accessor::Code
            } else {
                Accessor::Tag(s)
            }
        })
    ))
}

/// Variable reference: $var, $#var, $"var, $var(idx), $var.acc, ${name}
fn var_ref<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    ch('$').with(choice!(
        // $#var — count
        ch('#').with(varname()).map(Word::Count),
        // $"var — stringify
        ch('"').with(varname()).map(Word::Stringify),
        // ${name} — brace-delimited, uses word_char alphabet
        ch('{')
            .with(many1::<String, _, _>(satisfy(|c: char| c != '}')))
            .skip(ch('}'))
            .map(Word::BraceVar),
        // $var with optional accessor chain or index
        combine::parser(move |input: &mut I| {
            use combine::error::Commit;

            let (name, _) = varname().parse_stream(input).into_result()?;

            // Attempt accessor chain: .digit or .name
            let mut accs = Vec::new();
            loop {
                let cp = input.checkpoint();
                match attempt(accessor()).parse_stream(input).into_result() {
                    Ok((acc, _)) => accs.push(acc),
                    Err(_) => {
                        input.reset(cp).ok();
                        break;
                    }
                }
            }

            if !accs.is_empty() {
                return Ok((Word::VarAccess(name, accs), Commit::Commit(())));
            }

            // Attempt $var(idx)
            let cp = input.checkpoint();
            match ch('(').parse_stream(input).into_result() {
                Ok(_) => {
                    let (idx, _) = word_inner()
                        .skip(ch(')'))
                        .parse_stream(input)
                        .into_result()?;
                    Ok((Word::Index(name, Box::new(idx)), Commit::Commit(())))
                }
                Err(_) => {
                    input.reset(cp).ok();
                    Ok((Word::Var(name), Commit::Commit(())))
                }
            }
        })
    ))
}

/// Command substitution: `{ program }
fn cmd_sub<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    attempt(ch('`').skip(ch('{')))
        .with(full_trivia())
        .with(program_inner())
        .skip(full_trivia())
        .skip(ch('}'))
        .map(|prog| Word::CommandSub(prog.commands))
}

/// Process substitution: <{ program }
fn proc_sub<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    attempt(ch('<').skip(ch('{')))
        .with(full_trivia())
        .with(program_inner())
        .skip(full_trivia())
        .skip(ch('}'))
        .map(|prog| Word::ProcessSub(prog.commands))
}

/// Output process substitution: >{ program }
fn out_proc_sub<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    attempt(ch('>').skip(ch('{')))
        .with(full_trivia())
        .with(program_inner())
        .skip(full_trivia())
        .skip(ch('}'))
        .map(|prog| Word::OutputProcessSub(prog.commands))
}

/// Tilde: ~/path or bare ~
fn tilde<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    ch('~').with(choice!(
        // ~/path
        ch('/')
            .with(many::<String, _, _>(satisfy(is_word_char)))
            .map(Word::TildePath),
        // bare ~
        combine::value(Word::Tilde)
    ))
}

/// A literal: word_char+
fn literal<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    wname().map(Word::Literal)
}

/// A single word atom (before free caret handling).
fn word_atom<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    choice!(
        quoted(),
        var_ref(),
        cmd_sub(),
        proc_sub(),
        out_proc_sub(),
        tilde(),
        literal()
    )
}

// ── Layer 3: Words with free carets ─────────────────────────────

/// word = word_atom ('^' word_atom)* with implicit ^ on adjacency.
/// A single word (not consuming leading whitespace).
fn word_inner<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Word> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (first, _) = word_atom().parse_stream(input).into_result()?;
        let mut parts = vec![first];

        loop {
            let cp = input.checkpoint();
            match input.uncons() {
                Ok('^') => {
                    // Explicit caret — parse next atom
                    let (atom, _) = word_atom().parse_stream(input).into_result()?;
                    parts.push(atom);
                }
                Ok(c) if can_start_atom(c) => {
                    // Free caret — implicit concat. Put char back, parse as atom.
                    input.reset(cp).ok();
                    let (atom, _) = word_atom().parse_stream(input).into_result()?;
                    parts.push(atom);
                }
                Ok('<') => {
                    // Might be <{ for input process substitution
                    match input.uncons() {
                        Ok('{') => {
                            input.reset(cp).ok();
                            let (atom, _) = proc_sub().parse_stream(input).into_result()?;
                            parts.push(atom);
                        }
                        _ => {
                            input.reset(cp).ok();
                            break;
                        }
                    }
                }
                Ok('>') => {
                    // Might be >{ for output process substitution
                    match input.uncons() {
                        Ok('{') => {
                            input.reset(cp).ok();
                            let (atom, _) = out_proc_sub().parse_stream(input).into_result()?;
                            parts.push(atom);
                        }
                        _ => {
                            input.reset(cp).ok();
                            break;
                        }
                    }
                }
                _ => {
                    input.reset(cp).ok();
                    break;
                }
            }
        }

        let result = if parts.len() == 1 {
            parts.into_iter().next().unwrap()
        } else {
            Word::Concat(parts)
        };
        Ok((result, Commit::Commit(())))
    })
}

/// Lambda: \params => body — value-level thunk literal.
fn lambda<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Value> {
    // \ already means lambda here (line continuation is handled at trivia level)
    ch('\\').with(combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        // Parse params: () for nullary, or NAME+
        let cp = input.checkpoint();
        let params: Vec<String> = match input.uncons() {
            Ok('(') => {
                // () — nullary
                let _ = ch(')').parse_stream(input).into_result()?;
                vec![]
            }
            _ => {
                input.reset(cp).ok();
                // NAME+ — read names until we see =>
                let mut ps = Vec::new();
                loop {
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let cp2 = input.checkpoint();
                    // Check if next is =>
                    match attempt(string("=>")).parse_stream(input).into_result() {
                        Ok(_) => {
                            // Put => back — we'll consume it below
                            input.reset(cp2).ok();
                            break;
                        }
                        Err(_) => {
                            input.reset(cp2).ok();
                        }
                    }
                    let (n, _) = varname().parse_stream(input).into_result()?;
                    ps.push(n);
                }
                ps
            }
        };

        // => (arrow)
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let _ = string("=>").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;

        // lambda_body = '{' program '}' | command
        let cp = input.checkpoint();
        let body_cmds = match input.uncons() {
            Ok('{') => {
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let (prog, _) = program_inner().parse_stream(input).into_result()?;
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let _ = ch('}').parse_stream(input).into_result()?;
                prog.commands
            }
            _ => {
                input.reset(cp).ok();
                let (cmd, _) = command_().parse_stream(input).into_result()?;
                vec![cmd]
            }
        };

        Ok((
            Value::Lambda {
                params,
                body: body_cmds,
            },
            Commit::Commit(()),
        ))
    }))
}

/// value = '(' word* ')' | lambda | tagged_val | word
/// try { body } in value position — fallible capture.
fn try_value<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Value> {
    attempt(keyword("try").skip(trivia()).skip(ch('{')))
        .with(full_trivia())
        .with(program_inner())
        .skip(full_trivia())
        .skip(ch('}'))
        .map(|prog| Value::Try(prog.commands))
}

/// Value-producing block: if/match/while/for/{ } in value position.
/// Parses one control-flow command and wraps as Value::Compute.
fn compute_value<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Value> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        macro_rules! try_ctrl {
            ($parser:expr) => {{
                let cp = input.checkpoint();
                if let Ok((cmd, _)) = $parser.parse_stream(input).into_result() {
                    return Ok((Value::Compute(vec![cmd]), Commit::Commit(())));
                }
                input.reset(cp).ok();
            }};
        }

        try_ctrl!(if_cmd());
        try_ctrl!(match_cmd());
        try_ctrl!(while_cmd());
        try_ctrl!(for_cmd());

        // { block } in value position
        let cp = input.checkpoint();
        match ch('{').parse_stream(input).into_result() {
            Ok(_) => {
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let (prog, _) = program_inner().parse_stream(input).into_result()?;
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let _ = ch('}').parse_stream(input).into_result()?;
                Ok((Value::Compute(prog.commands), Commit::Commit(())))
            }
            Err(_) => {
                input.reset(cp).ok();
                Err(Commit::Peek(
                    <I::Error as ParseError<I::Token, I::Range, I::Position>>::empty(
                        input.position(),
                    )
                    .into(),
                ))
            }
        }
    })
}

fn value_<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Value> {
    choice!(
        // List literal: ( word* )
        attempt(
            ch('(')
                .with(trivia())
                .with(many::<Vec<Word>, _, _>(word_inner().skip(trivia())))
                .skip(ch(')'))
                .map(Value::List)
        ),
        // try { body } in value position
        try_value(),
        // Value-producing blocks: if/match/while/for/{ }
        attempt(compute_value()),
        // Lambda
        attempt(lambda()),
        // Single word
        word_inner().map(Value::Word)
    )
}

// ── Layer 4: Expression precedence tower ────────────────────────

/// Redirect parser — all redirect forms.
fn redirect<I: Stream<Token = char>>() -> impl CombineParser<I, Output = RedirectOp> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let cp = input.checkpoint();
        match input.uncons() {
            Ok('>') => {
                let cp2 = input.checkpoint();
                match input.uncons() {
                    Ok('>') => {
                        // >> or >>[fd] — append
                        let (_, _) = trivia().parse_stream(input).into_result()?;
                        let cp3 = input.checkpoint();
                        let fd = match input.uncons() {
                            Ok('[') => {
                                let (d, _) =
                                    many1::<String, _, _>(satisfy(|c: char| c.is_ascii_digit()))
                                        .parse_stream(input)
                                        .into_result()?;
                                let _ = ch(']').parse_stream(input).into_result()?;
                                let (_, _) = trivia().parse_stream(input).into_result()?;
                                d.parse::<u32>().unwrap_or(1)
                            }
                            _ => {
                                input.reset(cp3).ok();
                                1
                            }
                        };
                        let (target, _) = word_inner().parse_stream(input).into_result()?;
                        Ok((
                            RedirectOp::Output {
                                fd,
                                target: RedirectTarget::File(target),
                                append: true,
                            },
                            Commit::Commit(()),
                        ))
                    }
                    Ok('[') => {
                        // >[fd=fd], >[fd=], or >[fd] target
                        let (digits, _) =
                            many1::<String, _, _>(satisfy(|c: char| c.is_ascii_digit()))
                                .parse_stream(input)
                                .into_result()?;
                        let fd = digits.parse::<u32>().unwrap_or(1);
                        match input.uncons() {
                            Ok('=') => {
                                let cp_eq = input.checkpoint();
                                match input.uncons() {
                                    Ok(']') => Ok((RedirectOp::Close { fd }, Commit::Commit(()))),
                                    _ => {
                                        input.reset(cp_eq).ok();
                                        let (src_d, _) =
                                            many1::<String, _, _>(satisfy(|c: char| {
                                                c.is_ascii_digit()
                                            }))
                                            .parse_stream(input)
                                            .into_result()?;
                                        let src = src_d.parse::<u32>().unwrap_or(1);
                                        let _ = ch(']').parse_stream(input).into_result()?;
                                        Ok((RedirectOp::Dup { dst: fd, src }, Commit::Commit(())))
                                    }
                                }
                            }
                            Ok(']') => {
                                let (_, _) = trivia().parse_stream(input).into_result()?;
                                let (target, _) = word_inner().parse_stream(input).into_result()?;
                                Ok((
                                    RedirectOp::Output {
                                        fd,
                                        target: RedirectTarget::File(target),
                                        append: false,
                                    },
                                    Commit::Commit(()),
                                ))
                            }
                            _ => {
                                input.reset(cp).ok();
                                Err(Commit::Peek(I::Error::empty(input.position()).into()))
                            }
                        }
                    }
                    _ => {
                        // > target (default fd 1)
                        input.reset(cp2).ok();
                        let (_, _) = trivia().parse_stream(input).into_result()?;
                        let (target, _) = word_inner().parse_stream(input).into_result()?;
                        Ok((
                            RedirectOp::Output {
                                fd: 1,
                                target: RedirectTarget::File(target),
                                append: false,
                            },
                            Commit::Commit(()),
                        ))
                    }
                }
            }
            Ok('<') => {
                let cp2 = input.checkpoint();
                match input.uncons() {
                    Ok('<') => {
                        let cp3 = input.checkpoint();
                        match input.uncons() {
                            Ok('<') => {
                                // <<< here-string
                                let (_, _) = trivia().parse_stream(input).into_result()?;
                                let (w, _) = word_inner().parse_stream(input).into_result()?;
                                Ok((
                                    RedirectOp::Input {
                                        fd: 0,
                                        target: RedirectTarget::HereString(w),
                                    },
                                    Commit::Commit(()),
                                ))
                            }
                            _ => {
                                // << heredoc
                                input.reset(cp3).ok();
                                let (_, _) = trivia().parse_stream(input).into_result()?;
                                // Check if delimiter is quoted
                                let cp4 = input.checkpoint();
                                let (expand, delim) = match input.uncons() {
                                    Ok('\'') => {
                                        let (d, _) =
                                            many1::<String, _, _>(satisfy(|c: char| c != '\''))
                                                .parse_stream(input)
                                                .into_result()?;
                                        let _ = ch('\'').parse_stream(input).into_result()?;
                                        (false, d)
                                    }
                                    _ => {
                                        input.reset(cp4).ok();
                                        let (d, _) = many1::<String, _, _>(satisfy(|c: char| {
                                            !c.is_whitespace()
                                        }))
                                        .parse_stream(input)
                                        .into_result()?;
                                        (true, d)
                                    }
                                };
                                // Consume optional trailing whitespace + newline
                                let _ = trivia().parse_stream(input).into_result()?;
                                let _ = ch('\n').parse_stream(input).into_result()?;
                                // Read body until delimiter alone on a line
                                let mut body = String::new();
                                loop {
                                    let mut line = String::new();
                                    loop {
                                        match input.uncons() {
                                            Ok('\n') => break,
                                            Ok(c) => line.push(c),
                                            Err(_) => break,
                                        }
                                    }
                                    if line.trim() == delim {
                                        break;
                                    }
                                    body.push_str(&line);
                                    body.push('\n');
                                }
                                Ok((
                                    RedirectOp::Input {
                                        fd: 0,
                                        target: RedirectTarget::HereDoc { body, expand },
                                    },
                                    Commit::Commit(()),
                                ))
                            }
                        }
                    }
                    Ok('>') => {
                        // <> read-write
                        let (_, _) = trivia().parse_stream(input).into_result()?;
                        let (target, _) = word_inner().parse_stream(input).into_result()?;
                        Ok((
                            RedirectOp::ReadWrite {
                                fd: 0,
                                target: RedirectTarget::File(target),
                            },
                            Commit::Commit(()),
                        ))
                    }
                    Ok('[') => {
                        // <[fd] target or <[fd=fd] or <[fd=]
                        let (digits, _) =
                            many1::<String, _, _>(satisfy(|c: char| c.is_ascii_digit()))
                                .parse_stream(input)
                                .into_result()?;
                        let fd = digits.parse::<u32>().unwrap_or(0);
                        match input.uncons() {
                            Ok('=') => {
                                let cp_eq = input.checkpoint();
                                match input.uncons() {
                                    Ok(']') => Ok((RedirectOp::Close { fd }, Commit::Commit(()))),
                                    _ => {
                                        input.reset(cp_eq).ok();
                                        let (src_d, _) =
                                            many1::<String, _, _>(satisfy(|c: char| {
                                                c.is_ascii_digit()
                                            }))
                                            .parse_stream(input)
                                            .into_result()?;
                                        let src = src_d.parse::<u32>().unwrap_or(0);
                                        let _ = ch(']').parse_stream(input).into_result()?;
                                        Ok((RedirectOp::Dup { dst: fd, src }, Commit::Commit(())))
                                    }
                                }
                            }
                            Ok(']') => {
                                let (_, _) = trivia().parse_stream(input).into_result()?;
                                let (target, _) = word_inner().parse_stream(input).into_result()?;
                                Ok((
                                    RedirectOp::Input {
                                        fd,
                                        target: RedirectTarget::File(target),
                                    },
                                    Commit::Commit(()),
                                ))
                            }
                            _ => {
                                input.reset(cp).ok();
                                Err(Commit::Peek(I::Error::empty(input.position()).into()))
                            }
                        }
                    }
                    _ => {
                        // < target (default fd 0)
                        input.reset(cp2).ok();
                        let (_, _) = trivia().parse_stream(input).into_result()?;
                        let (target, _) = word_inner().parse_stream(input).into_result()?;
                        Ok((
                            RedirectOp::Input {
                                fd: 0,
                                target: RedirectTarget::File(target),
                            },
                            Commit::Commit(()),
                        ))
                    }
                }
            }
            _ => {
                input.reset(cp).ok();
                Err(Commit::Peek(I::Error::empty(input.position()).into()))
            }
        }
    })
}

/// simple_cmd = WORD+ — a command name followed by arguments.
fn simple_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Expr> {
    many1::<Vec<Word>, _, _>(word_inner().skip(trivia())).map(|words| {
        let mut iter = words.into_iter();
        let n = iter.next().unwrap();
        Expr::Simple(SimpleCommand {
            name: n,
            args: iter.collect(),
            assignments: vec![],
        })
    })
}

/// body = '{' program '}' | '=>' command
fn body<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Vec<Command>> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let cp = input.checkpoint();
        match input.uncons() {
            Ok('{') => {
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let (prog, _) = program_inner().parse_stream(input).into_result()?;
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let _ = ch('}').parse_stream(input).into_result()?;
                Ok((prog.commands, Commit::Commit(())))
            }
            _ => {
                input.reset(cp).ok();
                let _ = string("=>").parse_stream(input).into_result()?;
                let (_, _) = trivia().parse_stream(input).into_result()?;
                let (cmd, _) = command_().parse_stream(input).into_result()?;
                Ok((vec![cmd], Commit::Commit(())))
            }
        }
    })
}

/// cmd_expr = '!' cmd_expr | '{' program '}' | '@{' program '}' | simple_cmd redirect*
fn cmd_expr<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Expr> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let cp = input.checkpoint();
        match input.uncons() {
            Ok('!') => {
                let (_, _) = trivia().parse_stream(input).into_result()?;
                let (inner, _) = cmd_expr().parse_stream(input).into_result()?;
                Ok((Expr::Not(Box::new(inner)), Commit::Commit(())))
            }
            Ok('@') => {
                let _ = ch('{').parse_stream(input).into_result()?;
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let (prog, _) = program_inner().parse_stream(input).into_result()?;
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let _ = ch('}').parse_stream(input).into_result()?;
                Ok((Expr::Subshell(prog.commands), Commit::Commit(())))
            }
            Ok('{') => {
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let (prog, _) = program_inner().parse_stream(input).into_result()?;
                let (_, _) = full_trivia().parse_stream(input).into_result()?;
                let _ = ch('}').parse_stream(input).into_result()?;
                Ok((Expr::Block(prog.commands), Commit::Commit(())))
            }
            _ => {
                input.reset(cp).ok();
                let (base, _) = simple_cmd().parse_stream(input).into_result()?;
                // Collect redirects
                let mut expr = base;
                loop {
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let cp2 = input.checkpoint();
                    match redirect().parse_stream(input).into_result() {
                        Ok((redir, _)) => {
                            expr = Expr::Redirect(Box::new(expr), redir);
                        }
                        Err(_) => {
                            input.reset(cp2).ok();
                            break;
                        }
                    }
                }
                Ok((expr, Commit::Commit(())))
            }
        }
    })
}

/// pipeline = cmd_expr ('|' cmd_expr)* | cmd_expr '|&'
fn pipeline<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Expr> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (first, _) = cmd_expr().parse_stream(input).into_result()?;
        let mut stages = vec![first];

        loop {
            let (_, _) = trivia().parse_stream(input).into_result()?;
            let cp = input.checkpoint();
            match input.uncons() {
                Ok('|') => {
                    let cp2 = input.checkpoint();
                    match input.uncons() {
                        Ok('&') => {
                            // Coprocess
                            let inner = if stages.len() == 1 {
                                stages.into_iter().next().unwrap()
                            } else {
                                Expr::Pipeline(stages)
                            };
                            return Ok((Expr::Coprocess(Box::new(inner)), Commit::Commit(())));
                        }
                        Ok('|') => {
                            // || — put both back, break
                            input.reset(cp).ok();
                            break;
                        }
                        _ => {
                            // Regular pipe
                            input.reset(cp2).ok();
                            let (_, _) = trivia().parse_stream(input).into_result()?;
                            let (next, _) = cmd_expr().parse_stream(input).into_result()?;
                            stages.push(next);
                        }
                    }
                }
                _ => {
                    input.reset(cp).ok();
                    break;
                }
            }
        }

        let result = if stages.len() == 1 {
            stages.into_iter().next().unwrap()
        } else {
            Expr::Pipeline(stages)
        };
        Ok((result, Commit::Commit(())))
    })
}

/// match_expr = pipeline ('=~' value)?
fn match_expr<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Expr> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (left, _) = pipeline().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;

        let cp = input.checkpoint();
        match attempt(string("=~")).parse_stream(input).into_result() {
            Ok(_) => {
                let (_, _) = trivia().parse_stream(input).into_result()?;
                let (patterns, _) = value_().parse_stream(input).into_result()?;
                Ok((
                    Expr::PatternMatch {
                        expr: Box::new(left),
                        patterns,
                    },
                    Commit::Commit(()),
                ))
            }
            Err(_) => {
                input.reset(cp).ok();
                Ok((left, Commit::Commit(())))
            }
        }
    })
}

/// and_expr = match_expr ('&&' match_expr)*
fn and_expr<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Expr> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (first, _) = match_expr().parse_stream(input).into_result()?;
        let mut result = first;

        loop {
            let (_, _) = trivia().parse_stream(input).into_result()?;
            let cp = input.checkpoint();
            match attempt(string("&&")).parse_stream(input).into_result() {
                Ok(_) => {
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let (right, _) = match_expr().parse_stream(input).into_result()?;
                    result = Expr::And(Box::new(result), Box::new(right));
                }
                Err(_) => {
                    input.reset(cp).ok();
                    break;
                }
            }
        }
        Ok((result, Commit::Commit(())))
    })
}

/// or_expr = and_expr ('||' and_expr)*
fn or_expr<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Expr> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (first, _) = and_expr().parse_stream(input).into_result()?;
        let mut result = first;

        loop {
            let (_, _) = trivia().parse_stream(input).into_result()?;
            let cp = input.checkpoint();
            match attempt(string("||")).parse_stream(input).into_result() {
                Ok(_) => {
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let (right, _) = and_expr().parse_stream(input).into_result()?;
                    result = Expr::Or(Box::new(result), Box::new(right));
                }
                Err(_) => {
                    input.reset(cp).ok();
                    break;
                }
            }
        }
        Ok((result, Commit::Commit(())))
    })
}

/// expr_cmd = or_expr ('&')?
fn expr_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Expr> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (expr, _) = or_expr().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let cp = input.checkpoint();
        match input.uncons() {
            Ok('&') => {
                // Make sure it's not &&
                let cp2 = input.checkpoint();
                match input.uncons() {
                    Ok('&') => {
                        input.reset(cp).ok();
                        Ok((expr, Commit::Commit(())))
                    }
                    _ => {
                        input.reset(cp2).ok();
                        Ok((Expr::Background(Box::new(expr)), Commit::Commit(())))
                    }
                }
            }
            _ => {
                input.reset(cp).ok();
                Ok((expr, Commit::Commit(())))
            }
        }
    })
}

// ── Layer 5: Commands ──────────────────────────────────────────

/// Type annotation — atomic types, lists, tuples, unions.
/// Top-level type annotation: union (`->` union)?
/// `->` is right-associative, lowest precedence.
/// A -> B -> C = A -> (B -> C)
/// A | B -> C = (A | B) -> C
fn type_ann<I: Stream<Token = char>>() -> impl CombineParser<I, Output = TypeAnnotation> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (left, _) = type_union().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;

        // Check for ->
        let cp = input.checkpoint();
        match attempt(string("->")).parse_stream(input).into_result() {
            Ok(_) => {
                let (_, _) = trivia().parse_stream(input).into_result()?;
                // Right-associative: recurse into type_ann
                let (right, _) = type_ann().parse_stream(input).into_result()?;
                Ok((
                    TypeAnnotation::Fn(Box::new(left), Box::new(right)),
                    Commit::Commit(()),
                ))
            }
            Err(_) => {
                input.reset(cp).ok();
                Ok((left, Commit::Commit(())))
            }
        }
    })
}

/// Union level: atom (| atom)*
fn type_union<I: Stream<Token = char>>() -> impl CombineParser<I, Output = TypeAnnotation> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (base, _) = type_ann_atom().parse_stream(input).into_result()?;
        let mut result = base;

        // Check for union: type | type (not || and not |})
        loop {
            let (_, _) = trivia().parse_stream(input).into_result()?;
            let cp = input.checkpoint();
            match input.uncons() {
                Ok('|') => {
                    let cp2 = input.checkpoint();
                    match input.uncons() {
                        Ok('|') | Ok('}') => {
                            // || or |} — not a union, put back
                            input.reset(cp).ok();
                            break;
                        }
                        _ => {
                            input.reset(cp2).ok();
                        }
                    }
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let (right, _) = type_ann_atom().parse_stream(input).into_result()?;
                    result = match result {
                        TypeAnnotation::Union(mut branches) => {
                            branches.push(right);
                            TypeAnnotation::Union(branches)
                        }
                        _ => TypeAnnotation::Union(vec![result, right]),
                    };
                }
                _ => {
                    input.reset(cp).ok();
                    break;
                }
            }
        }
        Ok((result, Commit::Commit(())))
    })
}

/// Atomic type annotation.
fn type_ann_atom<I: Stream<Token = char>>() -> impl CombineParser<I, Output = TypeAnnotation> {
    choice!(
        attempt(keyword("Unit").map(|_| TypeAnnotation::Unit)),
        attempt(keyword("Bool").map(|_| TypeAnnotation::Bool)),
        attempt(keyword("Int").map(|_| TypeAnnotation::Int)),
        attempt(keyword("Str").map(|_| TypeAnnotation::Str)),
        attempt(keyword("Path").map(|_| TypeAnnotation::Path)),
        attempt(keyword("ExitCode").map(|_| TypeAnnotation::ExitCode)),
        attempt(
            keyword("Result")
                .skip(trivia())
                .skip(ch('['))
                .skip(trivia())
                .with(type_ann())
                .skip(trivia())
                .skip(ch(']'))
                .map(|t| TypeAnnotation::Result(Box::new(t)))
        ),
        attempt(
            keyword("Maybe")
                .skip(trivia())
                .skip(ch('['))
                .skip(trivia())
                .with(type_ann())
                .skip(trivia())
                .skip(ch(']'))
                .map(|t| TypeAnnotation::Maybe(Box::new(t)))
        ),
        attempt(
            keyword("List")
                .skip(trivia())
                .skip(ch('['))
                .skip(trivia())
                .with(type_ann())
                .skip(trivia())
                .skip(ch(']'))
                .map(|t| TypeAnnotation::List(Some(Box::new(t))))
        ),
        attempt(keyword("List").map(|_| TypeAnnotation::List(None))),
        // Fn[A, B] — canonical function type
        attempt(
            keyword("Fn")
                .skip(trivia())
                .skip(ch('['))
                .skip(trivia())
                .with(type_ann())
                .skip(trivia())
                .skip(ch(','))
                .skip(trivia())
                .and(type_ann())
                .skip(trivia())
                .skip(ch(']'))
                .map(|(param, ret)| TypeAnnotation::Fn(Box::new(param), Box::new(ret)))
        ),
        // (T) = List[T] sugar or (T, T) = Tuple
        attempt(type_ann_paren())
    )
}

/// Parse (T) or (T, T, ...) for type annotations.
fn type_ann_paren<I: Stream<Token = char>>() -> impl CombineParser<I, Output = TypeAnnotation> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = ch('(').parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (first, _) = type_ann().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;

        let cp = input.checkpoint();
        match input.uncons() {
            Ok(',') => {
                // Tuple
                let mut elems = vec![first];
                loop {
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let (t, _) = type_ann().parse_stream(input).into_result()?;
                    elems.push(t);
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let cp2 = input.checkpoint();
                    match input.uncons() {
                        Ok(',') => continue,
                        Ok(')') => return Ok((TypeAnnotation::Tuple(elems), Commit::Commit(()))),
                        _ => {
                            input.reset(cp2).ok();
                            return Ok((TypeAnnotation::Tuple(elems), Commit::Commit(())));
                        }
                    }
                }
            }
            Ok(')') => {
                // (T) = List[T] sugar
                Ok((
                    TypeAnnotation::List(Some(Box::new(first))),
                    Commit::Commit(()),
                ))
            }
            _ => {
                input.reset(cp).ok();
                let _ = ch(')').parse_stream(input).into_result()?;
                Ok((
                    TypeAnnotation::List(Some(Box::new(first))),
                    Commit::Commit(()),
                ))
            }
        }
    })
}

/// if_cmd = 'if' pipeline body ('else' (if_cmd | body))?
fn if_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = keyword("if").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (condition, _) = or_expr().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (then_body, _) = body().parse_stream(input).into_result()?;

        // Optional else clause
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let cp = input.checkpoint();
        let else_body = match keyword("else").parse_stream(input).into_result() {
            Ok(_) => {
                let (_, _) = trivia().parse_stream(input).into_result()?;
                // else if ... or else body
                let cp2 = input.checkpoint();
                match keyword("if").parse_stream(input).into_result() {
                    Ok(_) => {
                        input.reset(cp2).ok();
                        let (nested, _) = if_cmd().parse_stream(input).into_result()?;
                        Some(vec![nested])
                    }
                    Err(_) => {
                        input.reset(cp2).ok();
                        let (eb, _) = body().parse_stream(input).into_result()?;
                        Some(eb)
                    }
                }
            }
            Err(_) => {
                input.reset(cp).ok();
                None
            }
        };

        Ok((
            Command::If {
                condition,
                then_body,
                else_body,
            },
            Commit::Commit(()),
        ))
    })
}

/// for_cmd = 'for' NAME 'in' value body
fn for_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = keyword("for").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (var, _) = varname().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let _ = keyword("in").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (list, _) = value_().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (body_cmds, _) = body().parse_stream(input).into_result()?;

        Ok((
            Command::For {
                var,
                list,
                body: body_cmds,
            },
            Commit::Commit(()),
        ))
    })
}

/// while_cmd = 'while' pipeline body
fn while_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = keyword("while").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (cond, _) = or_expr().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (body_cmds, _) = body().parse_stream(input).into_result()?;

        Ok((
            Command::While {
                condition: cond,
                body: body_cmds,
            },
            Commit::Commit(()),
        ))
    })
}

/// Classify a word as a glob pattern.
fn to_glob_pattern(word: &str) -> Pattern {
    if word == "*" {
        Pattern::Star
    } else if word.contains('*') || word.contains('?') || word.contains('[') {
        Pattern::Glob(word.to_string())
    } else {
        Pattern::Literal(word.to_string())
    }
}

/// match_arm = glob_arm | structural_arm
///   glob_arm       = glob_pats '=>' lambda_body
///   structural_arm = NAME NAME '=>' lambda_body
///   glob_pats      = '(' NAME+ ')' | NAME
///
/// Disambiguation: '(' → multi-glob, two bare words → structural,
/// one bare word → single-pattern glob.
fn match_arm<I: Stream<Token = char>>(
) -> impl CombineParser<I, Output = (Vec<Pattern>, Vec<Command>)> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (_, _) = trivia().parse_stream(input).into_result()?;

        let cp_start = input.checkpoint();
        let pats: Vec<Pattern> = match input.uncons() {
            Ok('(') => {
                // Multi-pattern glob: (pat1 pat2 ...)
                let mut ps = Vec::new();
                loop {
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let cp = input.checkpoint();
                    match input.uncons() {
                        Ok(')') => break,
                        _ => {
                            input.reset(cp).ok();
                            let (w, _) = wname().parse_stream(input).into_result()?;
                            ps.push(to_glob_pattern(&w));
                        }
                    }
                }
                ps
            }
            _ => {
                // Single word — could be glob or start of structural
                input.reset(cp_start).ok();
                let (first, _) = wname().parse_stream(input).into_result()?;
                let (_, _) = trivia().parse_stream(input).into_result()?;

                // Peek: if next is =>, it's a single-pattern glob.
                // If next is another NAME then =>, it's structural.
                let cp2 = input.checkpoint();
                match attempt(string("=>")).parse_stream(input).into_result() {
                    Ok(_) => {
                        input.reset(cp2).ok();
                        vec![to_glob_pattern(&first)]
                    }
                    Err(_) => {
                        input.reset(cp2).ok();
                        // Try structural: NAME NAME =>
                        let cp3 = input.checkpoint();
                        match wname().parse_stream(input).into_result() {
                            Ok((binding, _)) => {
                                let (_, _) = trivia().parse_stream(input).into_result()?;
                                // Verify => follows
                                let cp4 = input.checkpoint();
                                match attempt(string("=>")).parse_stream(input).into_result() {
                                    Ok(_) => {
                                        input.reset(cp4).ok();
                                        vec![Pattern::Structural {
                                            tag: first,
                                            binding,
                                        }]
                                    }
                                    Err(_) => {
                                        // Not structural — shouldn't happen in
                                        // well-formed input, but reset
                                        input.reset(cp3).ok();
                                        vec![to_glob_pattern(&first)]
                                    }
                                }
                            }
                            Err(_) => {
                                input.reset(cp3).ok();
                                vec![to_glob_pattern(&first)]
                            }
                        }
                    }
                }
            }
        };

        // =>
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let _ = string("=>").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;

        // Body: '{' program '}' or single command
        let (cmds, _) = choice!(
            ch('{')
                .with(full_trivia())
                .with(program_inner())
                .skip(full_trivia())
                .skip(ch('}'))
                .map(|prog| prog.commands),
            command_().map(|cmd| vec![cmd])
        )
        .parse_stream(input)
        .into_result()?;

        Ok(((pats, cmds), Commit::Commit(())))
    })
}

/// match_cmd = 'match' value '{' match_arm (';' match_arm)* ';'? '}'
fn match_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = keyword("match").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (val, _) = value_().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let _ = ch('{').parse_stream(input).into_result()?;

        let mut arms = Vec::new();
        loop {
            let (_, _) = full_trivia().parse_stream(input).into_result()?;
            let cp = input.checkpoint();
            match input.uncons() {
                Ok('}') => break,
                Ok(';') => continue,
                _ => {
                    input.reset(cp).ok();
                }
            }
            let (arm, _) = match_arm().parse_stream(input).into_result()?;
            arms.push(arm);
        }
        Ok((Command::Match { value: val, arms }, Commit::Commit(())))
    })
}

/// try_cmd = 'try' body ('else' NAME body)?
fn try_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = keyword("try").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (try_body, _) = body().parse_stream(input).into_result()?;

        let (_, _) = trivia().parse_stream(input).into_result()?;
        let cp = input.checkpoint();
        let (else_var, else_body) = match keyword("else").parse_stream(input).into_result() {
            Ok(_) => {
                let (_, _) = trivia().parse_stream(input).into_result()?;
                // Optional binding name (bare, no $)
                let cp2 = input.checkpoint();
                match attempt(varname().skip(trivia()).skip(ch('{')))
                    .parse_stream(input)
                    .into_result()
                {
                    Ok((vname, _)) => {
                        // Put '{' back — body() expects it
                        input.reset(cp2).ok();
                        let (_, _) = varname().parse_stream(input).into_result()?;
                        let (_, _) = trivia().parse_stream(input).into_result()?;
                        let (eb, _) = body().parse_stream(input).into_result()?;
                        (Some(vname), Some(eb))
                    }
                    Err(_) => {
                        input.reset(cp2).ok();
                        let (eb, _) = body().parse_stream(input).into_result()?;
                        (None, Some(eb))
                    }
                }
            }
            Err(_) => {
                input.reset(cp).ok();
                (None, None)
            }
        };

        Ok((
            Command::Try {
                body: try_body,
                else_var,
                else_body,
            },
            Commit::Commit(()),
        ))
    })
}

/// fn_def = 'fn' NAME fn_params? body
fn fn_def<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = keyword("fn").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (fname, _) = wname().parse_stream(input).into_result()?;

        // Optional fn_params = '(' NAME* ')'
        let cp = input.checkpoint();
        let fn_params = match input.uncons() {
            Ok('(') => {
                let mut params = Vec::new();
                loop {
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let cp2 = input.checkpoint();
                    match input.uncons() {
                        Ok(')') => break,
                        _ => {
                            input.reset(cp2).ok();
                            let (pname, _) = varname().parse_stream(input).into_result()?;
                            params.push(pname);
                        }
                    }
                }
                params
            }
            _ => {
                input.reset(cp).ok();
                vec![]
            }
        };

        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (fbody, _) = body().parse_stream(input).into_result()?;

        Ok((
            Command::Bind(Binding::Fn {
                name: fname,
                params: fn_params,
                body: fbody,
            }),
            Commit::Commit(()),
        ))
    })
}

/// let_binding = 'let' quals NAME (':' type_ann)? '=' value
fn let_binding<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = keyword("let").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;

        // Qualifiers: mut, export (order free)
        let mut is_mut = false;
        let mut is_export = false;
        loop {
            let cp = input.checkpoint();
            if keyword("mut").parse_stream(input).into_result().is_ok() {
                is_mut = true;
                let _ = trivia().parse_stream(input).into_result()?;
                continue;
            }
            input.reset(cp).ok();
            let cp = input.checkpoint();
            if keyword("export").parse_stream(input).into_result().is_ok() {
                is_export = true;
                let _ = trivia().parse_stream(input).into_result()?;
                continue;
            }
            input.reset(cp).ok();
            break;
        }

        let (vname, _) = varname().parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;

        // Optional type annotation
        let cp = input.checkpoint();
        let tann = match input.uncons() {
            Ok(':') => {
                let (_, _) = trivia().parse_stream(input).into_result()?;
                let (ann, _) = type_ann().parse_stream(input).into_result()?;
                let (_, _) = trivia().parse_stream(input).into_result()?;
                Some(ann)
            }
            _ => {
                input.reset(cp).ok();
                None
            }
        };

        let _ = ch('=').parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let (val, _) = value_().parse_stream(input).into_result()?;

        Ok((
            Command::Bind(Binding::Let {
                name: vname,
                value: val,
                mutable: is_mut,
                export: is_export,
                type_ann: tann,
            }),
            Commit::Commit(()),
        ))
    })
}

/// ref_def = 'ref' NAME '=' NAME
fn ref_def<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    keyword("ref")
        .with(trivia())
        .with(varname())
        .skip(trivia())
        .skip(ch('='))
        .skip(trivia())
        .and(varname())
        .map(|(rname, target)| {
            Command::Bind(Binding::Ref {
                name: rname,
                target,
            })
        })
}

/// return_cmd = 'return' value?
fn return_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let _ = keyword("return").parse_stream(input).into_result()?;
        let (_, _) = trivia().parse_stream(input).into_result()?;
        let cp = input.checkpoint();
        match value_().parse_stream(input).into_result() {
            Ok((val, _)) => Ok((Command::Return(Some(val)), Commit::Commit(()))),
            Err(_) => {
                input.reset(cp).ok();
                Ok((Command::Return(None), Commit::Commit(())))
            }
        }
    })
}

/// take value — contribute to enclosing for-in-value accumulator
fn take_cmd<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    keyword("take")
        .skip(trivia())
        .with(value_())
        .map(Command::Take)
}

/// assignment = NAME '=' value
fn assignment<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    attempt(varname().skip(trivia()).skip(ch('=')).skip(trivia()))
        .and(value_())
        .map(|(vname, val)| Command::Bind(Binding::Assignment(vname, val)))
}

/// A single command.
fn command_<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Command> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let (_, _) = trivia().parse_stream(input).into_result()?;

        // Try keyword commands first, then assignment, then expression.
        // Each attempt needs a fresh checkpoint since reset consumes it.
        macro_rules! try_parser {
            ($parser:expr) => {{
                let cp = input.checkpoint();
                if let Ok(r) = $parser.parse_stream(input).into_result() {
                    return Ok(r);
                }
                input.reset(cp).ok();
            }};
        }

        try_parser!(if_cmd());
        try_parser!(for_cmd());
        try_parser!(while_cmd());
        try_parser!(match_cmd());
        try_parser!(try_cmd());
        try_parser!(fn_def());
        try_parser!(let_binding());
        try_parser!(ref_def());
        try_parser!(return_cmd());
        try_parser!(take_cmd());
        try_parser!(assignment());

        let (expr, _) = expr_cmd().parse_stream(input).into_result()?;
        Ok((Command::Exec(expr), Commit::Commit(())))
    })
}

// ── Layer 6: Program ──────────────────────────────────────────

/// program = terminator* (command terminator+)* command?
fn program_inner<I: Stream<Token = char>>() -> impl CombineParser<I, Output = Program> {
    combine::parser(move |input: &mut I| {
        use combine::error::Commit;

        let mut commands = Vec::new();

        // Skip leading trivia and terminators
        loop {
            let (_, _) = trivia().parse_stream(input).into_result()?;
            let cp = input.checkpoint();
            match input.uncons() {
                Ok('\n') | Ok(';') => continue,
                _ => {
                    input.reset(cp).ok();
                    break;
                }
            }
        }

        loop {
            let (_, _) = trivia().parse_stream(input).into_result()?;
            let cp = input.checkpoint();
            match command_().parse_stream(input).into_result() {
                Ok((cmd, _)) => {
                    commands.push(cmd);
                    // Consume terminators after the command
                    let (_, _) = trivia().parse_stream(input).into_result()?;
                    let mut found_terminator = false;
                    loop {
                        let cp2 = input.checkpoint();
                        match input.uncons() {
                            Ok('\n') | Ok(';') => {
                                found_terminator = true;
                                let (_, _) = trivia().parse_stream(input).into_result()?;
                            }
                            _ => {
                                input.reset(cp2).ok();
                                break;
                            }
                        }
                    }
                    if !found_terminator {
                        break;
                    }
                }
                Err(_) => {
                    input.reset(cp).ok();
                    break;
                }
            }
        }
        Ok((Program { commands }, Commit::Peek(())))
    })
}

// ── Public API ─────────────────────────────────────────────────

/// Parse the psh language. Public entry point.
pub struct PshParser;

impl PshParser {
    pub fn parse(input: &str) -> anyhow::Result<Program> {
        use combine::EasyParser;
        let result = program_inner()
            .skip(trivia())
            .easy_parse(combine::stream::position::Stream::new(input));
        match result {
            Ok((prog, remaining)) => {
                let rest = remaining.input;
                if !rest.is_empty() {
                    anyhow::bail!(
                        "parse error: unexpected input near '{}'",
                        &rest[..rest.len().min(40)]
                    );
                }
                Ok(prog)
            }
            Err(e) => {
                anyhow::bail!("parse error: {e}");
            }
        }
    }
}

/// Re-export as Parser for compatibility with main.rs.
pub type Parser = PshParser;

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Program {
        PshParser::parse(input).unwrap_or_else(|e| panic!("parse failed: {e}\ninput: {input}"))
    }

    fn parse_err(input: &str) {
        assert!(
            PshParser::parse(input).is_err(),
            "expected parse error for: {input}"
        );
    }

    // ── L1: Lexical primitives ─────────────────────────────

    #[test]
    fn var_char_predicate() {
        assert!(is_var_char('a'));
        assert!(is_var_char('Z'));
        assert!(is_var_char('0'));
        assert!(is_var_char('_'));
        assert!(is_var_char('*'));
        assert!(!is_var_char('.'));
        assert!(!is_var_char('/'));
        assert!(!is_var_char('-'));
    }

    #[test]
    fn word_char_predicate() {
        assert!(is_word_char('a'));
        assert!(is_word_char('.'));
        assert!(is_word_char('/'));
        assert!(is_word_char('-'));
        assert!(is_word_char('+'));
        assert!(is_word_char('@'));
        assert!(is_word_char('*'));
        assert!(is_word_char('?'));
        assert!(!is_word_char('~'));
        assert!(!is_word_char('$'));
        assert!(!is_word_char('\''));
        assert!(!is_word_char('{'));
    }

    #[test]
    fn empty_program() {
        let prog = parse("");
        assert!(prog.commands.is_empty());
    }

    #[test]
    fn whitespace_only() {
        let prog = parse("   \t  ");
        assert!(prog.commands.is_empty());
    }

    #[test]
    fn comment_only() {
        let prog = parse("# this is a comment");
        assert!(prog.commands.is_empty());
    }

    #[test]
    fn line_continuation_test() {
        let prog = parse("echo hello \\\n  world");
        assert_eq!(prog.commands.len(), 1);
    }

    // ── L2: Word atoms ─────────────────────────────────────

    #[test]
    fn literal_word() {
        let prog = parse("echo hello");
        assert_eq!(prog.commands.len(), 1);
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.name, Word::Literal("echo".into()));
                assert_eq!(sc.args, vec![Word::Literal("hello".into())]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn quoted_string_test() {
        let prog = parse("echo 'hello world'");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.args, vec![Word::Quoted("hello world".into())]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn quoted_escape_test() {
        let prog = parse("echo 'it''s'");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.args, vec![Word::Quoted("it's".into())]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn var_ref_simple() {
        let prog = parse("echo $x");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.args, vec![Word::Var("x".into())]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn var_ref_count() {
        let prog = parse("echo $#list");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.args, vec![Word::Count("list".into())]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn var_ref_stringify() {
        let prog = parse(r#"echo $"list"#);
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.args, vec![Word::Stringify("list".into())]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn var_ref_indexed() {
        let prog = parse("echo $x(1)");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::Index("x".into(), Box::new(Word::Literal("1".into())))]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn brace_var_test() {
        let prog = parse("echo ${x.get}");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.args, vec![Word::BraceVar("x.get".into())]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn tilde_bare() {
        let prog = parse("echo ~");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.args, vec![Word::Tilde]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn tilde_path_test() {
        let prog = parse("echo ~/bin");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(sc.args, vec![Word::TildePath("bin".into())]);
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    // ── L3: Free carets ────────────────────────────────────

    #[test]
    fn free_caret_var_literal() {
        let prog = parse("echo $home/bin");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::Concat(vec![
                        Word::Var("home".into()),
                        Word::Literal("/bin".into()),
                    ])]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn accessor_takes_priority_over_free_caret() {
        // $stem.c is now an accessor (Tag("c")), not a free caret.
        // Use ${stem}.c for the old concat behavior.
        let prog = parse("echo $stem.c");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::VarAccess(
                        "stem".into(),
                        vec![Accessor::Tag("c".into())]
                    )]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn brace_var_dot_escape_hatch() {
        // ${stem}.c is the escape hatch: BraceVar + free caret
        let prog = parse("echo ${stem}.c");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::Concat(vec![
                        Word::BraceVar("stem".into()),
                        Word::Literal(".c".into()),
                    ])]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn explicit_caret_test() {
        let prog = parse("echo a^b");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::Concat(vec![
                        Word::Literal("a".into()),
                        Word::Literal("b".into()),
                    ])]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn quoted_free_caret() {
        let prog = parse("echo 'hello'$name");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::Concat(vec![
                        Word::Quoted("hello".into()),
                        Word::Var("name".into()),
                    ])]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    // ── Accessor parsing ────────────────────────────────────

    #[test]
    fn accessor_tuple_index() {
        let prog = parse("echo $x.0");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::VarAccess("x".into(), vec![Accessor::Index(0)])]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn accessor_tag_ok() {
        let prog = parse("echo $result.ok");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::VarAccess(
                        "result".into(),
                        vec![Accessor::Tag("ok".into())]
                    )]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn accessor_tag_err() {
        let prog = parse("echo $result.err");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::VarAccess(
                        "result".into(),
                        vec![Accessor::Tag("err".into())]
                    )]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn accessor_code() {
        let prog = parse("echo $e.code");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::VarAccess("e".into(), vec![Accessor::Code])]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn accessor_chain() {
        // $result.ok.0 — Prism then Lens
        let prog = parse("echo $result.ok.0");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::VarAccess(
                        "result".into(),
                        vec![Accessor::Tag("ok".into()), Accessor::Index(0)]
                    )]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    #[test]
    fn accessor_multi_digit_index() {
        let prog = parse("echo $t.12");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert_eq!(
                    sc.args,
                    vec![Word::VarAccess("t".into(), vec![Accessor::Index(12)])]
                );
            }
            other => panic!("expected simple command, got {other:?}"),
        }
    }

    // ── L4: Expressions ────────────────────────────────────

    #[test]
    fn pipeline_two_stages() {
        let prog = parse("ls | grep foo");
        match &prog.commands[0] {
            Command::Exec(Expr::Pipeline(stages)) => {
                assert_eq!(stages.len(), 2);
            }
            other => panic!("expected pipeline, got {other:?}"),
        }
    }

    #[test]
    fn and_expression() {
        let prog = parse("test -f file && echo exists");
        match &prog.commands[0] {
            Command::Exec(Expr::And(_, _)) => {}
            other => panic!("expected And, got {other:?}"),
        }
    }

    #[test]
    fn or_expression() {
        let prog = parse("test -f file || echo missing");
        match &prog.commands[0] {
            Command::Exec(Expr::Or(_, _)) => {}
            other => panic!("expected Or, got {other:?}"),
        }
    }

    #[test]
    fn background_command() {
        let prog = parse("sleep 10 &");
        match &prog.commands[0] {
            Command::Exec(Expr::Background(_)) => {}
            other => panic!("expected Background, got {other:?}"),
        }
    }

    #[test]
    fn not_command() {
        let prog = parse("! test -f file");
        match &prog.commands[0] {
            Command::Exec(Expr::Not(_)) => {}
            other => panic!("expected Not, got {other:?}"),
        }
    }

    #[test]
    fn block_expression() {
        let prog = parse("{ echo hello }");
        match &prog.commands[0] {
            Command::Exec(Expr::Block(cmds)) => {
                assert_eq!(cmds.len(), 1);
            }
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[test]
    fn subshell_expression() {
        let prog = parse("@{ echo hello }");
        match &prog.commands[0] {
            Command::Exec(Expr::Subshell(cmds)) => {
                assert_eq!(cmds.len(), 1);
            }
            other => panic!("expected Subshell, got {other:?}"),
        }
    }

    #[test]
    fn redirect_output_test() {
        let prog = parse("echo hello >file.txt");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Output {
                    fd: 1,
                    append: false,
                    ..
                },
            )) => {}
            other => panic!("expected Redirect Output, got {other:?}"),
        }
    }

    #[test]
    fn redirect_append_test() {
        let prog = parse("echo hello >>file.txt");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Output {
                    fd: 1,
                    append: true,
                    ..
                },
            )) => {}
            other => panic!("expected Redirect Append, got {other:?}"),
        }
    }

    #[test]
    fn redirect_input_test() {
        let prog = parse("cat <file.txt");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(_, RedirectOp::Input { fd: 0, .. })) => {}
            other => panic!("expected Redirect Input, got {other:?}"),
        }
    }

    #[test]
    fn redirect_fd_test() {
        let prog = parse("cmd >[2] /tmp/err");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(_, RedirectOp::Output { fd: 2, .. })) => {}
            other => panic!("expected Redirect Output fd 2, got {other:?}"),
        }
    }

    #[test]
    fn redirect_dup_test() {
        let prog = parse("cmd >[2=1]");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(_, RedirectOp::Dup { dst: 2, src: 1 })) => {}
            other => panic!("expected Dup 2=1, got {other:?}"),
        }
    }

    #[test]
    fn redirect_close_test() {
        let prog = parse("cmd >[2=]");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(_, RedirectOp::Close { fd: 2 })) => {}
            other => panic!("expected Close fd 2, got {other:?}"),
        }
    }

    #[test]
    fn here_string_test() {
        let prog = parse("cat <<<hello");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Input {
                    fd: 0,
                    target: RedirectTarget::HereString(_),
                },
            )) => {}
            other => panic!("expected HereString, got {other:?}"),
        }
    }

    #[test]
    fn coprocess_test() {
        let prog = parse("cat |&");
        match &prog.commands[0] {
            Command::Exec(Expr::Coprocess(_)) => {}
            other => panic!("expected Coprocess, got {other:?}"),
        }
    }

    // ── L5: Commands ───────────────────────────────────────

    #[test]
    fn if_then() {
        let prog = parse("if test -f file { echo exists }");
        match &prog.commands[0] {
            Command::If {
                else_body: None, ..
            } => {}
            other => panic!("expected If without else, got {other:?}"),
        }
    }

    #[test]
    fn if_then_else() {
        let prog = parse("if test -f file { echo exists } else { echo missing }");
        match &prog.commands[0] {
            Command::If {
                else_body: Some(_), ..
            } => {}
            other => panic!("expected If with else, got {other:?}"),
        }
    }

    #[test]
    fn if_else_if_test() {
        let prog = parse("if test -f a { echo a } else if test -f b { echo b } else { echo c }");
        match &prog.commands[0] {
            Command::If {
                else_body: Some(else_cmds),
                ..
            } => {
                assert!(matches!(else_cmds[0], Command::If { .. }));
            }
            other => panic!("expected If with nested else-if, got {other:?}"),
        }
    }

    #[test]
    fn if_arrow_body() {
        let prog = parse("if test -f file => echo exists");
        match &prog.commands[0] {
            Command::If { then_body, .. } => {
                assert_eq!(then_body.len(), 1);
            }
            other => panic!("expected If with arrow body, got {other:?}"),
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
            other => panic!("expected For, got {other:?}"),
        }
    }

    #[test]
    fn while_loop() {
        let prog = parse("while true { echo forever }");
        match &prog.commands[0] {
            Command::While { .. } => {}
            other => panic!("expected While, got {other:?}"),
        }
    }

    #[test]
    fn match_glob() {
        let prog = parse("match $file { *.txt => echo text; *.rs => echo rust; * => echo other }");
        match &prog.commands[0] {
            Command::Match { arms, .. } => {
                assert_eq!(arms.len(), 3);
                assert!(matches!(arms[0].0[0], Pattern::Glob(_)));
                assert!(matches!(arms[2].0[0], Pattern::Star));
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_structural() {
        let prog = parse("match $result { ok v => echo $v; err e => echo $e }");
        match &prog.commands[0] {
            Command::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
                match &arms[0].0[0] {
                    Pattern::Structural { tag, binding } => {
                        assert_eq!(tag, "ok");
                        assert_eq!(binding, "v");
                    }
                    other => panic!("expected Structural, got {other:?}"),
                }
                match &arms[1].0[0] {
                    Pattern::Structural { tag, binding } => {
                        assert_eq!(tag, "err");
                        assert_eq!(binding, "e");
                    }
                    other => panic!("expected Structural, got {other:?}"),
                }
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn match_multiline() {
        let prog = parse("match $type {\n    editor => echo edit\n    terminal => echo term\n    * => echo other\n}");
        match &prog.commands[0] {
            Command::Match { arms, .. } => {
                assert_eq!(arms.len(), 3);
            }
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[test]
    fn try_block() {
        let prog = parse("try { echo hello }");
        match &prog.commands[0] {
            Command::Try {
                else_var: None,
                else_body: None,
                ..
            } => {}
            other => panic!("expected Try without else, got {other:?}"),
        }
    }

    #[test]
    fn try_with_else() {
        let prog = parse("try { echo hello } else e { echo $e }");
        match &prog.commands[0] {
            Command::Try {
                else_var: Some(v),
                else_body: Some(_),
                ..
            } => {
                assert_eq!(v, "e");
            }
            other => panic!("expected Try with else, got {other:?}"),
        }
    }

    #[test]
    fn try_in_value_position() {
        let prog = parse("let r = try { echo hello }");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { value, .. }) => {
                assert!(matches!(value, Value::Try(_)));
            }
            other => panic!("expected Let with Try value, got {other:?}"),
        }
    }

    #[test]
    fn fn_definition() {
        let prog = parse("fn greet { echo hello }");
        match &prog.commands[0] {
            Command::Bind(Binding::Fn {
                name, params, body, ..
            }) => {
                assert_eq!(name, "greet");
                assert!(params.is_empty());
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn fn_named_params() {
        let prog = parse("fn add(a b) { echo $a $b }");
        match &prog.commands[0] {
            Command::Bind(Binding::Fn { name, params, .. }) => {
                assert_eq!(name, "add");
                assert_eq!(params, &["a", "b"]);
            }
            other => panic!("expected Fn with params, got {other:?}"),
        }
    }

    #[test]
    fn fn_nullary() {
        let prog = parse("fn noop() { true }");
        match &prog.commands[0] {
            Command::Bind(Binding::Fn { name, params, .. }) => {
                assert_eq!(name, "noop");
                assert!(params.is_empty());
            }
            other => panic!("expected Fn nullary, got {other:?}"),
        }
    }

    #[test]
    fn fn_discipline_with_param() {
        let prog = parse("fn x.set(val) { echo $val }");
        match &prog.commands[0] {
            Command::Bind(Binding::Fn { name, params, .. }) => {
                assert_eq!(name, "x.set");
                assert_eq!(params, &["val"]);
            }
            other => panic!("expected Fn discipline with param, got {other:?}"),
        }
    }

    #[test]
    fn fn_discipline() {
        let prog = parse("fn x.get { echo called }");
        match &prog.commands[0] {
            Command::Bind(Binding::Fn { name, .. }) => {
                assert_eq!(name, "x.get");
            }
            other => panic!("expected Fn discipline, got {other:?}"),
        }
    }

    #[test]
    fn let_basic() {
        let prog = parse("let x = 42");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                name,
                mutable: false,
                export: false,
                type_ann: None,
                ..
            }) => {
                assert_eq!(name, "x");
            }
            other => panic!("expected Let, got {other:?}"),
        }
    }

    #[test]
    fn let_mut_export() {
        let prog = parse("let mut export x = hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                mutable: true,
                export: true,
                ..
            }) => {}
            other => panic!("expected Let mut export, got {other:?}"),
        }
    }

    #[test]
    fn compute_value_match() {
        let prog = parse("let x = match $y { a => return 1; * => return 2 }");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { value, .. }) => {
                assert!(matches!(value, Value::Compute(_)));
            }
            other => panic!("expected Let with Compute, got {other:?}"),
        }
    }

    #[test]
    fn compute_value_if() {
        let prog = parse("let x = if true { return 1 }");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { value, .. }) => {
                assert!(matches!(value, Value::Compute(_)));
            }
            other => panic!("expected Let with Compute, got {other:?}"),
        }
    }

    #[test]
    fn compute_value_block() {
        let prog = parse("let x = { return 42 }");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { value, .. }) => {
                assert!(matches!(value, Value::Compute(_)));
            }
            other => panic!("expected Let with Compute, got {other:?}"),
        }
    }

    #[test]
    fn take_in_for() {
        let prog = parse("let x = for i in (a b) { take $i }");
        match &prog.commands[0] {
            Command::Bind(Binding::Let { value, .. }) => {
                assert!(matches!(value, Value::Compute(_)));
            }
            other => panic!("expected Let with Compute, got {other:?}"),
        }
    }

    #[test]
    fn take_parsed_as_command() {
        let prog = parse("take hello");
        assert!(matches!(prog.commands[0], Command::Take(_)));
    }

    #[test]
    fn let_with_type() {
        let prog = parse("let x : Int = 42");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                type_ann: Some(TypeAnnotation::Int),
                ..
            }) => {}
            other => panic!("expected Let with Int type, got {other:?}"),
        }
    }

    #[test]
    fn let_result_type() {
        let prog = parse("let x : Result[Int] = 42");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                type_ann: Some(TypeAnnotation::Result(_)),
                ..
            }) => {}
            other => panic!("expected Let with Result type, got {other:?}"),
        }
    }

    #[test]
    fn type_ann_arrow() {
        let prog = parse("let f : Int -> Str = 0");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                type_ann: Some(TypeAnnotation::Fn(param, ret)),
                ..
            }) => {
                assert_eq!(**param, TypeAnnotation::Int);
                assert_eq!(**ret, TypeAnnotation::Str);
            }
            other => panic!("expected Let with Fn type, got {other:?}"),
        }
    }

    #[test]
    fn type_ann_fn_bracket() {
        // Fn[Int, Str] — canonical form
        let prog = parse("let f : Fn[Int, Str] = 0");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                type_ann: Some(TypeAnnotation::Fn(param, ret)),
                ..
            }) => {
                assert_eq!(**param, TypeAnnotation::Int);
                assert_eq!(**ret, TypeAnnotation::Str);
            }
            other => panic!("expected Let with Fn[Int, Str], got {other:?}"),
        }
    }

    #[test]
    fn type_ann_arrow_right_assoc() {
        // Int -> Int -> Int = Int -> (Int -> Int)
        let prog = parse("let f : Int -> Int -> Int = 0");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                type_ann: Some(TypeAnnotation::Fn(param, ret)),
                ..
            }) => {
                assert_eq!(**param, TypeAnnotation::Int);
                assert!(matches!(**ret, TypeAnnotation::Fn(_, _)));
            }
            other => panic!("expected Let with curried Fn type, got {other:?}"),
        }
    }

    #[test]
    fn type_ann_arrow_with_tuple_param() {
        let prog = parse("let f : (Int, Str) -> Bool = 0");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                type_ann: Some(TypeAnnotation::Fn(param, ret)),
                ..
            }) => {
                assert!(matches!(**param, TypeAnnotation::Tuple(_)));
                assert_eq!(**ret, TypeAnnotation::Bool);
            }
            other => panic!("expected Let with tuple->Bool Fn type, got {other:?}"),
        }
    }

    #[test]
    fn ref_binding() {
        let prog = parse("ref y = x");
        match &prog.commands[0] {
            Command::Bind(Binding::Ref { name, target }) => {
                assert_eq!(name, "y");
                assert_eq!(target, "x");
            }
            other => panic!("expected Ref, got {other:?}"),
        }
    }

    #[test]
    fn return_with_value() {
        let prog = parse("return hello");
        match &prog.commands[0] {
            Command::Return(Some(_)) => {}
            other => panic!("expected Return with value, got {other:?}"),
        }
    }

    #[test]
    fn return_bare() {
        let prog = parse("return");
        match &prog.commands[0] {
            Command::Return(None) => {}
            other => panic!("expected Return bare, got {other:?}"),
        }
    }

    #[test]
    fn assignment_simple() {
        let prog = parse("x = hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Assignment(name, _)) => {
                assert_eq!(name, "x");
            }
            other => panic!("expected Assignment, got {other:?}"),
        }
    }

    #[test]
    fn assignment_list() {
        let prog = parse("x = (a b c)");
        match &prog.commands[0] {
            Command::Bind(Binding::Assignment(_, Value::List(words))) => {
                assert_eq!(words.len(), 3);
            }
            other => panic!("expected Assignment with list, got {other:?}"),
        }
    }

    #[test]
    fn lambda_single_param() {
        let prog = parse("let f = \\x => echo $x");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                value: Value::Lambda { params, body },
                ..
            }) => {
                assert_eq!(params, &["x"]);
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected Let with Lambda, got {other:?}"),
        }
    }

    #[test]
    fn lambda_multi_param() {
        let prog = parse("let f = \\x y => echo $x $y");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                value: Value::Lambda { params, .. },
                ..
            }) => {
                assert_eq!(params, &["x", "y"]);
            }
            other => panic!("expected Let with multi-param Lambda, got {other:?}"),
        }
    }

    #[test]
    fn lambda_nullary() {
        let prog = parse("let f = \\() => echo hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                value: Value::Lambda { params, .. },
                ..
            }) => {
                assert!(params.is_empty());
            }
            other => panic!("expected Let with nullary Lambda, got {other:?}"),
        }
    }

    #[test]
    fn lambda_braced_body() {
        let prog = parse("let f = \\x => { echo $x; echo done }");
        match &prog.commands[0] {
            Command::Bind(Binding::Let {
                value: Value::Lambda { body, .. },
                ..
            }) => {
                assert_eq!(body.len(), 2);
            }
            other => panic!("expected Let with braced Lambda body, got {other:?}"),
        }
    }

    // ── L6: Program ────────────────────────────────────────

    #[test]
    fn multi_command_newlines() {
        let prog = parse("echo a\necho b\necho c");
        assert_eq!(prog.commands.len(), 3);
    }

    #[test]
    fn multi_command_semicolons() {
        let prog = parse("echo a; echo b; echo c");
        assert_eq!(prog.commands.len(), 3);
    }

    #[test]
    fn mixed_terminators() {
        let prog = parse("echo a\necho b; echo c\n");
        assert_eq!(prog.commands.len(), 3);
    }

    #[test]
    fn trailing_terminator() {
        let prog = parse("echo hello\n");
        assert_eq!(prog.commands.len(), 1);
    }

    #[test]
    fn command_sub_test() {
        let prog = parse("echo `{date}");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert!(matches!(sc.args[0], Word::CommandSub(_)));
            }
            other => panic!("expected command sub, got {other:?}"),
        }
    }

    #[test]
    fn pipeline_three_stages() {
        let prog = parse("cat file | grep foo | wc -l");
        match &prog.commands[0] {
            Command::Exec(Expr::Pipeline(stages)) => {
                assert_eq!(stages.len(), 3);
            }
            other => panic!("expected 3-stage pipeline, got {other:?}"),
        }
    }

    #[test]
    fn nested_blocks() {
        let prog = parse("{ { echo inner } }");
        match &prog.commands[0] {
            Command::Exec(Expr::Block(cmds)) => {
                assert!(matches!(cmds[0], Command::Exec(Expr::Block(_))));
            }
            other => panic!("expected nested blocks, got {other:?}"),
        }
    }

    #[test]
    fn heredoc_expanding() {
        let prog = parse("cat <<EOF\nhello world\nEOF");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Input {
                    target: RedirectTarget::HereDoc { body, expand: true },
                    ..
                },
            )) => {
                assert_eq!(body, "hello world\n");
            }
            other => panic!("expected heredoc, got {other:?}"),
        }
    }

    #[test]
    fn heredoc_literal() {
        let prog = parse("cat <<'EOF'\nhello $user\nEOF");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(
                _,
                RedirectOp::Input {
                    target:
                        RedirectTarget::HereDoc {
                            body,
                            expand: false,
                        },
                    ..
                },
            )) => {
                assert_eq!(body, "hello $user\n");
            }
            other => panic!("expected literal heredoc, got {other:?}"),
        }
    }

    #[test]
    fn read_write_redirect() {
        let prog = parse("cmd <>file");
        match &prog.commands[0] {
            Command::Exec(Expr::Redirect(_, RedirectOp::ReadWrite { fd: 0, .. })) => {}
            other => panic!("expected ReadWrite redirect, got {other:?}"),
        }
    }

    #[test]
    fn fn_arrow_body() {
        let prog = parse("fn greet => echo hello");
        match &prog.commands[0] {
            Command::Bind(Binding::Fn { name, body, .. }) => {
                assert_eq!(name, "greet");
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected Fn with arrow body, got {other:?}"),
        }
    }

    #[test]
    fn output_process_sub() {
        let prog = parse("tee >{cat}");
        match &prog.commands[0] {
            Command::Exec(Expr::Simple(sc)) => {
                assert!(matches!(sc.args[0], Word::OutputProcessSub(_)));
            }
            other => panic!("expected simple command with output proc sub, got {other:?}"),
        }
    }

    // ── F11: Negative parse tests ─────────────────────────────

    #[test]
    fn parse_err_unterminated_quote() {
        parse_err("'hello");
    }

    #[test]
    fn parse_err_unterminated_cmd_sub() {
        parse_err("`{ echo");
    }

    #[test]
    fn parse_err_missing_close_brace() {
        parse_err("{ echo hello");
    }

    #[test]
    fn parse_err_empty_redirect_target() {
        // Redirect with no file target
        parse_err("echo hello >");
    }

    #[test]
    fn parse_err_unterminated_match() {
        parse_err("match foo {");
    }
}
