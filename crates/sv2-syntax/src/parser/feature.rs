// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Features and feature specialization, `KerML` 8.2.4.3 and `SysML` 8.2.2.6: prefixes,
//! declarations, typing, subsetting, redefinition, reference subsetting, cross subsetting,
//! feature chains, and feature values.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::kernel::KERML_FEATURE_ELEMENT_KEYWORDS;
use crate::parser::lookahead::keyword;

impl Parser<'_> {
    /// Whether a `UsageCompletion` closes the construct that starts at the `n`th token.
    ///
    /// The bare-name collision, settled by looking for the thing a usage has and an
    /// expression has not. `Usage = UsageDeclaration UsageCompletion` and
    /// `UsageCompletion` ends in a body that is `';'` or a braced one (`SysML`
    /// 8.2.2.6.2), so a usage always reaches a `;` or a `{` before the enclosing body
    /// closes. An expression reaches the enclosing `}` instead.
    ///
    /// Brackets are counted so that a `;` or `{` inside a nested construct does not
    /// answer for this one. A `{` at depth zero says "usage" unless it opens a
    /// `BodyExpression`, which `opens_body_expression` decides; that one opens a depth
    /// like any other bracket, so the `;` items inside it do not answer either.
    pub(super) fn usage_completion_follows(&self, n: usize) -> bool {
        let mut depth = 0u32;
        let mut i = n;
        while let Some(token) = self.peek_nth(i) {
            match token.kind {
                SyntaxKind::LBrace if depth == 0 && self.opens_body_expression(n, i) => {
                    depth += 1;
                }
                // A completion, so what starts at `n` is a usage and not an expression.
                SyntaxKind::Semicolon | SyntaxKind::LBrace if depth == 0 => return true,
                // The enclosing body closed and no completion was reached, so what is
                // here is the trailing expression.
                SyntaxKind::RBrace if depth == 0 => return false,
                // Guarded arms first, so these two only ever run nested.
                SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::LBrace => depth += 1,
                SyntaxKind::RParen | SyntaxKind::RBracket | SyntaxKind::RBrace => {
                    // Saturating because a stray closer is ordinary in an editor, and
                    // an underflow here would panic in debug (invariant 3).
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            i += 1;
        }
        // End of input with nothing closed. Truncated text is the editor's normal
        // state, and reading the remainder as an expression reports one error rather
        // than one per token.
        false
    }

    // production: OwnedCrossFeatureMember
    //
    // OwnedCrossFeatureMember : OwningMembership =
    //     ownedRelatedElement += OwnedCrossFeature     (SysML 8.2.2.6.2, KerML 8.2.4.3.1)
    //
    // production: OwnedCrossFeature@sysml
    //
    // OwnedCrossFeature : ReferenceUsage = BasicUsagePrefix UsageDeclaration
    //                                                                (SysML 8.2.2.6.2)
    //
    // production: OwnedCrossFeature@kerml
    //
    // OwnedCrossFeature : Feature = BasicFeaturePrefix FeatureDeclaration
    //                                                                (KerML 8.2.4.3.1)
    //
    // One shared member over each language's own feature: SysML's is a ReferenceUsage
    // declared as a usage, KerML's a Feature declared as one. The caller has already
    // found the kind keyword after it (`skip_end_usage_prefix`,
    // `skip_kerml_end_feature_prefix`), so this reads what stands before it.
    pub(super) fn owned_cross_feature_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedCrossFeatureMember);
        self.start_node(SyntaxKind::OwnedCrossFeature);
        match self.language {
            Language::SysMl => {
                self.basic_usage_prefix();
                self.usage_declaration();
            }
            Language::KerMl => {
                // The declaration is required, and `feature_declaration` reads an empty
                // FeatureSpecializationPart rather than report one, so it is asked first:
                // `end derived feature y;` is a prefix with nothing it declares.
                self.basic_feature_prefix();
                if self.at_feature_declaration() {
                    self.feature_declaration();
                } else {
                    self.error_expected("the end feature's cross feature declaration");
                }
            }
        }
        self.finish_node();
        self.finish_node();
    }

    /// A `KerML` body's feature item, if one is written here. Returns whether it was.
    ///
    /// `NamespaceMember = NonFeatureMember | NamespaceFeatureMember` (`KerML` 8.2.3.4.1),
    /// and `TypeBodyElement`'s `FeatureMember` (8.2.4.1.1, defined 8.2.4.1.6), which a
    /// `FunctionBodyPart` reads too (8.2.5.7.1) with its own `ReturnFeatureMember`. A
    /// feature is owned through one of those, so it gets its own membership node rather
    /// than the one `membership` builds. `member`, a `TypeFeatureMember`, is a type
    /// body's alone.
    pub(super) fn kerml_feature_item(&mut self, body: Body) -> bool {
        let n = usize::from(self.at_visibility());
        let type_body = matches!(body, Body::Type | Body::Function);
        if body == Body::Function && self.nth_is_keyword(n, "return") {
            self.return_feature_member();
        } else if type_body && self.nth_is_keyword(n, "member") {
            self.feature_member();
        } else if self.at_feature(n) || self.at_kerml_keyword_feature_element(n) {
            if type_body {
                self.feature_member();
            } else {
                self.namespace_feature_member();
            }
        } else {
            return false;
        }
        true
    }

    // production: PackageMember
    //
    // PackageMember : OwningMembership =
    //     MemberPrefix ( ownedRelatedElement += DefinitionElement
    //                  | ownedRelatedElement = UsageElement )
    //
    // production: DefinitionMember
    //
    // DefinitionMember : OwningMembership =
    //     MemberPrefix ownedRelatedElement += DefinitionElement      (SysML 8.2.2.6.1)
    //
    // The two differ only in PackageMember's UsageElement alternative. A package owns a
    // usage through PackageMember itself; a definition or action body owns one through
    // a usage membership instead, never through DefinitionMember — see below. One method
    // builds all of them, the node chosen by `Body::member` from the body and the element.
    //
    // `DefinitionElement` and `UsageElement` get no node of their own. They are
    // alternations over element productions, and the element that matched already
    // says which alternative was taken, so a node here would add a level carrying
    // nothing. Neither is marked for coverage: `Package` and `PartDefinition` are
    // the only two of `DefinitionElement`'s 30 alternatives implemented, and the
    // seven in SIMPLE_USAGES are what is implemented of `UsageElement`'s.
    //
    // A definition is tried before a usage. The two share every prefix keyword and
    // the keyword after them, so `at_part_definition` — which requires the `def` —
    // must decide first; the usages are what is left.
    // Feature : Feature =
    //     ( FeaturePrefix ( 'feature' | ownedRelationship += PrefixMetadataMember )
    //       FeatureDeclaration?
    //     | ( EndFeaturePrefix | BasicFeaturePrefix ) FeatureDeclaration
    //     ) ValuePart? TypeBody                                   (KerML 8.2.4.3.1)
    //
    // production: Feature@kerml
    //
    // Both alternatives are read, and the first in both its forms: a feature may be
    // introduced by a `#` prefix instead of the word, `#M f;`, and then the declaration
    // stays optional, since the `#` says a feature is here as the keyword would. Which
    // `#` is that one is settled by `feature_prefix`: the last of a run that no keyword
    // follows. Marked although FeaturePrefix is not, for the reason Succession gives.
    //
    // The two alternatives differ in exactly two things, and one method reads both:
    // whether the keyword is written, and whether the declaration is optional. It is
    // optional after the keyword and REQUIRED without one, because with no keyword the
    // declaration is the only thing that says a feature is here at all. That is why
    // `feature;` parses and `end;` does not.
    fn feature(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Feature);
        self.feature_prefix();
        if self.at_keyword("feature") {
            // The first alternative: the keyword carries it, so the declaration after
            // it is optional and `feature;` is a feature that declares nothing.
            self.bump_as(keyword("feature").unwrap_or(SyntaxKind::BasicName));
            if self.at_feature_declaration() {
                self.feature_declaration();
            }
        } else if self.at(SyntaxKind::Hash) {
            // The first alternative's other form: a PrefixMetadataMember in the
            // keyword's place, the one `feature_prefix` left.
            self.prefix_metadata_member();
            if self.at_feature_declaration() {
                self.feature_declaration();
            }
        } else {
            // The second: no keyword at all, so the DECLARATION carries it and is
            // required. That asymmetry is the whole difference between the two, and it
            // is what makes `end;` an error while `end f;` is a feature.
            self.feature_declaration();
        }
        if self.at_value_part() {
            self.value_part();
        }
        self.type_body();
        self.finish_node();
    }

    /// Whether a `Feature` starts at the `n`th meaningful token, in either form.
    ///
    /// The keyword form is a prefix and then `feature`. The keywordless form is a prefix
    /// and then a declaration (`nth_opens_feature_declaration`) -- and a keyword is not a
    /// name (`KerML` 8.2.2.6), so `package P;` and `class A;` are not mistaken for
    /// features that happen to be called `package` and `class`.
    ///
    /// That includes the declaration's two bare forms, a `FeatureSpecializationPart` or a
    /// `ConjugationPart` with no name: `:>> self : Timeslice;` (Variable Feature
    /// Examples/Enhancements/ExtendedOccurrences.kerml:6) and `portion :>> startShot {`
    /// are features. Nothing else a `KerML` member position admits opens on those
    /// tokens, so they decide as a name does: the standalone relationship declarations
    /// beside `Feature` there (`specialization`, `subtype`, `typing`, `subset`,
    /// `redefinition`, `conjugation`, `disjoining`, `featuring`, `inverting`, `KerML`
    /// 8.2.4) open on reserved words, which are no names.
    ///
    /// A `#` in the keyword's place is the first alternative too, and there the
    /// declaration is optional, so after a run of `#` anything `nth_continues_feature`
    /// admits is a feature: `#M;` declares nothing and is one.
    pub(super) fn at_feature(&self, n: usize) -> bool {
        let keywords = self.skip_feature_prefix_keywords(n);
        let after = self.skip_prefix_metadata(keywords);
        self.nth_is_keyword(after, "feature")
            || self.nth_opens_feature_declaration(after)
            || (after > keywords && self.nth_continues_feature(after))
    }

    /// Whether the `n`th token may follow the `PrefixMetadataMember` that stands in
    /// `feature`'s place: `FeatureDeclaration? ValuePart? TypeBody` (`KerML` 8.2.4.3.1),
    /// so the first tokens of any of the three.
    ///
    /// A `FeatureDeclaration` opens on `all`, a name, `<`, a `FeatureSpecializationPart` (a
    /// specialization or a multiplicity) or a `ConjugationPart`; a `ValuePart` on `=`, `:=`
    /// or `default`; a `TypeBody` on `;` or `{`. None of these is a reserved keyword that begins
    /// another element, which is what lets `feature_prefix` tell a `#` that prefixes
    /// `connector` or `feature` from the `#` that replaces `feature`.
    fn nth_continues_feature(&self, n: usize) -> bool {
        self.nth_opens_feature_declaration(n)
            || self.nth_is(n, SyntaxKind::Eq)
            || self.nth_is(n, SyntaxKind::ColonEq)
            || self.nth_is_keyword(n, "default")
            || self.nth_is(n, SyntaxKind::Semicolon)
            || self.nth_is(n, SyntaxKind::LBrace)
    }

    /// The index just past a `FeaturePrefix` written from the `n`th token.
    ///
    /// `FeaturePrefix = ( EndFeaturePrefix OwnedCrossFeatureMember? | BasicFeaturePrefix )
    /// PrefixMetadataMember*` (`KerML` 8.2.4.3.1), every part looked past.
    pub(super) fn skip_feature_prefix(&self, n: usize) -> usize {
        self.skip_prefix_metadata(self.skip_feature_prefix_keywords(n))
    }

    /// The index just past a `FeaturePrefix`'s keywords, its `EndFeaturePrefix` with any
    /// `OwnedCrossFeatureMember` or its `BasicFeaturePrefix`, before any `#`.
    fn skip_feature_prefix_keywords(&self, n: usize) -> usize {
        // EndFeaturePrefix = 'const'? 'end'. Tried first: it may open with `const`,
        // which is also BasicFeaturePrefix's last slot, and only the `end` tells them
        // apart.
        let end = n + usize::from(self.nth_is_keyword(n, "const"));
        if self.nth_is_keyword(end, "end") {
            return self.skip_kerml_end_feature_prefix(end).unwrap_or(end + 1);
        }
        self.skip_basic_feature_prefix(n)
    }

    /// The index of the element's keyword (or first `#`) after a `KerML` `end` at the
    /// `end`th token, or `None` when neither follows before the statement's `;`, brace,
    /// `=` or `:=`.
    ///
    /// `FeaturePrefix = EndFeaturePrefix OwnedCrossFeatureMember? ...` (`KerML` 8.2.4.3.1),
    /// and, as `SysML`'s end usages have it, the cross feature is everything between the
    /// `end` and the element's keyword: a `BasicFeaturePrefix FeatureDeclaration`, which
    /// writes no reserved word that begins a `FeatureElement`, and no `#`, `;`, brace or
    /// value. So a keyword found is the one the prefix stands before, and the cross
    /// feature is present when anything stands between. With none found the `end` is the
    /// keywordless `Feature`'s `EndFeaturePrefix`, whose declaration is not a cross
    /// feature: `end f : T;`.
    fn skip_kerml_end_feature_prefix(&self, end: usize) -> Option<usize> {
        let mut k = end + 1;
        loop {
            let token = self.peek_nth(k)?;
            if matches!(
                token.kind,
                SyntaxKind::Semicolon
                    | SyntaxKind::LBrace
                    | SyntaxKind::RBrace
                    | SyntaxKind::Eq
                    | SyntaxKind::ColonEq
            ) {
                return None;
            }
            if token.kind == SyntaxKind::Hash
                || self.nth_is_any_keyword(k, &KERML_FEATURE_ELEMENT_KEYWORDS)
            {
                return Some(k);
            }
            k += 1;
        }
    }

    /// The index just past a `BasicFeaturePrefix` written from the `n`th token.
    ///
    /// Every part is optional, so this returns `n` unchanged when none is written. The
    /// keywords are counted in the clause's order and each at most once, which is what
    /// makes `derived in feature f;` two errors rather than a longer prefix.
    fn skip_basic_feature_prefix(&self, n: usize) -> usize {
        let mut n = n;
        for words in [
            &["in", "out", "inout"][..],
            &["derived"],
            &["abstract"],
            &["composite", "portion"],
            &["var", "const"],
        ] {
            if self.nth_is_any_keyword(n, words) {
                n += 1;
            }
        }
        n
    }

    // FeaturePrefix : Feature =
    //     ( EndFeaturePrefix ownedRelationship += OwnedCrossFeatureMember?
    //     | BasicFeaturePrefix ) ownedRelationship += PrefixMetadataMember*
    //                                                            (KerML 8.2.4.3.1)
    //
    // production: FeaturePrefix@kerml
    //
    // Every part read. The OwnedCrossFeatureMember is present when anything stands between
    // the `end` and the element's keyword (`skip_kerml_end_feature_prefix`): `end [0..1]
    // feature cart: ShoppingCart[1];` (Association Examples/ProductSelection_N_ary.kerml:9)
    // crosses by the multiplicity alone. The node is built even when empty, as
    // MemberPrefix's is.
    //
    // The PrefixMetadataMember* is read, all but one case: Feature writes `( 'feature' |
    // PrefixMetadataMember )` after this prefix, so in `#A #B f;` the `#B` is Feature's,
    // in the keyword's place, and only `#A` is the prefix's. A `#` is left when it is the
    // last of its run and what follows it continues a feature (`nth_continues_feature`)
    // rather than being a keyword -- `feature`, `connector`, `succession`, `binding` --
    // that the prefix stands before. The grammar is unambiguous: the star cannot take a
    // `#` that the Feature alternative then lacks.
    pub(super) fn feature_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeaturePrefix);
        if self.at_end_feature_prefix() {
            let end = usize::from(self.at_keyword("const"));
            let cross = self
                .skip_kerml_end_feature_prefix(end)
                .is_some_and(|keyword| keyword > end + 1);
            self.end_feature_prefix();
            if cross {
                self.owned_cross_feature_member();
            }
        } else {
            self.basic_feature_prefix();
        }
        while let Some(end) = self.skip_one_prefix_metadata(0) {
            if !self.nth_is(end, SyntaxKind::Hash) && self.nth_continues_feature(end) {
                break;
            }
            self.prefix_metadata_member();
        }
        self.finish_node();
    }

    /// Whether an `EndFeaturePrefix` rather than a `BasicFeaturePrefix` is written here.
    fn at_end_feature_prefix(&self) -> bool {
        self.nth_is_keyword(usize::from(self.at_keyword("const")), "end")
    }

    // production: EndFeaturePrefix
    //
    // EndFeaturePrefix : Feature = ( isConstant ?= 'const' )? isEnd ?= 'end'
    //                                                            (KerML 8.2.4.3.1)
    fn end_feature_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EndFeaturePrefix);
        self.eat_optional_keyword("const");
        self.expect_keyword("end");
        self.finish_node();
    }

    // production: BasicFeaturePrefix
    //
    // BasicFeaturePrefix : Feature =
    //     ( direction = FeatureDirection )? ( isDerived ?= 'derived' )?
    //     ( isAbstract ?= 'abstract' )?
    //     ( isComposite ?= 'composite' | isPortion ?= 'portion' )?
    //     ( isVariable ?= 'var' | isConstant ?= 'const' )?        (KerML 8.2.4.3.1)
    //
    // Every part is optional, so the node may be empty — `feature f;` writes a
    // FeaturePrefix whose BasicFeaturePrefix holds nothing.
    //
    // The two alternations forecloses: taking `composite` rules out `portion`, so
    // `composite portion feature f;` leaves the second word for the caller to report.
    fn basic_feature_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BasicFeaturePrefix);
        if self.at_keyword("in") || self.at_keyword("out") || self.at_keyword("inout") {
            self.feature_direction();
        }
        self.eat_optional_keyword("derived");
        self.eat_optional_keyword("abstract");
        self.eat_one_of(&["composite", "portion"]);
        self.eat_one_of(&["var", "const"]);
        self.finish_node();
    }

    /// Whether a `FeatureDeclaration` starts here.
    ///
    /// It is optional after the `feature` keyword, and it cannot be empty: a
    /// `FeatureIdentification` needs a name, and the other two alternatives need a
    /// specialization or a conjugation. So `feature;` is a feature with no declaration,
    /// and `feature : A;` is one whose declaration is a bare specialization.
    pub(super) fn at_feature_declaration(&self) -> bool {
        self.nth_opens_feature_declaration(0)
    }

    /// Whether a `FeatureDeclaration` starts at the `n`th meaningful token (`KerML`
    /// 8.2.4.3.1): `all`, a `FeatureIdentification` (a name or `<`), a
    /// `FeatureSpecializationPart` (a specialization, or a `MultiplicityPart`: `[`,
    /// `ordered`, `nonunique`), or a `ConjugationPart` (`~`, `conjugates`).
    fn nth_opens_feature_declaration(&self, n: usize) -> bool {
        self.nth_is(n, SyntaxKind::Lt)
            || self.nth_is(n, SyntaxKind::LBracket)
            || self.nth_is(n, SyntaxKind::Tilde)
            || self.nth_is_name(n)
            || self.nth_at_feature_specialization(n)
            || self.nth_is_any_keyword(n, &["all", "ordered", "nonunique", "conjugates"])
    }

    // production: FeatureDeclaration@kerml
    //
    // FeatureDeclaration : Feature =
    //     ( isSufficient ?= 'all' )?
    //     ( FeatureIdentification ( FeatureSpecializationPart | ConjugationPart )?
    //     | FeatureSpecializationPart
    //     | ConjugationPart )
    //     FeatureRelationshipPart*                                (KerML 8.2.4.3.1)
    //
    // Whole: all three alternatives of the group, and all four of FeatureRelationshipPart
    // in `feature_relationship_parts`.
    //
    // A specialization OR a conjugation, never both: a conjugated type "may not also be
    // the specific Type in any Specialization" (KerML 8.3.3.1.2, receipt eabb0d9b), and a
    // MultiplicityPart is the FeatureSpecializationPart's, so `feature f [1] ~ g;` is
    // rejected too. The two open on disjoint tokens, so one decides.
    pub(super) fn feature_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureDeclaration);
        self.eat_optional_keyword("all");
        if self.at_name() || self.at(SyntaxKind::Lt) {
            self.feature_identification();
            if self.at_conjugation_part() {
                self.conjugation_part();
            } else {
                self.optional_feature_specialization_part();
            }
        } else if self.at_conjugation_part() {
            self.conjugation_part();
        } else {
            self.feature_specialization_part();
        }
        self.feature_relationship_parts();
        self.finish_node();
    }

    // production: FeatureIdentification
    //
    // FeatureIdentification : Feature =
    //     '<' declaredShortName = NAME '>' ( declaredName = NAME )?
    //   | declaredName = NAME                                     (KerML 8.2.4.3.1)
    //
    // NOT Identification, whose two parts are BOTH optional (SysML 8.2.2.2). A feature
    // declaration must name something, and that difference is the whole of what
    // tests/rejection/end-feature-requires-a-declaration.kerml records: the Pilot writes
    // Identification here and so accepts a declaration that names nothing.
    fn feature_identification(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureIdentification);
        if self.at(SyntaxKind::Lt) {
            self.bump();
            self.expect_name("a short name");
            self.expect(SyntaxKind::Gt, "`>`");
            if self.at_name() {
                self.bump();
            }
        } else {
            self.expect_name("a feature name");
        }
        self.finish_node();
    }

    // production: NamespaceFeatureMember
    //
    // NamespaceFeatureMember : OwningMembership =
    //     MemberPrefix ownedRelatedElement += FeatureElement      (KerML 8.2.3.4.1)
    //
    // FeatureElement's ten alternatives are Feature, Step, Expression,
    // BooleanExpression, Invariant, Connector, BindingConnector, Succession, Flow and
    // SuccessionFlow. Five are implemented, Feature, Step, Connector, BindingConnector
    // and Succession; the member itself is, which is what this marks, exactly as
    // NonFeatureMember marks its own shape rather than MemberElement's alternatives.
    fn namespace_feature_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NamespaceFeatureMember);
        self.member_prefix();
        self.feature_element();
        self.finish_node();
    }

    // production: FeatureMember@kerml
    //
    // FeatureMember : OwningMembership = TypeFeatureMember | OwnedFeatureMember
    //                                                            (KerML 8.2.4.1.6)
    //
    // production: OwnedFeatureMember@kerml
    //
    // OwnedFeatureMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += FeatureElement
    //
    // production: TypeFeatureMember@kerml
    //
    // TypeFeatureMember : OwningMembership =
    //     MemberPrefix 'member' ownedRelatedElement += FeatureElement
    //
    // A TypeBody's feature, where a namespace body's is a NamespaceFeatureMember
    // (8.2.3.4.1): the same text, a different membership. OwnedFeatureMember is a
    // FeatureMembership, making the feature an ownedFeature of the type;
    // TypeFeatureMember's `member` makes it a member only, an OwningMembership, as
    // `member feature isLicensed : Boolean [1] featured by Person_snapshots {`
    // (Variable Feature Examples/TimeVaryingCarDriver.kerml:65) writes it. The alternation
    // builds no node; the member says which. `member` is reserved (KerML 8.2.2.6), so it
    // decides after the MemberPrefix.
    fn feature_member(&mut self) {
        self.eat_trivia();
        let member = self.nth_is_keyword(usize::from(self.at_visibility()), "member");
        self.start_node(if member {
            SyntaxKind::TypeFeatureMember
        } else {
            SyntaxKind::OwnedFeatureMember
        });
        self.member_prefix();
        if member {
            self.expect_keyword("member");
        }
        self.feature_element();
        self.finish_node();
    }

    // production: FeatureElement@kerml
    //
    // FeatureElement : Feature =
    //       Feature | Step | Expression | BooleanExpression | Invariant | Connector
    //     | BindingConnector | Succession | Flow | SuccessionFlow  (KerML 8.2.3.4.3)
    //
    // The `FeatureElement` a feature member owns, all ten alternatives. Nine open on a
    // reserved word after a FeaturePrefix and are asked first; Feature, whose keyword is
    // optional, is what is left. An alternation with no node: the element says which.
    pub(super) fn feature_element(&mut self) {
        if self.at_kerml_succession(0) {
            self.kerml_succession();
        } else if self.at_kerml_binding_connector(0) {
            self.kerml_binding_connector();
        } else if self.at_kerml_connector(0) {
            self.kerml_connector();
        } else if self.at_kerml_step(0) {
            self.kerml_step();
        } else if self.at_kerml_invariant(0) {
            self.kerml_invariant();
        } else if self.at_kerml_expression(0) {
            self.kerml_expression();
        } else if self.at_kerml_boolean_expression(0) {
            self.kerml_boolean_expression();
        } else if let Some(flow) = self.at_kerml_flow(0) {
            self.kerml_flow(flow);
        } else {
            self.feature();
        }
    }

    /// Whether a `KerML` `FeatureElement` that opens on its own reserved word, after a
    /// `FeaturePrefix`, starts at the `n`th meaningful token: every implemented one but
    /// `Feature`, whose keyword is optional and which `at_feature` asks about.
    pub(super) fn at_kerml_keyword_feature_element(&self, n: usize) -> bool {
        self.at_kerml_succession(n)
            || self.at_kerml_binding_connector(n)
            || self.at_kerml_connector(n)
            || self.at_kerml_step(n)
            || self.at_kerml_invariant(n)
            || self.at_kerml_expression(n)
            || self.at_kerml_boolean_expression(n)
            || self.at_kerml_flow(n).is_some()
    }

    // production: FeatureDirection
    //
    // FeatureDirection : FeatureDirectionKind = 'in' | 'out' | 'inout'
    //                                                            (SysML 8.2.2.6.2)
    pub(super) fn feature_direction(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureDirection);
        match ["in", "out", "inout"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            None => self.error_expected("`in`, `out` or `inout`"),
        }
        self.finish_node();
    }

    /// Whether a `FeatureSpecialization` is written here.
    ///
    /// `FeatureSpecialization = Typings | Subsettings | References | Crosses
    /// | Redefinitions` (`SysML` 8.2.2.6.5). Each opens with one of the special
    /// lexical terminals of 8.2.2.1.2, which has a symbol form and a word form, and
    /// the five are told apart on that one token.
    ///
    /// The symbols are checked before the words because the lexer produces a distinct
    /// kind for each symbol, while every word arrives as a `BasicName`.
    pub(super) fn at_feature_specialization(&self) -> bool {
        self.nth_at_feature_specialization(0)
    }

    /// `at_feature_specialization`, asked of the `n`th meaningful token.
    ///
    /// Needed where the specialization is not next: a `DefaultReferenceUsage` may open
    /// on one after its prefix, and a `PayloadFeature` decides its alternative on one
    /// after a name and a multiplicity. There was a second copy of this test, for the
    /// first of those, that spelled only `SysML`'s `defined by`; one method now answers
    /// for both positions and both languages.
    pub(super) fn nth_at_feature_specialization(&self, n: usize) -> bool {
        const SYMBOLS: [SyntaxKind; 5] = [
            SyntaxKind::Colon,        // DEFINED_BY
            SyntaxKind::ColonGt,      // SUBSETS
            SyntaxKind::ColonGtGt,    // REDEFINES
            SyntaxKind::ColonColonGt, // REFERENCES
            SyntaxKind::FatArrow,     // CROSSES
        ];
        SYMBOLS.iter().any(|kind| self.nth_is(n, *kind))
            || self.nth_is_any_keyword(n, &["subsets", "redefines", "references", "crosses"])
            || (self.nth_is_keyword(
                n,
                match self.language {
                    Language::KerMl => "typed",
                    Language::SysMl => "defined",
                },
            ) && self.nth_is_keyword(n + 1, "by"))
    }

    // production: FeatureSpecialization
    //
    // FeatureSpecialization = Typings | Subsettings | References | Crosses
    //                       | Redefinitions        (SysML 8.2.2.6.5, KerML 8.2.4.3.1)
    //
    // A shared unit, the same five alternatives in both grammars, every one read here:
    // `typings` reads each language's own TypedBy. No node of its own, as
    // DefinitionElement and UsageElement have none: it is an alternation, and the
    // alternative that matched already says which was taken, so a node here would add
    // a level carrying nothing.
    pub(super) fn feature_specialization(&mut self) {
        if self.at(SyntaxKind::ColonGt) || self.at_keyword("subsets") {
            self.subsettings();
        } else if self.at(SyntaxKind::ColonGtGt) || self.at_keyword("redefines") {
            self.redefinitions();
        } else if self.at(SyntaxKind::ColonColonGt) || self.at_keyword("references") {
            self.references();
        } else if self.at(SyntaxKind::FatArrow) || self.at_keyword("crosses") {
            self.crosses();
        } else {
            self.typings();
        }
    }

    // production: Subsettings
    //
    // Subsettings : Feature = Subsets ( ',' ownedRelationship += OwnedSubsetting )*
    //                                                            (SysML 8.2.2.6.5)
    fn subsettings(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Subsettings);
        self.subsets();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_subsetting();
        }
        self.finish_node();
    }

    // production: Subsets
    //
    // Subsets : Feature = SUBSETS ownedRelationship += OwnedSubsetting
    //                                                            (SysML 8.2.2.6.5)
    //
    // SUBSETS = ':>' | 'subsets'                                 (SysML 8.2.2.1.2)
    pub(super) fn subsets(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Subsets);
        self.terminal(SyntaxKind::ColonGt, "subsets", "`:>` or `subsets`");
        self.owned_subsetting();
        self.finish_node();
    }

    // production: OwnedSubsetting@sysml
    //
    // OwnedSubsetting : Subsetting =
    //     subsettedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    //
    // production: OwnedSubsetting@kerml
    //
    // OwnedSubsetting : Subsetting = GeneralType                     (KerML 8.2.4.3.3)
    //
    // KerML's GeneralType, `[QualifiedName] | OwnedFeatureChain` (8.2.4.1.2), contributed
    // into the relationship: the text and the tree `chainable_target` reads and builds, so
    // one method carries both markers. KerML reaches it through its own Subsets.
    fn owned_subsetting(&mut self) {
        self.chainable_target(SyntaxKind::OwnedSubsetting);
    }

    // production: Redefinitions
    //
    // Redefinitions : Feature =
    //     Redefines ( ',' ownedRelationship += OwnedRedefinition )*
    //                                                            (SysML 8.2.2.6.5)
    fn redefinitions(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Redefinitions);
        self.redefines();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_redefinition();
        }
        self.finish_node();
    }

    // production: Redefines
    //
    // Redefines : Feature = REDEFINES ownedRelationship += OwnedRedefinition
    //                                                            (SysML 8.2.2.6.5)
    //
    // REDEFINES = ':>>' | 'redefines'                            (SysML 8.2.2.1.2)
    fn redefines(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Redefines);
        self.terminal(SyntaxKind::ColonGtGt, "redefines", "`:>>` or `redefines`");
        self.owned_redefinition();
        self.finish_node();
    }

    // production: OwnedRedefinition@sysml
    //
    // OwnedRedefinition : Redefinition =
    //     redefinedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    //
    // production: OwnedRedefinition@kerml
    //
    // OwnedRedefinition : Redefinition = GeneralType                     (KerML 8.2.4.3.4)
    //
    // KerML's GeneralType, `[QualifiedName] | OwnedFeatureChain` (8.2.4.1.2), contributed
    // into the relationship: the text and the tree `chainable_target` reads and builds, so
    // one method carries both markers. KerML reaches it through its own Redefines.
    pub(super) fn owned_redefinition(&mut self) {
        self.chainable_target(SyntaxKind::OwnedRedefinition);
    }

    // production: References
    //
    // References : Feature =
    //     REFERENCES ownedRelationship += OwnedReferenceSubsetting
    //                                                            (SysML 8.2.2.6.5)
    //
    // REFERENCES = '::>' | 'references'                          (SysML 8.2.2.1.2)
    //
    // One target, with no repetition — unlike Subsettings and Redefinitions beside
    // it, so a ',' after the target belongs to whatever encloses this.
    fn references(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::References);
        self.terminal(
            SyntaxKind::ColonColonGt,
            "references",
            "`::>` or `references`",
        );
        self.owned_reference_subsetting();
        self.finish_node();
    }

    // production: OwnedReferenceSubsetting@sysml
    //
    // OwnedReferenceSubsetting : ReferenceSubsetting =
    //     referencedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    //
    // The second alternative was MARKED AND ABSENT until this change: the marker claimed
    // the production and the body read only a QualifiedName, so `part p :>> a.b;` was
    // rejected while coverage counted the production as done. The corpus writes a chained
    // reference 60 times. A false `implemented` is the one kind of coverage error that
    // cannot be found by reading the report, which is why it survived.
    //
    // production: OwnedReferenceSubsetting@kerml
    //
    // OwnedReferenceSubsetting : ReferenceSubsetting = GeneralType                     (KerML 8.2.4.3.3)
    //
    // KerML's GeneralType, `[QualifiedName] | OwnedFeatureChain` (8.2.4.1.2), contributed
    // into the relationship: the text and the tree `chainable_target` reads and builds, so
    // one method carries both markers. KerML reaches it through its own References.
    pub(super) fn owned_reference_subsetting(&mut self) {
        self.chainable_target(SyntaxKind::OwnedReferenceSubsetting);
    }

    /// One of the chainable targets, under `node`: the five of `SysML` 8.2.2.6.5 below,
    /// and `KerML`'s `OwnedConjugation`, `OwnedDisjoining`, `Unioning`, `Intersecting` and
    /// `Differencing` (8.2.4.1.3 to 8.2.4.1.5), which state the same name-or-chain shape.
    ///
    /// `OwnedFeatureTyping`, `OwnedSubsetting`, `OwnedRedefinition`,
    /// `OwnedReferenceSubsetting` and `OwnedCrossSubsetting` are stated with one shape and
    /// differ only in which feature the target is assigned to:
    ///
    /// ```text
    /// <x>Feature = [QualifiedName] | <x>Feature = OwnedFeatureChain
    /// ```
    ///
    /// All five were MARKED with the chain alternative absent, so `part p :>> a.b;` and
    /// `attribute x : a.b;` were rejected while coverage counted five productions as done.
    /// One method now reads the shape they share, which is also what keeps them from
    /// drifting apart again — four were fixed together and the fifth was missed precisely
    /// because it was a separate copy of the same three lines.
    pub(super) fn chainable_target(&mut self, node: SyntaxKind) {
        self.eat_trivia();
        self.start_node(node);
        self.name_or_owned_feature_chain();
        self.finish_node();
    }

    /// `[QualifiedName] | OwnedFeatureChain`, the shape every chainable target writes,
    /// with no node of its own: a name, or a chain of two or more.
    pub(super) fn name_or_owned_feature_chain(&mut self) {
        self.eat_trivia();
        let start = self.builder.checkpoint();
        self.qualified_name();
        if self.at_feature_chain() {
            self.owned_feature_chain(start);
        }
    }

    // production: OwnedFeatureChain@sysml
    //
    // OwnedFeatureChain : Feature =
    //     ownedRelationship += OwnedFeatureChaining
    //     ( '.' ownedRelationship += OwnedFeatureChaining )+     (SysML 8.2.2.6.5)
    //
    // production: OwnedFeatureChain@kerml
    //
    // OwnedFeatureChain : Feature = FeatureChain                  (KerML 8.2.4.3.5)
    //
    // KerML states it through FeatureChain, whose body is SysML's text exactly, so this
    // method reads both units. KerML reaches it by name from OwnedFeatureInverting
    // (8.2.4.3.6), through `chainable_target`.
    //
    // production: OwnedFeatureChaining
    //
    // OwnedFeatureChaining : FeatureChaining =
    //     chainingFeature = [QualifiedName]                      (SysML 8.2.2.6.5)
    //
    // NOT the expression layer's chain. `a.b` after `:>>` is this; `a.b` in an expression
    // is a FeatureChainExpression (KerML 8.2.5.8.2). The two tokens are identical and the
    // POSITION decides, exactly as it does for `[` between a MultiplicityRange and a
    // BracketExpression — and for the same structural reason: this one is reachable only
    // from a reference, and that one only from a primary operand, and neither position
    // can be reached from the other.
    //
    // The shapes differ too, which is why one could not serve for both. This is FLAT —
    // `( '.' link )+` over one node, with every link a sibling — where the expression
    // chain FOLDS, nesting one FeatureChainExpression per link. The `+` means a chain has
    // at least two links, so a bare `a` is the QualifiedName alternative and never an
    // OwnedFeatureChain of one.
    //
    // The first link is already in the tree when the `.` is seen, so the node is opened
    // retroactively at `start`, as the postfix expressions do.
    pub(super) fn owned_feature_chain(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::OwnedFeatureChain);
        self.wrap_at(start, &[SyntaxKind::OwnedFeatureChaining]);
        while self.at_feature_chain() {
            self.bump();
            self.owned_feature_chaining();
        }
        self.finish_node();
    }

    pub(super) fn owned_feature_chaining(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedFeatureChaining);
        self.qualified_name();
        self.finish_node();
    }

    // production: Crosses
    //
    // Crosses : Feature = CROSSES ownedRelationship += OwnedCrossSubsetting
    //                                                            (SysML 8.2.2.6.5)
    //
    // CROSSES = '=>' | 'crosses'                                 (SysML 8.2.2.1.2)
    //
    // One target, as References has.
    fn crosses(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Crosses);
        self.terminal(SyntaxKind::FatArrow, "crosses", "`=>` or `crosses`");
        self.owned_cross_subsetting();
        self.finish_node();
    }

    // production: OwnedCrossSubsetting@sysml
    //
    // OwnedCrossSubsetting : CrossSubsetting =
    //     crossedFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain              (SysML 8.2.2.6.5)
    //
    // production: OwnedCrossSubsetting@kerml
    //
    // OwnedCrossSubsetting : CrossSubsetting = GeneralType                     (KerML 8.2.4.3.3)
    //
    // KerML's GeneralType, `[QualifiedName] | OwnedFeatureChain` (8.2.4.1.2), contributed
    // into the relationship: the text and the tree `chainable_target` reads and builds, so
    // one method carries both markers. KerML reaches it through its own Crosses.
    fn owned_cross_subsetting(&mut self) {
        self.chainable_target(SyntaxKind::OwnedCrossSubsetting);
    }

    /// Consume one of the special lexical terminals of `SysML` 8.2.2.1.2, in either
    /// spelling: the operator `symbol`, or `word` written out.
    pub(super) fn terminal(&mut self, symbol: SyntaxKind, word: &str, what: &str) {
        if self.at(symbol) {
            self.bump();
        } else if self.at_keyword(word) {
            self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName));
        } else {
            self.error_expected(what);
        }
    }

    // FeatureSpecializationPart : Feature =
    //     FeatureSpecialization+ MultiplicityPart? FeatureSpecialization*
    //     | MultiplicityPart FeatureSpecialization*              (KerML 8.2.4.3.1)
    //
    // FeatureSpecialization = Typings | Subsettings | References | Crosses
    //                       | Redefinitions                      (SysML 8.2.2.6.5)
    //
    /// `FeatureSpecializationPart?`, read when one is written here.
    ///
    /// `FeatureSpecializationPart = FeatureSpecialization+ MultiplicityPart?
    /// FeatureSpecialization* | MultiplicityPart FeatureSpecialization*` (`SysML` 8.2.2.6.5,
    /// alike in `KerML`), so it opens on a specialization OR a multiplicity. ONE question,
    /// so that no production asks half of it: four that wrote `OwnedReferenceSubsetting
    /// FeatureSpecializationPart?` asked only for the specialization, and `assume c1
    /// [0..*];` (examples/Simple Tests/RequirementTest.sysml:34) was rejected -- pending
    /// decision feature-specialization-multiplicity-gap.
    pub(super) fn optional_feature_specialization_part(&mut self) {
        if self.at_feature_specialization() || self.at_multiplicity_part() {
            self.feature_specialization_part();
        }
    }

    // production: FeatureSpecializationPart
    //
    // The two alternatives differ only in where the MultiplicityPart may sit: the
    // first requires at least one FeatureSpecialization before it, the second puts it
    // first, and both allow FeatureSpecializations after it. Their union is therefore
    // any number of FeatureSpecializations with AT MOST ONE MultiplicityPart anywhere
    // among them, which is what the loop below reads — and neither alternative admits
    // an empty part, which is why `usage_declaration` asks before entering.
    //
    // A second MultiplicityPart is what the one-shot flag rejects; neither
    // alternative can produce one, and
    // tests/rejection/multiplicity-part-appears-once.sysml holds that.
    //
    // The production above is the clause's, verbatim. The Pilot writes the first
    // alternative as `( -> FeatureSpecialization )+` (SysML.xtext:366); that `->` is a
    // syntactic predicate, an LL workaround for its parser generator, and it appears
    // in NEITHER specification's BNF (KerML-textual-bnf.kebnf:632,
    // SysML-textual-bnf.kebnf:440). It is not ported, and it is not quoted here as
    // though the clause contained it — deviations.json makes Tier B `never_used_for`
    // rule bodies, and a predicate copied into a citation is exactly that.
    pub(super) fn feature_specialization_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureSpecializationPart);
        let mut multiplicity_taken = false;
        loop {
            if self.at_feature_specialization() {
                self.feature_specialization();
            } else if !multiplicity_taken && self.at_multiplicity_part() {
                self.multiplicity_part();
                multiplicity_taken = true;
            } else {
                break;
            }
        }
        self.finish_node();
    }

    // production: Typings@sysml
    // production: Typings@kerml
    //
    // Typings : Feature = TypedBy ( ',' ownedRelationship += FeatureTyping )*
    //                                                            (SysML 8.2.2.6.5)
    // Typings : Feature = TypedBy ( ',' ownedRelationship += OwnedFeatureTyping )*
    //                                                            (KerML 8.2.4.3.1)
    //
    // Two units, and this reads both: each language's typing after the `,` is
    // `typing_target`'s.
    fn typings(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Typings);
        self.typed_by();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.typing_target();
        }
        self.finish_node();
    }

    /// The typing a `TypedBy` or `Typings` owns, which the two languages state
    /// differently: `SysML`'s `FeatureTyping`, `OwnedFeatureTyping | ConjugatedPortTyping`
    /// (8.2.2.6.5), and `KerML`'s `OwnedFeatureTyping` itself (8.2.4.3.1), with no
    /// alternation around it and so no `FeatureTyping` node. A .kerml `feature f : ~T;`
    /// is therefore reported: `~` opens no `GeneralType`.
    fn typing_target(&mut self) {
        match self.language {
            Language::SysMl => self.feature_typing(),
            Language::KerMl => self.owned_feature_typing(),
        }
    }

    // production: TypedBy@sysml
    // production: TypedBy@kerml
    //
    // TypedBy : Feature = DEFINED_BY ownedRelationship += FeatureTyping
    //                                                            (SysML 8.2.2.6.5)
    // TypedBy : Feature = TYPED_BY ownedRelationship += OwnedFeatureTyping
    //                                                            (KerML 8.2.4.3.1)
    //
    // DEFINED_BY = ':' | 'defined' 'by'                          (SysML 8.2.2.1.2)
    //
    // SysML reserves `defined`, not `typed`: KerML's TypedBy spells the same position
    // `':' | 'typed' 'by'`, and deviations.json records that split under
    // MetadataUsageDeclaration.
    fn typed_by(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypedBy);
        if self.at(SyntaxKind::Colon) {
            self.bump();
        } else {
            // The one place the two grammars spell this differently:
            // TYPED_BY = ':' | 'typed' 'by'    (KerML 8.2.4.3.1)
            // DEFINED_BY = ':' | 'defined' 'by' (SysML 8.2.2.1.2)
            // The `:` form is shared, which is what the corpus almost always writes.
            self.expect_keyword(match self.language {
                Language::KerMl => "typed",
                Language::SysMl => "defined",
            });
            self.expect_keyword("by");
        }
        self.typing_target();
        self.finish_node();
    }

    // production: FeatureTyping@sysml
    //
    // FeatureTyping = OwnedFeatureTyping | ConjugatedPortTyping  (SysML 8.2.2.6.5)
    //
    // Both alternatives are read, and the `~` decides between them: an OwnedFeatureTyping
    // opens on a name. SysML's alone: KerML's TypedBy owns an OwnedFeatureTyping with no
    // alternation around it (see `typing_target`).
    fn feature_typing(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureTyping);
        if self.at(SyntaxKind::Tilde) {
            self.conjugated_port_typing();
        } else {
            self.owned_feature_typing();
        }
        self.finish_node();
    }

    // production: OwnedFeatureTyping@sysml
    // production: OwnedFeatureTyping@kerml
    //
    // OwnedFeatureTyping : FeatureTyping =
    //     type = [QualifiedName] | ownedRelatedElement += OwnedFeatureChain
    //                                                            (SysML 8.2.2.6.5)
    // OwnedFeatureTyping : FeatureTyping = GeneralType           (KerML 8.2.4.3.2)
    //
    // The same text in both: KerML's GeneralType is `[QualifiedName] | OwnedFeatureChain`
    // (8.2.4.1.2), SysML's alternatives written out, so one method reads both units and
    // builds the one node, as `owned_subsetting` does for Subsetting's pair.
    //
    // OwnedFeatureChain needs two or more segments joined by '.', and a FeatureChain has
    // at least two by construction, so a bare QualifiedName is never ambiguous with one.
    //
    // This was the FIFTH production marked with the chain alternative absent, and the one
    // the earlier sweep missed: the other four are subsettings and redefinitions, and this
    // is a TYPING, so a search for reference productions did not reach it. Its own comment
    // said the alternative was not implemented and the marker claimed it anyway. Every
    // production in the grammar that admits an OwnedFeatureChain has now been checked; see
    // the [coverage-marker-audit] pending decision for the method.
    pub(super) fn owned_feature_typing(&mut self) {
        self.chainable_target(SyntaxKind::OwnedFeatureTyping);
    }

    // production: UsageCompletion
    //
    // UsageCompletion : Usage = ValuePart? UsageBody             (SysML 8.2.2.6.2)
    pub(super) fn usage_completion(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UsageCompletion);
        if self.at_value_part() {
            self.value_part();
        }
        self.usage_body();
        self.finish_node();
    }

    /// Whether a `ValuePart` is written here (`SysML` 8.2.2.6.2).
    ///
    /// `FeatureValue` opens with `'='`, `':='` or `'default'`, and nothing else a
    /// `UsageCompletion` may hold opens with any of them.
    pub(super) fn at_value_part(&self) -> bool {
        self.at(SyntaxKind::Eq) || self.at(SyntaxKind::ColonEq) || self.at_keyword("default")
    }

    // production: ValuePart
    //
    // ValuePart : Usage = ownedRelationship += FeatureValue      (SysML 8.2.2.6.2)
    pub(super) fn value_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ValuePart);
        self.feature_value();
        self.finish_node();
    }

    // production: FeatureValue
    //
    // FeatureValue : FeatureValue =
    //     ( '=' | isInitial ?= ':='
    //     | isDefault ?= 'default' ( '=' | isInitial ?= ':=' )? )
    //     ownedRelatedElement += OwnedExpression                 (SysML 8.2.2.6.2)
    //
    // The three prefixes are three flags on one relationship, not three productions:
    // `=` binds, `:=` initialises, and `default` marks either as a default. After
    // `default` the `=` is optional, so `default x` and `default = x` are the same
    // FeatureValue with isDefault set.
    fn feature_value(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureValue);
        if self.at_keyword("default") {
            self.bump_as(SyntaxKind::KwDefault);
            if self.at(SyntaxKind::Eq) || self.at(SyntaxKind::ColonEq) {
                self.bump();
            }
        } else if self.at(SyntaxKind::Eq) || self.at(SyntaxKind::ColonEq) {
            self.bump();
        } else {
            self.error_expected("`=`, `:=` or `default`");
        }
        self.owned_expression();
        self.finish_node();
    }

    // production: FeatureChainMember@sysml
    //
    // FeatureChainMember : Membership =
    //     memberElement = [QualifiedName]
    //     | ownedRelationship += OwnedFeatureChainMember             (SysML 8.2.2.17.5)
    //
    // production: OwnedFeatureChainMember@sysml
    //
    // OwnedFeatureChainMember : OwningMembership =
    //     ownedRelatedElement += OwnedFeatureChain                   (SysML 8.2.2.17.5)
    //
    // Stated at 8.2.2.17.5, Assignment Action Usages (receipt dfe847fa), and used by four
    // productions elsewhere — GuardedSuccession is the first of them this parser reads.
    // NOT KerML's FeatureChainMember, whose first alternative is a FeatureReferenceMember
    // (8.2.5.8.2); see `kerml_feature_chain_member`.
    //
    // OwnedFeatureChain's `( '.' link )+` needs at least two links (8.2.2.6.5), so one
    // name is the reference alternative and two or more the owned one. The alternative is
    // chosen by looking past the name rather than by building one and repairing it: the
    // member node differs between the two, and a checkpoint cannot un-own an element.
    //
    // The deviation register carries ONE entry for OwnedFeatureChainMember, bare-named as
    // every entry is, and it is evidenced at KerML 8.2.5.8.2 — not at this SysML clause:
    // spec_only, resolved follow_spec, because the Pilot has no such rule and the
    // specification is normative. The unit implemented here is the SysML one, verified in
    // its own right at 8.2.2.17.5 (receipt dfe847fa), and the register's reasoning covers
    // it as far as it goes: no pinned file exercises the production, which is why
    // a_guarded_successions_source_takes_either_alternative constructs the case from the
    // production rather than from the corpus.
    pub(super) fn sysml_feature_chain_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureChainMember);
        if self.at_owned_feature_chain() {
            self.start_node(SyntaxKind::OwnedFeatureChainMember);
            let start = self.builder.checkpoint();
            self.qualified_name();
            self.owned_feature_chain(start);
            self.finish_node();
        } else {
            self.qualified_name();
        }
        self.finish_node();
    }

    /// Whether the name starting here is followed by a `.` and another name, making it an
    /// `OwnedFeatureChain` rather than a bare `QualifiedName`.
    pub(super) fn at_owned_feature_chain(&self) -> bool {
        self.skip_qualified_name(0)
            .is_some_and(|after| self.nth_is(after, SyntaxKind::Dot) && self.nth_is_name(after + 1))
    }
}
