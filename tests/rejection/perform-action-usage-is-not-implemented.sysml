// REJECTED BY ABSENCE. `perform` is its own production and is not implemented.
// SysML 8.2.2.17.2 — PerformActionUsage = OccurrenceUsagePrefix 'perform'
//                                         PerformActionUsageDeclaration ActionBody
//                    ActionUsage        = OccurrenceUsagePrefix 'action'
//                                         ActionUsageDeclaration ActionBody
// This file replaces action-usage-is-not-an-action-definition.sysml, which held both
// `action brake;` and `perform action stop;`. ActionUsage now parses, so that file went
// on rejecting for a reason its own comment no longer gave -- a rejection that passes
// for the wrong reason is worse than one that fails, because nothing reports it.
// PerformActionUsage is now the single largest first-error in the corpus's action files:
// 8 of the 41 that still fail, against 5 on `first`. It takes the ActionBody that
// already exists, so what it needs is PerformActionUsageDeclaration.
package P {
	perform action stop;
}
