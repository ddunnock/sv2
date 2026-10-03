// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Imports, `SysML` 8.2.2.5.2 and `KerML` 8.2.3.5: visibility, imported names, filter
//! packages, recursive imports, and `expose`.

use rowan::Language as _;

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::language::Sv2Language;
use crate::parser::Parser;
use crate::parser::lookahead::{VISIBILITY, keyword};

impl Parser<'_> {
    /// Whether an `Import` starts here rather than a `PackageMember`.
    ///
    /// Both may open with a `VisibilityIndicator`, so the indicator alone does not
    /// say which: `Import`'s visibility is required and `MemberPrefix`'s is
    /// optional. The keyword after it is what separates them. A bare `import` with
    /// no visibility is neither, and falls through to recovery — which is the rule
    /// tests/rejection/import-without-visibility.sysml holds.
    pub(super) fn at_import(&self) -> bool {
        self.at_visibility() && self.nth_is_keyword(1, "import")
    }

    /// `[QualifiedName] ( ',' [QualifiedName] )*`, a dependency's clients or suppliers.
    pub(super) fn qualified_name_list(&mut self) {
        self.qualified_name();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.qualified_name();
        }
    }

    // production: Import
    //
    // Import = visibility = VisibilityIndicator 'import' ( isImportAll ?= 'all' )?
    //          ImportDeclaration RelationshipBody
    //
    // visibility carries no `( )?`: an import states its visibility. The
    // specification BNF, the Pilot's ImportPrefix fragment, and all 741 imports in
    // the pinned corpus agree, and `MemberPrefix`'s optional visibility one clause
    // away is what makes that worth stating rather than assuming.
    pub(super) fn import(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Import);
        self.visibility_indicator();
        self.bump_as(keyword("import").unwrap_or(SyntaxKind::BasicName));
        if self.at_keyword("all") {
            self.bump_as(keyword("all").unwrap_or(SyntaxKind::BasicName));
        }
        self.import_declaration();
        self.relationship_body();
        self.finish_node();
    }

    // production: VisibilityIndicator
    pub(super) fn visibility_indicator(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::VisibilityIndicator);
        match VISIBILITY.iter().find(|word| self.at_keyword(word)) {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            // Unreachable from body_elements, which only enters on at_visibility.
            // Reported rather than consumed, so a future caller cannot lose a token.
            None => self.error_expected("`public`, `private` or `protected`"),
        }
        self.finish_node();
    }

    // production: ImportDeclaration
    //
    // ImportDeclaration = MembershipImport | NamespaceImport
    //
    // production: MembershipImport
    //
    // MembershipImport = importedMembership = [QualifiedName]
    //                    ( '::' isRecursive ?= '**' )?
    //
    // production: NamespaceImport
    //
    // NamespaceImport = importedNamespace = [QualifiedName] '::' '*'
    //                   ( '::' isRecursive ?= '**' )?
    //                 | importedNamespace = FilterPackage
    //                   { ownedRelatedElement += importedNamespace }
    //                                               (SysML 8.2.2.5.1, KerML 8.2.3.4.2)
    //
    // All three are the same in both languages, so shared units. MembershipImport was read
    // whole long before it was marked; NamespaceImport is whole now that its FilterPackage
    // alternative is read.
    //
    // Which import it is cannot be known until after the QualifiedName, because the
    // first alternatives share that prefix, and a FilterPackage cannot be known until
    // after a whole declaration, because it OPENS with one: `vehicle::**[@Safety]` is the
    // membership import `vehicle::**` and then its condition. So the nodes are opened
    // retroactively at checkpoints rather than guessed and repaired. A `[` after a
    // declaration can only be a FilterPackageMember: a RelationshipBody, the one other
    // thing that follows, opens on `;` or `{`.
    fn import_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ImportDeclaration);
        self.imported();
        self.finish_node();
    }

    /// A whole `MembershipImport` or `NamespaceImport`, returning which.
    ///
    /// What an `ImportDeclaration` holds, and what `SysML`'s `MembershipExpose` and
    /// `NamespaceExpose` hold directly (8.2.2.26.2), so both callers wrap this.
    fn imported(&mut self) -> SyntaxKind {
        self.eat_trivia();
        let outer = self.builder.checkpoint();
        self.qualified_name();
        let kind = self.import_suffix();
        self.builder
            .start_node_at(outer, Sv2Language::kind_to_raw(kind));
        self.finish_node();
        if self.at(SyntaxKind::LBracket) {
            self.filter_package(outer);
            return SyntaxKind::NamespaceImport;
        }
        kind
    }

    // production: FilterPackage@sysml
    //
    // FilterPackage : Package =
    //     ownedRelationship += FilterPackageImport
    //     ( ownedRelationship += FilterPackageMember )+          (SysML 8.2.2.5.1)
    //
    // production: FilterPackage@kerml
    //
    // FilterPackage : Package =
    //     ownedRelationship += ImportDeclaration
    //     ( ownedRelationship += FilterPackageMember )+          (KerML 8.2.3.4.2)
    //
    // production: FilterPackageImport@sysml
    //
    // FilterPackageImport : Import = ImportDeclaration { visibility = 'public' }
    //                                                            (SysML 8.2.2.5.1)
    //
    // Two units, one method: the languages differ only in whether the declaration is
    // wrapped in a FilterPackageImport. SysML's clause uses FilterPackageImport without
    // defining it (SYSML21-449); the definition is the specification's own Tier B' BNF,
    // deviation FilterPackageImport (conflict, follow_spec). Its action sets visibility and
    // consumes no tokens, so the node holds the declaration alone. The Pilot's
    // FilterPackageMembershipImport and FilterPackageNamespaceImport are factorings of it,
    // recorded xtext_only/follow_spec, and add no production here.
    //
    // `outer` is where the import already read began. It becomes the FilterPackage's own
    // first member, an ImportDeclaration, so the NamespaceImport and FilterPackage are
    // opened around it there, then (SysML) the FilterPackageImport, then the declaration
    // itself around the import alone.
    fn filter_package(&mut self, outer: rowan::Checkpoint) {
        self.start_node_at(outer, SyntaxKind::NamespaceImport);
        self.start_node_at(outer, SyntaxKind::FilterPackage);
        let own: &[SyntaxKind] = match self.language {
            Language::SysMl => &[
                SyntaxKind::FilterPackageImport,
                SyntaxKind::ImportDeclaration,
            ],
            Language::KerMl => &[SyntaxKind::ImportDeclaration],
        };
        self.wrap_at(outer, own);
        while self.at(SyntaxKind::LBracket) {
            self.filter_package_member();
        }
        self.finish_node();
        self.finish_node();
    }

    // production: Expose@sysml
    //
    // Expose = 'expose' ( MembershipExpose | NamespaceExpose ) RelationshipBody
    //                                                            (SysML 8.2.2.26.2)
    //
    // production: MembershipExpose@sysml
    //
    // MembershipExpose = MembershipImport                        (SysML 8.2.2.26.2)
    //
    // production: NamespaceExpose@sysml
    //
    // NamespaceExpose = NamespaceImport                          (SysML 8.2.2.26.2)
    //
    // `expose vehicle::**[@Safety];` (training/42. Views/Views Example.sysml:25). The
    // metaclass is Expose (8.3.26.2, receipt d5e99e59), an Import, but the text is not an
    // Import's: no VisibilityIndicator and no `all`. "An Expose always has protected
    // visibility" and "always imports all Elements", which validateExposeVisibility and
    // validateExposeIsImportAll state of the model, so there is nothing to write. The Pilot
    // reads the keyword as an ExposePrefix that sets the visibility; the literal is matched
    // here. Which of the two alternatives it is, is which import `imported` read.
    //
    // constraint: Expose::validateExposeOwningNamespace (8.3.26.2): the owner is a
    //     ViewUsage. The grammar already reaches this from ViewBodyItem alone.
    pub(super) fn expose(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Expose);
        self.expect_keyword("expose");
        self.eat_trivia();
        let start = self.builder.checkpoint();
        let alternative = match self.imported() {
            SyntaxKind::NamespaceImport => SyntaxKind::NamespaceExpose,
            _ => SyntaxKind::MembershipExpose,
        };
        self.wrap_at(start, &[alternative]);
        self.relationship_body();
        self.finish_node();
    }

    // production: FilterPackageMember
    //
    // FilterPackageMember : ElementFilterMembership =
    //     '[' ownedRelatedElement += OwnedExpression ']'
    //                                               (SysML 8.2.2.5.1, KerML 8.2.3.4.2)
    //
    // A shared unit. The `+` is the loop in `filter_package`, which enters only on `[`.
    // The expression is a whole OwnedExpression, bounded by the brackets.
    fn filter_package_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FilterPackageMember);
        self.expect(SyntaxKind::LBracket, "`[`");
        self.owned_expression();
        self.expect(SyntaxKind::RBracket, "`]`");
        self.finish_node();
    }

    /// What follows the `QualifiedName`, and therefore which import this is.
    fn import_suffix(&mut self) -> SyntaxKind {
        if !self.at(SyntaxKind::ColonColon) {
            return SyntaxKind::MembershipImport;
        }
        self.bump();
        if self.at(SyntaxKind::Star) {
            self.bump();
            self.recursive_suffix();
            return SyntaxKind::NamespaceImport;
        }
        if self.at(SyntaxKind::StarStar) {
            self.bump();
            return SyntaxKind::MembershipImport;
        }
        // A `::` that qualified_name left behind is followed by neither, so it ends
        // the name with nothing after it.
        self.error_expected("`*` or `**` after `::`");
        SyntaxKind::MembershipImport
    }

    /// `( '::' isRecursive ?= '**' )?`, the recursive suffix a `NamespaceImport` may carry.
    fn recursive_suffix(&mut self) {
        if self.at(SyntaxKind::ColonColon) {
            self.bump();
            self.expect(SyntaxKind::StarStar, "`**`");
        }
    }

    // production: QualifiedName
    //
    // QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME   (`KerML` 8.2.3.4.1)
    //
    // A `::` is only part of the name when a NAME follows it. `A::*` ends the name at
    // `A`, and the `::` belongs to the NamespaceImport — which is why this needs two
    // tokens of lookahead rather than consuming the separator and backing out.
    pub(super) fn qualified_name(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::QualifiedName);
        if self.at(SyntaxKind::Dollar) {
            self.bump();
            self.expect(SyntaxKind::ColonColon, "`::`");
        }
        self.expect_name("a name");
        while self.at(SyntaxKind::ColonColon) && self.name_follows_separator() {
            self.bump();
            self.expect_name("a name");
        }
        self.finish_node();
    }
}
