---
title: "ADR-0021: Security markings are model metadata, and diagrams derive their banners"
status: "proposed"
date: 2026-09-22
version: "0.1.0"
decision-makers: David Dunnock
consulted: TBD
informed: TBD
---

# ADR-0021: Security markings are model metadata, and diagrams derive their banners

## Context and Problem Statement

Models built with this tool will carry classified or controlled content. Every diagram
rendered from them, and every image exported from that diagram, needs a banner giving the
highest marking of anything it shows, and a portion mark on each element it draws.

Marking by hand is where this goes wrong. A diagram is a projection (ADR-0001): add one
element to a view and its correct banner may change without anyone touching the view.
Nobody re-derives a banner by hand every time a view or a type changes. A tool that knows
each element's marking can. That is only possible if the marking is recorded on the
element, in a form the tool can evaluate.

The pinned standard libraries define no security-marking metadata. A search of all ten
20250201 KPARs for "classif", "security", "marking", and "clearance" finds only the word
"classifier". The language does supply the mechanism:

- **Metadata definitions and usages**, SysML clause 7.27.2. A metadata usage annotates
  its containing namespace when written inside that namespace's body, or the elements
  named after `about`.
- **Restricting what metadata can annotate**, also 7.27.2. A metadata definition subsets
  `Metaobject::annotatedElement` with a narrower reflective metaclass type. The clause's
  own example, `CommandMetadata`, restricts itself to `SysML::ActionDefinition` and
  `SysML::ActionUsage`. KerML's `validateMetadataFeatureAnnotatedElement` (8.3.4.12.3)
  enforces it.
- **User-defined keywords**, SysML clause 7.27.4. `#name` before a declaration applies
  the named metadata definition, and the short name works as the keyword.
- **Model-level evaluation of metadata values**, KerML 8.4.4.13.2 and
  `validateMetadataFeatureBody` (8.3.4.12.3). Metadata feature values are expressions
  a tool can evaluate statically, which is what lets a tool compute a banner.
- **Ranked enumerations**, as the pinned `RiskMetadata` library does it
  (`Metadata-Domain-Library.kpar`, ptc/25-04-28): `attribute def Level :> Real`,
  specialized by `enum def LevelEnum :> Level { low = 0.25; ... }`.

Marking schemes differ by program and by nation: which levels exist, their order, which
handling controls apply, and how controls combine. sv2 cannot own that knowledge, and
should not try.

## Decision Drivers

- **DD-1: Errors must go high, never low.** Over-marking a diagram is an annoyance.
  Under-marking it is a reportable incident. Every rule resolves an ambiguity upward.
- **DD-2: Text is authoritative (ADR-0001).** A marking lives in the model text next to
  what it marks, is reviewed in the same merge request, and cannot be lost by deleting a
  sidecar.
- **DD-3: Portability.** Another conformant tool opening the model must at least see
  the markings as ordinary metadata, even if it does not draw banners.
- **DD-4: The scheme belongs to the program.** Levels, order, controls, and banner text
  come from the program's marking guide, not from sv2.
- **DD-5: The resolver does not special-case library content (ADR-0019 R-7).**
  Interpreting markings is an extension's job.
- **DD-6: The tool assists; people decide.** sv2 derives markings from markings people
  wrote. It never decides a classification and never proposes lowering one.

## Considered Options

1. **Model metadata.** A small sv2-provided library defines the marking metadata. Each
   program defines its own ranked scheme as a library that specializes it. Diagrams
   derive banners and portion marks.
2. **Markings in a sidecar or in `sv2.toml`**, keyed by ADR-0016 element IDs.
3. **A convention in doc comments or names**, such as `doc /* (S) ... */`.
4. **A fixed built-in scheme** compiled into sv2.

## Decision Outcome

Chosen option: **Option 1, model metadata with a program-owned scheme.**

It is the only option in which the marking sits in the model text (DD-2), is visible to
other tools (DD-3), and can be evaluated rather than parsed out of prose. Splitting a
generic base library from a program scheme library keeps marking-guide knowledge out of
sv2 (DD-4).

