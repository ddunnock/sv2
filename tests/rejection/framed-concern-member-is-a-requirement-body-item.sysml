// REJECTED BY RULE. A framed concern is framed by a requirement: FramedConcernMember is one
// of RequirementBodyItem's own alternatives, and neither DefinitionBodyItem nor CaseBodyItem
// has it.
// SysML 8.2.2.21.1 — RequirementBodyItem = DefinitionBodyItem | SubjectMember
//                      | RequirementConstraintMember | FramedConcernMember | ...
// SysML 8.2.2.6.1  — DefinitionBodyItem = DefinitionMember | VariantUsageMember | ...
// rejects: PARSE-UNEXPECTED on: frame brakingConcern;
part def Vehicle {
	frame brakingConcern;
}
