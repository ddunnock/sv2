// Clause: SysML 8.2.2.5.1, KerML 8.2.3.4.2 (.claude/state/grammar/units/Import.json,
// spec_clause_ref region_sha256 3ee603b1b89d4345 / a2610efd00be16cc) --
//
//   Import = visibility = VisibilityIndicator 'import' ( isImportAll ?= 'all' )?
//            ImportDeclaration RelationshipBody
//   ImportDeclaration = MembershipImport | NamespaceImport
//   MembershipImport  = importedMembership = [QualifiedName] ( '::' isRecursive ?= '**' )?
//
// So an ImportDeclaration must contain a QualifiedName, and a QualifiedName must contain
// at least one NAME (KerML 8.2.3.4.1). The text below supplies none: `public import ;`
// states its visibility, says `import`, and then names nothing at all. That is all the
// specification establishes here -- it says what an import must contain, and says nothing
// about what a tree should look like when the name is missing.
//
// Recovery (NOT a clause, and NOT an ADR): this parser OPENS the declaration's nodes
// before it looks for the name, and closes them empty when none arrives. The only source
// is the code -- `import_declaration` starts SyntaxKind::ImportDeclaration, `imported`
// takes a checkpoint and calls `qualified_name`, and `qualified_name` starts its own node
// and then calls `expect_name("a name")`, which REPORTS rather than consuming
// (crates/sv2-syntax/src/parser/import.rs). The result is an ImportDeclaration ->
// MembershipImport -> QualifiedName chain that is present and zero width.
//
// This is the case that matters for a typed accessor, and it is the opposite of a missing
// node: `declaration()` returns Some. A caller reading that as "the import names
// something" is confidently wrong, which is the failure ADR-0025 exists to catch and
// which neither a panic check nor an absence check sees. ADR-0025 sanctions pinning it:
// where the grammar admits one shape and the tree carries another, the case pins which.
// ADR-0004 supports only the shape -- a partial tree survives -- never this outcome.
//
// Do not cite 8.2.2.5.1 for the empty-but-present nodes, and do not cite ADR-0022, which
// is about deviation diagnostics and strict conformance and says nothing about recovery.
public import ;
