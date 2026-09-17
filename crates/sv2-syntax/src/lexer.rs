// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The lexical layer, implemented from `KerML` 8.2.2.
//!
//! Every byte of the input ends up in exactly one token, trivia included. That is
//! what makes the tree lossless (ADR-0004): the lexer never skips whitespace or a
//! comment, it emits them, and the parser attaches them to the tree.
//!
//! Unknown bytes become [`SyntaxKind::Error`] tokens rather than being dropped, so
//! `parse(s).text() == s` holds for malformed input too.

use crate::generated::kinds::{OPERATORS, SyntaxKind};

/// One token: its kind and its half-open byte range in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// What the token is.
    pub kind: SyntaxKind,
    /// Byte offset of the first character.
    pub start: usize,
    /// Byte offset one past the last character.
    pub end: usize,
}

impl Token {
    /// The token's text, or `None` if the range is not a character boundary.
    ///
    /// Returns `Option` rather than slicing: a `str` slice landing inside a
    /// multi-byte character panics, and the parser must not panic on any input
    /// (invariant 3).
    #[must_use]
    pub fn text<'a>(&self, source: &'a str) -> Option<&'a str> {
        source.get(self.start..self.end)
    }
}

/// Whether a kind is trivia — attached to the tree, never skipped.
#[must_use]
pub fn is_trivia(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Whitespace
            | SyntaxKind::SingleLineNote
            | SyntaxKind::MultilineNote
            | SyntaxKind::RegularComment
    )
}

/// `WHITE_SPACE`, `KerML` 8.2.2.1.
///
/// `WHITE_SPACE = space | tab | form_feed | LINE_TERMINATOR`. Form feed is in the
/// specification and absent from the Pilot's `WS` terminal; this parser follows the
/// specification, which is recorded as a reviewed deviation and is deliberately more
/// permissive than the reference implementation here.
fn is_white_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{000C}' | '\n' | '\r')
}

/// `BASIC_INITIAL_CHARACTER`, `KerML` 8.2.2.3: `ALPHABETIC_CHARACTER | '_'`.
fn is_basic_initial(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// `BASIC_NAME_CHARACTER`, `KerML` 8.2.2.3: `BASIC_INITIAL_CHARACTER | DECIMAL_DIGIT`.
fn is_basic_name_char(c: char) -> bool {
    is_basic_initial(c) || c.is_ascii_digit()
}

/// A cursor over the source that never slices and never panics.
///
/// `Copy` so a rule can lex ahead on a throwaway copy and only commit it — see
/// [`lex_number`] — when the whole production matched.
#[derive(Clone, Copy)]
struct Cursor<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, offset: 0 }
    }

    /// The character at the cursor, without consuming it.
    fn peek(&self) -> Option<char> {
        self.rest().and_then(|s| s.chars().next())
    }

    fn rest(&self) -> Option<&'a str> {
        self.source.get(self.offset..)
    }

    /// Consume one character, returning it.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.offset += c.len_utf8();
        Some(c)
    }

    /// Consume while `predicate` holds.
    fn eat_while(&mut self, predicate: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&predicate) {
            self.bump();
        }
    }

    /// Consume a literal prefix if it is present.
    fn eat(&mut self, prefix: &str) -> bool {
        if self.rest().is_some_and(|s| s.starts_with(prefix)) {
            self.offset += prefix.len();
            return true;
        }
        false
    }
}

/// Split `source` into tokens. Every byte lands in exactly one of them.
///
/// Never fails: text that matches no production becomes an `Error` token, so the
/// caller always gets a complete cover of the input.
#[must_use]
pub fn tokenize(source: &str) -> Vec<Token> {
    let mut cursor = Cursor::new(source);
    let mut tokens = Vec::new();
    while let Some(first) = cursor.peek() {
        let start = cursor.offset;
        let kind = next_kind(&mut cursor, first);
        // A rule that consumed nothing would loop forever; take one character as an
        // error instead. Defensive: no branch below should be able to do this.
        if cursor.offset == start {
            cursor.bump();
        }
        tokens.push(Token {
            kind,
            start,
            end: cursor.offset,
        });
    }
    tokens
}

/// Lex the one token starting at the cursor, which is known to sit on `first`.
fn next_kind(cursor: &mut Cursor<'_>, first: char) -> SyntaxKind {
    if is_white_space(first) {
        cursor.eat_while(is_white_space);
        return SyntaxKind::Whitespace;
    }
    // A lone '/' is not a comment; it falls through to the operator table.
    if first == '/'
        && let Some(kind) = lex_comment(cursor)
    {
        return kind;
    }
    if is_basic_initial(first) {
        cursor.eat_while(is_basic_name_char);
        return SyntaxKind::BasicName;
    }
    if first.is_ascii_digit() {
        return lex_number(cursor);
    }
    match first {
        '\'' => lex_quoted(cursor, '\'', SyntaxKind::UnrestrictedName),
        '"' => lex_quoted(cursor, '"', SyntaxKind::StringValue),
        _ => lex_operator(cursor),
    }
}

