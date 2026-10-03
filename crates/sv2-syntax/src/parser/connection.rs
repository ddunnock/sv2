// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Ports, connections, interfaces and allocations, `SysML` 8.2.2.10 to 8.2.2.14, and
//! the connector parts they share.

use crate::generated::kinds::SyntaxKind;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;

impl Parser<'_> {
    /// Whether a `PortDefinition` starts at the `n`th meaningful token.
    ///
    /// Its own question rather than a row in `SIMPLE_DEFINITIONS`, because it is not on
    /// that spine: it takes a `DefinitionPrefix` like an attribute and then carries a
    /// trailing member none of the eight has.
    pub(super) fn at_port_definition(&self, n: usize) -> bool {
        let after = self.skip_definition_prefix(n);
        self.nth_is_keyword(after, "port") && self.nth_is_keyword(after + 1, "def")
    }

    // production: ConjugatedPortTyping@sysml
    //
    // ConjugatedPortTyping : ConjugatedPortTyping =
    //     '~' originalPortDefinition = ~[QualifiedName]          (SysML 8.2.2.12)
    //
    // `port p : ~P;` "is equivalent to port p : P::'~P';" (7.12.3, receipt f0b805cf): the
    // `~` names the conjugated port definition every port definition implicitly declares,
    // and is not part of the name, so it stays outside the quotes of `~'P-1'`. The
    // Pilot writes the same text as `[ConjugatedPortDefinition | ConjugatedQualifiedName]`
    // over an xtext-only rule that deviations.json resolves follow_spec: no production of
    // its own, the `~` and the name read here.
    //
    // `~[QualifiedName]` is a QualifiedName parsed as written and resolved as its last
    // segment with `~` prepended, appended to the whole (8.2.2.12, Note 2): sv2-resolve's.
    // A name only, never a feature chain.
    //
    // The BNF assigns the name to `originalPortDefinition`, which the metaclass does not
    // have: ConjugatedPortTyping (8.3.12.3) has `conjugatedPortDefinition`, redefining
    // `type`, and a derived `portDefinition`, and the Pilot assigns the first. sv2-resolve
    // sets `conjugatedPortDefinition` from the resolved `~[QualifiedName]`.
    //
    // constraint: ConjugatedPortTyping::deriveConjugatedPortTypingPortDefinition
    //     (8.3.12.3, receipt 486ec938) — portDefinition is derived, for sv2-resolve.
    pub(super) fn conjugated_port_typing(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConjugatedPortTyping);
        self.expect(SyntaxKind::Tilde, "`~`");
        self.qualified_name();
        self.finish_node();
    }

    // production: PortDefinition
    //
    // PortDefinition = DefinitionPrefix 'port' 'def' Definition
    //                  ownedRelationship += ConjugatedPortDefinitionMember
    //                                                            (SysML 8.2.2.12)
    //
    // Not in SIMPLE_DEFINITIONS, and the one part that keeps it out consumes no tokens.
    // Every port definition implicitly declares its conjugate — `port def P;` gives you
    // `~P` — so reading it with the shared spine would accept the text and silently drop
    // three elements the abstract syntax says are there.
    //
    // A DefinitionPrefix, not an OccurrenceDefinitionPrefix: a port is not an occurrence,
    // so `individual port def P;` is an error, as it is for an attribute.
    pub(super) fn port_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PortDefinition);
        self.definition_prefix();
        self.expect_keyword("port");
        self.expect_keyword("def");
        self.definition();
        self.conjugated_port_definition_member();
        self.finish_node();
    }

    // production: ConjugatedPortDefinitionMember
    //
    // ConjugatedPortDefinitionMember : OwningMembership =
    //     ownedRelatedElement += ConjugatedPortDefinition        (SysML 8.2.2.12)
    //
    // production: ConjugatedPortDefinition
    //
    // ConjugatedPortDefinition = ownedRelationship += PortConjugation
    //
    // production: PortConjugation
    //
    // PortConjugation = { }
    //
    // Three nested nodes and not one token, exactly as EmptyMultiplicityMember is two.
    // The nodes are built rather than omitted so the tree carries the elements the
    // abstract syntax puts there; a consumer asking a port definition for its conjugate
    // finds it, rather than having to know to synthesise one.
    fn conjugated_port_definition_member(&mut self) {
        self.start_node(SyntaxKind::ConjugatedPortDefinitionMember);
        self.start_node(SyntaxKind::ConjugatedPortDefinition);
        self.start_node(SyntaxKind::PortConjugation);
        self.finish_node();
        self.finish_node();
        self.finish_node();
    }

    /// Whether a `ConnectionUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix`, then `connection` with no `def` after it — the `def` makes
    /// it the `ConnectionDefinition` — or `connect`, the shorthand with no declaration
    /// (`SysML` 8.2.2.13.1). `connect` also opens an `InterfaceUsage`'s connector part
    /// (8.2.2.14), but only after `interface` and its declaration, never at the start of a
    /// member, so asking at `n` cannot find it.
    pub(super) fn at_connection_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        (self.nth_is_keyword(after, "connection") && !self.nth_is_keyword(after + 1, "def"))
            || self.nth_is_keyword(after, "connect")
    }

    // production: ConnectionUsage@sysml
    //
    // ConnectionUsage : ConnectionUsage =
    //     OccurrenceUsagePrefix
    //     ( 'connection' UsageDeclaration ValuePart?
    //       ( 'connect' ConnectorPart )?
    //     | 'connect' ConnectorPart
    //     ) UsageBody                                                (SysML 8.2.2.13.1)
    //
    // "the related features of the connection usage may be identified in a comma-separated
    // list, between parentheses (...), preceded by the keyword connect, placed after the
    // connection usage declaration and before its body ... If the declaration part of the
    // connection usage is empty when using this notation, then the keyword connection may
    // be omitted" (7.13.2, receipt 5a3a8867). The binary form writes `to` between its two
    // ends instead.
    //
    // A StructureUsageElement (8.2.2.6.4), owned as FlowUsage is. Marked although
    // OccurrenceUsagePrefix is not, as ActionUsage is.
    //
    // implied specialization: Connections::connections, Connections::binaryConnections
    // constraint: ConnectionUsage::checkConnectionUsageSpecialization and
    //     checkConnectionUsageBinarySpecialization (8.3.13.4, receipt cf52fa8d), the second
    //     for `ownedEndFeature->size() = 2`. Injections belong in sv2-hir; this layer
    //     builds the tree only (ADR-0002).
    pub(super) fn connection_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConnectionUsage);
        self.occurrence_usage_prefix();
        if self.at_keyword("connection") {
            self.expect_keyword("connection");
            self.usage_declaration();
            if self.at_value_part() {
                self.value_part();
            }
            if self.at_keyword("connect") {
                self.expect_keyword("connect");
                self.connector_part();
            }
        } else {
            self.expect_keyword("connect");
            self.connector_part();
        }
        self.usage_body();
        self.finish_node();
    }

    /// Whether an `AllocationUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix`, then `allocation` with no `def` after it -- the `def` makes
    /// it the `AllocationDefinition` -- or `allocate`, the shorthand with no declaration
    /// (`SysML` 8.2.2.15), as `at_connection_usage` asks of `connection` and `connect`.
    pub(super) fn at_allocation_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        (self.nth_is_keyword(after, "allocation") && !self.nth_is_keyword(after + 1, "def"))
            || self.nth_is_keyword(after, "allocate")
    }

    // production: AllocationUsage@sysml
    //
    // AllocationUsage =
    //     OccurrenceUsagePrefix
    //     AllocationUsageDeclaration UsageBody                   (SysML 8.2.2.15)
    //
    // production: AllocationUsageDeclaration@sysml
    //
    // AllocationUsageDeclaration : AllocationUsage =
    //       'allocation' UsageDeclaration
    //       ( 'allocate' ConnectorPart )?
    //     | 'allocate' ConnectorPart                             (SysML 8.2.2.15)
    //
    // "An allocation definition or usage is declared like a connection definition or usage
    // (see 7.13.2 ), but using the kind keyword allocation ... Shorthand notations similar
    // to those for connection usages ... may also be used for allocation usages, but using
    // the keyword allocate instead of connect. If the declaration part of the allocation
    // usage is empty when using this notation, then the keyword allocation may be omitted"
    // (7.15.2, receipt 546a588c). ConnectionUsage's shape, with two differences the
    // grammar states: the declaration is a production and a node of its own, and it takes
    // no ValuePart, so `allocation a = x;` is reported. The Pilot writes `UsageDeclaration?`
    // (SysML.xtext:1219), which accepts the same text, a UsageDeclaration being nullable;
    // its four keyword rules are deviations AllocationKeyword, AllocationUsageKeyword,
    // AllocateKeyword and AllocationDefKeyword (xtext_only, follow_spec), so the literals
    // are matched here.
    //
    // "Allocation definitions and usages are always binary, having exactly two end
    // features, even if abstract" (7.15.2), but ConnectorPart admits NaryConnectorPart,
    // and the one constraint of 8.3.15.3 says nothing of arity; the grammar is followed,
    // and the arity is left to the layers above.
    //
    // The metaclass is AllocationUsage (8.3.15.3, receipt 13aa56df), a ConnectionUsage. A
    // StructureUsageElement (8.2.2.6.4), as ConnectionUsage is.
    //
    // implied specialization: Allocations::allocations
    // constraint: AllocationUsage::checkAllocationUsageSpecialization (8.3.15.3). An
    //     injection, so sv2-hir's (ADR-0002).
    pub(super) fn allocation_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::AllocationUsage);
        self.occurrence_usage_prefix();
        self.eat_trivia();
        self.start_node(SyntaxKind::AllocationUsageDeclaration);
        if self.at_keyword("allocation") {
            self.expect_keyword("allocation");
            self.usage_declaration();
            if self.at_keyword("allocate") {
                self.expect_keyword("allocate");
                self.connector_part();
            }
        } else {
            self.expect_keyword("allocate");
            self.connector_part();
        }
        self.finish_node();
        self.usage_body();
        self.finish_node();
    }

    /// Whether an `InterfaceDefinition` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceDefinitionPrefix 'interface' 'def'` (`SysML` 8.2.2.14.1). Only the `def`
    /// separates it from an `InterfaceUsage`.
    pub(super) fn at_interface_definition(&self, n: usize) -> bool {
        let after = self.skip_occurrence_definition_prefix(n);
        self.nth_is_keyword(after, "interface") && self.nth_is_keyword(after + 1, "def")
    }

    // production: InterfaceDefinition@sysml
    //
    // InterfaceDefinition =
    //     OccurrenceDefinitionPrefix 'interface' 'def'
    //     DefinitionDeclaration InterfaceBody                     (SysML 8.2.2.14.1)
    //
    // "An interface definition or usage is declared like a connection definition or usage
    // (see 7.13.2), but using the kind keyword interface" (7.14.2, receipt a994b0e7). Not
    // on the SIMPLE_DEFINITIONS spine, which takes a Definition: this takes a declaration
    // and an InterfaceBody of its own. The metaclass is InterfaceDefinition, "a
    // ConnectionDefinition all of whose ends are PortUsages" (8.3.14.2, receipt bc244764);
    // that its ends are ports is the metaclass's, and `interface_body` reads `end p : P;`
    // as a DefaultInterfaceEnd, a PortUsage, for that reason.
    //
    // implied specialization: Interfaces::Interface, Interfaces::BinaryInterface
    // constraint: InterfaceDefinition::checkInterfaceDefinitionSpecialization and
    //     checkInterfaceDefinitionBinarySpecialization (8.3.14.2), the second for
    //     `ownedEndFeature->size() = 2`. Injections, so sv2-hir's (ADR-0002).
    pub(super) fn interface_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InterfaceDefinition);
        self.occurrence_definition_prefix();
        self.expect_keyword("interface");
        self.expect_keyword("def");
        self.definition_declaration();
        self.interface_body();
        self.finish_node();
    }

    // production: InterfaceBody@sysml
    //
    // InterfaceBody : Type = ';' | '{' InterfaceBodyItem* '}'   (SysML 8.2.2.14.1)
    //
    // production: InterfaceBodyItem@sysml
    //
    // InterfaceBodyItem : Type =
    //       ownedRelationship += DefinitionMember
    //     | ownedRelationship += VariantUsageMember
    //     | ownedRelationship += InterfaceNonOccurrenceUsageMember
    //     | ( ownedRelationship += SourceSuccessionMember )?
    //       ownedRelationship += InterfaceOccurrenceUsageMember
    //     | ownedRelationship += AliasMember
    //     | ownedRelationship += Import                            (SysML 8.2.2.14.1)
    //
    // InterfaceBodyItem gets no node, as DefinitionBodyItem gets none: the member is the
    // item. `body_elements` reads it over Body::Interface, which answers each of the six.
    fn interface_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InterfaceBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Interface);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after an interface declaration");
        }
        self.finish_node();
    }

    // production: InterfaceNonOccurrenceUsageMember@sysml
    //
    // InterfaceNonOccurrenceUsageMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += InterfaceNonOccurrenceUsageElement
    //
    // production: InterfaceNonOccurrenceUsageElement@sysml
    //
    // InterfaceNonOccurrenceUsageElement : Usage =
    //     ReferenceUsage | AttributeUsage | EnumerationUsage
    //   | BindingConnectorAsUsage | SuccessionAsUsage
    //
    // production: InterfaceOccurrenceUsageMember@sysml
    //
    // InterfaceOccurrenceUsageMember : FeatureMembership =
    //     MemberPrefix ownedRelatedElement += InterfaceOccurrenceUsageElement
    //
    // production: InterfaceOccurrenceUsageElement@sysml
    //
    // InterfaceOccurrenceUsageElement : Usage =
    //     DefaultInterfaceEnd | StructureUsageElement | BehaviorUsageElement
    //                                                            (SysML 8.2.2.14.1)
    //
    // None has a method: `membership` builds the member node from `Body::member`, and the
    // element is read by `default_interface_end`, asked first in an interface body, or
    // by `usage_element_of_class`. InterfaceNonOccurrenceUsageElement is marked because
    // all five alternatives are read and `at_usage_no_interface_body_admits` refuses the
    // two NonOccurrenceUsageElements it leaves out; InterfaceOccurrenceUsageElement
    // because all three are, the last two whole (StructureUsageElement@sysml,
    // BehaviorUsageElement@sysml at `occurrence_usage_element`).

    /// Whether a usage `InterfaceNonOccurrenceUsageElement` leaves out starts here.
    ///
    /// `DefaultReferenceUsage` and `ExtendedUsage` are the two `NonOccurrenceUsageElement`s
    /// it does not list (`SysML` 8.2.2.14.1, 8.2.2.6.4). A keywordless `end` is neither
    /// here: it is `DefaultInterfaceEnd`, asked first.
    pub(super) fn at_usage_no_interface_body_admits(&self) -> bool {
        let n = usize::from(self.at_visibility());
        !self.at_default_interface_end(n)
            && (self.at_default_reference_usage(n) || self.at_extended_usage(n))
    }

    /// Whether a `DefaultInterfaceEnd` starts at the `n`th meaningful token: an `end` with
    /// no kind keyword after it before the declaration's `;`, brace or `=`, which is what
    /// `skip_end_usage_prefix` answers `None` for. With one, the `end` is that usage's
    /// `EndUsagePrefix` (`end port p : P;`).
    pub(super) fn at_default_interface_end(&self, n: usize) -> bool {
        self.nth_is_keyword(n, "end") && self.skip_end_usage_prefix(n).is_none()
    }

    // production: DefaultInterfaceEnd@sysml
    //
    // DefaultInterfaceEnd : PortUsage = isEnd ?= 'end' Usage    (SysML 8.2.2.14.1)
    //
    // "All the end features of an interface definition or usage must be port usages, so
    // the use of the port keyword is optional on such end features if no owned cross
    // feature is declared on the end" (7.14.2, receipt a994b0e7). So `end supplierPort :
    // FuelOutPort;` (training/11. Interfaces/Interface Example.sysml:7) is a PortUsage in an
    // interface body, where the same text is a ReferenceUsage, by deviation
    // DefaultReferenceUsage, in a connection definition's DefinitionBody.
    //
    // implied specialization: Ports::ports
    // constraint: PortUsage::checkPortUsageSpecialization (8.3.12.6, receipt 542cf245),
    //     as for any PortUsage. An injection, so sv2-hir's (ADR-0002).
    pub(super) fn default_interface_end(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefaultInterfaceEnd);
        self.expect_keyword("end");
        self.usage();
        self.finish_node();
    }

    /// Whether an `InterfaceUsage` starts at the `n`th meaningful token.
    ///
    /// `OccurrenceUsagePrefix 'interface'` with no `def` after it (`SysML` 8.2.2.14.2).
    pub(super) fn at_interface_usage(&self, n: usize) -> bool {
        let after = self.skip_occurrence_usage_prefix(n);
        self.nth_is_keyword(after, "interface") && !self.nth_is_keyword(after + 1, "def")
    }

    // production: InterfaceUsage@sysml
    //
    // InterfaceUsage =
    //     OccurrenceUsagePrefix 'interface'
    //     InterfaceUsageDeclaration InterfaceBody                  (SysML 8.2.2.14.2)
    //
    // A StructureUsageElement (8.2.2.6.4), owned as ConnectionUsage is. The metaclass is
    // InterfaceUsage, a ConnectionUsage (8.3.14.3, receipt 63f04edc). "An interface usage
    // must only be defined by interface definitions" (7.14.2) is typing, resolution's.
    //
    // implied specialization: Interfaces::interfaces, Interfaces::binaryInterfaces
    // constraint: InterfaceUsage::checkInterfaceUsageSpecialization and
    //     checkInterfaceUsageBinarySpecialization (8.3.14.3), the second for
    //     `ownedEndFeature->size() = 2`. Injections, so sv2-hir's (ADR-0002).
    pub(super) fn interface_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InterfaceUsage);
        self.occurrence_usage_prefix();
        self.expect_keyword("interface");
        self.interface_usage_declaration();
        self.interface_body();
        self.finish_node();
    }

    // production: InterfaceUsageDeclaration@sysml
    //
    // InterfaceUsageDeclaration : InterfaceUsage =
    //       UsageDeclaration ValuePart? ( 'connect' InterfacePart )?
    //     | InterfacePart                                          (SysML 8.2.2.14.2)
    //
    // "if the declaration part of an interface usage is empty, then the interface keyword
    // is still included, but the connect keyword may be omitted" (7.14.2, receipt
    // a994b0e7): `interface fuelTank.fuelingPort to engine.fuelingPort;` is the second
    // alternative. Both may open on a name, and on a `[` (a multiplicity or an end's cross
    // multiplicity), so a whole InterfaceEnd is looked past and a `to` after it decides,
    // as `flow_declaration` decides between its two; a `(` can open only an
    // NaryInterfacePart.
    fn interface_usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InterfaceUsageDeclaration);
        let part_first = self.at(SyntaxKind::LParen)
            || self
                .skip_interface_end(0)
                .is_some_and(|n| self.nth_is_keyword(n, "to"));
        if part_first {
            self.interface_part();
        } else {
            self.usage_declaration();
            if self.at_value_part() {
                self.value_part();
            }
            if self.at_keyword("connect") {
                self.expect_keyword("connect");
                self.interface_part();
            }
        }
        self.finish_node();
    }

    // production: InterfacePart@sysml
    //
    // InterfacePart : InterfaceUsage =
    //     BinaryInterfacePart | NaryInterfacePart                  (SysML 8.2.2.14.2)
    //
    // An alternation with no node, as ConnectorPart has none, told by the `(` as that is.
    fn interface_part(&mut self) {
        if self.at(SyntaxKind::LParen) {
            self.nary_interface_part();
        } else {
            self.binary_interface_part();
        }
    }

    // production: BinaryInterfacePart@sysml
    //
    // BinaryInterfacePart : InterfaceUsage =
    //     ownedRelationship += InterfaceEndMember 'to'
    //     ownedRelationship += InterfaceEndMember                  (SysML 8.2.2.14.2)
    fn binary_interface_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BinaryInterfacePart);
        self.interface_end_member();
        self.expect_keyword("to");
        self.interface_end_member();
        self.finish_node();
    }

    // production: NaryInterfacePart@sysml
    //
    // NaryInterfacePart : InterfaceUsage =
    //     '(' ownedRelationship += InterfaceEndMember ','
    //         ownedRelationship += InterfaceEndMember
    //         ( ',' ownedRelationship += InterfaceEndMember )* ')' (SysML 8.2.2.14.2)
    //
    // At least TWO ends, and no trailing comma, as NaryConnectorPart.
    fn nary_interface_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NaryInterfacePart);
        self.expect(SyntaxKind::LParen, "`(`");
        self.interface_end_member();
        self.expect(SyntaxKind::Comma, "`,` and a second end");
        self.interface_end_member();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.interface_end_member();
        }
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish_node();
    }

    // production: InterfaceEndMember@sysml
    //
    // InterfaceEndMember : EndFeatureMembership =
    //     ownedRelatedElement += InterfaceEnd                      (SysML 8.2.2.14.2)
    fn interface_end_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InterfaceEndMember);
        self.interface_end();
        self.finish_node();
    }

    // production: InterfaceEnd@sysml
    //
    // InterfaceEnd : PortUsage =
    //     ( ownedRelationship += OwnedCrossMultiplicityMember )?
    //     ( declaredName = NAME REFERENCES )?
    //     ownedRelationship += OwnedReferenceSubsetting            (SysML 8.2.2.14.2)
    //
    // ConnectorEnd's text with a PortUsage for its metaclass, read by the same
    // `end_reference` -- and WITHOUT the multiplicity after the reference that deviation
    // ConnectorEnd-trailing-multiplicity adds to ConnectorEnd alone, so `interface a[1] to
    // b;` is reported. `skip_interface_end` walks the same parts.
    //
    // implied specialization: Ports::ports
    // constraint: PortUsage::checkPortUsageSpecialization (8.3.12.6, receipt 542cf245),
    //     as for DefaultInterfaceEnd, the other PortUsage an interface declares. sv2-hir's.
    fn interface_end(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InterfaceEnd);
        self.end_reference();
        self.finish_node();
    }

    // production: ConnectorPart@sysml
    //
    // ConnectorPart : ConnectionUsage =
    //     BinaryConnectorPart | NaryConnectorPart                    (SysML 8.2.2.13.1)
    //
    // An alternation with no node, as ControlNode has none: the part taken is the node.
    // A `(` decides, since a ConnectorEnd opens on `[` or a name and never on `(`.
    fn connector_part(&mut self) {
        if self.at(SyntaxKind::LParen) {
            self.nary_connector_part();
        } else {
            self.binary_connector_part();
        }
    }

    // production: BinaryConnectorPart@sysml
    //
    // BinaryConnectorPart : ConnectionUsage =
    //     ownedRelationship += ConnectorEndMember 'to'
    //     ownedRelationship += ConnectorEndMember                    (SysML 8.2.2.13.1)
    fn binary_connector_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BinaryConnectorPart);
        self.connector_end_member();
        self.expect_keyword("to");
        self.connector_end_member();
        self.finish_node();
    }

    // production: NaryConnectorPart@sysml
    //
    // NaryConnectorPart : ConnectionUsage =
    //     '(' ownedRelationship += ConnectorEndMember ','
    //         ownedRelationship += ConnectorEndMember
    //         ( ',' ownedRelationship += ConnectorEndMember )* ')'   (SysML 8.2.2.13.1)
    //
    // At least TWO ends, and no trailing comma.
    fn nary_connector_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::NaryConnectorPart);
        self.expect(SyntaxKind::LParen, "`(`");
        self.connector_end_member();
        self.expect(SyntaxKind::Comma, "`,` and a second end");
        self.connector_end_member();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.connector_end_member();
        }
        self.expect(SyntaxKind::RParen, "`)`");
        self.finish_node();
    }

    /// The index just past an `InterfaceEnd` written at the `n`th token, walked as
    /// `end_reference` reads it: `ConnectorEnd`'s walk less the trailing multiplicity only
    /// `ConnectorEnd` takes (`SysML` 8.2.2.14.2).
    pub(super) fn skip_interface_end(&self, n: usize) -> Option<usize> {
        let mut n = n;
        if self.nth_is(n, SyntaxKind::LBracket) {
            n = self.skip_bracketed(n)?;
        }
        if self.nth_is_name(n)
            && (self.nth_is(n + 1, SyntaxKind::ColonColonGt)
                || self.nth_is_keyword(n + 1, "references"))
        {
            n += 2;
        }
        let mut n = self.skip_qualified_name(n)?;
        while self.nth_is(n, SyntaxKind::Dot) && self.nth_is_name(n + 1) {
            n = self.skip_qualified_name(n + 1)?;
        }
        Some(n)
    }

    /// Whether a `BindingConnectorAsUsage` starts at the `n`th meaningful token.
    ///
    /// `UsagePrefix`, then `binding` or `bind`. Both are reserved (`SysML` 8.2.2.1.2) and
    /// open no other production, so the keyword decides on its own, with none of the
    /// lookahead `at_succession_as_usage` needs to tell `first` apart: a binding missing
    /// its `=` or an end is read, and reported where the part is missing.
    ///
    /// The prefix skipped is `UsagePrefix`, which is what `binding_connector_as_usage`
    /// reads, and NOT `OccurrenceUsagePrefix`, for the reason `at_succession_as_usage`
    /// gives: a recogniser that looked past `snapshot` would accept a member the parser
    /// then cannot consume.
    pub(super) fn at_binding_connector_as_usage(&self, n: usize) -> bool {
        let n = self.skip_usage_prefix(n);
        self.nth_is_keyword(n, "binding") || self.nth_is_keyword(n, "bind")
    }

    // production: BindingConnectorAsUsage@sysml
    //
    // BindingConnectorAsUsage =
    //     UsagePrefix ( 'binding' UsageDeclaration )?
    //     'bind' ownedRelationship += ConnectorEndMember
    //     '=' ownedRelationship += ConnectorEndMember
    //     UsageBody                                                 (SysML 8.2.2.13.2)
    //
    // A binding declared as a usage, naming its two related features (receipt
    // 6c24121f). The metaclass is BindingConnectorAsUsage (8.3.13.2, receipt 9cf9f357),
    // both a ConnectorAsUsage and a KerML BindingConnector. "A binding is not a kind of
    // occurrence usage", so it takes UsagePrefix and not OccurrenceUsagePrefix, and "if
    // the declaration part is empty, then the keyword binding may be omitted" (7.13.3,
    // receipt 6db87b41) — which is what the corpus's `bind a = b;` is.
    //
    // ConnectorEnd is read whole, its cross multiplicity included (`bind [1] a = b;`).
    //
    // implied specialization: Links::selfLinks
    // constraint: BindingConnector::checkBindingConnectorSpecialization,
    //     `specializesFromLibrary('Links::selfLinks')` (KerML 8.3.4.5.2), which "requires
    //     that BindingConnectorAsUsages specialize the kernel Feature Links:selfLink"
    //     (SysML 8.4.9.3, receipt 6b0fb7c5). An injection, so sv2-hir's; this layer builds
    //     the tree only (ADR-0002).
    // constraint: BindingConnector::validateBindingConnectorIsBinary,
    //     `relatedFeature->size() = 2` (KerML 8.3.4.5.2). Holds by construction here: the
    //     production writes exactly two ends.
    pub(super) fn binding_connector_as_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BindingConnectorAsUsage);
        self.usage_prefix();
        if self.at_keyword("binding") {
            self.bump_as(keyword("binding").unwrap_or(SyntaxKind::BasicName));
            self.usage_declaration();
        }
        self.expect_keyword("bind");
        self.connector_end_member();
        self.expect(SyntaxKind::Eq, "`=`");
        self.connector_end_member();
        self.usage_body();
        self.finish_node();
    }
}
