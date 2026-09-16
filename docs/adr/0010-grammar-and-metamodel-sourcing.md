---
title: "Grammar and metamodel sourcing"
status: accepted
date: 2026-09-15
deciders: [David]
---

# Grammar and metamodel sourcing

## Context and problem statement

The parser needs a pinned, verifiable definition of the language. Investigation of what
OMG and the Systems-Modeling organization actually publish produced an asymmetric result:
almost everything is available in normative machine-readable form except the one thing the
parser most needs.

Available as normative machine-readable artifacts at the `20250201` namespace, IPR mode
Non-Assert: the abstract syntax as MOF XMI (`SysML.xmi` ptc/25-02-15, `KerML.xmi`
ptc/25-04-04), JSON Schemas of the abstract syntax (`SysML.json` ptc/25-04-32,
`KerML.json` ptc/25-04-21), the KPAR project interchange schema
(`KerML-Model-Interchange.json` ptc/25-04-20), and ten model libraries as KPAR archives of
textual notation.

Not available in machine-readable form: the concrete textual syntax. The normative BNF
exists only inside the specification PDFs. The only machine-readable concrete syntax is
the Pilot Implementation's Xtext, which is EPL-2.0 and not normative.

> **Correction, 2026-09-16 (ADR-0011).** The sentence "The SysML-v2-Release repository
> contains no grammar at all" stood here and was false. That repository ships
> `bnf/*.kebnf`, a machine-readable transcription of the normative BNF carrying the
> specification's clause numbers. It is still not normative, so the decision below is
> unaffected; ADR-0011 adds it as Tier B′. The sentence is struck rather than quietly
> deleted because the decision was argued partly from it.

## Decision drivers

- Everything the parser asserts about the language must trace to a citable source
- Air-gapped and CUI environments: no network access at build or test time
- The Xtext encodes pilot parser limitations as if they were language rules
- A single source of truth cannot be cross-checked, and at least one known discrepancy
  exists between the normative XMI and the normative document

## Considered options

1. Single source — port the Xtext grammar and treat it as authoritative
2. Single source — derive everything from the specification PDFs via the wikis
3. Three-tier pin with a differential between the two concrete-syntax sources

## Decision outcome

Option 3.

**Tier A — OMG normative.** Fifteen artifacts at namespace-dated permalinks, pinned by
OMG File ID plus sha256. The URL namespace is the version; the hash guards against silent
republication at the same URL.

**Tier B — Pilot Xtext.** Three grammar files pinned by git sha, isolated in
`vendor/pilot/` with the EPL-2.0 notice intact. Used for the production inventory, the
token set, and the production-to-metaclass map. **Not** used for rule bodies.

**Tier C — corpus.** Release repo examples plus the informative Simple Vehicle Model
(ptc/25-04-31), pinned by release tag.

The tiers are tied together by `scripts/check_namespace_link.py`: the metamodel namespace imported
by the Xtext must equal the namespace of the pinned XMI. That is what detects the Pilot
drifting ahead of the specification, which no per-file hash would catch.

### Consequences

Good: every assertion traces to a File ID or a commit sha. The abstract syntax does not
have to be hand-written. The library bootstrap has a normative source and a normative
container format. Disagreements between the two concrete-syntax sources become a generated
report rather than a discovery made months later.

Bad: three pin mechanisms to maintain, and the differential produces permanent
disagreements that each require a recorded decision. That work is real, but it is work
that would otherwise happen invisibly and wrongly.

Note: at the grammar level `SysML.xtext` extends `KerMLExpressions`, not `KerML.xtext` —
they are siblings over a shared expression base. This is the opposite of the metamodel
relationship in ADR-0003, where SysML genuinely specializes KerML. Conflating the two would
produce a wrong crate decomposition.

## Pros and cons

**Option 1** — one source, immediately usable; inherits LL workarounds as if they were the
language, making the parser less conformant while appearing more so, and provides nothing
to cross-check against.

**Option 2** — normative and uncontaminated; PDF-bound, so extraction needs verification,
and it gives no signal about what real tools actually accept.

**Option 3** — traceable, cross-checkable, detects upstream drift; three pins and an
ongoing review obligation.