Option 2 separates the marking from what it marks. A deleted or stale sidecar then
under-marks silently (DD-1, DD-2), and no other tool sees it (DD-3). Option 3 cannot be
evaluated and cannot be validated. Option 4 puts a marking guide into sv2 that sv2 cannot
maintain authoritatively (DD-4).

### The base library

sv2 provides `SecurityMarking`. The sketch below states the design; the library file
itself is written when the metadata productions are implemented (see Prerequisites).

```sysml
library package SecurityMarking {
    private import ScalarValues::Natural;

    /* Rank of a classification level. A higher value is more restrictive. */
    abstract attribute def Level :> Natural;

    /* A handling or dissemination control. Controls combine by union. */
    abstract attribute def Control;

    metadata def Marking {
        :> annotatedElement : SysML::Definition;
        :> annotatedElement : SysML::Usage;

        attribute level : Level[1];
        attribute controls : Control[0..*];
    }
}
```

The two `annotatedElement` subsettings restrict a marking to definitions and usages, by
the 7.27.2 `CommandMetadata` pattern. Views are usages, so a view is marked the same way
as anything else.

### A program scheme

This example is illustrative. The levels, their order, and every string come from the
program's marking guide, not from sv2.

```sysml
library package AcmeMarkings {
    private import SecurityMarking::*;

    enum def AcmeLevel :> Level {
        U = 0;
        CUI = 1;
        S = 2;
    }

    enum def AcmeControl :> Control {
        NOFORN;
    }

    metadata def <u>   Unclassified :> Marking { :>> level = AcmeLevel::U; }
    metadata def <cui> Controlled   :> Marking { :>> level = AcmeLevel::CUI; }
    metadata def <s>   Secret       :> Marking { :>> level = AcmeLevel::S; }
}
```

### Applying a marking

```sysml
// Keyword form (7.27.4): the level is bound by the metadata definition.
#cui part def Seeker;

// Body form (7.27.2): annotates the containing definition, and can carry controls.
part def Guidance {
    @Marking { level = AcmeLevel::S; controls = AcmeControl::NOFORN; }
}

// A view is a usage, so it is marked the same way.
#s view seekerContext { expose Seeker; expose Guidance; }
```

### Configuration

`sv2.toml` names the scheme and sets the repository's policy (ADR-0020 R-8):

```toml
[classification]
levels   = "AcmeMarkings::AcmeLevel"
controls = "AcmeMarkings::AcmeControl"
allowed  = ["U", "CUI"]      # levels this repository may hold
minimum  = "U"               # floor for every banner and portion mark
separator = "//"

[classification.text.U]
banner  = "UNCLASSIFIED"
portion = "U"

[classification.text.CUI]
banner  = "CUI"
portion = "CUI"

[classification.text.NOFORN]
banner  = "NOFORN"
portion = "NF"
```

Levels and their ranks live in the model, because they are what a marking means.
Banner and portion text live in `sv2.toml`, because they are how a marking is printed.

### Definitions

| Term | Meaning |
|---|---|
| Own marking | The `Marking` metadata annotating an element directly, in either form |
| Effective marking | The level and controls sv2 computes for an element under M-2 |
| Banner | The marking printed at the top and bottom of a rendered view or export |
| Portion mark | The short marking printed with each rendered element, such as `(S//NF)` |

### Normative rules

