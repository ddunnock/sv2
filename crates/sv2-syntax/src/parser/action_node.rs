// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Action nodes, `SysML` 8.2.2.16: accept, send, assignment, terminate, `if`, `while`
//! and `for` nodes, and the control nodes.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;

/// The four `ControlNode`s: `ControlNodePrefix KEYWORD UsageDeclaration ActionBody`,
/// differing in the keyword and the metaclass (`SysML` 8.2.2.17.3). The keywords are
/// reserved and disjoint, so the order decides nothing.
const CONTROL_NODES: [(&str, SyntaxKind); 4] = [
    ("merge", SyntaxKind::MergeNode),
    ("decide", SyntaxKind::DecisionNode),
    ("join", SyntaxKind::JoinNode),
    ("fork", SyntaxKind::ForkNode),
];

/// Which `ActionNode` a member is, of the alternatives this parser reads.
///
/// `ActionNode = ControlNode | SendNode | AcceptNode | AssignmentNode | TerminateNode |
/// IfNode | WhileLoopNode | ForLoopNode` (`SysML` 8.2.2.17.1). The control nodes open on a
/// `ControlNodePrefix` and their keyword; the other three on `OccurrenceUsagePrefix
/// ActionNodeUsageDeclaration?` and theirs, which is why they are told apart here rather
/// than folded into `CONTROL_NODES`.
#[derive(Clone, Copy)]
pub(super) enum ActionNode {
    /// One of the four `CONTROL_NODES`: its keyword and its node.
    Control(&'static str, SyntaxKind),
    /// `AcceptNode`, 8.2.2.17.4.
    Accept,
    /// `SendNode`, 8.2.2.17.4.
    Send,
    /// `AssignmentNode`, 8.2.2.17.5.
    Assignment,
    /// `TerminateNode`, 8.2.2.17.6.
    Terminate,
    /// `WhileLoopNode`, 8.2.2.17.7, by `while` or `loop`.
    WhileLoop,
    /// `IfNode`, 8.2.2.17.7.
    If,
    /// `ForLoopNode`, 8.2.2.17.7.
    ForLoop,
}

impl Parser<'_> {
    /// Which action node the declaration here is, when it is one: `accept`, `send` or
    /// `assign`, written first or after an `action` declaration.
    ///
    /// Every such node opens on `ActionNodeUsageDeclaration? KEYWORD` (`SysML` 8.2.2.17.4,
    /// 8.2.2.17.5), where `ActionNodeUsageDeclaration = 'action' UsageDeclaration?`. A
    /// perform declaration opens on `action` too, so the keyword has to be looked for past
    /// the declaration, which contains no reserved word and ends before a `;` or a body.
    pub(super) fn action_node_keyword(&self) -> Option<&'static str> {
        self.action_node_keyword_at(0, &["accept", "send", "assign"])
    }

    /// `action_node_keyword`, asked from the `start`th meaningful token over the keywords
    /// `nodes`. An action body asks for `terminate` as well (8.2.2.17.6); the state and
    /// transition forms do not, having no terminate alternative (8.2.2.18.1, 8.2.2.18.3).
    fn action_node_keyword_at(&self, start: usize, nodes: &[&'static str]) -> Option<&'static str> {
        self.action_node_keyword_index_at(start, nodes)
            .map(|(word, _)| word)
    }

    /// `action_node_keyword_at`, answering where the keyword stands as well.
    fn action_node_keyword_index_at(
        &self,
        start: usize,
        nodes: &[&'static str],
    ) -> Option<(&'static str, usize)> {
        let found = |n: usize| {
            nodes
                .iter()
                .copied()
                .find(|word| self.nth_is_keyword(n, word))
        };
        if let Some(word) = found(start) {
            return Some((word, start));
        }
        if !self.nth_is_keyword(start, "action") {
            return None;
        }
        let mut n = start + 1;
        loop {
            if let Some(word) = found(n) {
                return Some((word, n));
            }
            if self.peek_nth(n).is_none()
                || self.nth_is_any_keyword(n, &["then", "if", "do"])
                || self.nth_is(n, SyntaxKind::Semicolon)
                || self.nth_is(n, SyntaxKind::LBrace)
                || self.nth_is(n, SyntaxKind::RBrace)
            {
                return None;
            }
            n += 1;
        }
    }

    // production: AcceptNodeDeclaration@sysml
    //
    // AcceptNodeDeclaration : AcceptActionUsage =
    //     ActionNodeUsageDeclaration? 'accept' AcceptParameterPart   (SysML 8.2.2.17.4)
    //
    // production: ActionNodeUsageDeclaration@sysml
    //
    // ActionNodeUsageDeclaration : ActionUsage =
    //     'action' UsageDeclaration?                                 (SysML 8.2.2.17.2)
    //
    // "If the action declaration part is empty, then the action keyword may be omitted"
    // (7.17.8, receipt bb0d6dc7). Read for the state and effect forms and for AcceptNode,
    // the action-body form, which puts an OccurrenceUsagePrefix before it.
    pub(super) fn accept_node_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AcceptNodeDeclaration);
        self.action_node_usage_declaration("accept");
        self.expect_keyword("accept");
        self.accept_parameter_part();
        self.finish_node();
    }

    /// `ActionNodeUsageDeclaration?` before an action node's `word`: `'action'
    /// UsageDeclaration?` when an `action` is written, nothing otherwise.
    ///
    /// `word` is reserved and a `UsageDeclaration` never contains it, so the declaration
    /// ends at `word`; an `action` with nothing between it and `word` is the declaration
    /// with its optional part empty.
    fn action_node_usage_declaration(&mut self, word: &str) {
        if self.at_keyword("action") {
            self.eat_trivia();
            self.start_node(SyntaxKind::ActionNodeUsageDeclaration);
            self.expect_keyword("action");
            if !self.at_keyword(word) {
                self.usage_declaration();
            }
            self.finish_node();
        }
    }

    // production: AcceptNode@sysml
    //
    // AcceptNode : AcceptActionUsage =
    //     OccurrenceUsagePrefix AcceptNodeDeclaration ActionBody     (SysML 8.2.2.17.4)
    //
    // The action-body form of the declaration the state and transition forms already read.
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Actions::acceptActions, Actions::Action::acceptSubactions
    // constraint: AcceptActionUsage::checkAcceptActionUsageSpecialization and
    //     checkAcceptActionUsageSubactionSpecialization (8.3.17.2, receipt e2e8fcd4).
    //     Injections belong in sv2-hir; this layer builds the tree only (ADR-0002).
    fn accept_node(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AcceptNode);
        self.occurrence_usage_prefix();
        self.accept_node_declaration();
        self.action_body();
        self.finish_node();
    }

    // production: SendNode@sysml
    //
    // SendNode : SendActionUsage =
    //     OccurrenceUsagePrefix ActionNodeUsageDeclaration? 'send'
    //     ( ownedRelationship += NodeParameterMember SenderReceiverPart?
    //     | ownedRelationship += EmptyParameterMember SenderReceiverPart )?
    //     ActionBody            (SysML 8.2.2.17.4, as deviations SendNode and
    //                            SendReceiverPart-misspelling read it)
    //
    // The clause prints `SendReceiverPart` here; deviation SendReceiverPart-misspelling
    // (spec_only, follow_spec, SYSML21-408) reads it as the SenderReceiverPart the same
    // clause defines.
    //
    // The clause's own line writes ActionUsageDeclaration? where its three siblings write
    // ActionNodeUsageDeclaration?; deviations.json records follow_xtext (SYSML21-624), and
    // `action publish send new Publish(...) via publicationPort;` (examples/Interaction
    // Sequencing Examples/ServerSequenceRealization-2.sysml:19) parses under that reading
    // and no other. So a declared send writes `action`, and `snd send x;` is not one.
    //
    // "values for the three SendAction parameters are given after the action declaration
    // part, identified by the keywords send (payload), via (sender) and to (receiver)"
    // (7.17.7, receipt db730711), and validateSendActionParameters wants all three as owned
    // input parameters "whether or not they have FeatureValues" (8.3.17.15, receipt
    // 320cf1d4). That is what the EmptyParameterMembers are: the payload of `send via p`,
    // the sender of `send x to q`, each a parameter the text does not write.
    //
    // The parameter group is optional, and `send {` is the bare form (examples/Simple
    // Tests/ActionTest.sysml:34). A `{` there is read as the ActionBody. BodyExpression,
    // which also opens on `{`, is unimplemented, and the Pilot resolves the same choice
    // the same way, trying ActionBody first (SysML.xtext SendNode).
    //
    // implied specialization: Actions::sendActions, Actions::Action::sendSubactions
    // constraint: SendActionUsage::checkSendActionUsageSpecialization and
    //     checkSendActionUsageSubactionSpecialization (8.3.17.15, receipt 320cf1d4), whose
    //     OCL names 'Actions::Action::acceptSubactions' where its prose says sendSubactions;
    //     the semantic rule 8.4.13.5 (receipt e2a3572d) also says sendSubactions. The
    //     conflict is unrecorded in deviations.json and must be adjudicated before sv2-hir
    //     injects it.
    //     Injections belong in sv2-hir; this layer builds the tree only (ADR-0002).
    fn send_node(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SendNode);
        self.occurrence_usage_prefix();
        if self.at_keyword("action") {
            // deviation: SendNode
            self.note_deviation("SendNode", "`action` declaring a send action node");
        }
        self.action_node_usage_declaration("send");
        self.expect_keyword("send");
        if self.at_sender_receiver_part() {
            self.empty_parameter_member();
            self.sender_receiver_part();
        } else if !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::LBrace) {
            self.node_parameter_member();
            if self.at_sender_receiver_part() {
                self.sender_receiver_part();
            }
        }
        self.action_body();
        self.finish_node();
    }

    // production: SendNodeDeclaration@sysml
    //
    // SendNodeDeclaration : SendActionUsage =
    //     ActionNodeUsageDeclaration? 'send'
    //     ownedRelationship += NodeParameterMember SenderReceiverPart?  (SysML 8.2.2.17.4)
    //
    // The state and transition forms' declaration. Unlike SendNode's, its payload is NOT
    // optional: `entry send via p;` is no StateSendActionUsage.
    pub(super) fn send_node_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SendNodeDeclaration);
        self.action_node_usage_declaration("send");
        self.expect_keyword("send");
        self.node_parameter_member();
        if self.at_sender_receiver_part() {
            self.sender_receiver_part();
        }
        self.finish_node();
    }

    /// Whether a `SenderReceiverPart` starts here: `via`, or `to`.
    fn at_sender_receiver_part(&self) -> bool {
        self.at_keyword("via") || self.at_keyword("to")
    }

    // production: SenderReceiverPart@sysml
    //
    // SenderReceiverPart : SendActionUsage =
    //       'via' ownedRelationship += NodeParameterMember
    //       ( 'to' ownedRelationship += NodeParameterMember )?
    //     | ownedRelationship += EmptyParameterMember
    //       'to' ownedRelationship += NodeParameterMember            (SysML 8.2.2.17.4)
    //
    // `via` before `to`, never after: the second alternative has no `via`.
    fn sender_receiver_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SenderReceiverPart);
        if self.at_keyword("via") {
            self.expect_keyword("via");
            self.node_parameter_member();
            if self.at_keyword("to") {
                self.expect_keyword("to");
                self.node_parameter_member();
            }
        } else {
            self.empty_parameter_member();
            self.expect_keyword("to");
            self.node_parameter_member();
        }
        self.finish_node();
    }

    // production: AssignmentNode@sysml
    //
    // AssignmentNode : AssignmentActionUsage =
    //     OccurrenceUsagePrefix AssignmentNodeDeclaration ActionBody (SysML 8.2.2.17.5)
    //
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Actions::assignmentActions, Actions::Action::assignments
    // constraint: AssignmentActionUsage::checkAssignmentActionUsageSpecialization and
    //     checkAssignmentActionUsageSubactionSpecialization (8.3.17.5, receipt d0143fb3).
    //     Injections belong in sv2-hir; this layer builds the tree only (ADR-0002).
    fn assignment_node(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AssignmentNode);
        self.occurrence_usage_prefix();
        self.assignment_node_declaration();
        self.action_body();
        self.finish_node();
    }

    // production: TerminateNode@sysml
    //
    // TerminateNode : TerminateActionUsage =
    //     OccurrenceUsagePrefix ActionNodeUsageDeclaration?
    //     'terminate' ( ownedRelationship += NodeParameterMember )?
    //     ActionBody                                                 (SysML 8.2.2.17.6)
    //
    // "the value for the terminated occurrence parameter is given after the action
    // declaration part, after the keyword terminate. If the declaration part is empty,
    // then the action keyword may be omitted" (7.17.10, receipt aa4c3e77). With no value,
    // "the default is to terminate the immediately containing action" -- a default of the
    // model, so the tree holds no parameter the text does not write.
    //
    // `terminate {` is the ActionBody, not a BodyExpression as the parameter: 7.17.10's
    // `action terminateProccess terminate { in terminatedProcess; }` binds the parameter
    // by a flow into the body, and the Pilot resolves the choice the same way, trying
    // ActionBody first (SysML.xtext TerminateNode), as SendNode's `send {` is read.
    //
    // The metaclass is TerminateActionUsage (8.3.17.16, receipt 95d9959c), an ActionUsage.
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Actions::terminateActions, Actions::Action::terminateSubactions
    // constraint: TerminateActionUsage::checkTerminateActionUsageSpecialization and
    //     checkTerminateActionUsageSubactionSpecialization (8.3.17.16; 8.4.13.8, receipt
    //     e7812698). Injections belong in sv2-hir; this layer builds the tree only
    //     (ADR-0002).
    fn terminate_node(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TerminateNode);
        self.occurrence_usage_prefix();
        self.action_node_usage_declaration("terminate");
        self.expect_keyword("terminate");
        if !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::LBrace) {
            self.node_parameter_member();
        }
        self.action_body();
        self.finish_node();
    }

    // production: ActionNodePrefix@sysml
    //
    // ActionNodePrefix : ActionUsage =
    //     OccurrenceUsagePrefix ActionNodeUsageDeclaration?          (SysML 8.2.2.17.2)
    //
    // The prefix of the three structured nodes, WhileLoopNode, IfNode and ForLoopNode
    // (8.2.2.17.7), before `word`. It builds no node of its own: it returns the ActionUsage
    // it prefixes, the Pilot states it as a fragment (SysML.xtext ActionNodePrefix), and
    // TerminateNode and SendNode, which write the same two parts inline, own them directly.
    // Marked although OccurrenceUsagePrefix is not, as ActionUsage is.
    fn action_node_prefix(&mut self, word: &str) {
        self.occurrence_usage_prefix();
        self.action_node_usage_declaration(word);
    }

    // production: WhileLoopNode@sysml
    //
    // WhileLoopNode : WhileLoopActionUsage =
    //     ActionNodePrefix
    //     ( 'while' ownedRelationship += ExpressionParameterMember
    //     | 'loop' ownedRelationship += EmptyParameterMember
    //     )
    //     ownedRelationship += ActionBodyParameterMember
    //     ( 'until' ownedRelationship += ExpressionParameterMember ';' )?
    //                                                            (SysML 8.2.2.17.7)
    //
    // "the action declaration part is followed by the keyword while, which introduces a
    // Boolean-valued while expression, followed by a body clause, and then, optionally,
    // the keyword until, which introduces a Boolean-valued until expression terminated
    // with a semicolon ... The keyword loop may be used as a shorthand for while true"
    // (7.17.12, receipt b0446148). The EmptyParameterMember of `loop` holds the while
    // slot: the whileArgument is the first input parameter, the bodyAction the second and
    // the untilArgument the third (deriveWhileLoopActionUsageWhileArgument, 8.3.17.19,
    // receipt b915a32c; deriveLoopActionUsageBodyAction, 8.3.17.12, receipt 2d51ae94),
    // so the body stays second whichever keyword is written.
    //
    // The expression ends at the body clause's `{` or `action`: neither continues an
    // OwnedExpression, and a BodyExpression's `{` is reached only as an argument.
    //
    // implied specialization: Actions::whileLoopActions, Actions::Action::whileLoops
    // constraint: WhileLoopActionUsage::checkWhileLoopActionUsageSpecialization and
    //     checkWhileLoopActionUsageSubactionSpecialization (8.3.17.19). Injections belong
    //     in sv2-hir; this layer builds the tree only (ADR-0002).
    // constraint: WhileLoopActionUsage::validateWhileLoopActionUsage, "at least two owned
    //     input parameters" (8.3.17.19): the tree always holds the while slot and the
    //     body, so the text cannot violate it.
    fn while_loop_node(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::WhileLoopNode);
        // The keyword `at_action_node` found past the prefix and the declaration.
        let word = self
            .action_node_keyword_at(self.skip_occurrence_usage_prefix(0), &["while", "loop"])
            .unwrap_or("while");
        self.action_node_prefix(word);
        if word == "loop" {
            self.expect_keyword("loop");
            self.empty_parameter_member();
        } else {
            self.expect_keyword("while");
            self.expression_parameter_member();
        }
        self.action_body_parameter_member();
        if self.at_keyword("until") {
            self.expect_keyword("until");
            self.expression_parameter_member();
            self.expect(SyntaxKind::Semicolon, "`;` after an `until` expression");
        }
        self.finish_node();
    }

    // production: IfNode@sysml
    //
    // IfNode : IfActionUsage =
    //     ActionNodePrefix
    //     'if' ownedRelationship += ExpressionParameterMember
    //     ownedRelationship += ActionBodyParameterMember
    //     ( 'else' ownedRelationship +=
    //       ( ActionBodyParameterMember | IfNodeParameterMember ) )?
    //                                                            (SysML 8.2.2.17.7)
    //
    // production: IfNodeParameterMember@sysml
    //
    // IfNodeParameterMember : ParameterMembership =
    //     ownedRelatedElement += IfNode                              (SysML 8.2.2.17.7)
    //
    // "the action declaration part is followed by the keyword if, which introduces a
    // Boolean-valued condition expression, followed by a then clause and, for an
    // IfThenElseAction, the keyword else and an else clause ... if the else-clause is
    // itself an if action usage, then the special if action usage notation can be used"
    // (7.17.11, receipt 5a98cecc). The then clause is the ActionBodyParameterMember; the
    // grammar writes no `then` keyword before it. The ifArgument, thenAction and
    // elseAction are the first, second and third parameters (deriveIfActionUsage*,
    // 8.3.17.10, receipt 11d9efbf), which is the order the members are owned in.
    //
    // The `else` is taken only when an else clause follows it: a `{`, an `action`, or an
    // IfNode. Otherwise it is a DefaultTargetSuccession, `else stop;` (8.2.2.17.8), which
    // may follow an ActionBehaviorMember as an ActionTargetSuccessionMember (8.2.2.17.1),
    // and whose target is a ConnectorEnd, never one of those three.
    //
    // implied specialization: Actions::ifThenActions, or Actions::ifThenElseActions with
    //     an else clause; Actions::Action::ifSubactions
    // constraint: IfActionUsage::checkIfActionUsageSpecialization and
    //     checkIfActionUsageSubactionSpecialization (8.3.17.10). Injections belong in
    //     sv2-hir; this layer builds the tree only (ADR-0002).
    fn if_node(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::IfNode);
        self.action_node_prefix("if");
        self.expect_keyword("if");
        self.expression_parameter_member();
        self.action_body_parameter_member();
        if self.at_keyword("else")
            && (self.nth_is(1, SyntaxKind::LBrace)
                || self.nth_is_keyword(1, "action")
                || matches!(self.at_action_node(1), Some(ActionNode::If)))
        {
            self.expect_keyword("else");
            if matches!(self.at_action_node(0), Some(ActionNode::If)) {
                self.eat_trivia();
                self.start_node(SyntaxKind::IfNodeParameterMember);
                self.if_node();
                self.finish_node();
            } else {
                self.action_body_parameter_member();
            }
        }
        self.finish_node();
    }

    // production: ForLoopNode@sysml
    //
    // ForLoopNode : ForLoopActionUsage =
    //     ActionNodePrefix
    //     'for' ownedRelationship += ForVariableDeclarationMember
    //     'in' ownedRelationship += NodeParameterMember
    //     ownedRelationship += ActionBodyParameterMember             (SysML 8.2.2.17.7)
    //
    // "the action declaration part is followed by the keyword for, which introduces a loop
    // variable declaration followed by the keyword in and a sequence expression, and,
    // after that, a body clause" (7.17.12, receipt b0446148). The sequence is a
    // NodeParameterMember, bound to the seq input (deriveForLoopActionUsageSeqArgument,
    // 8.3.17.9, receipt 277f7243). The declaration ends at `in`, which is reserved; the
    // sequence expression ends at the body clause's `{` or `action`.
    //
    // implied specialization: Actions::forLoopActions, Actions::Action::forLoops, and the
    //     loop variable's redefinition of Actions::ForLoopAction::var
    // constraint: ForLoopActionUsage::checkForLoopActionUsageSpecialization,
    //     checkForLoopActionUsageSubactionSpecialization and
    //     checkForLoopActionUsageVarRedefinition (8.3.17.9). Injections belong in sv2-hir;
    //     this layer builds the tree only (ADR-0002).
    fn for_loop_node(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ForLoopNode);
        self.action_node_prefix("for");
        self.expect_keyword("for");
        self.for_variable_declaration_member();
        self.expect_keyword("in");
        self.node_parameter_member();
        self.action_body_parameter_member();
        self.finish_node();
    }

    // production: ForVariableDeclarationMember@sysml
    //
    // ForVariableDeclarationMember : FeatureMembership =
    //     ownedRelatedElement += UsageDeclaration                    (SysML 8.2.2.17.7)
    //
    // read, by deviation ForVariableDeclarationMember (conflict, follow_xtext), as
    // `ownedRelatedElement += ForVariableDeclaration`: the clause's line assigns the
    // fragment where its three sibling members assign an element-producing production,
    // and ForVariableDeclaration is otherwise unreachable. Both readings accept identical
    // text, so the deviation has no site: it changes the element, not the language. The
    // element it builds is the one validateForLoopActionUsageLoopVariable asks for, "The
    // first ownedFeature of a ForLoopActionUsage must be a ReferenceUsage" (8.3.17.9).
    //
    // production: ForVariableDeclaration@sysml
    //
    // ForVariableDeclaration : ReferenceUsage = UsageDeclaration   (SysML 8.2.2.17.7)
    fn for_variable_declaration_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ForVariableDeclarationMember);
        self.start_node(SyntaxKind::ForVariableDeclaration);
        self.usage_declaration();
        self.finish_node();
        self.finish_node();
    }

    // production: ExpressionParameterMember@sysml
    //
    // ExpressionParameterMember : ParameterMembership =
    //     ownedRelatedElement += OwnedExpression                     (SysML 8.2.2.17.7)
    fn expression_parameter_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ExpressionParameterMember);
        self.owned_expression();
        self.finish_node();
    }

    // production: AssignmentNodeDeclaration@sysml
    //
    // AssignmentNodeDeclaration : ActionUsage =
    //     ActionNodeUsageDeclaration? 'assign'
    //     ownedRelationship += AssignmentTargetMember
    //     ownedRelationship += FeatureChainMember ':='
    //     ownedRelationship += NodeParameterMember                  (SysML 8.2.2.17.5)
    //
    // "An assignment part consists of the keyword assign followed by an expression that
    // evaluates to the target and a feature chain identifying the referent, separated by a
    // dot (.), followed by the symbol := and an expression whose result is the assigned
    // value" (7.17.9, receipt 7d690ecc). The target may be omitted; the referent may not.
    pub(super) fn assignment_node_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AssignmentNodeDeclaration);
        self.action_node_usage_declaration("assign");
        self.expect_keyword("assign");
        self.assignment_target_member();
        self.sysml_feature_chain_member();
        self.expect(SyntaxKind::ColonEq, "`:=`");
        self.node_parameter_member();
        self.finish_node();
    }

    // production: AssignmentTargetMember@sysml
    //
    // AssignmentTargetMember : ParameterMembership =
    //     ownedRelatedElement += AssignmentTargetParameter           (SysML 8.2.2.17.5)
    //
    // production: AssignmentTargetParameter@sysml
    //
    // AssignmentTargetParameter : ReferenceUsage =
    //     ( ownedRelationship += AssignmentTargetBinding '.' )?      (SysML 8.2.2.17.5)
    //
    // production: AssignmentTargetBinding@sysml
    //
    // AssignmentTargetBinding : FeatureValue =
    //     ownedRelatedElement += NonFeatureChainPrimaryExpression    (SysML 8.2.2.17.5)
    //
    // The clause's decomposition, not the Pilot's TargetParameter, which carries the
    // referent inside itself; the derived unit AssignmentTargetMember@sysml records why.
    //
    // The target is the FIRST primary and the referent everything after its `.`: 7.17.9's
    // example says of `assign sim.vehicle.position := ...` that "The target of the
    // assignment below is "sim". The referent feature chain is "vehicle.position"" (receipt
    // 7d690ecc). With no `.` the parameter is empty, built from no tokens, and the target
    // is "implicitly the occurrence owning the assignment action usage".
    //
    // Which it is is decided by looking past the primary for the `.`; see
    // `skip_assignment_target`, which also says which primaries it can look past.
    //
    // MARKED, BUT NOT EVERY TARGET IS READ. NonFeatureChainPrimaryExpression includes the
    // postfix forms (Index, Bracket, Select, Collect, FunctionOperation), whose operand
    // may itself be a chain — `a.b#(1).c` — and there the target is not the first
    // primary; 7.17.9 does not address that case, and it is not read. Nor are literal and
    // null targets. Each such input is reported, never misread.
    fn assignment_target_member(&mut self) {
        let binding = self
            .skip_assignment_target(0)
            .is_some_and(|after| self.nth_is(after, SyntaxKind::Dot));
        if binding {
            self.eat_trivia();
        }
        self.start_node(SyntaxKind::AssignmentTargetMember);
        self.start_node(SyntaxKind::AssignmentTargetParameter);
        if binding {
            self.start_node(SyntaxKind::AssignmentTargetBinding);
            self.non_feature_chain_primary_expression();
            self.finish_node();
            self.expect(SyntaxKind::Dot, "`.`");
        }
        self.finish_node();
        self.finish_node();
    }

    /// The index just past a `NonFeatureChainPrimaryExpression` written from the `n`th
    /// token, for the primaries an assignment's target is written as.
    ///
    /// A name, an invocation (`Increment(c).count`), a constructor or a parenthesised
    /// expression. A literal or `null` target is NOT looked past, so a `.` after one is
    /// not seen and the literal is read as the referent and reported: 7.17.9 requires the
    /// target to "evaluate to an occurrence" (receipt 7d690ecc), which neither can, but
    /// the grammar admits both, and this is a gap in the lookahead rather than a rule.
    fn skip_assignment_target(&self, n: usize) -> Option<usize> {
        let after = if self.nth_is(n, SyntaxKind::LParen) {
            return self.skip_parenthesised(n);
        } else if self.nth_is_keyword(n, "new") {
            self.skip_qualified_name(n + 1)?
        } else {
            self.skip_qualified_name(n)?
        };
        if self.nth_is(after, SyntaxKind::LParen) {
            self.skip_parenthesised(after)
        } else {
            Some(after)
        }
    }

    // production: NodeParameterMember@sysml
    //
    // NodeParameterMember : ParameterMembership =
    //     ownedRelatedElement += NodeParameter                       (SysML 8.2.2.17.4)
    //
    // production: NodeParameter@sysml
    //
    // NodeParameter : ReferenceUsage =
    //     ownedRelationship += FeatureBinding                        (SysML 8.2.2.17.4)
    //
    // production: FeatureBinding@sysml
    //
    // FeatureBinding : FeatureValue =
    //     ownedRelatedElement += OwnedExpression                     (SysML 8.2.2.17.4)
    //
    // Three nodes over one expression: a parameter whose value is bound to what `via`
    // names. `via commPort` binds the receiver to the port.
    pub(super) fn node_parameter_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NodeParameterMember);
        self.start_node(SyntaxKind::NodeParameter);
        self.start_node(SyntaxKind::FeatureBinding);
        self.owned_expression();
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    /// Which `ActionNode` starts at the `n`th meaningful token, of those this parser reads.
    ///
    /// A `ControlNode`, or `OccurrenceUsagePrefix ActionNodeUsageDeclaration?` and one of
    /// `accept`, `send`, `assign`, `terminate`, `while`, `loop`, `if` or `for` (`SysML`
    /// 8.2.2.17.4-7) — the prefix looked past here, the declaration by
    /// `action_node_keyword_index_at`. An `if` is an `IfNode` only when its body clause
    /// follows the condition; see `if_node_body_follows`.
    pub(super) fn at_action_node(&self, n: usize) -> Option<ActionNode> {
        if let Some((word, node)) = self.at_control_node(n) {
            return Some(ActionNode::Control(word, node));
        }
        match self.action_node_keyword_index_at(
            self.skip_occurrence_usage_prefix(n),
            &[
                "accept",
                "send",
                "assign",
                "terminate",
                "while",
                "loop",
                "if",
                "for",
            ],
        )? {
            ("if", at) => self.if_node_body_follows(at + 1).then_some(ActionNode::If),
            (word, _) => Self::action_node_of_keyword(word),
        }
    }

    /// The `ActionNode` that opens on `word`, of the keywords `at_action_node` asks for
    /// other than `if`.
    fn action_node_of_keyword(word: &str) -> Option<ActionNode> {
        match word {
            "accept" => Some(ActionNode::Accept),
            "send" => Some(ActionNode::Send),
            "assign" => Some(ActionNode::Assignment),
            "terminate" => Some(ActionNode::Terminate),
            "while" | "loop" => Some(ActionNode::WhileLoop),
            "for" => Some(ActionNode::ForLoop),
            _ => None,
        }
    }

    /// Whether an `IfNode`'s body clause follows the condition that starts at the `n`th
    /// meaningful token: whether a `{` or an `action` comes before a `then`, a `;` or a
    /// `}`.
    ///
    /// Three productions open on `if` where an item may stand: `IfNode`, whose condition
    /// is followed by its `ActionBodyParameterMember` (`'action'` or `'{'`, `SysML`
    /// 8.2.2.17.7); `GuardedTargetSuccession`, whose guard is followed by `then`
    /// (8.2.2.17.8); and, as a calculation body's result, `KerML`'s `ConditionalExpression`
    /// (`'if' Expression '?' Expression 'else' Expression`, 8.2.5.8.1), which ends at the
    /// body's `}` or a `;`. `action` is reserved and no expression contains it; a
    /// `BodyExpression`'s `{` can stand inside a condition (`xs->forAll { ... }`), and is
    /// then taken as the body clause, as it is by `at_guarded_target_succession`, which
    /// stops at the same brace.
    fn if_node_body_follows(&self, n: usize) -> bool {
        let mut n = n;
        loop {
            if self.nth_is(n, SyntaxKind::LBrace) || self.nth_is_keyword(n, "action") {
                return true;
            }
            if self.peek_nth(n).is_none()
                || self.nth_is_keyword(n, "then")
                || self.nth_is(n, SyntaxKind::Semicolon)
                || self.nth_is(n, SyntaxKind::RBrace)
            {
                return false;
            }
            n += 1;
        }
    }

    // production: ActionNode@sysml
    //
    // ActionNode : ActionUsage =
    //       ControlNode
    //     | SendNode | AcceptNode
    //     | AssignmentNode
    //     | TerminateNode
    //     | IfNode | WhileLoopNode | ForLoopNode                  (SysML 8.2.2.17.2)
    //
    // An alternation with no node, marked because all eight alternatives are read — the
    // convention ActionBehaviorMember and OwnedExpression follow.
    /// The `ActionNode` `at_action_node` found.
    pub(super) fn action_node(&mut self, node: ActionNode) {
        match node {
            ActionNode::Control(word, kind) => self.control_node(kind, word),
            ActionNode::Accept => self.accept_node(),
            ActionNode::Send => self.send_node(),
            ActionNode::Assignment => self.assignment_node(),
            ActionNode::Terminate => self.terminate_node(),
            ActionNode::WhileLoop => self.while_loop_node(),
            ActionNode::If => self.if_node(),
            ActionNode::ForLoop => self.for_loop_node(),
        }
    }

    /// Which `ControlNode` starts at the `n`th meaningful token, if one does.
    ///
    /// A `ControlNodePrefix` looked past, then one of the four keywords. The prefix is
    /// `RefPrefix 'individual'? PortionKind?` (`SysML` 8.2.2.17.3) — `RefPrefix`, not
    /// `BasicUsagePrefix`, so a `ref` is not looked past and `ref merge m;` is reported.
    /// Its `UsageExtensionKeyword*` is.
    fn at_control_node(&self, n: usize) -> Option<(&'static str, SyntaxKind)> {
        let mut n = self.skip_ref_prefix(n);
        n += usize::from(self.nth_is_keyword(n, "individual"));
        n += usize::from(self.nth_is_keyword(n, "snapshot") || self.nth_is_keyword(n, "timeslice"));
        let n = self.skip_prefix_metadata(n);
        CONTROL_NODES
            .into_iter()
            .find(|(word, _)| self.nth_is_keyword(n, word))
    }

    // production: ControlNode@sysml
    //
    // ControlNode = MergeNode | DecisionNode | JoinNode | ForkNode   (SysML 8.2.2.17.3)
    //
    // production: MergeNode@sysml
    // production: DecisionNode@sysml
    // production: JoinNode@sysml
    // production: ForkNode@sysml
    //
    // MergeNode = ControlNodePrefix isComposite ?= 'merge' UsageDeclaration ActionBody
    //
    // and the other three the same over `decide`, `join` and `fork` (8.2.2.17.3, receipt
    // 08ce2499). One method reads all four, as `simple_usage` reads the seven, because
    // they differ in nothing the parser decides; each builds its own node so the tree
    // says which metaclass it is. ControlNode itself gets no node — an alternation, like
    // UsageElement — and is marked because every alternative is read.
    //
    // `isComposite ?= 'merge'` assigns a property from the keyword and adds no token:
    // validateControlNodeIsComposite requires a ControlNode to be composite (8.3.17.6,
    // receipt 695df335), and the keyword is how the text says so.
    //
    // The metaclasses are MergeNode (8.3.17.13, receipt e6e46c06), DecisionNode
    // (8.3.17.7, receipt 630e3433), JoinNode (8.3.17.11, receipt 48153851) and ForkNode
    // (8.3.17.8, receipt 58ebdb55), each a ControlNode, an ActionUsage (8.3.17.6).
    //
    // implied specialization: Actions::Action::merges, ::decisions, ::join, ::forks
    // constraint: MergeNode::checkMergeNodeSpecialization and its three siblings
    //     `specializesFromLibrary('Actions::Action::merges')` and so on. Injections
    //     belong in sv2-hir; this layer builds the tree only (ADR-0002).
    fn control_node(&mut self, node: SyntaxKind, word: &str) {
        self.eat_trivia();
        self.start_node(node);
        self.control_node_prefix();
        self.expect_keyword(word);
        self.usage_declaration();
        self.action_body();
        self.finish_node();
    }

    // production: ControlNodePrefix@sysml
    //
    // ControlNodePrefix : OccurrenceUsage =
    //     RefPrefix ( isIndividual ?= 'individual' )?
    //     ( portionKind = PortionKind { isPortion = true } )?
    //     UsageExtensionKeyword*                                   (SysML 8.2.2.17.3)
    //
    // The clause's `'individual` is missing
    // its closing quote; deviation ControlNodePrefix (spec_only, follow_spec, SYSML21-400)
    // closes it, and the keyword read here is that repaired one. The node is built even
    // when every slot is empty, as OccurrenceUsagePrefix's is.
    fn control_node_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ControlNodePrefix);
        self.ref_prefix();
        self.eat_optional_keyword("individual");
        if self.at_keyword("snapshot") || self.at_keyword("timeslice") {
            self.portion_kind();
        }
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        self.finish_node();
    }
}
