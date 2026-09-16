// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A parser for the package declaration, written from `SysML` 8.2.2.5.1.
//!
//! The productions it implements, as the specification states them:
//!
//! ```text
//! RootNamespace      = PackageBodyElement*
//! Package            = ( ownedRelationship += PrefixMetadataMember )*
//!                      PackageDeclaration PackageBody
//! PackageDeclaration = 'package' Identification
//! PackageBody        = ';' | '{' PackageBodyElement* '}'
//! Identification     = ( '<' declaredShortName = NAME '>' )? ( declaredName = NAME )?
//! ```
//!
//! Only `Package` is handled so far; every other `PackageBodyElement` alternative is
//! unimplemented and reports as such in the coverage report, which is the honest
//! state of a parser this young.
//!
//! Every token the lexer produced ends up in the tree, in source order. Text that no
//! production accepts becomes an `Error` node that still carries its bytes, so the
//! round-trip holds for malformed input.

use rowan::{GreenNode, GreenNodeBuilder, Language as _};

use crate::generated::kinds::{KEYWORDS, SyntaxKind};
use crate::language::{Sv2Language, SyntaxNode};
use crate::lexer::{Token, is_trivia, is_unterminated_comment, tokenize};

/// The result of parsing: a tree, plus what went wrong.
#[derive(Debug, Clone)]
pub struct Parse {
    green: GreenNode,
    errors: Vec<String>,
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
    #[must_use]
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
}

