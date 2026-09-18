# Vendored fonts

IBM Plex Sans and IBM Plex Mono, the two families the Model Workbench mockup uses.
They are committed rather than fetched because the target environment is air-gapped
(STD-004-TS §3.2) and because the test preload replaces `fetch` with a function that
throws. A stylesheet that reaches Google Fonts works on the workstation where it was
written and nowhere else.

Only the **latin** subset is here, at the three sans weights and two mono weights the
design uses. 108 KB for all five. The rest of IBM Plex — the other scripts, the
italics, the weights nothing references — is load cost for nothing, which is the same
argument `allowed-dependencies.toml` makes about packages.

## What these are, and where they came from

| File | Family | Weight | Upstream version | sha256 |
|---|---|---|---|---|
| `ibm-plex-sans-latin-400-normal.woff2` | IBM Plex Sans | 400 Regular | 3.201 | `3b646991d30055a93a4ecc499713d4347953a74a947ecab435ab72070cbdab0e` |
| `ibm-plex-sans-latin-500-normal.woff2` | IBM Plex Sans | 500 Medium | 3.201 | `0717336fb31fcdcde4b8deb3675bb4a0f7f6d484864afcd6751ac29975962203` |
| `ibm-plex-sans-latin-600-normal.woff2` | IBM Plex Sans | 600 SemiBold | 3.201 | `8960851d691c054ed38e259bdcf1a6190d157b4203ed5bb32c632a863fb8ec2f` |
| `ibm-plex-mono-latin-400-normal.woff2` | IBM Plex Mono | 400 Regular | 2.3 | `08949f728dc52d528e69b1667d15c89a5686a4ee9a296ff90983985f99c380f7` |
| `ibm-plex-mono-latin-500-normal.woff2` | IBM Plex Mono | 500 Medium | 2.3 | `01d285447409c8a588692162439a038b8cbd7871309ee20267b0d2d91c6e8e22` |

Fetched from the `@fontsource` redistribution on jsDelivr, which publishes the
subsetted woff2 builds:

```
https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-sans/files/ibm-plex-sans-latin-<weight>-normal.woff2
https://cdn.jsdelivr.net/npm/@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-<weight>-normal.woff2
```

`@fontsource` is **not** a dependency and is not on the allowlist. These are bytes
that were fetched once and committed; nothing in `package.json` refers to the
package, and no build step retrieves them.

The family, subfamily and version above were read out of each file's `name` table
after Brotli decompression, not taken from the filename — a filename is a claim and a
name table is the font.

## What is and is not checked

The sha256 column records **upstream provenance**: which bytes arrived from that URL,
so a later re-fetch can be compared against what was reviewed. It is not an integrity
check on this repository, and no gate step verifies it. Git already hashes these
files, so drift inside the repository shows up as a diff; what git cannot tell you is
whether the bytes ever matched upstream, and that is what the table is for.

They are deliberately **not** in `vendor/sources.lock.toml`. That file is the
ADR-0010 conformance tiers — OMG normative artifacts, the Pilot's Xtext, the
specification BNF, and the corpus — and `NOTICE.md` calls it "the machine-readable
form of this notice" for material this project reads to decide what SysML v2 *is*. A
typeface is not evidence about the language.

## Licence

SIL Open Font License 1.1. Copyright © 2017 IBM Corp. The OFL permits bundling and
redistribution with software; it requires the copyright and licence notice to travel
with the files, which is what this file and the entry in `NOTICE.md` do. IBM Plex is
a trademark of IBM Corp.

Upstream: <https://github.com/IBM/plex>
