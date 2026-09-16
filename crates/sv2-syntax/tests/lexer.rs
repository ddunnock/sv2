// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The lexical productions, checked against the clauses they were written from.
//!
//! Every expectation here comes from a `KerML` 8.2.2 clause, retrieved and recorded
//! in `.claude/state/deviations.json`, not from running the lexer and writing down
//! what it printed.

use sv2_syntax::{SyntaxKind, tokenize};

fn kinds(source: &str) -> Vec<SyntaxKind> {
    tokenize(source).into_iter().map(|t| t.kind).collect()
}

fn texts(source: &str) -> Vec<&str> {
    tokenize(source)
        .into_iter()
        .filter_map(|t| t.text(source))
        .collect()
}

/// The property every other test depends on: the tokens cover the input exactly,
/// with no gap, no overlap, and no backwards span.
fn assert_tokens_cover(source: &str) {
    let mut offset = 0;
    for token in tokenize(source) {
        assert_eq!(
            token.start, offset,
            "gap or overlap at {token:?} in {source:?}"
        );
        assert!(
            token.end >= token.start,
            "backwards span {token:?} in {source:?}"
        );
        offset = token.end;
    }
    assert_eq!(
        offset,
        source.len(),
        "tokens stop short of the end of {source:?}"
    );
}

#[test]
fn tokens_cover_the_input_with_no_gap_and_no_overlap() {
    for source in [
        "package P;",
        "  \t\n",
        "// note\npackage P { }",
        "/* c */ 'q' \"s\" 1e-3 ::> ~",
        "é→\u{1F600}",
        "",
    ] {
        assert_tokens_cover(source);
    }
}

// KerML 8.2.2.1: WHITE_SPACE = space | tab | form_feed | LINE_TERMINATOR.
#[test]
fn white_space_includes_form_feed() {
    // The reviewed deviation: the Pilot's WS terminal omits form feed and this
    // parser follows the specification. No corpus file exercises this, which is
    // exactly why it is asserted here.
    assert_eq!(kinds("\u{000C}"), [SyntaxKind::Whitespace]);
    assert_eq!(kinds("a\u{000C}b").len(), 3);
    assert_eq!(
        kinds(" \t\u{000C}\r\n"),
        [SyntaxKind::Whitespace],
        "one run of white space is one token"
    );
}

// KerML 8.2.2.2: the three comment forms.
#[test]
fn multiline_note_is_matched_before_single_line_note() {
    // '//*' must win over '//', or a multiline note's body leaks into the tree as
    // code — the note would end at the first newline instead of at '*/'.
    let source = "//* a\nb */";
    assert_eq!(kinds(source), [SyntaxKind::MultilineNote]);
    assert_eq!(texts(source), [source]);
}

#[test]
fn single_line_note_stops_before_the_line_terminator() {
    // LINE_TEXT excludes LINE_TERMINATORs; the terminator belongs to WHITE_SPACE.
    assert_eq!(
        kinds("// note\n"),
        [SyntaxKind::SingleLineNote, SyntaxKind::Whitespace]
    );
}

#[test]
fn regular_comment_is_distinct_from_a_note() {
    assert_eq!(kinds("/* c */"), [SyntaxKind::RegularComment]);
    assert_eq!(kinds("//* n */"), [SyntaxKind::MultilineNote]);
}

#[test]
fn an_unterminated_comment_keeps_its_bytes() {
    // Not an error at this layer: dropping the text would break the round-trip.
    let source = "/* never closed";
    assert_eq!(kinds(source), [SyntaxKind::RegularComment]);
    assert_eq!(texts(source), [source]);
}

// KerML 8.2.2.3: BASIC_NAME and UNRESTRICTED_NAME.
#[test]
fn a_basic_name_starts_with_a_letter_or_underscore() {
    assert_eq!(kinds("Vehicle"), [SyntaxKind::BasicName]);
    assert_eq!(kinds("_x9"), [SyntaxKind::BasicName]);
}

#[test]
fn a_digit_does_not_start_a_name() {
    // BASIC_INITIAL_CHARACTER is ALPHABETIC_CHARACTER | '_'. `9x` is a number then
    // a name, not one identifier.
    assert_eq!(
        kinds("9x"),
        [SyntaxKind::DecimalValue, SyntaxKind::BasicName]
    );
}

#[test]
fn an_unrestricted_name_is_single_quoted_and_honours_escapes() {
    assert_eq!(kinds("'a b'"), [SyntaxKind::UnrestrictedName]);
    // The escaped quote must not close the literal.
    let source = r"'a\'b'";
    assert_eq!(kinds(source), [SyntaxKind::UnrestrictedName]);
    assert_eq!(texts(source), [source]);
}

// KerML 8.2.2.4: DECIMAL_VALUE and EXPONENTIAL_VALUE.
#[test]
fn an_exponential_value_needs_digits_after_the_exponent() {
    assert_eq!(kinds("1e3"), [SyntaxKind::ExponentialValue]);
    assert_eq!(kinds("1E+3"), [SyntaxKind::ExponentialValue]);
    assert_eq!(kinds("1e-3"), [SyntaxKind::ExponentialValue]);
}

#[test]
fn an_incomplete_exponent_is_a_decimal_followed_by_a_name() {
    // The negative case for the rule above. `1e` has no digits after the marker, so
    // the specification's EXPONENTIAL_VALUE does not match; consuming it anyway
    // would turn a valid expression into a malformed number.
    assert_eq!(
        kinds("1e"),
        [SyntaxKind::DecimalValue, SyntaxKind::BasicName]
    );
    assert_eq!(
        kinds("1e+"),
        [
            SyntaxKind::DecimalValue,
            SyntaxKind::BasicName,
            SyntaxKind::Plus
        ]
    );
}

#[test]
fn operators_take_the_longest_match() {
    // Maximal munch: `::>` before `::` before `:`. Splitting these silently changes
    // what the parser sees.
    for (source, expected) in [
        ("::>", SyntaxKind::ColonColonGt),
        ("::", SyntaxKind::ColonColon),
        (":", SyntaxKind::Colon),
        (":>>", SyntaxKind::ColonGtGt),
        (":>", SyntaxKind::ColonGt),
        ("===", SyntaxKind::EqEqEq),
        ("==", SyntaxKind::EqEq),
    ] {
        assert_eq!(kinds(source), [expected], "{source:?} was split");
    }
}

#[test]
fn text_matching_nothing_becomes_an_error_token_rather_than_being_dropped() {
    let source = "\u{1F600}";
    assert_eq!(kinds(source), [SyntaxKind::Error]);
    assert_eq!(texts(source), [source], "the bytes survive");
}

#[test]
fn a_multi_byte_character_is_one_token_not_a_split_slice() {
    // `string_slice` is a workspace lint because a slice landing inside a character
    // panics, and the parser must not panic on any input.
    for source in ["é", "→", "\u{1F600}"] {
        let tokens = tokenize(source);
        assert_eq!(tokens.len(), 1, "{source:?} lexed as {tokens:?}");
        assert_eq!(tokens.first().and_then(|t| t.text(source)), Some(source));
    }
}

#[test]
fn the_empty_input_produces_no_tokens() {
    assert!(tokenize("").is_empty());
}
