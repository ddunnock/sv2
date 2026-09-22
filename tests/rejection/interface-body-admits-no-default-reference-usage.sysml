// REJECTED BY RULE. An interface body lists its non-occurrence usages, and the keywordless
// DefaultReferenceUsage is not among them, though a definition body admits one.
// SysML 8.2.2.14.1 — InterfaceNonOccurrenceUsageElement = ReferenceUsage | AttributeUsage
//                    | EnumerationUsage | BindingConnectorAsUsage | SuccessionAsUsage
// SysML 8.2.2.6.4  — NonOccurrenceUsageElement = DefaultReferenceUsage | ReferenceUsage | …
// rejects: PARSE-UNEXPECTED on: fuelFlowRate : Real;
interface def FuelInterface {
	end supplierPort : FuelOutPort;
	fuelFlowRate : Real;
}
