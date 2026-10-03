// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The postfix layer and primary expressions, `KerML` 8.2.5.8.2 and 8.2.5.8.3: feature
//! chains, index, collect and select, body expressions, sequences, and metadata access.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::operator::{INFIX, Spelling, UNARY_OPERATORS};
use crate::parser::{MAX_DEPTH, Parser};

/// The membership an operand of a postfix expression is owned through.
///
/// `PrimaryArgumentMember = ownedMemberParameter = PrimaryArgument`, `PrimaryArgument =
/// ownedRelationship += PrimaryArgumentValue`, `PrimaryArgumentValue = value =
/// PrimaryExpression` (`KerML` 8.2.5.8.2). Three productions and no tokens, wrapped
/// around an operand that is already in the tree.
const PRIMARY_ARGUMENT: [SyntaxKind; 3] = [
    SyntaxKind::PrimaryArgumentMember,
    SyntaxKind::PrimaryArgument,
    SyntaxKind::PrimaryArgumentValue,
];

/// The same three for a `FeatureChainExpression`, whose member has its own name.
///
/// Only the outermost differs. `NonFeatureChainPrimaryArgumentMember`'s body is
/// `PrimaryArgument` despite the name, which is what makes the chain fold left — see
/// `feature_chain_expression`.
const NON_FEATURE_CHAIN_PRIMARY_ARGUMENT: [SyntaxKind; 3] = [
    SyntaxKind::NonFeatureChainPrimaryArgumentMember,
    SyntaxKind::NonFeatureChainPrimaryArgument,
    SyntaxKind::NonFeatureChainPrimaryArgumentValue,
];

