---
paths:
  - "crates/sv2-ast/**"
---

# sv2-ast — typed accessors over the CST

This crate is a view. It owns no data, and every rule below follows from that.

## Owns no data

An accessor type wraps exactly one `SyntaxNode` and nothing else. It returns nodes and
tokens; the caller asks those for text. No `String`, `Box<str>`, `Vec<String>` field, and
no public function returning `String`. `scripts/check_ast_leaks.py` fails on each.

ADR-0004 justifies the lossless tree partly because this crate makes it affordable.
Copying text out is how that stops being true.

## The marker, and the six states

Every accessor carries `// node: Kind`, **in `src/`**. `scripts/ast_coverage.py` counts
markers under `src/` only — a marker in a test file counts for nothing, because it would
read as implemented with no accessor shipped, and `check_ast_leaks.py` inspects `src/`
alone. The counted scope and the inspected scope are the same one on purpose.

Counted against `.claude/state/node-kinds.json`:

- **implemented** — a marker claims a kind. Counted.
- **unimplemented** — a kind no marker claims. Tracked, never a failure.
- **absent** — a marker claims a kind that does not exist. A defect: the kind is added to
  the parser's `NODES` and regenerated, or the marker is wrong.
- **unproved** — an implemented kind with no non-empty malformed case. A defect.
- **unswept** — an implemented kind whose `assert_<kind>_accessor` the sweep does not both
  define and call, so no tree ever reaches it. A defect.
- **stray** — a malformed case naming no kind. A defect.

A large unimplemented count is the honest state of this crate, not a problem to fix. The
four defect states are the ones that mean something is wrong.

## A malformed case per accessor, in the same change

Three things, because absence is not the only way to be wrong:

1. the parse still yields a tree — it always does, and the accessor sees a partial one;
2. the accessor reports **absence** rather than guessing — and that something *reported*
   it: read `Parse::errors()`, because absence nothing reported is absence guessed;
3. the neighbouring accessors still answer. This is dischargeable now that the crate has
   more than one accessor, so discharge it: ask a different accessor about the same
   damaged tree and assert it answers correctly. `Import`'s case asks `RootNamespace`
   whether the broken import is still its one member. Where a fixture gives the
   neighbour nothing to answer — the root-namespace case writes no import — say that in
   the test and assert the only thing left, which is that the neighbour does not invent
   one. A `kind()` check on the accessor under test never stands in for this.

Where the grammar admits one child and the tree carries two, the case pins **which** is
chosen. `.claude/rules/tests.md` governs where expectations may come from; read it.

A recovery outcome is not a spec rule, and it is usually not an ADR either. The
specification says a package body closes with `}`; it says nothing about what this parser
does when one does not. For a recovery outcome the source is **the parser** — name the
function and the file. ADR-0004 supports only the shape (an error node is just another
node, so a partial tree survives), never a particular outcome.

Do not reach for ADR-0022. It is about deviation diagnostics and strict conformance and
says nothing about recovery; `crates/sv2-syntax/src/parser/recovery.rs` mentions both in
one sentence, which is where that mistake comes from. It has been made twice here.

### The case is a directory named for its kind

`crates/sv2-ast/tests/malformed/<kind-in-kebab>/<condition>.sysml` (or `.kerml`).
`RootNamespace` is proved by `root-namespace/unclosed-body.sysml`.

The kind is the directory, not a filename prefix, because a filename cannot carry both
unambiguously: `-` separates the words inside a kebab kind *and* would separate kind from
condition, and 91 kinds have a kebab form that prefixes another's. `package-body-missing`
reads equally as `Package` + "body missing" and as `PackageBody` + "missing", and no rule
can tell them apart. A directory keeps the two in separate namespaces.

`scripts/ast_coverage.py` fails on an implemented kind with no non-empty case (`unproved`)
and on a case that names no kind (`stray`) — a misspelled directory, or a file left at the
top level, which would otherwise stop counting in silence.

The gate checks the case *exists* and is not empty. It cannot check that it asserts the
right things, so review does.

## An accessor answers for its own production, and delegates for the rest

When a child has a node kind of its own, the accessor returns the **typed view** of it,
not a raw `SyntaxNode`, and does not reach past it:

```rust
pub fn prefix(&self) -> Option<MemberPrefix> { ... }   // not Option<SyntaxNode>
// the caller then asks the prefix its own question:
member.prefix().and_then(|prefix| prefix.visibility())
```

`PackageMember` and `AliasMember` both carry a `MemberPrefix`, and the visibility lives
inside it. An accessor on either that returned the visibility directly would be answering
for a node it is not a view of — which is the same move as `declaration()` returning a
grandchild, a defect caught by mutation on the `Import` round. Reaching across a
production boundary is how two accessors start disagreeing about one fact.

