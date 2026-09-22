// REJECTED BY RULE. The parenthesised form of an interface's ends lists at least two.
// SysML 8.2.2.14.2 — NaryInterfacePart = '(' ownedRelationship += InterfaceEndMember ','
//                      ownedRelationship += InterfaceEndMember
//                      ( ',' ownedRelationship += InterfaceEndMember )* ')'
// rejects: PARSE-EXPECTED on: interface fuelLine : FuelInterface connect (tank.fuelPort);
part vehicle {
	interface fuelLine : FuelInterface connect (tank.fuelPort);
}
