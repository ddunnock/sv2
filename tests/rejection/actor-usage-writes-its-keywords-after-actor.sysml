// REJECTED BY RULE. An actor's user-defined keywords come AFTER the `actor` keyword, where
// every other usage writes them before its kind keyword: ActorUsage has no UsagePrefix.
// SysML 8.2.2.21.1 — ActorMember = MemberPrefix ownedRelatedElement += ActorUsage
//                    ActorUsage = 'actor' UsageExtensionKeyword* Usage
// A case body has no item that opens on `#` and then `actor`, so its result-expression
// reader reports the `#`.
// rejects: PARSE-EXPECTED on: #Human actor driver;
case def Drive {
	#Human actor driver;
}
