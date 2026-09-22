// REJECTED BY RULE, AS ADJUDICATED. A declared send writes `action` before its name.
// The clause's own SendNode line writes ActionUsageDeclaration?, which has no keyword and
// would read `snd send x;`; its three sibling declarations, the Pilot and the corpus all
// write ActionNodeUsageDeclaration? = 'action' UsageDeclaration?, and deviations.json records
// SendNode as conflict/follow_xtext (SYSML21-624). The corpus line the decision turns on is
// `action publish send new Publish(someTopic, somePublication) via publicationPort;`
// (examples/Interaction Sequencing Examples/ServerSequenceRealization-2.sysml:19). This
// file fails if the parser ever drifts to the literal reading.
// SysML 8.2.2.17.4 — SendNode = OccurrenceUsagePrefix ActionNodeUsageDeclaration? 'send' …
// rejects: PARSE-EXPECTED on: snd send x;
action def A {
	snd send x;
}
