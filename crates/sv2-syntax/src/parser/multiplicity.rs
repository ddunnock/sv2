// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Multiplicity, `SysML` 8.2.2.6.6 and `KerML` 8.2.4.3.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::Parser;

impl Parser<'_> {
    /// Whether a `MultiplicityPart` is written here (`SysML` 8.2.2.6.6).
    ///
    /// Either the `'['` of its `OwnedMultiplicity` or one of the two keywords its
    /// second alternative may carry without one.
    pub(super) fn at_multiplicity_part(&self) -> bool {
        self.at(SyntaxKind::LBracket) || self.at_keyword("ordered") || self.at_keyword("nonunique")
    }

    // production: MultiplicityPart
    //
    // MultiplicityPart : Feature =
    //       ownedRelationship += OwnedMultiplicity
    //     | ( ownedRelationship += OwnedMultiplicity )?
    //       ( isOrdered ?= 'ordered' ( { isUnique = false } 'nonunique' )?
    //       | { isUnique = false } 'nonunique' ( isOrdered ?= 'ordered' )? )
    //                                                            (SysML 8.2.2.6.6)
    //
    // The two alternatives together admit an OwnedMultiplicity, the two keywords in
    // either order, or both — and the first alternative is what makes the keywords
    // optional. `nonunique` sets isUnique false in both orderings, which is why the
    // keyword appears twice in the clause and once here.
    pub(super) fn multiplicity_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MultiplicityPart);
        if self.at(SyntaxKind::LBracket) {
            self.owned_multiplicity();
        }
        if self.at_keyword("ordered") {
            self.bump_as(SyntaxKind::KwOrdered);
            if self.at_keyword("nonunique") {
                self.bump_as(SyntaxKind::KwNonunique);
            }
        } else if self.at_keyword("nonunique") {
            self.bump_as(SyntaxKind::KwNonunique);
            if self.at_keyword("ordered") {
                self.bump_as(SyntaxKind::KwOrdered);
            }
        }
        self.finish_node();
    }

    // production: OwnedMultiplicity@sysml
    //
    // OwnedMultiplicity : OwningMembership =
    //     ownedRelatedElement += MultiplicityRange               (SysML 8.2.2.6.6)
    //
    // production: OwnedMultiplicity@kerml
    //
    // OwnedMultiplicity : OwningMembership =
    //     ownedRelatedElement += OwnedMultiplicityRange           (KerML 8.2.5.11)
    //
    // SysML's OwnedMultiplicity owns a MultiplicityRange directly. KerML's owns an
    // OwnedMultiplicityRange, because in KerML the name MultiplicityRange is the named
    // `'multiplicity' Identification MultiplicityBounds TypeBody` declaration -- a
    // different production with the same name, split by scope in ADR-0015. The TEXT is
    // the same in both, `[` bounds `]`; the language decides the node. The element is a
    // MultiplicityRange either way (KerML 8.3.4.11.2, receipt 4c789163).
    pub(super) fn owned_multiplicity(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedMultiplicity);
        match self.language {
            Language::SysMl => self.multiplicity_range(),
            Language::KerMl => self.owned_multiplicity_range(),
        }
        self.finish_node();
    }

    // production: OwnedMultiplicityRange@kerml
    //
    // OwnedMultiplicityRange : MultiplicityRange = MultiplicityBounds   (KerML 8.2.5.11)
    //
    // production: MultiplicityBounds@kerml
    //
    // MultiplicityBounds : MultiplicityRange =
    //     '[' ( ownedRelationship += MultiplicityExpressionMember '..' )?
    //           ownedRelationship += MultiplicityExpressionMember ']'
    //                                                            (KerML 8.2.5.11)
    //
    // MultiplicityBounds returns the MultiplicityRange it is written into and builds no
    // node of its own -- the Pilot states it as a fragment (KerML.xtext:774) -- so its
    // tokens are OwnedMultiplicityRange's children. Its text is SysML's MultiplicityRange
    // exactly, the lower bound the optional one: "If no lowerBound Expression, then the
    // default is that the lower bound has the same value as the upper bound" (8.3.4.11.2).
    // The bounds are the shared MultiplicityExpressionMember.
    fn owned_multiplicity_range(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedMultiplicityRange);
        self.multiplicity_bounds();
        self.finish_node();
    }

    /// `MultiplicityBounds`' text, into the caller's node: the same as `SysML`'s
    /// `MultiplicityRange` body, which `multiplicity_range` wraps in its own node.
    fn multiplicity_bounds(&mut self) {
        self.expect(SyntaxKind::LBracket, "`[`");
        self.multiplicity_expression_member();
        if self.at(SyntaxKind::DotDot) {
            self.bump();
            self.multiplicity_expression_member();
        }
        self.expect(SyntaxKind::RBracket, "`]`");
    }

    // production: MultiplicityRange@sysml
    //
    // MultiplicityRange : MultiplicityRange =
    //     '[' ( ownedRelationship += MultiplicityExpressionMember '..' )?
    //           ownedRelationship += MultiplicityExpressionMember ']'
    //                                                            (SysML 8.2.2.6.6)
    //
    // The lower bound is the optional one, so `[2]` is an upper bound alone and
    // `[0..*]` is both. Read left to right: one member, then the second only if a
    // `'..'` separates them.
    fn multiplicity_range(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MultiplicityRange);
        self.multiplicity_bounds();
        self.finish_node();
    }

    // production: MultiplicityExpressionMember
    //
    // MultiplicityExpressionMember : OwningMembership =
    //     ownedRelatedElement += ( LiteralExpression
    //                            | FeatureReferenceExpression )  (SysML 8.2.2.6.6)
    //
    // A bound is a literal or a name and NOT an OwnedExpression: the clause names
    // two alternatives, neither of which is an operator expression. `[1+1]` is
    // therefore not a multiplicity, which is what keeps this production cheap and
    // what tests/rejection/multiplicity-bound-is-not-an-operator-expression.sysml
    // holds.
    fn multiplicity_expression_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MultiplicityExpressionMember);
        if self.at_literal_expression() {
            self.literal_expression();
        } else if self.at_feature_reference() {
            self.feature_reference_expression();
        } else {
            self.error_expected("a literal or a name");
        }
        self.finish_node();
    }

    // production: EmptyMultiplicityMember
    //
    // EmptyMultiplicityMember : OwningMembership =
    //     ownedRelatedElement += EmptyMultiplicity               (SysML 8.2.2.9.1)
    //
    // production: EmptyMultiplicity
    //
    // EmptyMultiplicity : Multiplicity = { }
    //
    // No tokens, but a real element: `individual` gives the definition an owned
    // Multiplicity. The nodes are empty rather than omitted, so the tree carries the
    // element the abstract syntax says is there.
    pub(super) fn empty_multiplicity_member(&mut self) {
        self.start_node(SyntaxKind::EmptyMultiplicityMember);
        self.start_node(SyntaxKind::EmptyMultiplicity);
        self.finish_node();
        self.finish_node();
    }

    // production: Multiplicity@kerml
    // production: MultiplicitySubset@kerml
    // production: MultiplicityRange@kerml
    //
    // Multiplicity       = MultiplicitySubset | MultiplicityRange
    // MultiplicitySubset = 'multiplicity' Identification Subsets TypeBody
    // MultiplicityRange  = 'multiplicity' Identification MultiplicityBounds TypeBody
    //                                                            (KerML 8.2.5.11)
    //
    // "A multiplicity feature [is declared] using the keyword multiplicity, optionally
    // followed by a short name and/or name, and including either a multiplicity range or
    // a subsetting of another multiplicity" (7.4.12, receipt a672468d). `multiplicity`
    // is reserved (8.2.2.6) and no prefix precedes it, so the keyword decides; the token
    // after the Identification decides which: a `[`, or SUBSETS. The alternation builds no
    // node, as FeatureElement builds none. The metaclasses are Multiplicity (8.3.3.1.9,
    // receipt 46f724a5), a Feature, and MultiplicityRange (8.3.4.11.2, receipt 4c789163).
    //
    // MultiplicityRange@kerml is KerML's name for this declaration, and SysML's for the
    // bracket a feature writes (8.2.2.6.6); the node is shared, as FeatureTyping's is.
    // MultiplicityBounds is a fragment and builds no node, so its tokens are this one's.
    // Subsets is the shared `SUBSETS OwnedSubsetting` a feature specialization reads.
    //
    // implied specialization: Base::naturals for a Multiplicity
    //     (checkMultiplicitySpecialization, KerML 8.3.3.1.9), sv2-hir's; nothing is
    //     written into the tree.
    pub(super) fn kerml_multiplicity(&mut self) {
        self.eat_trivia();
        let start = self.builder.checkpoint();
        self.expect_keyword("multiplicity");
        self.identification();
        let node = if self.at(SyntaxKind::LBracket) {
            self.multiplicity_bounds();
            SyntaxKind::MultiplicityRange
        } else {
            self.subsets();
            SyntaxKind::MultiplicitySubset
        };
        self.type_body();
        self.start_node_at(start, node);
        self.finish_node();
    }
}
