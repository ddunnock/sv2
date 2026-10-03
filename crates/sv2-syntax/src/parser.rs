// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A parser for the package declaration, written from `SysML` 8.2.2.5.1.
//!
//! The productions it implements, as the specification states them:
//!
//! ```text
//! RootNamespace      = PackageBodyElement*
//! PackageBodyElement = PackageMember | ElementFilterMember | AliasMember | Import
//! PackageMember      = MemberPrefix ( DefinitionElement | UsageElement )
//! MemberPrefix       = ( visibility = VisibilityIndicator )?
//! Package            = ( ownedRelationship += PrefixMetadataMember )*
//!                      PackageDeclaration PackageBody
//! PackageDeclaration = 'package' Identification
//! PackageBody        = ';' | '{' PackageBodyElement* '}'
//! Identification     = ( '<' declaredShortName = NAME '>' )? ( declaredName = NAME )?
//! Import             = VisibilityIndicator 'import' 'all'?
//!                      ImportDeclaration RelationshipBody
//! AliasMember        = MemberPrefix 'alias' ( '<' NAME '>' )? NAME?
//!                      'for' [QualifiedName] RelationshipBody
//! RelationshipBody   = ';' | '{' OwnedAnnotation* '}'
//! PartDefinition     = OccurrenceDefinitionPrefix 'part' 'def' Definition
//! ElementFilterMember = MemberPrefix 'filter' OwnedExpression ';'
//! OwnedExpression    = ConditionalExpression | ConditionalBinaryOperatorExpression
//!                    | BinaryOperatorExpression | UnaryOperatorExpression
//!                    | ClassificationExpression | MetaclassificationExpression
//!                    | ExtentExpression | PrimaryExpression
//! ValuePart          = FeatureValue
//! MultiplicityPart   = OwnedMultiplicity
//!                    | OwnedMultiplicity? ( 'ordered' 'nonunique'?
//!                                         | 'nonunique' 'ordered'? )
//! ```
//!
//! `OwnedExpression` carries no precedence and cannot: `KerML` 8.2.5.8.1 note 2 states
//! that the grouping of nested `OperatorExpression`s is not expressed in the
//! productions and is given by that clause's table 6. The table is data at
//! `docs/operator-precedence.toml`, and `INFIX` below is what the parser reads. The
//! operator core reads all fifteen tiers, and the postfix layer of 8.2.5.8.2 reads
//! all six of its forms: `a.b`, `x[kg]`, `x#(1)`, `x->f()`, `x.?{ ... }` and `x.{ ... }`.
//!
//! All four `PackageBodyElement` alternatives are handled: `PackageMember`, `Import`,
//! `AliasMember` and `ElementFilterMember`, the last of which waited on the expression
//! layer and needs only a bare classification operator (`filter @Safety;`).
//! `Package` and `PartDefinition` are the two `DefinitionElement`s of thirty, and
//! `Comment`, `Documentation` and `TextualRepresentation` the three `AnnotatingElement`s
//! of four that a `RelationshipBody` may own. Everything else is unimplemented and
//! reports as such in the coverage report, which is the honest state of a parser this
//! young.
//!
//! Every token the lexer produced ends up in the tree, in source order. Text that no
//! production accepts becomes an `Error` node that still carries its bytes, so the
//! round-trip holds for malformed input.

mod action;
mod action_node;
mod annotation;
mod body;
mod calculation;
mod case;
mod classifier;
mod connection;
mod definition;
mod expression;
mod feature;
mod flow;
mod import;
mod invocation;
mod kernel;
mod literal;
mod lookahead;
mod metadata;
mod multiplicity;
mod namespace;
mod operator;
mod primary;
mod recovery;
mod requirement;
mod state;
mod succession;
mod tree;
mod usage;
mod view;

use std::cell::Cell;

use rowan::{GreenNode, GreenNodeBuilder};

use crate::diagnostic::Diagnostic;
use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::language::SyntaxNode;
use crate::lexer::{Token, is_trivia, tokenize};

