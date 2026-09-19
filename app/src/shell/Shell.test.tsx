// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, mock, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { fixtureTransport } from "@/ipc/fixture-client";
import { THERMAL_CONTROL } from "@/ipc/fixture-thermal-control";
import { createModelQueries, type Transport } from "@/ipc/model-queries";
import { ok } from "@/model/result";

import { Shell } from "./Shell";

afterEach(cleanup);

/** The shell on the fixture transport — the real parse path (§11 rule 5). */
const shell = (transport: Transport = fixtureTransport(THERMAL_CONTROL)): void => {
  const queries = createModelQueries(transport, "fixture");
  render(<Shell services={{ queries, report: () => undefined }} />);
};

const navigator = (): HTMLElement | null => screen.queryByRole("navigation", { name: "Navigator" });
const sidebarPanel = (): HTMLElement | null =>
  screen.queryByRole("tabpanel", { name: /Specification|Element Source/ });

describe("Shell", () => {
  test("the mockup's regions are landmarks", () => {
    shell();
    expect(screen.getByRole("banner")).toBeDefined();
    expect(navigator()).not.toBeNull();
    expect(screen.getByRole("main", { name: "Views" })).toBeDefined();
    expect(screen.getByRole("complementary", { name: "Specification sidebar" })).toBeDefined();
    expect(screen.getByRole("contentinfo")).toBeDefined();
  });

  test("it opens as SCR-01: navigator on Files, sidebar on Specification", () => {
    shell();
    expect(screen.getByRole("tab", { name: "Files", selected: true })).toBeDefined();
    expect(screen.getByRole("tab", { name: "Specification", selected: true })).toBeDefined();
    expect(screen.getByText("Nothing is selected.")).toBeDefined();
  });

  describe("IX-02: the navigator hides and shows", () => {
    test("from the activity rail's Explorer, which reports whether it is shown", async () => {
      shell();
      const explorer = screen.getByRole("button", { name: "Explorer", pressed: true });
      await userEvent.click(explorer);
      expect(navigator()).toBeNull();
      expect(screen.getByRole("button", { name: "Explorer", pressed: false })).toBeDefined();
    });

    test("with Ctrl+B, returning to the mode it was in", async () => {
      shell();
      await userEvent.click(screen.getByRole("tab", { name: "Elements" }));
      await userEvent.keyboard("{Control>}b{/Control}");
      expect(navigator()).toBeNull();
      await userEvent.keyboard("{Control>}b{/Control}");
      expect(screen.getByRole("tab", { name: "Elements", selected: true })).toBeDefined();
    });
  });

  describe("IX-03: the sidebar collapses to a strip and expands again", () => {
    test("Ctrl+Alt+B collapses it to its tabs, with no panel", async () => {
      shell();
      await userEvent.keyboard("{Control>}{Alt>}b{/Alt}{/Control}");
      expect(sidebarPanel()).toBeNull();
      expect(
        screen.getByRole("tablist", { name: "Sidebar" }).getAttribute("aria-orientation"),
      ).toBe("vertical");
    });

    test("choosing a tab on the strip expands the sidebar to that tab", async () => {
      shell();
      await userEvent.keyboard("{Control>}{Alt>}b{/Alt}{/Control}");
      await userEvent.click(screen.getByRole("tab", { name: "Element Source" }));
      expect(screen.getByRole("tabpanel", { name: "Element Source" })).toBeDefined();
    });
  });

  test("IX-09: F11 enters focus mode and F11 again restores the layout exactly", async () => {
    // "[F11]" names the key by its code. user-event's key map gives "{F11}" the
    // code "Unknown", where a browser gives "F11", and the shortcut reads code.
    shell();
    await userEvent.click(screen.getByRole("tab", { name: "Element Source" }));
    await userEvent.keyboard("[F11]");
    expect(navigator()).toBeNull();
    expect(sidebarPanel()).toBeNull();
    expect(screen.getByText("Focus mode")).toBeDefined();
    await userEvent.keyboard("[F11]");
    expect(navigator()).not.toBeNull();
    expect(screen.getByRole("tabpanel", { name: "Element Source" })).toBeDefined();
    expect(screen.queryByText("Focus mode")).toBeNull();
  });

  test("deferred controls are disabled and say why", () => {
    shell();
    for (const name of ["Validation", "Version control", "Settings", "Editor window"]) {
      const button = screen.getByRole("button", { name });
      expect(button.getAttribute("aria-disabled")).toBe("true");
      expect(button.getAttribute("title")).toContain("not implemented yet");
    }
  });

  test("panels without data say what sv2 cannot do yet, rather than showing anything false", async () => {
    shell();
    await userEvent.click(screen.getByRole("tab", { name: "Elements" }));
    expect(
      screen.getByText(/sv2 does not yet support building the element hierarchy/),
    ).toBeDefined();
    await userEvent.click(screen.getByRole("tab", { name: "Element Source" }));
    expect(screen.getByText(/sv2 does not yet support showing an element's source/)).toBeDefined();
  });

  describe("the Files tree (IX-01), from the workspace query", () => {
    test("shows the workspace, its folders and files, with error counts in the names (IX-10)", async () => {
      shell();
      expect(await screen.findByRole("tree", { name: "Files" })).toBeDefined();
      expect(screen.getByRole("treeitem", { name: "ThermalControl, 1 error" })).toBeDefined();
      expect(screen.getByRole("treeitem", { name: "model, 1 error" })).toBeDefined();
      expect(screen.getByRole("treeitem", { name: "ThermalControl.sysml, 1 error" })).toBeDefined();
      expect(screen.getByRole("treeitem", { name: "Units.kerml" })).toBeDefined();
    });

    test("an unavailable answer is shown as the core's reason", async () => {
      shell(() => Promise.resolve(ok({ kind: "unavailable", reason: { kind: "no-workspace" } })));
      expect(await screen.findByText(/The Files tree needs an open workspace/)).toBeDefined();
    });

    test("a reply that fails its schema is an alert and a report, never shown as data", async () => {
      const report = mock((_what: string, _detail: unknown) => undefined);
      const queries = createModelQueries(
        // Only the workspace reply is malformed; every other query is honestly unavailable.
        (command) =>
          Promise.resolve(
            ok(
              command === "workspace"
                ? { kind: "ready", data: { name: "W", files: [{ path: "/abs" }] } }
                : { kind: "unavailable", reason: { kind: "no-workspace" } },
            ),
          ),
        "backend",
      );
      render(<Shell services={{ queries, report }} />);
      expect((await screen.findByRole("alert")).textContent).toContain(
        "The Files tree could not be read",
      );
      expect(report.mock.calls[0]?.[0]).toBe("query workspace failed");
    });
  });

  describe("the Views list and view tabs (UI-03, UI-04)", () => {
    test("lists the fixture's five views, each named by kind and what it exposes", async () => {
      shell();
      for (const name of [
        "General View, ThermalControl",
        "Interconnection View, thermalSubsystem",
        "Action Flow View, regulate",
        "State Transition View, HeaterModes",
        "Grid View, Requirements",
      ]) {
        expect(await screen.findByRole("button", { name })).toBeDefined();
      }
    });

    test("opening a view gives it a tab and says why it cannot be drawn yet", async () => {
      shell();
      await userEvent.click(
        await screen.findByRole("button", { name: "General View, ThermalControl" }),
      );
      expect(screen.getByRole("tab", { name: "GV ThermalControl", selected: true })).toBeDefined();
      expect(screen.getByRole("tabpanel", { name: "GV ThermalControl" }).textContent).toContain(
        "sv2 does not yet support drawing a General View",
      );
    });

    test("opening a view that is already open selects its tab rather than adding another", async () => {
      shell();
      const general = await screen.findByRole("button", { name: "General View, ThermalControl" });
      await userEvent.click(general);
      await userEvent.click(screen.getByRole("button", { name: "Grid View, Requirements" }));
      await userEvent.click(general);
      expect(screen.getAllByRole("tab", { name: "GV ThermalControl" })).toHaveLength(1);
      expect(screen.getByRole("tab", { name: "GV ThermalControl", selected: true })).toBeDefined();
    });

    test("the undesigned State Transition view (OD-03) opens and says so", async () => {
      shell();
      await userEvent.click(
        await screen.findByRole("button", { name: "State Transition View, HeaterModes" }),
      );
      expect(screen.getByRole("tabpanel", { name: "ST HeaterModes" }).textContent).toContain(
        "drawing a State Transition View",
      );
    });

    test("no view shows an empty canvas: nothing is drawn that is not true", async () => {
      shell();
      await userEvent.click(
        await screen.findByRole("button", { name: "General View, ThermalControl" }),
      );
      expect(
        screen.getByRole("tabpanel", { name: "GV ThermalControl" }).querySelector("svg, canvas"),
      ).toBeNull();
    });
  });

  describe("the status bar (UI-10)", () => {
    test("counts the workspace's problems", async () => {
      shell();
      const footer = screen.getByRole("contentinfo");
      await screen.findByRole("tree", { name: "Files" });
      expect(footer.textContent).toContain("1 error");
      expect(footer.textContent).toContain("0 warnings");
    });

    test("says, persistently, when the answers are fixture data", () => {
      shell();
      expect(screen.getByText("Fixture data")).toBeDefined();
    });

    test("says nothing about fixtures when the answers come from the core", () => {
      const queries = createModelQueries(fixtureTransport(THERMAL_CONTROL), "backend");
      render(<Shell services={{ queries, report: () => undefined }} />);
      expect(screen.queryByText("Fixture data")).toBeNull();
    });
  });

  test("the splitters size the navigator and the sidebar", () => {
    shell();
    expect(
      screen.getByRole("separator", { name: "Resize navigator" }).getAttribute("aria-valuenow"),
    ).toBe("280");
    expect(
      screen.getByRole("separator", { name: "Resize sidebar" }).getAttribute("aria-valuenow"),
    ).toBe("340");
  });
});
