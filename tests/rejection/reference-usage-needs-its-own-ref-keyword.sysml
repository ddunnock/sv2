// REJECTED BY RULE. ReferenceUsage's `ref` is the production's own keyword, not the
// optional one BasicUsagePrefix carries, so it cannot be written twice.
// SysML 8.2.2.6.2 — ReferenceUsage = ( EndUsagePrefix | RefPrefix ) 'ref' Usage
//                   BasicUsagePrefix = RefPrefix ( isReference ?= 'ref' )?
//                   RefPrefix = FeatureDirection? 'derived'?
//                               ( 'abstract' | 'variation' )? 'constant'?
// RefPrefix has no `ref` of its own, which is what makes `ref y;` one keyword and
// `ref ref y;` two. `ref attribute y;` parses, because there the first `ref` is
// BasicUsagePrefix's and `attribute` is the keyword.
package P { ref ref y; }
