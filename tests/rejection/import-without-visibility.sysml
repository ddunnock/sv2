// Rule: an import states its visibility.
// SysML 8.2.2.5.1 — Import = visibility = VisibilityIndicator 'import' ...
// The visibility carries no '( )?', unlike MemberPrefix one clause away.
// The Pilot's ImportPrefix fragment agrees, and all 741 imports in the
// pinned corpus carry one.
public package P { import A::*; }