pub use crate::parser::operator::infix_table_for_test;

/// The result of parsing: a tree, plus what went wrong.
#[derive(Debug, Clone)]
pub struct Parse {
    green: GreenNode,
    errors: Vec<Diagnostic>,
    deviations: Vec<Diagnostic>,
}

impl Parse {
    /// The root of the tree.
    #[must_use]
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// The source text, reconstructed from the tree.
    ///
    /// Equal to the input for every input. This is the losslessness invariant.
    #[must_use]
    pub fn text(&self) -> String {
        self.syntax().text().to_string()
    }

    /// What the parser could not make sense of, in source order.
    ///
    /// Each carries the range it is about, so a caller can point at it: underline the
    /// token in an editor, decorate the row in a diagram (ADR-0002), or print a line
    /// and column. A rendered sentence cannot be pointed at.
    #[must_use]
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    /// The places the text is admitted only because of a recorded deviation from the
    /// specification's BNF, in source order (ADR-0022).
    ///
    /// Each is a `PARSE-DEVIATION` diagnostic of severity `Info` whose message names its
    /// entry in `.claude/state/deviations.json`. They are NOT errors: the text parses, and
    /// [`Parse::errors`] does not contain them. A caller that needs the specification's
    /// literal reading treats them as errors — `sv2 parse --strict` does.
    ///
    /// Only EXTENSIONS are reported. A deviation that restricts the specification rejects
    /// text, and rejected text has no tree to carry a note; the register says which way
    /// each deviation runs.
    #[must_use]
    pub fn deviations(&self) -> &[Diagnostic] {
        &self.deviations
    }

    /// Whether the text parses AND depends on no recorded deviation: what a tool written
    /// from the specification's BNF alone would also accept, as far as this parser can
    /// tell (see [`Parse::deviations`] on restrictions).
    #[must_use]
    pub fn is_spec_conformant(&self) -> bool {
        self.errors.is_empty() && self.deviations.is_empty()
    }
}

/// The deepest nesting the parser will recurse into.
///
/// Invariant 3 is that the parser does not die on any input. A recursive-descent
/// parser dies on deeply nested input by overflowing the stack, and a stack overflow
/// ABORTS the process — it is not a panic and cannot be caught, so it is strictly
/// worse than the thing the invariant forbids. Measured on this parser, the shallowest
/// construct overflows somewhere between 2000 and 5000 levels; 400 is well inside that
/// and far past anything a person writes.
///
/// Reaching it is reported and recovered from, not silently truncated: the tokens
/// still enter the tree through the enclosing body's recovery, so the round-trip holds.
const MAX_DEPTH: u32 = 400;

