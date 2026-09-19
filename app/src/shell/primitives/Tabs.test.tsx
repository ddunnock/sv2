// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";

import { type TabItem, Tabs, type TabsProps } from "./Tabs";

afterEach(cleanup);

const SIDEBAR: readonly TabItem[] = [
  { id: "specification", label: "Specification" },
  { id: "source", label: "Element Source" },
];

const VIEWS: readonly TabItem[] = [
  { id: "gv", label: "GV ThermalControl" },
  { id: "iv", label: "IV thermalSubsystem" },
  { id: "gr", label: "GR Requirements" },
];

/** Tabs whose selection is real state, as the shell will hold it. */
function Harness(
  props: Readonly<{
    tabs: readonly TabItem[];
    activation: TabsProps["activation"];
    orientation?: TabsProps["orientation"];
    collapsed?: boolean;
  }>,
): React.JSX.Element {
  const [selected, setSelected] = useState(props.tabs[0]?.id ?? "");
  return (
    <Tabs
      label="Sidebar"
      tabs={props.tabs}
      selected={selected}
      onSelect={setSelected}
      activation={props.activation}
      orientation={props.orientation ?? "horizontal"}
      panel={props.collapsed === true ? null : <p>content of {selected}</p>}
    />
  );
}

describe("Tabs", () => {
  test("is a named tab list whose tabs report selection", () => {
    render(<Harness tabs={SIDEBAR} activation="automatic" />);
    expect(screen.getByRole("tablist", { name: "Sidebar" })).toBeDefined();
    expect(screen.getByRole("tab", { name: "Specification", selected: true })).toBeDefined();
    expect(screen.getByRole("tab", { name: "Element Source", selected: false })).toBeDefined();
  });

  test("the panel is named by its tab, and the tab controls the panel", () => {
    render(<Harness tabs={SIDEBAR} activation="automatic" />);
    const tab = screen.getByRole("tab", { name: "Specification" });
    const panel = screen.getByRole("tabpanel", { name: "Specification" });
    expect(tab.getAttribute("aria-controls")).toBe(panel.id);
  });

  test("is one tab stop, on the selected tab", async () => {
    render(<Harness tabs={SIDEBAR} activation="automatic" />);
    await userEvent.tab();
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Specification" }));
  });

  test("clicking a tab selects it and shows its panel", async () => {
    render(<Harness tabs={SIDEBAR} activation="automatic" />);
    await userEvent.click(screen.getByRole("tab", { name: "Element Source" }));
    expect(screen.getByRole("tab", { name: "Element Source", selected: true })).toBeDefined();
    expect(screen.getByRole("tabpanel").textContent).toBe("content of source");
  });

  describe("automatic activation", () => {
    test("an arrow key moves focus and selects", async () => {
      render(<Harness tabs={SIDEBAR} activation="automatic" />);
      await userEvent.tab();
      await userEvent.keyboard("{ArrowRight}");
      const source = screen.getByRole("tab", { name: "Element Source", selected: true });
      expect(document.activeElement).toBe(source);
    });
  });

  describe("manual activation", () => {
    test("an arrow key moves focus without selecting", async () => {
      render(<Harness tabs={VIEWS} activation="manual" />);
      await userEvent.tab();
      await userEvent.keyboard("{ArrowRight}");
      expect(document.activeElement).toBe(screen.getByRole("tab", { name: "IV thermalSubsystem" }));
      expect(screen.getByRole("tab", { name: "GV ThermalControl", selected: true })).toBeDefined();
    });

    test.each(["{Enter}", " "])("%s selects the focused tab", async (key) => {
      render(<Harness tabs={VIEWS} activation="manual" />);
      await userEvent.tab();
      await userEvent.keyboard(`{End}${key}`);
      expect(screen.getByRole("tab", { name: "GR Requirements", selected: true })).toBeDefined();
    });
  });

  test("arrows wrap, and Home and End jump", async () => {
    render(<Harness tabs={VIEWS} activation="manual" />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowLeft}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "GR Requirements" }));
    await userEvent.keyboard("{Home}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "GV ThermalControl" }));
  });

  test("a vertical list steps with up and down, not left and right", async () => {
    render(<Harness tabs={SIDEBAR} activation="automatic" orientation="vertical" />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowRight}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Specification" }));
    await userEvent.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Element Source" }));
  });

  describe("with no panel, as the collapsed sidebar (UI-09)", () => {
    test("renders no tabpanel", () => {
      render(
        <Harness tabs={SIDEBAR} activation="automatic" orientation="vertical" collapsed={true} />,
      );
      expect(screen.queryByRole("tabpanel")).toBeNull();
    });

    test("its tabs point at no panel, rather than one that is not there", () => {
      render(
        <Harness tabs={SIDEBAR} activation="automatic" orientation="vertical" collapsed={true} />,
      );
      for (const tab of screen.getAllByRole("tab")) {
        expect(tab.hasAttribute("aria-controls")).toBe(false);
      }
    });
  });
});
