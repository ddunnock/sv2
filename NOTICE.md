# Notice and attribution

`sv2` is MIT licensed (see `LICENSE`). It describes two languages it did not invent
and is built from material it does not own. This file records what came from where.

Every pinned file carries its own `license` and `sha256` in `vendor/sources.lock.toml`,
which is the machine-readable form of this notice.

## OMG — SysML v2 and KerML specifications

The Systems Modeling Language (SysML®) version 2 and the Kernel Modeling Language
(KerML) are specifications of the Object Management Group. SysML is a registered
trademark of OMG. This project is an independent implementation and is neither
endorsed by nor affiliated with OMG.

The conformance target — the document identifiers, the namespace, and the revision
this project is built against — is `docs/conformance-target.toml`.

**The specification documents are not redistributed here.** No clause text, note or
table from a specification PDF is in this repository:

- The derivation reads the clauses from LLM wikis built locally from OMG's published
  PDFs. Neither the PDFs nor the wikis are in the repository.
- A derivation unit under `.claude/state/grammar/units/` records its clause by
  **citation** (`inputs.spec_clause_ref`) and by **sha256**
  (`fingerprint.spec_clause`). A hash is one-way: it detects drift and reproduces
  nothing.
- `.claude/scripts/check_derivation_text.py` runs on every gate and fails if a unit's
  authored prose reproduces a run of its clause. It is a check because discipline
  alone did not hold: an early unit pasted a clause note and a precedence table into
  its own notes, and only the check found it.

What **is** recorded is the result of reading those clauses — which productions
exist, what each rule's structure is, and why it was decided that way. Those are
facts about the languages and the authors' own words about them, not the
specification's text.

`docs/operator-precedence.toml` is the clearest case: SysML v2 and KerML place
operator precedence outside the grammar by design, so an implementation must carry it
separately. The file states which operator binds tighter than which and which way
each groups — the arrangement, as data — and cites its clause. It does not reproduce
the document's table, prose or formatting.

## OMG — machine-readable artifacts (`vendor/omg/`)

The normative XMI, JSON Schema, and model library files, published by OMG alongside the
specifications, pinned by sha256 and OMG File ID in `vendor/sources.lock.toml`.

## Eclipse Public License 2.0 (`vendor/pilot/`, `vendor/spec-bnf/`)

From the **SysML v2 Pilot Implementation** and the **SysML v2 Release** repositories
of the Systems-Modeling organization, both under the Eclipse Public License 2.0:

- `vendor/pilot/*.xtext` — the Pilot's Xtext grammars. This project treats them as a
  second opinion and never as the source (`docs/DERIVATION.md`). Each derivation unit
  quotes the Xtext rule for the production it derives, in `inputs.xtext_rule_text`;
  that quotation is EPL-2.0 material, redistributed under these terms.
- `vendor/spec-bnf/*.kebnf` — the textual BNF transcribed from the specifications,
  `// Manual corrections by HP de Koning`.

The licence text is vendored at `vendor/pilot/LICENSE-EPL-2.0.txt`. EPL-2.0 requires
that recipients be told where to obtain the source; `vendor/sources.lock.toml` records
the upstream URL and commit for every file.

## Example models (`vendor/corpus/`)

Example and training models from the SysML v2 Release repository, EPL-2.0, used as a
conformance corpus: files a conformant tool accepts. They are read, never modified.

## SIL Open Font License 1.1 (`app/src/assets/fonts/`)

**IBM Plex Sans** and **IBM Plex Mono**, copyright © 2017 IBM Corp., under the SIL
Open Font License 1.1. Five woff2 files, latin subset only, are redistributed here:
the three sans weights and two mono weights the interface uses. The OFL permits
bundling with software and asks that the copyright and licence notice travel with the
files, which is what this section and `app/src/assets/fonts/README.md` do. That README
records each file's upstream URL, version and sha256.

They are committed rather than fetched because the target environment is air-gapped. A
stylesheet that reaches a font CDN works on the workstation where it was written and
nowhere else.

Unlike everything above, these are **not** in `vendor/sources.lock.toml`. That file is
the ADR-0010 conformance tiers — material this project reads to decide what SysML v2
is — and a typeface is not evidence about the language.

IBM® and IBM Plex® are trademarks of International Business Machines Corp.

## Trademarks

SysML® and OMG® are trademarks or registered trademarks of the Object Management
Group, Inc. Used here for identification only.

## If something here is wrong

Open an issue. Attribution errors are corrected on report and a file whose license
cannot be honored is removed rather than argued about.
