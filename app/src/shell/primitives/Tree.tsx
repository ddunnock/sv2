// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A tree of rows (WAI-ARIA APG, "Tree View"): the navigator in both its modes.
 *
 * THE MOCKUP'S TREE IS A FLAT LIST OF BUTTONS WITH PADDING FOR INDENTS, so a
 * screen reader hears a column of unrelated buttons with no levels, no
 * expanded state and no way to collapse. This is a tree: `role="tree"` over
 * `treeitem`s that state their level, position and expansion, one tab stop,
 * and the APG keyboard from `tree-rows.ts`.
 *
 * FOCUS IS NOT SELECTION. Arrow keys move focus; Enter, Space or a click
 * selects, and on a parent also opens or closes it — one action for all three,
 * so the keyboard and the mouse cannot disagree. Selecting fetches the
 * sidebar's answer for a node, so it happens when asked for, not for every row
 * the keyboard passes over.
 *
 * CONTROLLED. `expanded` and `selected` belong to the caller, because both
 * persist beyond the tree (the Files tree's open folders survive a reload of
 * the workspace) and a tree holding its own copy would disagree with them.
 */

import { type KeyboardEvent, useRef, useState } from "react";

import { Icon } from "./Icon";
import { type TreeNode, type TreeRow, treeMove, visibleRows } from "./tree-rows";

/** Props for `Tree`. */
export type TreeProps = Readonly<{
  /** Names the tree: "Files", "Elements". */
  label: string;
  nodes: readonly TreeNode[];
  expanded: ReadonlySet<string>;
  onToggle: (id: string) => void;
  selected: string | null;
  onSelect: (id: string) => void;
}>;

/** A tree with one tab stop and the APG keyboard. */
export function Tree(props: TreeProps): React.JSX.Element {
  const { label, nodes, expanded, selected } = props;
  const rows = visibleRows(nodes, expanded);
  const container = useRef<HTMLDivElement>(null);
  const [focusedId, setFocusedId] = useState<string | null>(null);
  // The tab stop: the focused row if it is still visible, else the selection, else the first row.
  const current = Math.max(
    rows.findIndex((row) => row.node.id === (focusedId ?? selected)),
    0,
  );

  const focusRow = (index: number): void => {
    const row = rows[index];
    if (row !== undefined) {
      setFocusedId(row.node.id);
      container.current?.querySelectorAll<HTMLElement>('[role="treeitem"]')[index]?.focus();
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    const move = treeMove(event.key, { rows, current, expanded });
    if (move.kind === "none") {
      return;
    }
    event.preventDefault();
    if (move.kind === "focus") {
      focusRow(move.index);
    } else {
      props.onToggle(move.id);
    }
  };

  return (
    <div
      ref={container}
      role="tree"
      aria-label={label}
      onKeyDown={onKeyDown}
      className="flex flex-col py-1 text-ui"
    >
      {rows.map((row, index) => (
        <Row
          key={row.node.id}
          row={row}
          expanded={expanded.has(row.node.id)}
          selected={row.node.id === selected}
          tabStop={index === current}
          onPress={() => {
            setFocusedId(row.node.id);
            props.onSelect(row.node.id);
            if (row.node.children !== null) {
              props.onToggle(row.node.id);
            }
          }}
        />
      ))}
    </div>
  );
}

type RowProps = Readonly<{
  row: TreeRow;
  expanded: boolean;
  selected: boolean;
  tabStop: boolean;
  onPress: () => void;
}>;

/** One `treeitem`. Its accessible name is the label, then the description. */
function Row({ row, expanded, selected, tabStop, onPress }: RowProps): React.JSX.Element {
  const { node } = row;
  const isParent = node.children !== null;
  return (
    <div
      role="treeitem"
      aria-level={row.level}
      aria-setsize={row.setsize}
      aria-posinset={row.posinset}
      aria-expanded={isParent ? expanded : undefined}
      aria-selected={selected}
      aria-label={
        node.description === undefined ? node.label : `${node.label}, ${node.description}`
      }
      tabIndex={tabStop ? 0 : -1}
      onClick={onPress}
      onKeyDown={(event) => {
        // Enter and Space do what a click does. Handled on the row, so the row
        // that has focus is the one that acts; the tree handles the arrows.
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onPress();
        }
      }}
      style={{ paddingInlineStart: `calc(var(--spacing) * ${1 + (row.level - 1) * 3})` }}
      className="flex h-6 cursor-default items-center gap-1 pr-2 text-fg outline-none hover:bg-hover focus-visible:ring-1 focus-visible:ring-accent aria-selected:bg-accent-bg"
    >
      <span className="inline-flex size-4 shrink-0 items-center justify-center text-faint">
        {isParent ? <Icon name={expanded ? "chevron-down" : "chevron-right"} /> : null}
      </span>
      <span className="min-w-0 truncate">{node.label}</span>
      {node.description === undefined ? null : (
        <span
          aria-hidden="true"
          className={`ml-auto shrink-0 whitespace-nowrap pl-2 text-meta ${node.tone === "error" ? "text-err" : "text-faint"}`}
        >
          {node.description}
        </span>
      )}
    </div>
  );
}
