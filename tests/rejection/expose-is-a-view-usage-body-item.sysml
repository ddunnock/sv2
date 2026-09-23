// REJECTED BY RULE. `expose` is an item of a view USAGE's body alone: ViewBodyItem names
// Expose and ViewDefinitionBodyItem does not, so a view definition says how its views
// render and a view usage what it exposes. The corpus exposes only in view usages:
// `expose vehicle;` in `view vehicleSafetyFeatureView` (validation/11-View and
// Viewpoint/11b-Safety and Security Feature Views.sysml:52).
// SysML 8.2.2.26.1 - ViewDefinitionBodyItem = DefinitionBodyItem | ElementFilterMember
//                                           | ViewRenderingMember
// SysML 8.2.2.26.2 - ViewBodyItem = DefinitionBodyItem | ElementFilterMember
//                                 | ViewRenderingMember | Expose
// rejects: PARSE-UNEXPECTED on: expose vehicle;
view def SafetyFeatureView {
	expose vehicle;
}