The consequence is an ordering one: **a shared child is worth taking out of worklist
order.** `MemberPrefix` went ahead of its position because three productions were about
to reach into it. Say why in the commit when you reorder; the worklist is advisory
(ADR-0025) and this is a good reason to leave it.

When the child production has *no* node of its own — `DefinitionElement` and
`UsageElement` are alternations that build nothing — there is nothing to delegate to, and
the accessor returns the concrete element it finds. Say so, because the next reader will
wonder why this one is untyped.

There is a third case, and it is not the second one: the child has a node kind of its
own, but **which** kind is not fixed. `ImportDeclaration` is `MembershipImport |
NamespaceImport`, and this parser builds a node for whichever matched — at a checkpoint,
in `imported`, which is why reading the production alone suggests no node and the tree
says otherwise. Both alternatives are counted kinds, so the rule above asks for a typed
view, and `AstNode` cannot give one: it declares one `KIND` per view, so no signature
admits both. The accessor returns a raw `SyntaxNode` and the returned `kind()` is the
discriminator. An enum over the alternatives would be a new abstraction invented before
any of them has an accessor, which is the same wall the expression layer hit
(`ElementFilterMember::filter`, decision `ast-expression-child`).

So: **measure the tree before deciding which case you are in.** The production text does
not tell you whether a node is built, and all three cases return something different.

### The delegation ripple is part of the change

A new view obliges every shipped accessor that returned that kind raw to be upgraded in
the same commit. This is not optional tidying: two accessors answering the same question
in different types is how callers start depending on the weaker one. Measured so far —
`QualifiedName` cost 3 call sites, `VisibilityIndicator` 7 (two parents, 821 instances),
`ImportDeclaration` 4.

Two consequences worth knowing before you start. `tests/sweep.rs`'s per-kind assertion
functions end up mixing typed and raw children, so each says in one line which are which;
and they outgrow clippy's 60-line limit, which is why several are split into a
`assert_<kind>_accessor` entry plus a helper. Both are expected, not signs of a wrong
turn.

A tautology appears too: once an accessor is typed, `assert_eq!(child.kind(), K)` is
guaranteed by `cast` and proves nothing. Replace it with what the view cannot guarantee —
that the child it wraps is there, or which alternative it is — rather than deleting it.

## The sweep is where an accessor is actually exercised

`tests/sweep.rs` walks every tree in the repository — 311 corpus files and 299 rejection
files. An accessor that is not called from there is not swept at all, and the suite stays
green because it has nothing to check, which is the failure ADR-0025 exists to prevent.

`ast_coverage.py` now catches that as **unswept**: a kind counts as swept only when
`tests/sweep.rs` both defines `assert_<kind_in_snake>_accessor` and calls it. Defining it
and forgetting the call fails the gate, which is the whole point — that was the shape of
the hole. Line comments are stripped before the scan, so a doc comment mentioning the call
does not satisfy it.

What the check cannot do is judge the assertions. It proves a tree reaches your accessor;
it says nothing about whether anything would fail if the accessor were wrong. That is
still on you, and the mutation step below is how you discharge it.

Each kind gets an `assert_<kind>_accessor` asserting:

- the cast contract — `cast` succeeds exactly when `kind()` matches, and `syntax()` hands
  back the node it was cast from;
- the differential — re-derive the same fact by walking the tree directly, then assert
  **equality**, not containment.

Equality, because a subset check reads like a differential and is not one. Measured on the
first accessor against six plausible `members()` defects, `children.contains(member)`
caught one and equality caught all six: containment passes an accessor that returns
nothing, one that reverses the order, and one that silently filters a kind out.

An accessor that legitimately filters — most will, being a view of one child — filters in
the re-derivation too, with a comment saying why. That is the point: the equality forces
the decision to be written down instead of made by accident. What is never acceptable is
relaxing equality to containment to make a filtering accessor pass.

Then mutate the accessor and watch the sweep go red before you trust it. An assertion
nobody has seen fail is a guess.

## What this crate does not do

It raises `ID-MALFORMED` and `ID-ORPHAN`, because attaching an `@id` note to its
declaration needs nothing beyond the current file. It does **not** allocate an identifier,
detect a duplicate, resolve a name, or construct a metaclass. Those need more than one
file, or need the metamodel, and writing one here is how the layering quietly dies.

## Test and development code

Unit tests at the bottom of the file they test, in `#[cfg(test)] mod tests` — the only
place a test reaches a private item, and compiled out of the shipped crate. Everything
else lives outside `src/`: shared support in `tests/trees/`, data in `tests/malformed/`,
tooling in `examples/`. `utils`, `helpers`, `common`, `misc`, `manager` and `handler` are
banned names anywhere under `crates/`, test files included.
