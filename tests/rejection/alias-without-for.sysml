// Rule: 'for' is not optional.
// SysML 8.2.2.5.1 — AliasMember = MemberPrefix 'alias' ( '<' NAME '>' )?
//                     ( NAME )? 'for' memberElement = [QualifiedName] RelationshipBody
// Only the two name slots carry '?'; the keyword and the target do not.
alias Car Vehicle;