/// The kind the pinned token set gives `text`, or `None` if it names no keyword.
///
/// The keyword table is generated from the pinned token set, so asking it is what
/// keeps this parser tied to the pin rather than to a constant written out here.
///
/// The lookup returning `None` does not fail loudly — the caller falls back to
/// `BasicName`, because a parser that panics because a token set moved is worse than
/// one that mis-tags a node. What makes the fallback safe to have is
/// `every_keyword_this_parser_names_is_in_the_pinned_token_set` below: a keyword
/// leaving the token set fails the gate there, not silently at run time.
fn keyword(text: &str) -> Option<SyntaxKind> {
    KEYWORDS
        .iter()
        .find(|(k, _)| *k == text)
        .map(|(_, kind)| *kind)
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<String>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            tokens: tokenize(source),
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    fn text_of(&self, token: Token) -> &'a str {
        token.text(self.source).unwrap_or("")
    }

    // -- looking ahead ----------------------------------------------------------

    /// The next non-trivia token, without consuming anything.
    fn peek(&self) -> Option<Token> {
        self.tokens
            .get(self.pos..)?
            .iter()
            .find(|token| !is_trivia(token.kind))
            .copied()
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.peek().is_some_and(|token| token.kind == kind)
    }

    /// Whether the next meaningful token is a NAME.
    ///
    /// NAME is `BASIC_NAME` | `UNRESTRICTED_NAME` (`KerML` 8.2.2.3).
    ///
    /// A reserved word is excluded. `KerML` 8.2.2.6: "a reserved keyword is a token
    /// that has the lexical structure of a basic name but cannot actually be used as a
    /// basic name". The lexer cannot make that distinction, because `package` and
    /// `Vehicle` are the same token shape; the pinned keyword table is what separates
    /// them, and asking it here is what keeps `package package;` from declaring a
    /// package named `package`.
    fn at_name(&self) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };
        match token.kind {
            SyntaxKind::UnrestrictedName => true,
            SyntaxKind::BasicName => keyword(self.text_of(token)).is_none(),
            _ => false,
        }
    }

    /// Whether the next meaningful token is this keyword.
    fn at_keyword(&self, text: &str) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };
        token.kind == SyntaxKind::BasicName && self.text_of(token) == text
    }

    fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    // -- building the tree ------------------------------------------------------

    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(Sv2Language::kind_to_raw(kind));
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    /// Attach pending trivia to the tree. Never skipped — losslessness depends on it.
    ///
    /// Every trivia token passes through here exactly once, which is why the
    /// unterminated-comment diagnostic is raised here rather than at each call site.
    fn eat_trivia(&mut self) {
        while let Some(token) = self.tokens.get(self.pos).copied() {
            if !is_trivia(token.kind) {
                break;
            }
            if is_unterminated_comment(token.kind, self.text_of(token)) {
                self.errors
                    .push("comment is never closed: expected `*/`".to_owned());
            }
            self.push(token, token.kind);
        }
    }

    fn push(&mut self, token: Token, kind: SyntaxKind) {
        self.builder
            .token(Sv2Language::kind_to_raw(kind), self.text_of(token));
        self.pos += 1;
    }

    /// Consume the next non-trivia token, with the trivia before it.
    fn bump(&mut self) {
        if let Some(token) = self.peek() {
            self.bump_as(token.kind);
        }
    }

    /// Consume the next non-trivia token, tagged as `kind` rather than as it lexed.
    ///
    /// Keywords reach the parser as `BasicName`; the production that recognises one
    /// says so here, and the tree records the keyword the token set names.
    fn bump_as(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        if let Some(token) = self.tokens.get(self.pos).copied() {
            self.push(token, kind);
        }
    }

    // -- diagnostics and recovery -----------------------------------------------

    /// Consume the next token as `kind`, or record an error without consuming.
    fn expect(&mut self, kind: SyntaxKind, what: &str) {
        if self.at(kind) {
            self.bump();
        } else {
            self.error_expected(what);
        }
    }

    /// Consume the next token as a NAME, or record an error without consuming.
    fn expect_name(&mut self, what: &str) {
        if self.at_name() {
            self.bump();
        } else {
            self.error_expected(what);
        }
    }

    fn error_expected(&mut self, what: &str) {
        let found = self
            .peek()
            .map_or("end of file", |token| self.text_of(token));
        self.errors
            .push(format!("expected {what}, found `{found}`"));
    }

    /// One token nothing accepts, wrapped so its bytes survive in the tree.
    fn error_token(&mut self) {
        self.eat_trivia();
        let Some(token) = self.tokens.get(self.pos).copied() else {
            return;
        };
        self.errors
            .push(format!("unexpected `{}`", self.text_of(token)));
        self.start_node(SyntaxKind::Error);
        self.push(token, token.kind);
        self.finish_node();
    }

    // -- productions ------------------------------------------------------------

    // production: RootNamespace
    fn root_namespace(mut self) -> (GreenNode, Vec<String>) {
        self.start_node(SyntaxKind::RootNamespace);
        self.body_elements(None);
        // Trailing trivia belongs to the tree as much as anything else.
        self.eat_trivia();
        self.finish_node();
        (self.builder.finish(), self.errors)
    }

    /// `PackageBodyElement*`, up to `until` or end of input.
    ///
    /// Only `Package` is implemented. Anything else is recovered over one token at a
    /// time rather than abandoning the enclosing body: an editor reparses invalid
    /// text constantly, and a body that vanishes on one bad token blanks the diagram
    /// on every keystroke.
    fn body_elements(&mut self, until: Option<SyntaxKind>) {
        while !self.at_end() && !until.is_some_and(|kind| self.at(kind)) {
            if self.at_keyword("package") {
                self.package();
            } else {
                self.error_token();
            }
        }
    }

    // production: Package
    fn package(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Package);
        self.package_declaration();
        self.package_body();
        self.finish_node();
    }

    // production: PackageDeclaration
    fn package_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PackageDeclaration);
        self.bump_as(keyword("package").unwrap_or(SyntaxKind::BasicName));
        self.identification();
        self.finish_node();
    }

    // production: Identification
    fn identification(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Identification);
        if self.at(SyntaxKind::Lt) {
            self.bump();
            self.expect_name("a short name");
            self.expect(SyntaxKind::Gt, "`>`");
        }
        if self.at_name() {
            self.bump();
        }
        self.finish_node();
    }

    // production: PackageBody
    fn package_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PackageBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.body_elements(Some(SyntaxKind::RBrace));
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.errors
                .push("expected `;` or `{` after a package declaration".to_owned());
        }
        self.finish_node();
    }
}

/// Parse `SysML` v2 or `KerML` text into a lossless tree.
///
/// Never fails and never panics: text no production accepts becomes `Error` nodes
/// that keep their bytes, so `parse(s).text() == s` for every `s`.
#[must_use]
pub fn parse(source: &str) -> Parse {
    let (green, errors) = Parser::new(source).root_namespace();
    Parse { green, errors }
}

#[cfg(test)]
mod tests {
    use super::{SyntaxKind, keyword};

    /// Every keyword this parser looks up by text must exist in the pinned token set.
    ///
    /// `package_declaration` falls back to `BasicName` when the lookup misses, so a
    /// keyword leaving the token set would not fail at run time — it would quietly
    /// mis-tag the node. This is what turns that into a gate failure instead. Extend
    /// the list as productions are added.
    /// Every keyword a production looks up by text. Extend as productions are added.
    const NAMED: &[&str] = &["package"];

    #[test]
    fn every_keyword_this_parser_names_is_in_the_pinned_token_set() {
        for text in NAMED {
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
}
