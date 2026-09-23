// REJECTED BY RULE. `render` owns a ViewRenderingMember, an item of the two view bodies
// alone: ViewDefinitionBodyItem and ViewBodyItem name it, and DefinitionBodyItem, which
// a part definition's body holds, does not. The corpus renders only inside views:
// `render asTreeDiagram;` in `view def SafetyFeatureView` (validation/11-View and
// Viewpoint/11b-Safety and Security Feature Views.sysml:38).
// SysML 8.2.2.26.1 - ViewDefinitionBodyItem = DefinitionBodyItem | ElementFilterMember
//                                           | ViewRenderingMember
// SysML 8.2.2.6.1  - DefinitionBodyItem has no ViewRenderingMember alternative
// rejects: PARSE-UNEXPECTED on: render asTreeDiagram;
part def SafetyFeatureView {
	render asTreeDiagram;
}
