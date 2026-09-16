# .claude/state

Machine-readable work state. Everything here is JSON so it can be validated, diffed
precisely, and read without prose parsing. Humans get `python3.11 .claude/scripts/state_report.py`.

`.claude/` is Claude Code's configuration namespace — settings, rules, agents, skills.
This subdirectory is deliberately separate: it holds _state of the work_, not
_configuration of the agent_. Nothing here is loaded automatically by Claude Code; the
SessionStart hook is what puts it in context.

| File                         | Owner   | Notes                                                                                                                                             |
| ---------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `state.json`                 | split   | `generated` by script, `authored` by you. Validated against `schema/state.schema.json`.                                                           |
| `deviations.json`            | **you** | Recorded grammar and metamodel deviations, with evidence. Structured so grammar_diff.py can tell reviewed from unreviewed without grepping prose. |
| `coverage.json`              | script  | Production coverage.                                                                                                                              |
| `grammar-diff.json`          | script  | Xtext inventory versus specification BNF.                                                                                                         |
| `decisions.json`             | script  | Index of `docs/adr/`, from their frontmatter.                                                                                                     |
| `grammar/productions.json`   | script  | Rule inventory derived from the pinned Xtext.                                                                                                     |
| `grammar/keywords.json`      | script  | Token set derived from the pinned Xtext.                                                                                                          |
| `grammar/metaclass-map.json` | script  | Production to metaclass, validated against the pinned XMI.                                                                                        |

Script-owned files are blocked by the PreToolUse hook. If one is wrong, the generator or
the pinned input is wrong — never the file.

Prose still lives in `docs/`: ADRs are arguments, and `docs/DERIVATION.md` is reasoning.
Neither becomes more useful as JSON. `docs/adr/` remains the source of truth for decisions;
`decisions.json` is only an index of it.
