// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The operator core of the expression layer, `KerML` 8.2.5.8.1: `OwnedExpression`, the
//! precedence climb over the table in `operator`, and the conditional, classification and
//! extent expressions.

use crate::generated::kinds::SyntaxKind;
use crate::parser::operator::{
    Assoc, InfixOperator, LEFT_ARGUMENT, Operand, TIER_CONDITIONAL, TIER_LOOSEST, TIER_UNARY,
};
use crate::parser::{MAX_DEPTH, Parser};

impl Parser<'_> {
    // -- the expression layer, KerML 8.2.5.8 -------------------------------------

    // production: OwnedExpression
    //
    // OwnedExpression : Expression =
    //       ConditionalExpression
    //     | ConditionalBinaryOperatorExpression
    //     | BinaryOperatorExpression
    //     | UnaryOperatorExpression
    //     | ClassificationExpression
    //     | MetaclassificationExpression
    //     | ExtentExpression
    //     | PrimaryExpression                                    (KerML 8.2.5.8.1)
    //
    // THE ALTERNATION CARRIES NO PRECEDENCE, AND CANNOT. KerML 8.2.5.8.1 note 2
    // states that the grouping of nested OperatorExpressions is not expressed in the
    // productions above it, and is instead determined by the precedence of the
    // operators as given in that clause's table 6. So this production is ambiguous
    // on its own, by the specification's own construction, and any recursive-descent
    // parser has to carry the table separately.
    //
    // deviations.json records the decision under BinaryOperatorExpression
    // (spec_only/follow_spec): implement the specification's one-production shape and
    // supply precedence from the table, rather than porting the Pilot's stratified
    // cascade. The cascade is docs/DERIVATION.md's named example of an Xtext LL
    // workaround; porting it would report coverage against a dozen productions the
    // language does not have, and would move every precedence question from the
    // table, where the specification answers it, to rule ordering.
    //
    // The table is data at docs/operator-precedence.toml, cited to the clause.
    // `INFIX` and `UNARY_OPERATORS` below are that file's tiers; the test
    // `the_infix_table_is_the_recorded_precedence_table` reads the file and holds the
    // two against each other, so an edit to either alone fails.
    pub(super) fn owned_expression(&mut self) {
        self.expression(TIER_LOOSEST);
    }

    /// An `OwnedExpression` whose top-level operator binds no looser than `tier`.
    ///
    /// This is the precedence climb. `tier` is the loosest tier this call may
    /// consume: a caller that has just taken a tier-5 operator asks for tier 4 on
    /// the right, and the `+` in `a + b + c` is therefore left for the caller's own
    /// loop rather than nested, which is what makes the tier group to the left.
    pub(super) fn expression(&mut self, tier: u8) {
        self.eat_trivia();
        if self.depth >= MAX_DEPTH {
            self.report_too_deep();
            return;
        }
        self.depth += 1;
        self.expression_inner(tier);
        self.depth -= 1;
    }

    /// [`Parser::expression`] proper, entered one nesting level deeper.
    fn expression_inner(&mut self, tier: u8) {
        // ConditionalExpression is tier 15, the loosest, and is written prefix, so
        // it is decided before anything else rather than found by the climb.
        if tier >= TIER_CONDITIONAL && self.at_keyword("if") {
            self.conditional_expression();
            return;
        }
        let start = self.builder.checkpoint();
        self.prefix_expression();
        self.infix_tail(start, tier);
    }

    /// The `OwnedExpression` alternatives written with their operator first, and the
    /// `PrimaryExpression` left when none of them is.
    fn prefix_expression(&mut self) {
        // ExtentExpression = 'all' TypeReferenceMember — tier 1, tighter than every
        // binary operator.
        if self.at_keyword("all") {
            self.extent_expression();
            return;
        }
        // MetaclassificationExpression's left operand is a MetadataArgumentMember,
        // which reaches a QualifiedName and not an OwnedExpression, so it cannot be
        // wrapped retroactively around an already-parsed expression the way the other
        // tier-8 operators are. It is decided here instead, by looking past the name.
        if self.at_metaclassification() {
            self.metaclassification_expression();
            return;
        }
        // ClassificationExpression's ArgumentMember is optional, so a classification
        // operator may open one with nothing to its left. `filter @Safety;` is the
        // corpus idiom for exactly that.
        if self.at_leading_classification() {
            self.classification_expression(None);
            return;
        }
        // UnaryOperatorExpression = UnaryOperator ArgumentMember EmptyResultMember —
        // tier 2, which binds TIGHTER than exponentiation at tier 3, so `-2 ** 2` is
        // `(-2) ** 2`. That is the opposite of the C and Python convention and is
        // what table 6 states.
        if self.at_unary_operator() {
            self.unary_operator_expression();
            return;
        }
        self.primary_expression();
    }

    // production: BinaryOperatorExpression
    //
    // BinaryOperatorExpression : OperatorExpression =
    //     ownedRelationship += ArgumentMember
    //     operator = BinaryOperator
    //     ownedRelationship += ArgumentMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // production: ConditionalBinaryOperatorExpression
    //
    // ConditionalBinaryOperatorExpression : OperatorExpression =
    //     ownedRelationship += ArgumentMember
    //     operator = ConditionalBinaryOperator
    //     ownedRelationship += ArgumentExpressionMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // Both are built here rather than in a method each, because they differ only in
    // the membership of their right operand and in which operators name them — and
    // both of those are columns of the table. ClassificationExpression is reached
    // from here too, but has its own method because its left operand is optional.
    //
    // production: BinaryOperator
    // production: ConditionalBinaryOperator
    //
    // BinaryOperator = '|' | '&' | 'xor' | '..' | '==' | '!=' | '===' | '!=='
    //     | '<' | '>' | '<=' | '>=' | '+' | '-' | '*' | '/' | '%' | '^' | '**'
    // ConditionalBinaryOperator = '??' | 'or' | 'and' | 'implies'
    //                                                            (KerML 8.2.5.8.1)
    //
    // Value productions, the `operator` of the two expressions above: every spelling of
    // each is a row of INFIX, bumped here as the operator of the node its row names.
    // They build no node, as BooleanValue builds none.
    //
    /// Consume infix operators at `max_tier` or tighter, folding the expression that
    /// starts at `start` into each one's left operand.
    ///
    /// The left operand is already in the tree when the operator is read, so the
    /// operator's node and the memberships around its left operand are both opened
    /// retroactively at `start` — the same technique `import_declaration` uses for
    /// two alternatives that share a prefix.
    fn infix_tail(&mut self, start: rowan::Checkpoint, max_tier: u8) {
        while let Some(op) = self.infix_operator_here() {
            if op.tier > max_tier {
                break;
            }
            self.start_node_at(start, op.node);
            self.wrap_at(start, op.left);
            self.bump_spelling(op.spelling);
            self.right_operand(op);
            // Every OperatorExpression the table reaches owns a result parameter,
            // written nowhere in the text. ExtentExpression is the one that does not,
            // and it is prefix, so it is not in the table.
            self.empty_result_member();
            self.finish_node();
        }
    }

    /// The right operand of an infix operator, in the membership the clause names.
    ///
    /// The tier asked for is what makes the group: strictly tighter for a
    /// left-associative operator, so a repeat is left to the caller's loop, and the
    /// same tier for a right-associative one, so a repeat nests here.
    fn right_operand(&mut self, op: &InfixOperator) {
        let tier = match op.assoc {
            Assoc::Left => op.tier.saturating_sub(1),
            Assoc::Right => op.tier,
        };
        match op.right {
            Operand::Argument => self.argument_member(tier),
            Operand::ArgumentExpression => self.argument_expression_member(tier),
            Operand::TypeReference => self.type_reference_member(SyntaxKind::TypeReferenceMember),
            Operand::TypeResult => self.type_reference_member(SyntaxKind::TypeResultMember),
        }
    }

    // production: ConditionalExpression
    //
    // ConditionalExpression : OperatorExpression =
    //     operator = 'if'
    //     ownedRelationship += ArgumentMember '?'
    //     ownedRelationship += ArgumentExpressionMember 'else'
    //     ownedRelationship += ArgumentExpressionMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // Tier 15, the loosest. Both branches are ArgumentExpressionMembers and the
    // condition is an ArgumentMember: the branches are referenced rather than
    // evaluated, which is how the abstract syntax reifies the short circuit. The
    // `else` branch is read at the same tier, so `if a? b else if c? d else e`
    // nests to the right without parentheses; the condition is read one tier tighter,
    // so a bare `if` there is not.
    fn conditional_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConditionalExpression);
        self.expect_keyword("if");
        self.argument_member(TIER_CONDITIONAL - 1);
        self.expect(SyntaxKind::Question, "`?`");
        self.argument_expression_member(TIER_CONDITIONAL);
        self.expect_keyword("else");
        self.argument_expression_member(TIER_CONDITIONAL);
        self.empty_result_member();
        self.finish_node();
    }

    // production: UnaryOperatorExpression
    //
    // UnaryOperatorExpression : OperatorExpression =
    //     operator = UnaryOperator
    //     ownedRelationship += ArgumentMember
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // UnaryOperator = '+' | '-' | '~' | 'not'                    (KerML 8.2.5.8.1)
    //
    // The operand is read at tier 2, which is what makes `- -a` nest — the
    // specification's ArgumentMember reaches OwnedExpression, so a unary operand may
    // itself be unary. The Pilot's UnaryExpression rule does not recurse and rejects
    // it; that is a property of its parser generator, not of the language, and it is
    // not ported.
    fn unary_operator_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::UnaryOperatorExpression);
        self.bump_unary_operator();
        self.argument_member(TIER_UNARY);
        self.empty_result_member();
        self.finish_node();
    }

    // production: ExtentExpression
    //
    // ExtentExpression : OperatorExpression =
    //     operator = 'all'
    //     ownedRelationship += TypeReferenceMember               (KerML 8.2.5.8.1)
    //
    // Tier 1, the tightest, and the one OperatorExpression in the clause that owns no
    // EmptyResultMember — its result comes from the type, so none is written here.
    fn extent_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ExtentExpression);
        self.expect_keyword("all");
        self.type_reference_member(SyntaxKind::TypeReferenceMember);
        self.finish_node();
    }

    // production: ClassificationExpression
    //
    // ClassificationExpression : OperatorExpression =
    //     ( ownedRelationship += ArgumentMember )?
    //     ( operator = ClassificationTestOperator
    //       ownedRelationship += TypeReferenceMember
    //     | operator = CastOperator
    //       ownedRelationship += TypeResultMember )
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // ClassificationTestOperator = 'istype' | 'hastype' | '@'    (KerML 8.2.5.8.1)
    // CastOperator              = 'as'                           (KerML 8.2.5.8.1)
    //
    // The ArgumentMember is optional, so this is reached two ways: from `infix_tail`
    // with a left operand already in the tree, and from `prefix_expression` with
    // none. `start` is the checkpoint of the left operand, or `None` when there is
    // no left operand to fold in.
    //
    // The right operand is a type and not an expression, in two different
    // memberships: a test names a TypeReferenceMember and a cast a TypeResultMember.
    fn classification_expression(&mut self, start: Option<rowan::Checkpoint>) {
        self.eat_trivia();
        match start {
            Some(start) => {
                self.start_node_at(start, SyntaxKind::ClassificationExpression);
                self.wrap_at(start, LEFT_ARGUMENT);
            }
            None => self.start_node(SyntaxKind::ClassificationExpression),
        }
        let is_cast = self.at_keyword("as");
        self.bump_classification_operator();
        let member = if is_cast {
            SyntaxKind::TypeResultMember
        } else {
            SyntaxKind::TypeReferenceMember
        };
        self.type_reference_member(member);
        self.empty_result_member();
        self.finish_node();
    }

    // production: MetaclassificationExpression
    // production: MetaclassificationTestOperator
    // production: MetaCastOperator
    //
    // MetaclassificationExpression : OperatorExpression =
    //     ownedRelationship += MetadataArgumentMember
    //     ( operator = MetaclassificationTestOperator
    //       ownedRelationship += TypeReferenceMember
    //     | operator = MetaCastOperator
    //       ownedRelationship += TypeResultMember )
    //     ownedRelationship += EmptyResultMember                 (KerML 8.2.5.8.1)
    //
    // MetaclassificationTestOperator = '@@'                      (KerML 8.2.5.8.1)
    // MetaCastOperator               = 'meta'                    (KerML 8.2.5.8.1)
    //
    // Tier 8 with the classification operators, but NOT interchangeable with them:
    // its left operand is a MetadataArgumentMember, and that reaches a QualifiedName
    // (MetadataArgument -> MetadataValue -> MetadataReference ->
    // ElementReferenceMember = [QualifiedName]) rather than an OwnedExpression. So
    // the left operand of `meta` is a reference, never an expression, and
    // `(a + b) meta T` is not something the clause can express. That also makes it
    // unchainable: `x meta A meta B` would need a MetaclassificationExpression where
    // a QualifiedName is required, so it is reported. The two operators are value
    // productions, read inline and building no node.
    fn metaclassification_expression(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetaclassificationExpression);
        self.metadata_argument_member();
        let is_cast = self.at_keyword("meta");
        if is_cast {
            self.bump_as(SyntaxKind::KwMeta);
        } else {
            self.expect(SyntaxKind::AtAt, "`@@`");
        }
        let member = if is_cast {
            SyntaxKind::TypeResultMember
        } else {
            SyntaxKind::TypeReferenceMember
        };
        self.type_reference_member(member);
        self.empty_result_member();
        self.finish_node();
    }

    // production: MetadataArgumentMember
    //
    // MetadataArgumentMember : ParameterMembership =
    //     ownedRelatedElement += MetadataArgument                (KerML 8.2.5.8.1)
    //
    // production: MetadataArgument
    //
    // MetadataArgument : Feature =
    //     ownedRelationship += MetadataValue                     (KerML 8.2.5.8.1)
    //
    // production: MetadataValue
    //
    // MetadataValue : FeatureValue = value = MetadataReference   (KerML 8.2.5.8.1)
    //
    // production: MetadataReference
    //
    // MetadataReference : MetadataAccessExpression =
    //     ownedRelationship += ElementReferenceMember            (KerML 8.2.5.8.1)
    //
    // production: ElementReferenceMember
    //
    // ElementReferenceMember : Membership =
    //     memberElement = [QualifiedName]                        (KerML 8.2.5.8.3)
    //
    // Five memberships over one name. They are nodes rather than collapsed because
    // the abstract syntax reifies each of them, and sv2-hir will need to walk them.
    fn metadata_argument_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataArgumentMember);
        self.start_node(SyntaxKind::MetadataArgument);
        self.start_node(SyntaxKind::MetadataValue);
        self.start_node(SyntaxKind::MetadataReference);
        self.start_node(SyntaxKind::ElementReferenceMember);
        self.qualified_name();
        self.finish_node();
        self.finish_node();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }
}
