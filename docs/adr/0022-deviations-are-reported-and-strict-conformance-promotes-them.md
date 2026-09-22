---
title: "Deviations are reported as diagnostics, and strict conformance promotes them"
status: accepted
date: 2026-09-22
deciders: [David]
---

# Deviations are reported as diagnostics, and strict conformance promotes them

Builds on ADR-0010 and ADR-0011, which make the specification normative and the Pilot a
second opinion, and on ADR-0002, which says the IR admits what parses.

## Context and problem statement

`.claude/state/deviations.json` records every place the parser does not read the
specification's BNF literally. Most entries follow the specification. Twenty-one are
`conflict`/`follow_xtext`: the specification's own text is defective, the Pilot and the
corpus agree on what was meant, and the parser follows them. Among them are SendNode's
`action` keyword, `end` on occurrence usages, a flow declaration that may be omitted, and
a flow end's dot. A further case, a multiplicity after a connector end's reference, is
written only in a specification example (7.13.2).

Each decision is sound, and each is documented. What a user cannot do today is tell
whether a *file* depends on any of them. A model that must be exchanged with a tool
written from the specification alone has no way to find out which of its lines that tool
would reject, short of reading the register.

## Decision drivers

- One grammar. Two parsers, or a mode that takes different branches at every deviation
  site, would have to be tested against each other at every site, and the strict tree
  would lack exactly the elements a user most needs to see.
- Invariant 2 (ADR-0002): what parses enters the IR. A deviation is not an error in the
  default reading, and must not become one there.
- The corpus sweep and every test read `Parse::errors()`. Whatever carries the new
  information must not change what that returns.
- Traceability (invariant 4): a report about a deviation must name the register entry,
  so the reason is one lookup away.

## Considered options

1. **Deviation diagnostics.** When text parses only because of a recorded deviation, the
   parser attaches an informational diagnostic, `PARSE-DEVIATION`, naming the entry. A
   strict run promotes those to errors.
2. **A parse-mode switch.** `parse(source, language, mode)` takes the specification's
   branch at every deviation site in strict mode.
3. **The register alone.** Document, and flag nothing.

## Decision outcome

Option 1.

- The parser reads one grammar, the one the register adjudicates, and at each deviation
  site where the text uses what the deviation adds, it records a `PARSE-DEVIATION`
  diagnostic of severity `Info` whose message names the register entry.
- They are kept apart from errors: `Parse::errors()` is unchanged and
  `Parse::deviations()` returns them. A file with deviations and no errors parses.
- `Parse::is_spec_conformant()` is true when there are neither. `sv2 parse --strict`
  prints the deviations as errors and fails the file when there are any; without
  `--strict` the command is unchanged.
- Each site is marked `// deviation: <entry>`, and `scripts/check_deviation_sites.py`
  enforces two things in the gate. Every name a site uses must be an entry that departs
  from the specification (`follow_xtext`, `follow_corpus` or `follow_spec_example`). And
  every such entry whose production is marked implemented must have a site.
- `follow_spec_example` joins the register's decisions. It covers a choice that follows a
  normative example in the specification's prose against the same specification's BNF,
  when the corpus does not decide the point.

### Consequences

- Good: a user can ask of any file whether it is specification-conformant, and learn
  which lines are not and why, without a second grammar.
- Good: the register becomes checkable from the code's side as well. A departure with no
  site, or a site naming no departure, fails the gate.
- Bad: strict mode can only report EXTENSIONS. Some deviations RESTRICT the
  specification instead. DefaultReferenceUsage requires a non-empty declaration where the
  clause's letter admits a lone `;`, and SendNode reads `action NAME send` where the
  clause's literal line would read `NAME send`. Text the parser rejects has no tree to
  attach a note to, so those inputs stay rejected in both modes. The register's entries
  say which way each deviation runs.
- Bad: sites are hand-placed. The gate can check that a site EXISTS for each departure,
  but not that it fires on exactly the text the deviation admits. That is what each
  site's tests are for.

## More information

- `.claude/state/deviations.json` is the register; `.claude/state/schema/deviations.schema.json`
  gains `follow_spec_example`.
- `crates/sv2-syntax/src/diagnostic.rs` gains `DiagnosticCode::Deviation`.
