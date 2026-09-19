// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";

import { Tree } from "./Tree";
import type { TreeNode } from "./tree-rows";

afterEach(cleanup);

const NODES: readonly TreeNode[] = [
  {
    id: "model",
    label: "model",
    description: "1 error",
    tone: "error",
    children: [
      {
        id: "tc",
        label: "ThermalControl.sysml",
        description: "1 error",
        tone: "error",
        children: null,
      },
      { id: "if", label: "Interfaces.sysml", children: null },
    ],
  },
  {
    id: "library",
    label: "library",
    children: [{ id: "units", label: "Units.kerml", children: null }],
  },
];

/** A tree whose expansion and selection are real state, as the navigator will hold them. */
function Harness(): React.JSX.Element {
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [selected, setSelected] = useState<string | null>(null);
  return (
    <Tree
      label="Files"
      nodes={NODES}
      expanded={expanded}
      onToggle={(id) => {
        const next = new Set(expanded);
        if (!next.delete(id)) {
          next.add(id);
        }
        setExpanded(next);
      }}
      selected={selected}
      onSelect={setSelected}
    />
  );
}

const item = (name: string | RegExp): HTMLElement => screen.getByRole("treeitem", { name });

describe("Tree", () => {
  test("is a named tree of items stating level, position and expansion", () => {
    render(<Harness />);
    expect(screen.getByRole("tree", { name: "Files" })).toBeDefined();
    const model = item(/^model/);
    expect(model.getAttribute("aria-level")).toBe("1");
    expect(model.getAttribute("aria-posinset")).toBe("1");
    expect(model.getAttribute("aria-setsize")).toBe("2");
    expect(model.getAttribute("aria-expanded")).toBe("false");
  });

  test("a leaf does not claim to be expandable", async () => {
    render(<Harness />);
    await userEvent.click(item(/^model/));
    expect(item(/^Interfaces/).hasAttribute("aria-expanded")).toBe(false);
  });

  test("a row's description is part of its accessible name (IX-10)", () => {
    render(<Harness />);
    expect(item("model, 1 error")).toBeDefined();
  });

  test("is one tab stop", async () => {
    render(
      <>
        <Harness />
        <button type="button">After</button>
      </>,
    );
    await userEvent.tab();
    expect(document.activeElement).toBe(item(/^model/));
    await userEvent.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "After" }));
  });

  test("the keyboard opens, enters, climbs out of and closes a folder", async () => {
    render(<Harness />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowRight}");
    expect(item(/^model/).getAttribute("aria-expanded")).toBe("true");
    await userEvent.keyboard("{ArrowRight}");
    expect(document.activeElement).toBe(item(/^ThermalControl/));
    await userEvent.keyboard("{ArrowLeft}");
    expect(document.activeElement).toBe(item(/^model/));
    await userEvent.keyboard("{ArrowLeft}");
    expect(item(/^model/).getAttribute("aria-expanded")).toBe("false");
  });

  test("arrows move focus without selecting", async () => {
    render(<Harness />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(item(/^library/));
    expect(screen.queryByRole("treeitem", { selected: true })).toBeNull();
  });

  test.each(["{Enter}", " "])("%s selects the focused row", async (key) => {
    render(<Harness />);
    await userEvent.tab();
    await userEvent.keyboard(`{ArrowDown}${key}`);
    expect(screen.getByRole("treeitem", { selected: true })).toBe(item(/^library/));
  });

  test("clicking a folder selects it and opens it", async () => {
    render(<Harness />);
    await userEvent.click(item(/^library/));
    expect(item(/^library/).getAttribute("aria-selected")).toBe("true");
    expect(item(/^Units/)).toBeDefined();
  });
});
