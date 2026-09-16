# Local grammar deviations

Every difference between the vendored grammar and its upstream, and every place this
parser knowingly departs from the specification.

An empty file here is a claim: that the vendored grammar is byte-identical to upstream
and that the parser follows the spec everywhere. Prior implementations in this space have
needed roughly ten entries, so an empty file after real work is more likely an
undocumented patch than a clean one.

## Source selection

The structured record is `.claude/state/deviations.json` under `source_selection`, which
is what `scripts/grammar_diff.py` and `spec-conformance-reviewer` read. This is the prose
summary; if the two disagree, the JSON is the one that is checked.

Four pinned sources, three tiers plus an amendment (ADR-0010, ADR-0011):

- **Tier A** — OMG normative XMI, JSON schemas, and ten KPAR model libraries at namespace
  `20250201`, pinned per file by sha256 and OMG File ID. Normative for the abstract
  syntax. Known defect going in: an open OMG issue reports `KerML.xmi` retaining
  `Connectors` and `Interactions` associations that the specification document does not
  have, so the two normative artifacts disagree in at least one place.
- **Tier B** — Pilot Implementation Xtext at commit `5cca16d` (bundle 0.63.0), EPL-2.0,
  not normative. Used for the production inventory, the token set, and the metaclass map.
  Never for rule bodies: it encodes its parser generator's LL limitations as language
  rules, and porting one makes the parser less conformant while making it look more so.
- **Tier B′** — `bnf/*.kebnf` from SysML-v2-Release at commit `fb97b75` (tag `2026-08`),
  not normative. A transcription of the specification BNF that carries clause numbers.
  Used for draft rule bodies and the spec-side inventory. Known defect going in: its own
  first line reads `// Manual corrections by HP de Koning`, so a transcription error
  reads exactly like a language rule. The PDF clause arbitrates.
- **Tier C** — corpus, at commit `fb97b75` (tag `2026-08`): 310 model files from
  `kerml/src` and `sysml/src`, plus the informative Simple Vehicle Model
  (`ptc/25-04-31`). `sysml/src/validation` is a scenario set, not a negative set, so
  its 56 files are positive cases. This pin makes the corpus traceable and available
  offline; it does **not** make acceptance meaningful. `corpus-sweep.sh` needs a
  parser that accepts something and a hand-written `tests/rejection/`, and stays
  inert until both exist.

Considered and declined: the community ANTLR4 ports, which are known to carry defects
that reject the specification's own examples.

## Deviations

The register is `.claude/state/deviations.json`, which holds all 281 reviewed grammar
differences — that is the checked artefact and `scripts/grammar_diff.py` reads it. This
section records only the ones that are defects in a **normative document**, because those
are the entries a reader would otherwise assume could not exist.

### Three specification defects, all silently repaired by the Tier B′ transcription

Each was found by comparing the transcription against the clause text retrieved from the
LLM wiki, which carries a `region_sha256` receipt back to the PDF span. In every case the
transcription is _right_ about what the language means and _silent_ about the fact that the
specification says otherwise — which is precisely the failure mode recorded for Tier B′
when it was pinned.

| Where                      | The specification says                                                                       | Should be                         | OMG issue                                                                      |
| -------------------------- | -------------------------------------------------------------------------------------------- | --------------------------------- | ------------------------------------------------------------------------------ |
| SysML 8.2.2.16, PDF p.210  | defines `FlowFeatureRefefinition` while referencing `FlowFeatureRedefinition` one line above | `FlowFeatureRedefinition`         | [SYSML21-399](https://issues.omg.org/issues/SYSML21-399), open — **partial**   |
| KerML 8.2.4.1.2, PDF p.112 | `SpecificType : Specialization :` — a colon where `=` belongs                                | `SpecificType : Specialization =` | [KERML11-108](https://issues.omg.org/issues/KERML11-108), open — exact, item 2 |
| SysML 8.2.2.16, PDF p.210  | `FlowDefinition :` with no `=`                                                               | `FlowDefinition =`                | [SYSML21-402](https://issues.omg.org/issues/SYSML21-402), open — exact, item 7 |

Read literally, the first leaves a dangling reference — nothing defines
`FlowFeatureRedefinition` and nothing references `FlowFeatureRefefinition`. This parser
follows the reference spelling in each case, because the reference is the load-bearing
occurrence: `FlowFeature`'s body cites it, and honouring the definition's spelling would
leave it pointing at nothing. KerML 8.2.5.9.2 spells the production correctly in both
positions, which settles what was intended.

All three are already on file upstream, all open, all filed by Kent Johnson (Emerson) in
late October 2025. Two caveats that matter:

- **SYSML21-399 is a partial match.** It reports `Refefinition` as a spelling typo at page
  179, never names the production, and does not record the dangling reference the typo
  creates. That consequence is unreported.
- The two defects in clause 8.2.2.16 are on file as **two separate issues** — SYSML21-402
  does not list the misspelling and SYSML21-399 does not list `FlowDefinition`.

Also worth knowing: SYSML21-402 independently lists `StateSendActionUsage : SendActionUsage`
as missing its `=`, which is exactly why the automated grounding pass could not index that
production. Upstream confirms the cause.

## Unreported upstream

`terminal WS` omitting form feed (below) was searched for across all 1304 SysML 2.0 issues,
all 409 KerML 1.0 issues including closed ones, the 131 unclassified issues, and the two
Systems-Modeling GitHub repositories. **No issue exists.** It appears to be genuinely
unreported, and we have not filed it either.

### One conformance gap in the Pilot

`terminal WS: (' ' | '\t' | '\r' | '\n')+` omits **form feed**, which KerML 8.2.2.1 lists:
`WHITE_SPACE = space | tab | form_feed | LINE_TERMINATOR`. A file separating tokens with a
form feed is well-formed by the specification and a lexical error for the Pilot. This
parser follows the specification. It needs a positive test written by hand, because no
corpus file is likely to contain a form feed and the sweep will stay silent either way.

## Known-unpatched deviations

<!--
Places where upstream is wrong and we have chosen NOT to patch, with the reason.
These matter as much as the patches — they are what a reader would otherwise assume
had been handled.
-->

_None yet._