| ID | Rule |
|---|---|
| M-1 | An element's own marking is the highest level and the union of controls across every `Marking` metadata annotating it. More than one marking on an element is reported as `MARK-MULTIPLE`. |
| M-2 | An element's effective marking is the highest level, and the union of controls, across: the `minimum`, its own marking, and the effective markings of every type it is defined by, specializes, redefines, or subsets. These names appear in the element's label, so their marking travels with it. Cycles are the resolver's diagnostic, and the computation runs as a fixpoint over what remains. |
| M-3 | An element with no own marking takes the rest of M-2. It is reported as `MARK-UNMARKED` at the level `[lint]` sets, default `warn`. |
| M-4 | An own marking lower than the effective marking is reported as `MARK-BELOW-DERIVED`. The text states less than the element shows. |
| M-5 | A view's banner is the highest of its own marking, the `minimum`, and the effective marking of every element it renders, with the union of their controls. A view's own marking may raise its banner, for example when a compilation is itself more sensitive, and never lowers it. A view marked below its content is reported as `MARK-VIEW-UNDERMARKED`, and the banner uses the computed marking. |
| M-6 | A level that appears in an own or effective marking but not in `allowed` is reported as `MARK-NOT-ALLOWED`. A marking that names no literal of the configured scheme, a `minimum` outside `allowed`, and an `allowed` entry with no `text` are configuration errors. |
| M-7 | `MARK-VIEW-UNDERMARKED`, `MARK-NOT-ALLOWED`, and the configuration errors in M-6 are fixed at `deny` and cannot be changed by `[lint]` (ADR-0020 R-10). |
| M-8 | Every rendered view, and every image or document exported from one, carries the banner at top and bottom and a portion mark on every rendered element. No `sv2.toml` key, style file entry, or view setting can hide, shrink, or restyle them. A style entry that targets a marking is ignored and reported. |
| M-9 | sv2 never suggests lowering a marking. Code actions may offer to raise an own marking to its effective marking, and nothing else. |
| M-10 | The resolver evaluates metadata feature values generally (KerML 8.4.4.13.2) and does not know `SecurityMarking`. An extension crate, `sv2-marking`, interprets it and owns the `MARK-` diagnostic prefix (ADR-0019 R-5, R-7). |
| M-11 | `SecurityMarking` is bundled with sv2 so that it is available offline. It is also published as a KPAR to a sysand index, so tools other than sv2 resolve the same definitions (ADR-0020). Program schemes are ordinary libraries distributed through sysand. |

M-2 and M-5 decide what a banner means: the highest marking of anything shown, including
anything a label names. M-8 is what makes the banner trustworthy. A banner that a style
can hide is only a suggestion.

### Prerequisites

This decision cannot be implemented until the following exist:

- **Parser productions.** Not one of these is implemented yet: `MetadataDefinition`,
  `MetadataUsage`, `MetadataUsageDeclaration`, `MetadataBody`, `PrefixMetadataMember`,
  `PrefixMetadataUsage`, `DefinitionPrefix`, and `UsagePrefix`.
- **Resolver support** for model-level evaluation of metadata feature values, and for
  `annotatedElement` restrictions (`validateMetadataFeatureAnnotatedElement`).
- **ADR-0019**, which is still proposed. It defines where `sv2-marking` sits.

### Consequences

- Good, because a banner is derived, so it follows every edit to a view or a type without
  anyone re-marking it (DD-1).
- Good, because markings are reviewed in the same merge request as the content they mark
  (DD-2).
- Good, because other conformant tools see the markings as ordinary metadata (DD-3).
- Good, because `allowed` catches content above what the repository may hold while it is
  being written, which a manually marked export does not.
- Good, because sv2 contains no marking guide and nothing sensitive (DD-4).
- Bad, because M-2 over-marks. An element that names a higher type in its label takes
  that type's level even when a person would judge otherwise. That is the intended
  direction of error (DD-1), but it will produce banners that need explaining.
- Bad, because "highest level plus union of controls" is simpler than real marking-guide
  combination rules. A scheme whose controls do not combine by union needs a later
  revision of this record.
- Bad, because nothing here marks `.sysml` files themselves, generated reports, or the
  diagnostics sv2 prints. See OI-3.

### Confirmation

