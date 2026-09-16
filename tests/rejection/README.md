# tests/rejection

Inputs the parser must **reject**. `scripts/corpus-sweep.sh` parses every `.sysml`
and `.kerml` file here and fails if any of them is accepted.

This set is what makes the acceptance claim mean anything. A parser that accepts
everything passes a positive-only sweep, so the positive corpus alone proves
nothing about the grammar.

## The convention

- One file per rule violated, named for the rule and not for the syntax.
- A header comment states the rule in prose and cites the clause it comes from,
  verbatim where the production is short enough to quote.
- The file contains the smallest text that violates that one rule, so a failure
  names a rule rather than a paragraph.
- Add the negative case in the same change as the positive one
  (`.claude/rules/tests.md`).

## known-permissive/

`known-permissive/` holds cases the parser is recorded as accepting **on purpose**.
The sweep prunes that directory, so those files are neither caught nor counted as
leaked. An entry there needs a clause citation and a reason; it is a recorded
deviation, not a place to put a case that is merely inconvenient.

It does not exist yet, because nothing has needed it.