/// The three comment forms, `KerML` 8.2.2.2. `None` if none of them starts here.
///
/// `MULTILINE_NOTE = '//*' COMMENT_TEXT '*/'` must be tried before
/// `SINGLE_LINE_NOTE = '//' LINE_TEXT`, or every multiline note is lexed as a single
/// line note and its body leaks into the tree as code.
///
/// A `//*` with no `*/` after it is not a `MULTILINE_NOTE`, because that production
/// requires the terminator. It is still `'//'` followed by `LINE_TEXT`, so it is a
/// `SINGLE_LINE_NOTE` running to the end of its line, and the lines after it are
/// ordinary text. It is tried on a copy of the cursor so the fallback starts over.
fn lex_comment(cursor: &mut Cursor<'_>) -> Option<SyntaxKind> {
    let mut note = *cursor;
    if note.eat("//*") && eat_until_close(&mut note) {
        *cursor = note;
        return Some(SyntaxKind::MultilineNote);
    }
    if cursor.eat("//") {
        // `LINE_TEXT` is "character sequence excluding `LINE_TERMINATOR`s". The
        // terminator belongs to `WHITE_SPACE`, so it is left for the next token.
        cursor.eat_while(|c| c != '\n' && c != '\r');
        return Some(SyntaxKind::SingleLineNote);
    }
    if cursor.eat("/*") {
        let _closed = eat_until_close(cursor);
        return Some(SyntaxKind::RegularComment);
    }
    None
}

/// Whether `text` is a regular comment that never reached its `*/` (`KerML` 8.2.2.2).
///
/// `REGULAR_COMMENT = '/*' COMMENT_TEXT '*/'` requires the terminator, so text that
/// runs to end of input does not match it, and no other token starts with `/*`. The
/// lexer still emits the token with every byte it covers — dropping it would break
/// the round-trip — which is what leaves the parser able to report it. An unclosed
/// `//*` never reaches here: it lexes as a `SINGLE_LINE_NOTE` (see `lex_comment`).
///
/// The opener is excluded before looking for the terminator, because `/*/` ends in
/// `*/` while being unterminated: those are the opener's own characters.
#[must_use]
pub(crate) fn is_unterminated_comment(kind: SyntaxKind, text: &str) -> bool {
    if kind != SyntaxKind::RegularComment {
        return false;
    }
    text.get("/*".len()..)
        .is_none_or(|rest| !rest.ends_with("*/"))
}

/// Consume through the next `*/`, or to end of input if there is none. Returns
/// whether the `*/` was found.
///
/// An unterminated comment is not an error here: the text still belongs to the
/// token, and dropping it would break the round-trip. [`is_unterminated_comment`]
/// is what turns it into a diagnostic, at the layer that has somewhere to put one.
fn eat_until_close(cursor: &mut Cursor<'_>) -> bool {
    while cursor.peek().is_some() {
        if cursor.eat("*/") {
            return true;
        }
        cursor.bump();
    }
    false
}

/// `DECIMAL_VALUE` and `EXPONENTIAL_VALUE`, `KerML` 8.2.2.4.
///
/// `EXPONENTIAL_VALUE` = `DECIMAL_VALUE` ('e' | 'E') ('+' | '-')? `DECIMAL_VALUE`. The
/// exponent is only taken when it is complete: `1e` is a `DECIMAL_VALUE` followed by
/// the name `e`, because the specification requires digits after the sign and a
/// lexer that consumed `1e` would turn a valid expression into a malformed number.
fn lex_number(cursor: &mut Cursor<'_>) -> SyntaxKind {
    cursor.eat_while(|c| c.is_ascii_digit());

    // Lex the exponent on a copy and commit it only once every part is there, so a
    // partial match leaves the cursor sitting on the 'e' for the next token.
    let mut exponent = *cursor;
    if !matches!(exponent.bump(), Some('e' | 'E')) {
        return SyntaxKind::DecimalValue;
    }
    if matches!(exponent.peek(), Some('+' | '-')) {
        exponent.bump();
    }
    if !exponent.peek().is_some_and(|c| c.is_ascii_digit()) {
        return SyntaxKind::DecimalValue;
    }
    exponent.eat_while(|c| c.is_ascii_digit());
    *cursor = exponent;
    SyntaxKind::ExponentialValue
}

/// `UNRESTRICTED_NAME` (`KerML` 8.2.2.3) and `STRING_VALUE` (8.2.2.5).
///
/// Both are a delimiter, a run of characters with backslash escapes, and the
/// delimiter again. An unterminated one runs to end of input and still keeps its
/// bytes, because losing them would break the round-trip.
fn lex_quoted(cursor: &mut Cursor<'_>, delimiter: char, kind: SyntaxKind) -> SyntaxKind {
    cursor.bump();
    while let Some(c) = cursor.bump() {
        if c == '\\' {
            // `ESCAPE_SEQUENCE`: the escaped character cannot close the literal.
            cursor.bump();
        } else if c == delimiter {
            break;
        }
    }
    kind
}

/// The operator set, longest match first.
///
/// `OPERATORS` is sorted longest first by the generator, which is what makes maximal
/// munch correct: `::>` before `::` before `:`.
fn lex_operator(cursor: &mut Cursor<'_>) -> SyntaxKind {
    for (text, kind) in OPERATORS {
        if cursor.eat(text) {
            return *kind;
        }
    }
    cursor.bump();
    SyntaxKind::Error
}