impl Parser<'_> {
    /// Whether the `{` at the `i`th token opens a `BodyExpression` inside the construct
    /// that starts at the `n`th, rather than a usage body that completes it.
    ///
    /// Two positions reach a `BodyExpression` (`KerML` 8.2.5.8.2, 8.2.5.8.3), and a usage
    /// body is at neither:
    ///
    /// - as a `BodyArgumentMember`: straight after `'->' InstantiatedTypeMember`, whose
    ///   member is read as a `QualifiedName`, so the test walks back over one to the
    ///   arrow; or straight after a select's `.?` or a collect's `.`.
    /// - where an operand is expected, as `BaseExpression`: after `=` or `:=`, an opening
    ///   `(` or `[`, a `,`, `?` or `else`, or an operator from table 6.
    ///
    /// A usage body follows what ends a declaration: a name, a literal, `]`, `)`, or a
    /// keyword such as `ordered`. Only the arrow case can put a NAME before an expression
    /// body, which is why it is asked for by name rather than by the token alone. Tokens
    /// before `n` are never looked at; they belong to whatever encloses this construct.
    ///
    /// Both languages ask it: the positions are `KerML`'s own clauses, and each language
    /// reads its own body there (`body_expression`). Inside a `KerML` function or
    /// expression body the collision it settles is `at_kerml_result_expression`'s:
    /// `x->collect { in xx; y->select { in z; z } }` has a nested body in its result
    /// expression, and that `{` completes nothing.
    pub(super) fn opens_body_expression(&self, n: usize, i: usize) -> bool {
        let Some(prev) = i.checked_sub(1).filter(|&p| p >= n) else {
            return false;
        };
        let mut k = prev;
        if self.nth_is_name(k) {
            while k >= n + 2
                && self.nth_is(k - 1, SyntaxKind::ColonColon)
                && self.nth_is_name(k - 2)
            {
                k -= 2;
            }
            return k > n && self.nth_is(k - 1, SyntaxKind::ThinArrow);
        }
        let spelled = |spelling: &Spelling| match spelling {
            Spelling::Symbol(kind) => self.nth_is(prev, *kind),
            Spelling::Word(word) => self.nth_is_keyword(prev, word),
        };
        [
            SyntaxKind::Eq,
            SyntaxKind::ColonEq,
            SyntaxKind::LParen,
            SyntaxKind::LBracket,
            SyntaxKind::Comma,
            SyntaxKind::Question,
            SyntaxKind::DotQuestion,
            SyntaxKind::Dot,
        ]
        .into_iter()
        .any(|kind| self.nth_is(prev, kind))
            || self.nth_is_keyword(prev, "else")
            || INFIX.iter().any(|op| spelled(&op.spelling))
            || UNARY_OPERATORS.iter().any(spelled)
    }

    // production: PrimaryExpression
    // production: NonFeatureChainPrimaryExpression
    //
    // PrimaryExpression = FeatureChainExpression
    //                   | NonFeatureChainPrimaryExpression       (KerML 8.2.5.8.2)
    //
    // NonFeatureChainPrimaryExpression = BracketExpression | IndexExpression
    //     | SequenceExpression | SelectExpression | CollectExpression
    //     | FunctionOperationExpression | BaseExpression         (KerML 8.2.5.8.2)
    //
    // Both alternations, every alternative, read here as the Pilot reads them
    // (KerMLExpressions.xtext:299-322): a SequenceExpression or a BaseExpression first,
    // by `non_feature_chain_primary_expression`, and then `postfix_tail`'s left fold,
    // which reads FeatureChainExpression, BracketExpression, IndexExpression,
    // SelectExpression, CollectExpression and FunctionOperationExpression, each over the
    // primary before it as its PrimaryArgument. Five of NonFeatureChainPrimaryExpression's
    // seven and PrimaryExpression's first alternative open on an operand, so no method
    // reads them before one; the fold is how the operand comes first. Each alternative
    // carries its own marker; these two claim the alternations, which is why they sit on
    // the one method that reads all of both.
    //
    // No node of its own for either, as DefinitionElement and FeatureSpecialization
    // have none: an alternation's node would add a level carrying nothing, because the
    // alternative that matched already says which was taken.
    pub(super) fn primary_expression(&mut self) {
        self.eat_trivia();
        let start = self.builder.checkpoint();
        self.non_feature_chain_primary_expression();
        self.postfix_tail(start);
    }

    // production: FeatureChainExpression
    //
    // FeatureChainExpression =
    //     ownedRelationship += NonFeatureChainPrimaryArgumentMember '.'
    //     ownedRelationship += FeatureChainMember                 (KerML 8.2.5.8.2)
    //
    // production: NonFeatureChainPrimaryArgumentMember
    //
    // NonFeatureChainPrimaryArgumentMember =
    //     ownedMemberParameter = NonFeatureChainPrimaryArgument   (KerML 8.2.5.8.2,
    //                                   by deviation NonFeatureChainPrimaryArgumentMember)
    //
    // production: PrimaryArgument
    // production: PrimaryArgumentValue
    //
    // PrimaryArgument      = ownedRelationship += PrimaryArgumentValue
    // PrimaryArgumentValue = value = PrimaryExpression            (KerML 8.2.5.8.2)
    //
    // NESTS RIGHT, so `a.b.c` is `a . (b.c)`: one FeatureChainExpression whose member is an
    // OwnedFeatureChainMember over the chain, as the Pilot's grammar reads it
    // (KerMLExpressions.xtext:299-322, the chain `( '.' FeatureChainMember )?` after the
    // base and after each other postfix operator, never repeated on its own). The BNF's
    // member body, `PrimaryArgument`, is a copy of PrimaryArgumentMember's that admits a
    // chain on the left and makes `a.b.c` ambiguous; deviation
    // NonFeatureChainPrimaryArgumentMember (follow_xtext) reads it as the
    // NonFeatureChainPrimaryArgument the clause defines beside it and nothing references.
    // PrimaryArgument and PrimaryArgumentValue are what a bracket, an index, an operation,
    // a select and a collect wrap their operand in. A chain after one of those starts a
    // new FeatureChainExpression over it: `a.b[1].c` is FCE([](FCE(a, b), 1), c).
    //
    // The postfix loop is why this is iterative rather than recursive, and a bare chain
    // of ten thousand links is one flat OwnedFeatureChain (invariant 3).
    //
    // NO EmptyResultMember, although the metaclass IS an OperatorExpression (8.3.4.8.4)
    // and every operator in the infix table owns one. The BNF writes EmptyResultMember
    // explicitly where a production has one — BinaryOperatorExpression and
    // FeatureReferenceExpression both name it — and this production does not. Adding one
    // by analogy would put an element in the tree that the grammar does not state.
    //
    // FeatureChainMember@kerml is marked at `kerml_feature_chain_member`. SysML's
    // AssignmentNodeDeclaration (8.2.2.17.5) reaches SysML's own FeatureChainMember, a
    // different unit; see `sysml_feature_chain_member`.
    // production: BracketExpression
    //
    // BracketExpression =
    //     ownedRelationship += PrimaryArgumentMember operator = '['
    //     ownedRelationship += SequenceExpressionListMember ']'   (KerML 8.2.5.8.2)
    //
    // production: PrimaryArgumentMember
    //
    // PrimaryArgumentMember = ownedMemberParameter = PrimaryArgument
    //                                                            (KerML 8.2.5.8.2)
    //
    // The quantity form: `1200 [kg]`. Postfix and left-folding like the chain, and in
    // the same loop because the Pilot puts them in the same loop
    // (KerMLExpressions.xtext:299–322) and because `a.b[kg]` and `a[1].b` both have to
    // work.
    //
    // No EmptyResultMember, for the reason FeatureChainExpression has none: the BNF
    // names one where a production has one, and 8.2.5.8.2 does not.
    //
    // deviations.json buckets this spec_only/follow_spec, whose generic rationale says
    // to expect the corpus not to exercise it. That is wrong for THIS production and the
    // record now says so: the corpus writes a bracketed quantity in hundreds of places,
    // and the Pilot implements the form — it simply does not give the rule a name,
    // inlining it as `{OperatorExpression.operand += current} operator = '['`
    // (KerMLExpressions.xtext:307). The bucket is a naming difference, not a gap.
    fn bracket_expression(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::BracketExpression);
        self.wrap_at(start, &PRIMARY_ARGUMENT);
        self.bump();
        self.sequence_expression_list_member();
        self.expect(SyntaxKind::RBracket, "`]`");
        self.finish_node();
    }

    // production: NonFeatureChainPrimaryArgument
    // production: NonFeatureChainPrimaryArgumentValue
    //
    // NonFeatureChainPrimaryArgument : Feature =
    //     ownedRelationship += NonFeatureChainPrimaryArgumentValue
    // NonFeatureChainPrimaryArgumentValue : FeatureValue =
    //     value = NonFeatureChainPrimaryExpression                (KerML 8.2.5.8.2)
    //
    // The left operand, wrapped retroactively as PRIMARY_ARGUMENT's three are for a
    // bracket. By deviation NonFeatureChainPrimaryArgumentMember (follow_xtext) the member
    // owns these two, which the BNF defines and no production references: the clause's
    // `ownedMemberParameter = PrimaryArgument` is a copy of PrimaryArgumentMember's. What
    // is at `start` is never a FeatureChainExpression, because `kerml_feature_chain_member`
    // reads every `.name` after the `.` into one member, so a bare chain cannot fold.
    //
    /// One `FeatureChainExpression`, over what is already at `start`.
    fn feature_chain_expression(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::FeatureChainExpression);
        self.wrap_at(start, &NON_FEATURE_CHAIN_PRIMARY_ARGUMENT);
        self.bump();
        self.kerml_feature_chain_member();
        self.finish_node();
    }

    // production: CollectExpression
    //
    // CollectExpression =
    //     ownedRelationship += PrimaryArgumentMember '.'
    //     ownedRelationship += BodyArgumentMember                (KerML 8.2.5.8.2)
    //
    // `x.{in xx; xx + 1}` (vendor/corpus/kerml/src/examples/Simple Tests/Expressions.kerml
    // :16). The Pilot writes the same alternative inline,
    // `{SysML::CollectExpression.operand += current} '.' operand += BodyExpression`
    // (KerMLExpressions.xtext:314-315).
    //
    // A body and nothing else, and no EmptyResultMember, as SelectExpression. It is told
    // from a FeatureChainExpression by the token after the `.`: see
    // `at_collect_expression`. A .kerml body is reported at its `{`, as a select's is.
    fn collect_expression(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::CollectExpression);
        self.wrap_at(start, &PRIMARY_ARGUMENT);
        self.bump();
        self.eat_trivia();
        if self.at(SyntaxKind::LBrace) {
            self.body_argument_member();
        } else {
            self.error_expected("a `{` body after `.`");
        }
        self.finish_node();
    }

    /// Whether a `.` here opens a `CollectExpression`.
    ///
    /// A `BodyArgumentMember` reaches `BodyExpression`, which opens on `{`
    /// (`ExpressionBody`, `KerML` 8.2.5.8.3), so a `.` with a `{` after it is a collect. A
    /// chain asks for a name after its `.` (`at_feature_chain`), and a `{` is not one, so
    /// the two never both answer. Asked in both languages, each of which reads its own
    /// `ExpressionBody` (`body_expression`).
    fn at_collect_expression(&self) -> bool {
        self.at(SyntaxKind::Dot) && self.nth_is(1, SyntaxKind::LBrace)
    }

    // production: SelectExpression
    //
    // SelectExpression =
    //     ownedRelationship += PrimaryArgumentMember '.?'
    //     ownedRelationship += BodyArgumentMember                (KerML 8.2.5.8.2)
    //
    // `subcomponents.totalMass.?{in p:>ISQ::mass; p >= minMass}`
    // (training/29. Expressions/MassRollup2.sysml:18). The Pilot writes the same
    // alternative inline, `{SysML::SelectExpression.operand += current} '.?' operand +=
    // BodyExpression` (KerMLExpressions.xtext:316-317).
    //
    // A body and nothing else: no ArgumentList, no function reference. No
    // EmptyResultMember, as the clause names none. `.?` lexes as a single DotQuestion, so
    // neither the feature chain nor anything else that reads a `.` can take it.
    //
    // The body is each language's own ExpressionBody: see `body_expression`.
    fn select_expression(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::SelectExpression);
        self.wrap_at(start, &PRIMARY_ARGUMENT);
        self.bump();
        self.eat_trivia();
        if self.at(SyntaxKind::LBrace) {
            self.body_argument_member();
        } else {
            self.error_expected("a `{` body after `.?`");
        }
        self.finish_node();
    }

    // production: IndexExpression
    //
    // IndexExpression =
    //     ownedRelationship += PrimaryArgumentMember '#'
    //     '(' ownedRelationship += SequenceExpressionListMember ')'
    //                                                            (KerML 8.2.5.8.2)
    //
    // `power#(i)` (training/33. Analysis/Analysis Case Definition Example.sysml:61). The
    // parentheses are this production's own, not a SequenceExpression's (8.2.5.8.2's
    // SequenceExpression writes its own pair). The index is a SequenceExpressionListMember,
    // as a bracket's is, so `m#(1, 2)` and `m#(1,)` read as SequenceExpressionList reads
    // them. The Pilot agrees: its `'#' '(' operand += SequenceExpression ')'`
    // (KerMLExpressions.xtext:305) names a rule that is the bare list with no parentheses
    // (:389-395), the clause's SequenceExpressionList under another name.
    //
    // No EmptyResultMember, for the reason BracketExpression has none: the BNF names one
    // where a production has one, and this production does not.
    //
    // deviations.json buckets this spec_only/follow_spec. As for BracketExpression, the
    // bucket is a naming difference and not a gap: the Pilot implements the form inline
    // under PrimaryExpression, and the corpus writes it 41 times.
    fn index_expression(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::IndexExpression);
        self.wrap_at(start, &PRIMARY_ARGUMENT);
        self.bump();
        self.expect(SyntaxKind::LParen, "`(`");
        self.sequence_expression_list_member();
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish_node();
    }

    /// Whether a `#` here opens an `IndexExpression`.
    ///
    /// `#` also opens `SysML`'s `PrefixMetadataMember` (8.2.2.27) and `KerML`'s
    /// `PrefixMetadataFeature` (8.2.5.12). Both reach an `OwnedFeatureTyping` after it,
    /// which is a `QualifiedName` or an `OwnedFeatureChain` (`KerML` through
    /// `GeneralType`), and a chain too opens on a `QualifiedName`. So a name follows that
    /// `#` either way. An index writes `(`, which no name opens on, so the `(` alone
    /// decides, and a `#M` after an expression is left for whatever encloses it to report.
    fn at_index_expression(&self) -> bool {
        self.at(SyntaxKind::Hash) && self.nth_is(1, SyntaxKind::LParen)
    }

    // production: FunctionOperationExpression
    //
    // FunctionOperationExpression : InvocationExpression =
    //     ownedRelationship += PrimaryArgumentMember '->'
    //     ownedRelationship += InstantiatedTypeMember
    //     ( ownedRelationship += BodyArgumentMember
    //     | ownedRelationship += FunctionReferenceArgumentMember
    //     | ArgumentList )
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.2)
    //
    // The member after the arrow is InstantiatedTypeMember. The clause prints
    // InvocationTypeMember, which no clause defines; deviations.json records
    // InvocationTypeMember-misnomer on OMG issue KERML11-83, where the technical editor
    // says InstantiatedTypeMember was meant, and the Pilot uses it in this slot
    // (KerMLExpressions.xtext:309), read by `instantiated_type_member`.
    //
    // The three argument forms open on three different tokens, so one token decides:
    // an ArgumentList on `(`, a BodyArgumentMember on `{` (ExpressionBody, 8.2.5.8.3),
    // and a FunctionReferenceArgumentMember on the NAME or `$` that opens its
    // QualifiedName (ReferenceTyping, 8.2.5.8.1). None is optional, so anything else is
    // reported, and the EmptyResultMember is still built so the node is whole.
    //
    // The body is each language's own ExpressionBody: see `body_expression`.
    fn function_operation_expression(&mut self, start: rowan::Checkpoint) {
        self.start_node_at(start, SyntaxKind::FunctionOperationExpression);
        self.wrap_at(start, &PRIMARY_ARGUMENT);
        self.bump();
        self.instantiated_type_member();
        if self.at(SyntaxKind::LParen) {
            self.argument_list();
        } else if self.at(SyntaxKind::LBrace) {
            self.body_argument_member();
        } else if self.at_feature_reference() {
            self.function_reference_argument_member();
        } else {
            self.error_expected("`(`, `{` or a function name after `->` and the function");
        }
        self.empty_result_member();
        self.finish_node();
    }

    // production: BodyArgumentMember
    //
    // BodyArgumentMember : ParameterMembership =
    //     ownedMemberParameter = BodyArgument                    (KerML 8.2.5.8.2)
    //
    // production: BodyArgument
    //
    // BodyArgument : Feature = ownedRelationship += BodyArgumentValue
    //                                                            (KerML 8.2.5.8.2)
    //
    // production: BodyArgumentValue
    //
    // BodyArgumentValue : FeatureValue = value = BodyExpression  (KerML 8.2.5.8.2)
    fn body_argument_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BodyArgumentMember);
        self.start_node(SyntaxKind::BodyArgument);
        self.start_node(SyntaxKind::BodyArgumentValue);
        self.body_expression();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    // production: FunctionReferenceArgumentMember
    //
    // FunctionReferenceArgumentMember : ParameterMembership =
    //     ownedMemberParameter = FunctionReferenceArgument       (KerML 8.2.5.8.2)
    //
    // production: FunctionReferenceArgument
    //
    // FunctionReferenceArgument : Feature =
    //     ownedRelationship += FunctionReferenceArgumentValue    (KerML 8.2.5.8.2)
    //
    // production: FunctionReferenceArgumentValue
    //
    // FunctionReferenceArgumentValue : FeatureValue =
    //     value = FunctionReferenceExpression                    (KerML 8.2.5.8.2)
    //
    // production: FunctionReferenceExpression
    //
    // FunctionReferenceExpression : FeatureReferenceExpression =
    //     ownedRelationship += FunctionReferenceMember           (KerML 8.2.5.8.2)
    //
    // production: FunctionReferenceMember
    //
    // FunctionReferenceMember : FeatureMembership =
    //     ownedMemberFeature = FunctionReference                 (KerML 8.2.5.8.2)
    //
    // production: FunctionReference
    //
    // FunctionReference : Expression = ownedRelationship += ReferenceTyping
    //                                                            (KerML 8.2.5.8.2)
    //
    // `x->reduce '+'`: the function is named by typing an Expression, and a
    // FeatureReferenceExpression refers to it. Unlike FeatureReferenceExpression in
    // 8.2.5.8.3, this one's production names NO EmptyResultMember, so none is built.
    fn function_reference_argument_member(&mut self) {
        const NESTED: [SyntaxKind; 7] = [
            SyntaxKind::FunctionReferenceArgumentMember,
            SyntaxKind::FunctionReferenceArgument,
            SyntaxKind::FunctionReferenceArgumentValue,
            SyntaxKind::FunctionReferenceExpression,
            SyntaxKind::FunctionReferenceMember,
            SyntaxKind::FunctionReference,
            SyntaxKind::ReferenceTyping,
        ];
        self.eat_trivia();
        for kind in NESTED {
            self.start_node(kind);
        }
        self.qualified_name();
        for _ in NESTED {
            self.finish_node();
        }
    }

    // production: BodyExpression
    //
    // BodyExpression : FeatureReferenceExpression =
    //     ownedRelationship += ExpressionBodyMember              (KerML 8.2.5.8.3)
    //
    // production: ExpressionBodyMember
    //
    // ExpressionBodyMember : FeatureMembership =
    //     ownedMemberFeature = ExpressionBody                    (KerML 8.2.5.8.3)
    //
    // production: ExpressionBody@sysml
    //
    // SysML states no ExpressionBody. Deviation ExpressionBody (follow_xtext, adjudicated
    // 2026-09-17 on the corpus's `in ref w` bodies) reads it as SysML's CalculationBody,
    // `';' | '{' CalculationBodyPart '}'` (8.2.2.19), NARROWED on 2026-09-23 (decision
    // expression-body-semicolon) to the braced alternative alone: taken literally the `;`
    // form makes `attribute x = ;;` an attribute valued by an empty body, and every
    // expression body in the corpus is braced. `braced_calculation_body` offers only the
    // braced alternative, so the narrowing is held here and not by the callers' dispatch
    // on `{`; tests/rejection/expression-body-is-braced.sysml holds it from the text.
    //
    // production: ExpressionBody@kerml
    //
    // ExpressionBody : Expression = '{' FunctionBodyPart '}'     (KerML 8.2.5.8.3)
    //
    // KerML's own, read in a .kerml file with no deviation: the FunctionBodyPart a
    // function's body has (8.2.5.7.1), so `x->collect {in xx; xx + 1}` owns a parameter
    // and a result expression (Simple Tests/Expressions.kerml:15). Braced only, as
    // SysML's is by the narrowing above, but here because the clause writes no `;`
    // alternative. Reading SysML's CalculationBody in a .kerml file would give it SysML's
    // elements, which is why the language decides.
    //
    // The CalculationBody node is inside an ExpressionBody node because the Pilot's
    // ExpressionBody is an Expression whose content IS a CalculationBody fragment
    // (SysML.xtext:2437), as a CalculationDefinition's is.
    pub(super) fn body_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BodyExpression);
        self.start_node(SyntaxKind::ExpressionBodyMember);
        self.start_node(SyntaxKind::ExpressionBody);
        match self.language {
            Language::SysMl => {
                // Every SysML expression body is read by the deviation: under the
                // printed grammar SysML has no ExpressionBody of its own and would reach
                // KerML's.
                // deviation: ExpressionBody
                self.note_deviation(
                    "ExpressionBody",
                    "an expression body read as a calculation body",
                );
                self.braced_calculation_body();
            }
            Language::KerMl => {
                self.expect(SyntaxKind::LBrace, "`{`");
                self.depth += 1;
                self.function_body_part();
                self.depth -= 1;
                self.expect(SyntaxKind::RBrace, "`}`");
            }
        }
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    /// The postfix layer of `PrimaryExpression`, folded left over `start`.
    ///
    /// `FeatureChainExpression` and `BracketExpression` are both written after their
    /// operand and both take that operand as a `PrimaryArgument`, so they nest in
    /// whatever order they are written: `a.b[kg]` is a bracket over a chain and `a[1].b`
    /// is a chain over a bracket. One loop is what makes that fall out rather than being
    /// arranged.
    ///
    /// `FunctionOperationExpression` (`->`) takes its operand the same way and is in the
    /// same loop, as the Pilot has it (KerMLExpressions.xtext:308).
    ///
    /// `IndexExpression` (`#(`) too, and it is the one form whose opening token is shared:
    /// see `at_index_expression`.
    ///
    /// `SelectExpression` (`.?{`) is the fifth and `CollectExpression` (`.{`) the sixth,
    /// which completes the postfix forms of 8.2.5.8.2. A collect shares the chain's `.`,
    /// and `at_collect_expression` and `at_feature_chain` split it by the token after.
    fn postfix_tail(&mut self, start: rowan::Checkpoint) {
        let mut levels: u32 = 0;
        while self.at_feature_chain()
            || self.at(SyntaxKind::LBracket)
            || self.at(SyntaxKind::ThinArrow)
            || self.at_index_expression()
            || self.at(SyntaxKind::DotQuestion)
            || self.at_collect_expression()
        {
            // BOUNDED, although this loop uses no stack of its own. Every level wraps
            // what is already there, so the TREE is as deep as the expression is long
            // even when the parser's own recursion is flat — and a consumer walking a
            // 50000-level tree overflows on a thread with a 2 MiB stack, which aborts
            // rather than panics (invariant 3). Folding also stops being linear at that
            // size. The depth counter is the mechanism the rest of the parser already
            // uses, so a chain is counted against the same budget as a nested body.
            if self.depth >= MAX_DEPTH {
                self.report_too_deep();
                break;
            }
            self.depth += 1;
            levels += 1;
            if self.at(SyntaxKind::LBracket) {
                self.bracket_expression(start);
            } else if self.at(SyntaxKind::ThinArrow) {
                self.function_operation_expression(start);
            } else if self.at(SyntaxKind::Hash) {
                self.index_expression(start);
            } else if self.at(SyntaxKind::DotQuestion) {
                self.select_expression(start);
            } else if self.at_collect_expression() {
                self.collect_expression(start);
            } else {
                self.feature_chain_expression(start);
            }
        }
        // The levels belong to this expression, not to anything enclosing it, so the
        // budget is returned when the run ends. A `.` or `[` past the limit is left
        // where it stands and reaches the tree through the enclosing body's recovery,
        // which is what keeps the text lossless.
        self.depth -= levels;
    }

    /// Whether a `'.'` here opens a `FeatureChainExpression` rather than something else.
    ///
    /// Three other productions put a `.` after a primary expression, and none of them is
    /// a chain:
    ///
    /// ```text
    /// SelectExpression          = PrimaryArgumentMember '.?' BodyArgumentMember
    /// CollectExpression         = PrimaryArgumentMember '.'  BodyArgumentMember
    /// MetadataAccessExpression  = ElementReferenceMember '.' 'metadata'
    /// ```
    ///
    /// `.?` lexes as one token, so a select is already not a `Dot`. The other two are
    /// separated by what FOLLOWS the dot: a collect takes a `BodyExpression`, which opens
    /// on `'{'`, and a metadata access takes the keyword `metadata`. A
    /// `FeatureChainMember` reaches a `QualifiedName`, which opens on a NAME — and a
    /// keyword is not a name (`KerML` 8.2.2.6), so asking for a name excludes both.
    ///
    /// Select and collect are read, by `select_expression` and `collect_expression`, and
    /// the metadata access by `metadata_access_expression`, as a base expression before
    /// any postfix. This check is what keeps a chain from quietly accepting text it is
    /// not: `a.b.metadata` is reported, never read as a feature called `metadata`.
    pub(super) fn at_feature_chain(&self) -> bool {
        self.at(SyntaxKind::Dot) && self.nth_is_name(1)
    }

    // production: FeatureChainMember@kerml
    //
    // FeatureChainMember : Membership =
    //     FeatureReferenceMember | OwnedFeatureChainMember         (KerML 8.2.5.8.2)
    //
    // `KerML`'s `FeatureChainMember` after the dot. SCOPED IN THE NAME because `SysML`
    // states a production of the same name with a different body — `memberElement =
    // [QualifiedName] | OwnedFeatureChainMember` (8.2.2.17.5), read by
    // `sysml_feature_chain_member` — and one Rust name for both would hide that.
    //
    // Both alternatives. A name with `.name` after it is an OwnedFeatureChainMember over
    // the whole chain, so `a.b.c` is `a . (b.c)`, one FeatureChainExpression whose target
    // is the chain, as the Pilot reads it (deviation NonFeatureChainPrimaryArgumentMember,
    // follow_xtext). A lone name is a FeatureReferenceMember: its nodes are
    // `FeatureReferenceMember` and `FeatureReference`, built without the
    // `FeatureReferenceExpression` around them and the `EmptyResultMember` after them,
    // because neither is in this production. A `.` before `{`, `?` or `metadata` is not
    // a link: `at_feature_chain` asks for a name after it.
    fn kerml_feature_chain_member(&mut self) {
        self.eat_trivia();
        if self.at_owned_feature_chain() {
            self.start_node(SyntaxKind::OwnedFeatureChainMember);
            let start = self.builder.checkpoint();
            self.qualified_name();
            self.owned_feature_chain(start);
            self.finish_node();
        } else {
            self.start_node(SyntaxKind::FeatureReferenceMember);
            self.start_node(SyntaxKind::FeatureReference);
            self.qualified_name();
            self.finish_node();
            self.finish_node();
        }
    }

    /// `PrimaryExpression`'s alternatives other than `FeatureChainExpression`.
    pub(super) fn non_feature_chain_primary_expression(&mut self) {
        // NullExpression's `'(' ')'` is decided before SequenceExpression, which would
        // otherwise read the '(' and find no expression.
        if self.at(SyntaxKind::LParen) && !self.at_empty_parentheses() {
            self.sequence_expression();
        } else {
            self.base_expression();
        }
    }

    // production: BaseExpression
    //
    // BaseExpression = NullExpression | LiteralExpression
    //     | FeatureReferenceExpression | MetadataAccessExpression
    //     | InvocationExpression | ConstructorExpression
    //     | BodyExpression                                       (KerML 8.2.5.8.3)
    //
    // All seven, in both languages: the BodyExpression is each language's own (see
    // `body_expression`). The Pilot adds `'(' SequenceExpression ')'` here
    // (KerMLExpressions.xtext:356); the clause puts SequenceExpression in
    // NonFeatureChainPrimaryExpression instead, the same text, and it is read there.
    // An alternation with no node: the expression says which.
    fn base_expression(&mut self) {
        if self.at_keyword("null") || self.at_empty_parentheses() {
            self.null_expression();
        } else if self.at_literal_expression() {
            self.literal_expression();
        } else if self.at_metadata_access_expression() {
            // BEFORE the invocation and the feature reference, which open on the same
            // QualifiedName; the `.` and the reserved `metadata` after it decide.
            self.metadata_access_expression();
        } else if self.at_keyword("new") {
            // In the keyword table, so never a name here and nothing else can open on it.
            // See `constructor_expression` for why that holds in KerML only by the table.
            self.constructor_expression();
        } else if self.at_invocation_expression() {
            // BEFORE the feature reference, and the two are told apart by ONE token:
            // both open on a QualifiedName and only an invocation has a `(` after it.
            self.invocation_expression();
        } else if self.at_feature_reference() {
            self.feature_reference_expression();
        } else if self.at(SyntaxKind::LBrace) {
            // BaseExpression's BodyExpression alternative (8.2.5.8.3), each language's
            // own body: see `body_expression`.
            self.body_expression();
        } else {
            self.error_expected("an expression");
        }
    }

    /// Whether a `MetadataAccessExpression` starts here: a `QualifiedName`, then `'.'`
    /// and the reserved `metadata` (`KerML` 8.2.5.8.3).
    fn at_metadata_access_expression(&self) -> bool {
        self.skip_qualified_name(0).is_some_and(|n| {
            self.nth_is(n, SyntaxKind::Dot) && self.nth_is_keyword(n + 1, "metadata")
        })
    }

    // production: MetadataAccessExpression
    //
    // MetadataAccessExpression =
    //     ownedRelationship += ElementReferenceMember '.' 'metadata'
    //                                                            (KerML 8.2.5.8.3)
    //
    // A BaseExpression, shared by both languages: "suffixing the qualified name of any
    // kind of element with the notation .metadata" (7.4.9.4, receipt f77ceb64), `feature
    // sysMetadata = SecureSystem.metadata;`. The metaclass is MetadataAccessExpression
    // (8.3.4.8.15, receipt 284d137d), whose referencedElement is the ElementReferenceMember's
    // memberElement. The reference is a QualifiedName alone, and `.metadata` is no
    // postfix, so it follows no chain and no other primary: `a.b.metadata` and
    // `(E).metadata` are reported. A postfix may follow the access, as it may any
    // primary: `E.metadata.x`. No EmptyResultMember: the BNF writes none here.
    //
    // constraint: MetadataAccessExpression::validateMetadataAccessExpressionReferencedElement
    //     and deriveMetadataAccessExpressionReferencdElement (8.3.4.8.15), sv2-resolve's;
    //     the ElementReferenceMember is the Membership both read.
    // implied specialization: Performances::metadataAccessEvaluations
    //     (checkMetadataAccessExpressionSpecialization, 8.3.4.8.15), sv2-hir's.
    fn metadata_access_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataAccessExpression);
        self.start_node(SyntaxKind::ElementReferenceMember);
        self.qualified_name();
        self.finish_node();
        self.expect(SyntaxKind::Dot, "`.`");
        self.expect_keyword("metadata");
        self.finish_node();
    }

    // production: SequenceExpression
    //
    // SequenceExpression = '(' SequenceExpressionList ')'        (KerML 8.2.5.8.2)
    fn sequence_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SequenceExpression);
        self.expect(SyntaxKind::LParen, "`(`");
        self.sequence_expression_list();
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish_node();
    }

    // production: SequenceExpressionList
    //
    // SequenceExpressionList = OwnedExpression ','?
    //                        | SequenceOperatorExpression        (KerML 8.2.5.8.2)
    //
    // production: SequenceOperatorExpression
    //
    // SequenceOperatorExpression : OperatorExpression =
    //     ownedRelationship += OwnedExpressionMember
    //     operator = ','
    //     ownedRelationship += SequenceExpressionListMember      (KerML 8.2.5.8.2)
    //
    // Both alternatives are an expression followed by a `','`, so which one it is
    // depends on what comes after the comma: another expression makes it a
    // SequenceOperatorExpression, and a `')'` makes it the first alternative's
    // trailing comma. The node is opened retroactively once that is known.
    fn sequence_expression_list(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SequenceExpressionList);
        let start = self.builder.checkpoint();
        self.owned_expression();
        if self.at(SyntaxKind::Comma) {
            if self.nth_is(1, SyntaxKind::RParen) {
                // `( a , )` — the trailing `','?` of the first alternative.
                self.bump();
            } else {
                self.start_node_at(start, SyntaxKind::SequenceOperatorExpression);
                self.wrap_at(start, &[SyntaxKind::OwnedExpressionMember]);
                self.bump();
                self.sequence_expression_list_member();
                self.finish_node();
            }
        }
        self.finish_node();
    }

    // production: SequenceExpressionListMember
    //
    // SequenceExpressionListMember : OwningMembership =
    //     ownedRelatedElement += SequenceExpressionList          (KerML 8.2.5.8.2)
    fn sequence_expression_list_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SequenceExpressionListMember);
        self.sequence_expression_list();
        self.finish_node();
    }

    /// Whether a `FeatureReferenceExpression` starts here.
    ///
    /// `FeatureReference = [QualifiedName]`, and a `QualifiedName` opens with a NAME
    /// or with the `'$'` of its global-scope prefix (`KerML` 8.2.3.4.1).
    pub(super) fn at_feature_reference(&self) -> bool {
        self.at_name() || self.at(SyntaxKind::Dollar)
    }

    // production: FeatureReferenceExpression
    //
    // FeatureReferenceExpression : FeatureReferenceExpression =
    //     FeatureReferenceMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.3)
    //
    // production: FeatureReferenceMember
    //
    // FeatureReferenceMember : Membership =
    //     memberElement = FeatureReference                       (KerML 8.2.5.8.3)
    //
    // production: FeatureReference
    //
    // FeatureReference : Feature = [QualifiedName]               (KerML 8.2.5.8.3)
    pub(super) fn feature_reference_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureReferenceExpression);
        self.start_node(SyntaxKind::FeatureReferenceMember);
        self.start_node(SyntaxKind::FeatureReference);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
        self.empty_result_member();
        self.finish_node();
    }

    // production: TypeReference
    //
    // TypeReference : Feature =
    //     ownedRelationship += ReferenceTyping                   (KerML 8.2.5.8.1)
    //
    // production: ReferenceTyping
    //
    // ReferenceTyping : FeatureTyping = type = [QualifiedName]   (KerML 8.2.5.8.1)
    //
    // production: TypeReferenceMember
    //
    // TypeReferenceMember : FeatureMembership =
    //     ownedMemberFeature = TypeReference                     (KerML 8.2.5.8.1)
    //
    // production: TypeResultMember
    //
    // TypeResultMember : ReturnParameterMembership =
    //     ownedMemberFeature = TypeReference                     (KerML 8.2.5.8.1)
    //
    // The two memberships differ in the abstract syntax and not in the text: a test
    // names its type through a FeatureMembership and a cast through a
    // ReturnParameterMembership, because a cast's type IS its result. `member` is
    // which one the operator named.
    pub(super) fn type_reference_member(&mut self, member: SyntaxKind) {
        self.eat_trivia();
        self.start_node(member);
        self.start_node(SyntaxKind::TypeReference);
        self.start_node(SyntaxKind::ReferenceTyping);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }
}