| ID | Fitness function | Method |
|---|---|---|
| FIT-1 | Effective marking rises and never falls (M-2) | Fixtures: an unmarked usage of a marked definition, a specialization of a marked type, and a redefinition. Expected markings are worked out by hand from M-2, never from sv2's output. |
| FIT-2 | Banners derive and cannot be lowered (M-5) | Fixtures: a view below its content (`MARK-VIEW-UNDERMARKED`, computed banner), a view above its content (own marking wins), and a view whose content changes (banner changes with no edit to the view). |
| FIT-3 | The ceiling holds (M-6, M-7) | Negative fixtures: a level outside `allowed`, and a `[lint]` entry trying to lower `MARK-NOT-ALLOWED`. Both are rejected. |
| FIT-4 | Rendering cannot hide markings (M-8) | Snapshot tests of exported SVG assert the banner at top and bottom and a portion mark on every element, including with a style file that targets the banner. |
| FIT-5 | The resolver stays library-neutral (M-10) | ADR-0019 FIT-4: no `SecurityMarking` qualified name appears in `sv2-resolve`. |
| FIT-6 | Restriction to definitions and usages (base library) | A negative fixture puts `@Marking` on a package and expects the `validateMetadataFeatureAnnotatedElement` diagnostic. |

## Pros and Cons of the Options

### Option 1: Model metadata with a program-owned scheme

- Good, because it uses only standard language mechanisms (7.27.2, 7.27.4).
- Good, because it is evaluable, reviewable, and portable.
- Bad, because it waits on parser and resolver work.

### Option 2: Sidecar or sv2.toml

- Good, because it could be built before the metadata productions exist.
- Bad, because losing the sidecar under-marks silently (DD-1).
- Bad, because no other tool sees it (DD-3).

### Option 3: Doc-comment or naming convention

- Good, because it needs no tooling to write.
- Bad, because prose is not evaluable, and a typo under-marks silently.

### Option 4: Fixed built-in scheme

- Good, because it needs no configuration.
- Bad, because schemes differ by program and nation, and sv2 would encode a marking guide
  it has no authority over (DD-4).

## More Information

### Risks

| ID | Risk | Handling |
|---|---|---|
| RISK-0021-1 | Users read a derived banner as a classification decision | Documentation and the M-9 rule. sv2 derives from what people marked, and the text says so wherever banners are explained. |
| RISK-0021-2 | The keyword form is judged invalid, because 7.27.2 says that a metadata usage whose definition has features "must" have a body | 7.27.4 shows `#situation` used with no body, where `SituationMetadata` binds its only feature in the definition. That suggests features bound in the definition do not need a body. Confirm before implementing (OI-1). If the reading fails, only the body form is standard, and the keyword form is dropped. |
| RISK-0021-3 | A scheme needs controls that do not combine by union | A review trigger. The rule would move into the scheme library as data. |

### Open Items

- **OI-1.** Settle RISK-0021-2 against the clause text and the pilot corpus. Record the
  outcome in `.claude/state/deviations.json` if the pilot and the specification disagree.
- **OI-2.** Whether packages may be marked. For now they may not. A package drawn on a
  diagram takes the highest marking of what it shows inside it.
- **OI-3.** Marking outputs other than diagrams: text exports, reports, and printed
  diagnostics that quote marked content.
- **OI-4.** Where the banner's visual form comes from, such as colors per level, and how
  it stays out of the style cascade while still being configurable per scheme.

### Review Triggers

- A program's marking guide requires combination rules beyond highest level and union of
  controls.
- A standard or widely adopted library for security markings appears for SysML v2. This
  record should then adopt it and retire `SecurityMarking`.

### Related Decisions

- [ADR-0001](0001-text-is-authoritative.md): markings live in the text (DD-2).
- [ADR-0002](0002-ir-admits-what-parses.md): a marking error is a diagnostic, and the
  element still enters the IR.
- [ADR-0006](0006-view-membership-in-model.md): views are model elements, which is why a
  view is marked like any other usage.
- [ADR-0017](0017-view-scoped-json-lines-sidecar-for-diagram-layout-and-styling.md): the
  style files that M-8 overrides.
- [ADR-0019](0019-extensions-are-in-tree-consumers-of-the-resolved-model.md): the
  extension model that `sv2-marking` follows.
- [ADR-0020](0020-sysand-is-the-package-manager-and-sv2-toml-configures-the-tool.md):
  `sv2.toml` and distribution through sysand.
