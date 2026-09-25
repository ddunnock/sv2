// REJECTED BY RULE. A user-defined keyword "is placed immediately before the
// language-defined (reserved) keyword for the declaration" (KerML 7.4.13, receipt
// 5e755297): after `abstract`, never before it.
// KerML 8.2.4.1.1 — TypePrefix = ( isAbstract ?= 'abstract' )?
//                                ( ownedRelationship += PrefixMetadataMember )*
// rejects: PARSE-UNEXPECTED on: #atom abstract classifier Bicycle;
#atom abstract classifier Bicycle;