struct Parser<'a> {
    source: &'a str,
    /// The grammar this text is read against, chosen once by the caller (ADR-0014).
    language: Language,
    tokens: Vec<Token>,
    /// The indices into `tokens` of every token lookahead sees while comments are trivia,
    /// ascending. `peek_nth` finds the cursor in it by binary search and steps `n` from
    /// there, so asking for the `n`th meaningful token costs O(log len) rather than a
    /// filter from the cursor. A recogniser that walks by index was quadratic in the
    /// length of what it walked while it did not.
    meaningful: Vec<usize>,
    /// The same, while `REGULAR_COMMENT` is significant (`with_significant_comments`).
    meaningful_with_comments: Vec<usize>,
    /// The last cursor located in an index: `(pos, comments_significant, position)`.
    /// Most lookahead asks a step or two ahead many times at one cursor, and the binary
    /// search alone made error recovery several times slower than the filter it replaced.
    cursor: Cell<(usize, bool, usize)>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<Diagnostic>,
    /// The `PARSE-DEVIATION` notes, kept apart from `errors` (ADR-0022).
    deviations: Vec<Diagnostic>,
    /// How many nesting levels of recursive production this parser is inside.
    ///
    /// Bounded by [`MAX_DEPTH`]. Invariant 3 is that the parser does not die on any
    /// input, and a recursive-descent parser on deeply nested input dies by
    /// overflowing the stack — which aborts the process rather than panicking, so it
    /// is not even catchable. Counting the levels is what keeps that from happening.
    depth: u32,
    /// Whether the depth limit has already been reported, so it is said once rather
    /// than once per level.
    depth_reported: bool,
    /// Whether a `REGULAR_COMMENT` is a token here rather than trivia.
    ///
    /// `KerML` 8.2.2.2 makes `/* ... */` a token, and `Comment`, `Documentation` and
    /// `TextualRepresentation` take it as their body (`SysML` 8.2.2.4.2, 8.2.2.4.3).
    /// Inside a braced `RelationshipBody` every regular comment is therefore either an
    /// annotation's body or an error, never text to skip past. At a member position a
    /// bare one is a `Comment` member (see `at_bare_comment_member`), and anywhere else
    /// `eat_trivia` reports it: the Pilot hides only `WS`, `ML_NOTE` and `SL_NOTE`
    /// (KerMLExpressions.xtext:29), never `REGULAR_COMMENT`.
    comments_significant: bool,
    /// The `depth` of the `RequirementBody` a `FramedConcernUsage`'s reference alternative
    /// is reading, while it reads it, and `None` otherwise.
    ///
    /// Deviation `FramedConcernUsage` gives that alternative a `RequirementBody` where the
    /// printed clause gives a `CalculationBody` (`SysML` 8.2.2.21.1). The two share every
    /// item but `RequirementBodyItem`'s six own members, so a note falls on each of those
    /// six read at exactly this depth: `note_framed_concern_body_item` says so, and a
    /// body nested in one reads at a greater depth and draws none.
    framed_concern_body: Option<u32>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, language: Language) -> Self {
        let tokens = tokenize(source);
        let indices = |keep: fn(SyntaxKind) -> bool| -> Vec<usize> {
            tokens
                .iter()
                .enumerate()
                .filter(|(_, token)| keep(token.kind))
                .map(|(i, _)| i)
                .collect()
        };
        let meaningful = indices(|kind| !is_trivia(kind));
        let meaningful_with_comments =
            indices(|kind| !is_trivia(kind) || kind == SyntaxKind::RegularComment);
        Self {
            source,
            language,
            tokens,
            meaningful,
            meaningful_with_comments,
            cursor: Cell::new((0, false, 0)),
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            deviations: Vec::new(),
            depth: 0,
            depth_reported: false,
            comments_significant: false,
            framed_concern_body: None,
        }
    }

    fn text_of(&self, token: Token) -> &'a str {
        token.text(self.source).unwrap_or("")
    }
}

/// Parse `source` against `language`'s grammar into a lossless tree.
///
/// `KerML` and `SysML` are two grammars with two start symbols, not one grammar that
/// extends the other, so the caller says which is meant (ADR-0014). There is no default:
/// reading a file against the grammar its author did not write it in accepts constructs
/// the language does not have, and a positive-only corpus sweep cannot see that happen.
/// [`Language::from_path`] is how a caller holding a path chooses.
///
/// Never fails and never panics: text no production accepts becomes `Error` nodes
/// that keep their bytes, so `parse(s, l).text() == s` for every `s` and every `l`.
#[must_use]
pub fn parse(source: &str, language: Language) -> Parse {
    let (green, errors, deviations) = Parser::new(source, language).root_namespace();
    Parse {
        green,
        errors,
        deviations,
    }
}

#[cfg(test)]
mod tests {
    use super::{Language, Parser, SyntaxKind};
    use crate::parser::lookahead::{VISIBILITY, keyword};

