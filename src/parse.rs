//! Parser skeleton for psh — combine 4 implementation.
//!
//! This file is the combine-based parser infrastructure retained
//! across the VDC reframing retirement of the prior type system.
//! It provides the lexical primitives, character predicates, and
//! trivia handling that any psh parser will need. The grammar
//! productions themselves were retired along with the old AST,
//! evaluator, and value model; the next implementation will rebuild
//! them against the current spec (docs/spec/04-syntax.md).
//!
//! What remains:
//!   - Character predicates (var_char, word_char, can_start_atom)
//!   - Whitespace and comment handling (hspace, comment, line_cont,
//!     trivia, full_trivia)
//!   - Keyword / name primitives (keyword, varname, wname)
//!   - The PshParser entry-point shell (returns an error until the
//!     grammar is reimplemented)
//!
//! rc heritage informs the character sets; combine provides the
//! combinator substrate. When the grammar is rebuilt, this file is
//! the expected starting point.

#![allow(dead_code)]

use combine::{
    attempt, choice,
    many1, not_followed_by,
    parser::{
        char::{char as ch, string},
        token::satisfy,
    },
    skip_many, Parser as CombineParser, Stream,
};

// ── Character predicates ───────────────────────────────────────

/// var_char = [a-zA-Z0-9_*]
///
/// rc's variable-name alphabet (used after `$`, `$#`, `$"`).
/// Variable names terminate at the first character not in this set.
pub(crate) fn is_var_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '*'
}

/// word_char = [a-zA-Z0-9_\-./+:,%*?@]
///
/// Bare-word alphabet used for literals, function names, and other
/// name positions. Includes `.` (for discipline function names like
/// `def x.set { }`) and `/` (for paths). Square brackets are not
/// included — psh does not use square-bracket syntax.
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '_' | '-' | '.' | '/' | '+' | ':' | ',' | '%' | '*' | '?' | '@'
        )
}

/// Can the character start a new word atom? Used for rc's free-caret
/// rule: adjacent atoms concatenate implicitly when the next
/// character can start an atom.
pub(crate) fn can_start_atom(c: char) -> bool {
    c == '$' || c == '\'' || c == '`' || is_word_char(c)
}

// ── Primitive parsers ──────────────────────────────────────────

/// Parse one var_char.
pub(crate) fn var_char_<I: Stream<Token = char>>() -> impl CombineParser<I, Output = char> {
    satisfy(is_var_char)
}

/// Horizontal whitespace only (space, tab, CR). Not newlines —
/// newlines are terminators and must be handled by the grammar.
pub(crate) fn hspace<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    satisfy(|c: char| c == ' ' || c == '\t' || c == '\r').map(|_| ())
}

/// Comment: `#` to end of line (does not consume the newline).
pub(crate) fn comment<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    ch('#').with(skip_many(satisfy(|c: char| c != '\n')))
}

/// Line continuation: backslash followed by newline, consumed as
/// whitespace. Part of the backslash escape rules (see
/// docs/spec/04-syntax.md §Backslash escapes). `\<newline>` is trivia,
/// alongside `\<space>` and `\<tab>`.
pub(crate) fn line_cont<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    attempt(ch('\\').with(ch('\n'))).map(|_| ())
}

/// Skip trivia: horizontal whitespace, comments, line continuations.
/// Does NOT skip newlines — those are terminators at the statement
/// level.
pub(crate) fn trivia<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    skip_many(choice!(hspace(), comment(), line_cont()))
}

/// Skip trivia including newlines. Used inside `{ }` blocks,
/// `match { }` arms, tuple literals, and other brace-delimited
/// contexts where newlines are not statement terminators.
pub(crate) fn full_trivia<I: Stream<Token = char>>() -> impl CombineParser<I, Output = ()> {
    skip_many(choice!(
        hspace(),
        comment(),
        line_cont(),
        ch('\n').map(|_| ())
    ))
}

/// A keyword followed by a word boundary (not followed by a
/// var_char). `attempt()` so the parser backtracks if the prefix
/// matches but the boundary doesn't — necessary for disambiguating
/// keywords from identifiers.
pub(crate) fn keyword<I: Stream<Token = char>>(
    kw: &'static str,
) -> impl CombineParser<I, Output = &'static str> {
    attempt(string(kw).skip(not_followed_by(var_char_())))
}

/// VARNAME = var_char+ (used after `$`, `$#`, `$"`).
pub(crate) fn varname<I: Stream<Token = char>>() -> impl CombineParser<I, Output = String> {
    many1(var_char_())
}

/// NAME for word positions = word_char+ (literals, function names,
/// discipline-function names with embedded dots).
pub(crate) fn wname<I: Stream<Token = char>>() -> impl CombineParser<I, Output = String> {
    many1(satisfy(is_word_char))
}

// ── Public API ─────────────────────────────────────────────────

/// Public parser entry point.
///
/// Returns an error until the grammar is reimplemented against
/// the current spec. The infrastructure above is retained so the
/// next implementation can start from the lexical primitives
/// without rebuilding the combine scaffolding.
pub struct PshParser;

impl PshParser {
    pub fn parse(_input: &str) -> anyhow::Result<()> {
        anyhow::bail!(
            "psh parser is not yet implemented — the prior grammar \
             was retired during the VDC reframing. The next \
             implementation will rebuild it against docs/spec/04-syntax.md."
        )
    }
}
