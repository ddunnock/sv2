// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Relationship bodies and annotations, `SysML` 8.2.2.4 and `KerML` 8.2.3.3: comments,
//! documentation, textual representations, and the elements a relationship body may own.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::lexer::is_trivia;
use crate::parser::lookahead::{VISIBILITY, keyword};
use crate::parser::{Body, MAX_DEPTH, Parser};

impl Parser<'_> {
    /// Whether an implemented `KerML` `NonFeatureElement` starts at the `n`th meaningful
    /// token (8.2.3.4.3): the `MemberElement`s that are no `AnnotatingElement` and no
    /// `FeatureElement`. A `MetadataBody` asks this alone, since its members are
    /// `NonFeatureMember`s and its features are its own (8.2.5.12).
    pub(super) fn at_kerml_non_feature_element(&self, n: usize) -> bool {
        self.at_package(n)
            || self.at_library_package(n)
            || self.at_dependency(n)
            || self.at_classifier(n).is_some()
            || self.at_relationship_declaration(n).is_some()
            || self.at_kerml_type(n)
            || self.at_kerml_function(n).is_some()
            || self.at_kerml_namespace(n)
            || self.nth_is_keyword(n, "multiplicity")
    }

    /// Whether an implemented `AnnotatingElement` starts here (`SysML` 8.2.2.4.1).
    ///
    /// `Comment` may open with `comment`, `locale` or its bare `REGULAR_COMMENT` body;
    /// `Documentation` with `doc`; `TextualRepresentation` with `rep` or `language`;
    /// `MetadataUsage` in `SysML` and `MetadataFeature` in `KerML` with `@` or `metadata`,
    /// or the `#` prefix metadata before either (see `at_metadata_element`).
    fn at_annotating_element(&self) -> bool {
        self.at(SyntaxKind::RegularComment)
            || ["comment", "locale", "doc", "rep", "language"]
                .iter()
                .any(|word| self.at_keyword(word))
            || self.at_metadata_element(0)
    }

    /// Whether a bare `REGULAR_COMMENT` opens the next member, after an optional
    /// visibility: a `Comment` with none of its optional parts (`KerML` 8.2.3.3.2, `SysML`
    /// 8.2.2.4.2), and so an `AnnotatingElement` wherever a member may stand. Asked by
    /// every member loop before anything that would eat it as trivia; the member is then
    /// read with comments significant, so `membership` sees the comment and reads it as
    /// the `Comment` it is. A visibility before it is that `Comment`'s `MemberPrefix`, so in
    /// `private /* a */ part x;` the `part` after it is a member of its own, and public.
    pub(super) fn at_bare_comment_member(&self) -> bool {
        let mut meaningful = self
            .tokens
            .get(self.pos..)
            .unwrap_or(&[])
            .iter()
            .filter(|token| !is_trivia(token.kind) || token.kind == SyntaxKind::RegularComment);
        match meaningful.next() {
            Some(token) if token.kind == SyntaxKind::RegularComment => true,
            Some(token) if VISIBILITY.contains(&self.text_of(*token)) => meaningful
                .next()
                .is_some_and(|token| token.kind == SyntaxKind::RegularComment),
            _ => false,
        }
    }

    /// The `Comment` member `at_bare_comment_member` found, owned through the membership
    /// `body` gives its other members, read with comments significant.
    pub(super) fn bare_comment_member(&mut self, body: Body) {
        self.with_significant_comments(|p| {
            p.membership(body);
        });
    }

    // production: AnnotatingMember@sysml
    //
    // AnnotatingMember : OwningMembership =
    //     MemberPrefix ownedRelatedElement += AnnotatingElement       (SysML 8.2.2.4.1)
    //
    // Referenced by EnumerationBody alone, which is why it had no caller until now.
    // Its own two parts are read, and AnnotatingElement@sysml is marked at
    // `annotating_element`.
    pub(super) fn annotating_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AnnotatingMember);
        if self.at_visibility() {
            // deviation: AnnotatingMember
            self.note_deviation(
                "AnnotatingMember",
                "a visibility on an enumeration body's annotation",
            );
        }
        self.member_prefix();
        self.annotating_element();
        self.finish_node();
    }

    /// The `RelationshipBody` that closes an import, an alias, a dependency or `SysML`'s
    /// `InitialNodeMember` — which is two productions, one per language, and the one
    /// reached is the file's.
    ///
    /// `SysML`'s owns annotations only (8.2.2.2); `KerML`'s owns related elements as
    /// well (8.2.3.1). Every caller ends in the production its own language states, so
    /// the choice is made here once rather than at each of them.
    pub(super) fn relationship_body(&mut self) {
        match self.language {
            Language::SysMl => self.sysml_relationship_body(),
            Language::KerMl => self.kerml_relationship_body(),
        }
    }

    // production: RelationshipBody@sysml
    //
    // RelationshipBody = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'
    //                                                            (SysML 8.2.2.2)
    //
    // Inside the braces a regular comment is a token at item position (see
    // comments_significant), so `{ /* text */ }` owns a Comment rather than skipping one
    // — the corpus's `private import Definitions::* { /* ... */ }` is exactly that.
    // Anything that is not an implemented AnnotatingElement is recovered over one token
    // at a time and reported: accepting it silently would report an annotation this
    // parser cannot read as one it understood.
    //
    // A comment is a token only while deciding whether an annotation starts, and each
    // annotation scopes the mode itself, as `kerml_relationship_body` does. The whole run
    // was once read with comments significant, which was harmless while every annotation
    // here ended in a comment body; a MetadataUsage does not, and its own MetadataBody
    // holds ordinary comments between tokens and Comment members that must be read as
    // they are anywhere else.
    fn sysml_relationship_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RelationshipBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.owned_annotations();
            self.depth -= 1;
        } else {
            self.error_expected("`;` or `{` to close the relationship");
        }
        self.finish_node();
    }

    // production: RelationshipBody@kerml
    //
    // RelationshipBody : Relationship =
    //     ';' | '{' RelationshipOwnedElement* '}'                    (KerML 8.2.3.1)
    //
    // RelationshipOwnedElement : Relationship =
    //       ownedRelatedElement += OwnedRelatedElement
    //     | ownedRelationship += OwnedAnnotation                     (KerML 8.2.3.1)
    //
    // OwnedRelatedElement : Element = NonFeatureElement | FeatureElement
    //
    // RelationshipOwnedElement and OwnedRelatedElement are marked at
    // `relationship_owned_elements` and `owned_related_element`.
    // Neither alternation has a node, as FeatureSpecialization has none: the element
    // read says which was taken.
    //
    // An owned related element is the relationship's ownedRelatedElement, with no
    // Membership between them, so no MemberPrefix is read: `private feature e;` is
    // reported. AliasMember and Import are NamespaceBodyElements and are no items here.
    //
    // A regular comment is a token only while deciding whether an annotation starts, and
    // `owned_annotation` makes it one while reading it. A nested element's own body reads
    // comments as it does anywhere else, which it could not if the whole run were read
    // under `with_significant_comments`, as SysML's is.
    fn kerml_relationship_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::RelationshipBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.relationship_owned_elements();
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` to close the relationship");
        }
        self.finish_node();
    }

    // production: RelationshipOwnedElement@kerml
    //
    // RelationshipOwnedElement : Relationship =
    //       ownedRelatedElement += OwnedRelatedElement
    //     | ownedRelationship += OwnedAnnotation                     (KerML 8.2.3.1)
    //
    // Both alternatives, asked once per item of the `*` in RelationshipBody: an
    // annotation when one starts here with comments significant, since a bare
    // REGULAR_COMMENT is a Comment in this position, and otherwise an owned related
    // element. An alternation with no node: the OwnedAnnotation node, or the element
    // itself, says which.
    //
    /// `RelationshipOwnedElement*` in a `KerML` `RelationshipBody`, up to its `}`.
    fn relationship_owned_elements(&mut self) {
        loop {
            let outer = self.comments_significant;
            self.comments_significant = true;
            let annotating = self.at_annotating_element();
            self.comments_significant = outer;
            if self.at_end() || (!annotating && self.at(SyntaxKind::RBrace)) {
                return;
            }
            let start = self.pos;
            if self.depth >= MAX_DEPTH {
                // As `body_element` does: recover a token at a time rather than recurse
                // into another body (invariant 3).
                self.report_too_deep();
                self.error_token();
            } else if annotating {
                self.owned_annotation();
            } else if !self.owned_related_element() {
                self.recover_statement();
            }
            if self.pos == start {
                self.error_token();
            }
        }
    }

    // production: OwnedRelatedElement@kerml
    //
    // OwnedRelatedElement : Element = NonFeatureElement | FeatureElement
    //                                                            (KerML 8.2.3.1)
    //
    // Both alternatives, each whole: `kerml_non_feature_element` and `feature_element`.
    // The non-feature elements are asked first;
    // `body_element` asks a namespace body's feature member before its non-feature one,
    // the other way round, so the order is not what tells them apart: each recogniser
    // does. `feature_element` asks the keyword FeatureElements before the
    // keywordless-capable Feature. No node: the element says which.
    //
    /// An `OwnedRelatedElement` of `KerML`, with no `MemberPrefix`. Returns whether one
    /// was read.
    fn owned_related_element(&mut self) -> bool {
        if self.kerml_non_feature_element() {
            // Read by the call.
        } else if self.at_kerml_keyword_feature_element(0) || self.at_feature(0) {
            self.feature_element();
        } else {
            return false;
        }
        true
    }

    // production: NonFeatureElement@kerml
    //
    // NonFeatureElement : Element =
    //       Dependency | Namespace | Type | Classifier | DataType | Class | Structure
    //     | Metaclass | Association | AssociationStructure | Interaction | Behavior
    //     | Function | Predicate | Multiplicity | Package | LibraryPackage
    //     | Specialization | Conjugation | Subclassification | Disjoining
    //     | FeatureInverting | FeatureTyping | Subsetting | Redefinition
    //     | TypeFeaturing                                          (KerML 8.2.3.4.3)
    //
    // All twenty-six alternatives, if one is written here. Returns whether one was read.
    // Asked by `membership` for a MemberElement and by `owned_related_element` for an
    // OwnedRelatedElement (8.2.3.1), the two places a NonFeatureElement is owned, so the
    // list cannot drift between them. The caller guards the language. An alternation
    // with no node, as FeatureElement has none: the element read says which.
    //
    // Package and LibraryPackage are shared units, and Dependency is stated in both
    // grammars; SysML reaches those three by its own routes. The eight classifiers are
    // `classifier`'s, the nine relationship declarations `relationship_declaration`'s.
    // The order is the one `membership` asked in before this was one method.
    // `owned_related_element` asked the classifiers after the KerML-only elements; no
    // input reads differently, since each recogniser requires its own reserved word
    // after the prefixes it skips, and no two share one.
    pub(super) fn kerml_non_feature_element(&mut self) -> bool {
        if self.at_package(0) {
            self.package();
        } else if self.at_library_package(0) {
            self.library_package();
        } else if self.at_dependency(0) {
            self.dependency();
        } else if let Some(classifier) = self.at_classifier(0) {
            self.classifier(classifier);
        } else if let Some(declaration) = self.at_relationship_declaration(0) {
            self.relationship_declaration(declaration);
        } else if self.at_kerml_type(0) {
            self.kerml_type();
        } else if let Some(function) = self.at_kerml_function(0) {
            self.kerml_function(function);
        } else if self.at_kerml_namespace(0) {
            self.kerml_namespace();
        } else if self.at_keyword("multiplicity") {
            self.kerml_multiplicity();
        } else {
            return false;
        }
        true
    }

    /// `OwnedAnnotation* '}'`, the rest of a braced `RelationshipBody`.
    fn owned_annotations(&mut self) {
        loop {
            let outer = self.comments_significant;
            self.comments_significant = true;
            let annotating = self.at_annotating_element();
            self.comments_significant = outer;
            if self.at_end() || (!annotating && self.at(SyntaxKind::RBrace)) {
                break;
            }
            let start = self.pos;
            if self.depth >= MAX_DEPTH {
                // As `body_element` does (invariant 3).
                self.report_too_deep();
                self.error_token();
            } else if annotating {
                self.owned_annotation();
            } else {
                self.error_token();
            }
            if self.pos == start {
                self.error_token();
            }
        }
        self.expect(SyntaxKind::RBrace, "`}`");
    }

    // production: OwnedAnnotation
    //
    // OwnedAnnotation : Annotation = ownedRelatedElement += AnnotatingElement
    //                                                            (SysML 8.2.2.4.1)
    //
    // AnnotatingElement = Comment | Documentation | TextualRepresentation
    //                   | MetadataUsage
    //
    // The fourth alternative is MetadataUsage by deviation AnnotatingElement
    // (follow_xtext); the clause prints MetadataFeature. MetadataUsage is read whole, its
    // `#` extension keywords included, and AnnotatingElement@sysml is marked at
    // `annotating_element`. It gets no node —
    // like DefinitionElement it is an alternation whose matched element already says
    // which alternative was taken; `annotating_element` reads it.
    //
    // The trivia before the node is eaten in the mode the element will be read in: with
    // comments significant for the three comment-bodied alternatives, whose body may be
    // the very next comment, and without for a metadata element, which
    // `annotating_element` asks about with the same peek.
    fn owned_annotation(&mut self) {
        if self.at_metadata_element_significantly() {
            self.eat_trivia();
        } else {
            self.with_significant_comments(Self::eat_trivia);
        }
        self.start_node(SyntaxKind::OwnedAnnotation);
        self.annotating_element();
        self.finish_node();
    }

    // production: AnnotatingElement@kerml
    // production: AnnotatingElement@sysml
    //
    // AnnotatingElement : AnnotatingElement =
    //     Comment | Documentation | TextualRepresentation | MetadataFeature
    //                                                            (KerML 8.2.3.3.1)
    //
    // The alternation, all four alternatives, and the one dispatch for every place an
    // annotating element is reached: `OwnedAnnotation` in a relationship body,
    // `MemberElement` in KerML (8.2.3.4.1), and `DefinitionElement` in SysML (8.2.2.6.1).
    // Writing it twice is how they would drift apart. No node: the element read says
    // which alternative was taken.
    //
    // Two units, both read here. KerML's fourth alternative is the clause's own
    // MetadataFeature (MetadataFeature@kerml); SysML's is a MetadataUsage by deviation
    // AnnotatingElement (follow_xtext), read whole with its `UsageExtensionKeyword`s
    // (MetadataUsage@sysml). The deviation changes the element kind, not the text, and
    // was adjudicated to need no site (see `metadata_annotating_element`).
    //
    // The three comment-bodied alternatives read their body as a TOKEN, so they run with
    // comments significant. A metadata element has no such body, and inside that mode an
    // ordinary `/* */` between its tokens would be read as a stray token, so it is
    // dispatched before the mode is entered.
    //
    // A bare REGULAR_COMMENT is a Comment wherever it is asked to be one: at a member
    // position, which `at_bare_comment_member` finds, and in a relationship body.
    pub(super) fn annotating_element(&mut self) {
        if self.at_metadata_element_significantly() {
            self.metadata_annotating_element();
        } else {
            self.with_significant_comments(Self::comment_bodied_annotating_element);
        }
    }

    /// `AnnotatingElement`'s fourth alternative, as deviation `AnnotatingElement` reads it.
    ///
    /// No `PARSE-DEVIATION` note, because the deviation adds no TEXT. `SysML` 8.2.2.4.1
    /// prints `KerML`'s `MetadataFeature` here, but under ADR-0015's resolution — `SysML`
    /// reads its own productions and `KerML`'s only for what it does not state — that
    /// literal alternative reaches `SysML`'s own `PrefixMetadataMember`, `MetadataBody`
    /// and `OwnedFeatureTyping`, and its one `KerML`-only production,
    /// `MetadataFeatureDeclaration`, prints the same text as `MetadataUsageDeclaration`.
    /// What the deviation changes is the element built, a `MetadataUsage` rather than a
    /// bare `MetadataFeature`, and ADR-0022 notes text, not element kinds. `defined by` in
    /// the declaration IS a textual departure, deviation `MetadataUsageDeclaration`'s,
    /// noted where it is read. Adjudicated 2026-09-22; the entry is listed as needing no
    /// site.
    ///
    /// In `KerML` the alternative is the clause's own, `MetadataFeature`, and needs no
    /// deviation.
    fn metadata_annotating_element(&mut self) {
        match self.language {
            Language::SysMl => self.metadata_usage(),
            Language::KerMl => self.metadata_feature(),
        }
    }

    /// `Comment | Documentation | TextualRepresentation`, the three of `AnnotatingElement`'s
    /// alternatives whose body is a `REGULAR_COMMENT` (`KerML` 8.2.3.3.1, `SysML`
    /// 8.2.2.4.1). Read only by `annotating_element`, which dispatches the fourth first.
    ///
    /// The caller is responsible for `with_significant_comments`: every one of these
    /// productions ends in a `REGULAR_COMMENT` body, which is trivia unless the enclosing
    /// context has made it a token.
    fn comment_bodied_annotating_element(&mut self) {
        if self.at_keyword("doc") {
            self.documentation();
        } else if self.at_keyword("rep") || self.at_keyword("language") {
            self.textual_representation();
        } else {
            self.comment();
        }
    }

    /// Whether a keyword-introduced `AnnotatingElement` starts at the `n`th token.
    ///
    /// Not the same question as `at_annotating_element`, deliberately. That one also
    /// answers yes to a bare `REGULAR_COMMENT`, which is `Comment`'s shortest form and
    /// is correct inside a relationship body, where every regular comment is either an
    /// annotation's body or an error.
    ///
    /// At member position it is not correct yet. A bare `/* ... */` in a package body IS
    /// a `Comment` element by 8.2.2.4.2, and this parser still attaches it as trivia —
    /// both readings keep every byte, so the round trip holds either way, but the tree
    /// shape differs from the specification's. Making the switch changes every tree that
    /// has a comment in it, so it is its own change with its own snapshot review rather
    /// than a side effect of this one.
    pub(super) fn at_annotating_member(&self, n: usize) -> bool {
        ["comment", "locale", "doc", "rep", "language"]
            .iter()
            .any(|word| self.nth_is_keyword(n, word))
            || self.at_metadata_element(n)
            // Only ever true with comments significant, which is how a member loop reads
            // the member `at_bare_comment_member` found.
            || self.nth_is(n, SyntaxKind::RegularComment)
    }

    // production: Comment
    //
    // Comment =
    //     ( 'comment' Identification
    //       ( 'about' ownedRelationship += Annotation
    //         ( ',' ownedRelationship += Annotation )* )? )?
    //     ( 'locale' locale = STRING_VALUE )?
    //     body = REGULAR_COMMENT                                 (SysML 8.2.2.4.2)
    //
    // `locale` belongs after the whole optional header, not inside it.
    fn comment(&mut self) {
        self.with_significant_comments(|p| {
            p.eat_trivia();
            p.start_node(SyntaxKind::Comment);
            if p.at_keyword("comment") {
                p.comment_header();
            }
            p.locale();
            p.expect(SyntaxKind::RegularComment, "a comment body `/* ... */`");
            p.finish_node();
        });
    }

    /// `'comment' Identification ( 'about' Annotation ( ',' Annotation )* )?`.
    fn comment_header(&mut self) {
        self.bump_as(keyword("comment").unwrap_or(SyntaxKind::BasicName));
        self.identification();
        if !self.at_keyword("about") {
            return;
        }
        self.bump_as(keyword("about").unwrap_or(SyntaxKind::BasicName));
        self.annotation();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.annotation();
        }
    }

    // production: Documentation
    //
    // Documentation =
    //     'doc' Identification ( 'locale' locale = STRING_VALUE )?
    //     body = REGULAR_COMMENT                                 (SysML 8.2.2.4.2)
    fn documentation(&mut self) {
        self.with_significant_comments(|p| {
            p.eat_trivia();
            p.start_node(SyntaxKind::Documentation);
            p.bump_as(keyword("doc").unwrap_or(SyntaxKind::BasicName));
            p.identification();
            p.locale();
            p.expect(
                SyntaxKind::RegularComment,
                "a documentation body `/* ... */`",
            );
            p.finish_node();
        });
    }

    // production: TextualRepresentation
    //
    // TextualRepresentation =
    //     ( 'rep' Identification )?
    //     'language' language = STRING_VALUE
    //     body = REGULAR_COMMENT                                 (SysML 8.2.2.4.3)
    fn textual_representation(&mut self) {
        self.with_significant_comments(|p| {
            p.eat_trivia();
            p.start_node(SyntaxKind::TextualRepresentation);
            if p.at_keyword("rep") {
                p.bump_as(keyword("rep").unwrap_or(SyntaxKind::BasicName));
                p.identification();
            }
            p.expect_keyword("language");
            p.expect(SyntaxKind::StringValue, "a language name string");
            p.expect(
                SyntaxKind::RegularComment,
                "a representation body `/* ... */`",
            );
            p.finish_node();
        });
    }

    /// `( 'locale' locale = STRING_VALUE )?`, shared by `Comment` and `Documentation`.
    fn locale(&mut self) {
        if self.at_keyword("locale") {
            self.bump_as(keyword("locale").unwrap_or(SyntaxKind::BasicName));
            self.expect(SyntaxKind::StringValue, "a locale string");
        }
    }

    // production: Annotation
    //
    // Annotation = annotatedElement = [QualifiedName]            (SysML 8.2.2.4.1)
    pub(super) fn annotation(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Annotation);
        self.qualified_name();
        self.finish_node();
    }
}