    /// Every keyword this parser looks up by text must exist in the pinned token set.
    ///
    /// `package_declaration` falls back to `BasicName` when the lookup misses, so a
    /// keyword leaving the token set would not fail at run time — it would quietly
    /// mis-tag the node. This is what turns that into a gate failure instead. Extend
    /// the list as productions are added.
    /// Every keyword a production looks up by text, beyond the `VISIBILITY` table
    /// the test below chains on. Extend as productions are added.
    const NAMED: &[&str] = &[
        "package",
        "import",
        "all",
        "alias",
        "for",
        "part",
        "def",
        "abstract",
        "variation",
        "individual",
        "merge",
        "decide",
        "join",
        "fork",
        "specializes",
        "comment",
        "about",
        "locale",
        "doc",
        "rep",
        "language",
        // The keywords a KerML FeaturePrefix stands before. `nth_continues_feature`
        // counts a NAME as continuing a feature, so `feature_prefix` tells the `#` before
        // one of these from the `#` in `feature`'s place only while each is reserved
        // (KerML 8.2.2.6) and so no NAME.
        "feature",
        "connector",
        "succession",
        "binding",
        "step",
        "expr",
        "inv",
        "flow",
    ];

    #[test]
    fn every_keyword_this_parser_names_is_in_the_pinned_token_set() {
        // VISIBILITY is chained rather than copied: it is the list the parser itself
        // uses, so a word added there cannot drift out of this check.
        for text in NAMED.iter().chain(VISIBILITY.iter()) {
            assert!(
                keyword(text).is_some(),
                "`{text}` is not in the pinned token set; \
                 .claude/state/grammar/keywords.json moved and this parser did not"
            );
        }
    }

    #[test]
    fn a_word_that_is_not_a_keyword_has_no_kind() {
        // The negative case: the lookup must not match everything, or the fallback
        // above would never be exercised and the test would prove nothing.
        assert_eq!(keyword("Vehicle"), None);
        assert_eq!(keyword(""), None);
    }

    #[test]
    fn the_package_keyword_is_not_tagged_as_a_name() {
        // The property the snapshots pin as a side effect, stated directly.
        assert_ne!(keyword("package"), Some(SyntaxKind::BasicName));
    }

    /// `SysML`'s `ExpressionBody` is `CalculationBody`'s braced alternative alone
    /// (decision expression-body-semicolon). Every caller dispatches on `{`, so no text reaches
    /// `body_expression` at a `;`; this drives it there directly, so the narrowing is
    /// held by the production and not only by its callers.
    #[test]
    fn a_sysml_expression_body_does_not_take_the_semicolon_alternative() {
        let mut braced = Parser::new("{ x }", Language::SysMl);
        braced.body_expression();
        assert!(braced.errors.is_empty());

        let mut bare = Parser::new(";", Language::SysMl);
        bare.body_expression();
        assert!(!bare.errors.is_empty());

        // The calculation body a `calc def` owns still takes it (SysML 8.2.2.19).
        let mut calc = Parser::new(";", Language::SysMl);
        calc.calculation_body();
        assert!(calc.errors.is_empty());
    }

    /// The meaningful-token index answers exactly what filtering the tokens from the
    /// cursor answered, at every cursor, every distance and in both comment modes,
    /// including past the end. Text with both comment forms, a note, whitespace runs and
    /// an unterminated comment, so every kind of trivia sits somewhere in it.
    #[test]
    fn the_meaningful_index_agrees_with_filtering_from_the_cursor() {
        let source = "package /* c */ P { // n\n  part   a : A; /* d */ doc /* e */ } /* open";
        let mut parser = Parser::new(source, Language::SysMl);
        let total = parser.tokens.len();
        for significant in [false, true] {
            parser.comments_significant = significant;
            for pos in 0..=total + 1 {
                parser.pos = pos;
                assert_lookahead_matches_filter(&parser);
            }
        }
    }

    /// Every distance from the parser's cursor, against the filter `peek_nth` replaced.
    fn assert_lookahead_matches_filter(parser: &Parser<'_>) {
        let expected: Vec<_> = parser
            .tokens
            .get(parser.pos..)
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|token| !parser.skippable(token.kind))
            .collect();
        for n in 0..=expected.len() + 1 {
            assert_eq!(parser.peek_nth(n), expected.get(n).copied());
            let from: Vec<_> = parser.meaningful_from(n).collect();
            assert_eq!(from, expected.get(n..).unwrap_or(&[]));
        }
    }
}
