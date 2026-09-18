// REJECTED BY ABSENCE, not by rule. A SubjectMember is a RequirementBodyItem and this
// text is well-formed SysML; SubjectMember is simply not implemented.
// SysML 8.2.2.21.1 — RequirementBodyItem = DefinitionBodyItem
//                                        | SubjectMember
//                                        | RequirementConstraintMember
//                                        | FramedConcernMember
//                                        | RequirementVerificationMember
//                                        | ActorMember
//                                        | StakeholderMember
// RequirementBodyItem is a superset of DefinitionBodyItem, which is why the body loop
// reads a requirement at all. These six are the part it does not reach. Removing this
// rejection means implementing SubjectMember, not widening anything here.
requirement def R {
	subject vehicle : Vehicle;
}
