// REJECTION BY ABSENCE, not by rule. EndUsagePrefix is the first alternative of
// OccurrenceUsagePrefix and is not implemented. The pinned corpus writes no
// `end part`, but the clause admits it, and connection ends do use `end`.
// SysML 8.2.2.9.2 — OccurrenceUsagePrefix =
//                     ( EndUsagePrefix | BasicUsagePrefix 'individual'? PortionKind? )
//                     UsageExtensionKeyword*
// SysML 8.2.2.6.2 — EndUsagePrefix = isEnd ?= 'end'
//                     ( ownedRelationship += OwnedCrossFeatureMember )?
// Remove this case when EndUsagePrefix lands.
end part p;
