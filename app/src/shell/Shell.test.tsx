// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, mock, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { EditorHandoff } from "@/contract/editor-window";
import { type EditorWindow, NO_EDITOR_WINDOW } from "@/ipc/editor-window";
import { fixtureTransport } from "@/ipc/fixture-client";
import { THERMAL_CONTROL } from "@/ipc/generated/thermal-control";
import { createModelQueries, type Transport } from "@/ipc/model-queries";
import { err, ok } from "@/model/result";

import { Shell } from "./Shell";

afterEach(cleanup);

/** The shell on the fixture transport — the real parse path (§11 rule 5). */
const shell = (
  transport: Transport = fixtureTransport(THERMAL_CONTROL),
  editorWindow: EditorWindow = NO_EDITOR_WINDOW,
  report: (what: string, detail: unknown) => void = () => undefined,
): void => {
  const queries = createModelQueries(transport, "fixture");
  render(<Shell services={{ queries, report, editorWindow }} />);
};

const READY = () => Promise.resolve(ok({ kind: "ready", data: null } as const));

/**
 * An editor window that records what it was asked, and lets a test dock a
 * file back as Rust would: leave a handoff, then send the event.
 */
function fakeEditorWindow(undock: EditorWindow["undock"] = () => READY()): {
  editorWindow: EditorWindow;
  calls: string[];
  sent: EditorHandoff[];
  dockBack: (handoff: EditorHandoff) => void;
} {
  const calls: string[] = [];
  const sent: EditorHandoff[] = [];
  let waiting: EditorHandoff | null = null;
  let docked: (() => void) | null = null;
  const editorWindow: EditorWindow = {
    ...NO_EDITOR_WINDOW,
    available: true,
    undock: (handoff) => {
      calls.push("undock");
      sent.push(handoff);
      return undock(handoff);
    },
    focus: () => {
      calls.push("focus");
      return READY();
    },
    requestDock: () => {
      calls.push("requestDock");
      return READY();
    },
    handoff: () =>
      Promise.resolve(
        ok(
          waiting === null
            ? ({ kind: "unavailable", reason: { kind: "not-found" } } as const)
            : ({ kind: "ready", data: waiting } as const),
        ),
      ),
    onDocked: (listener) => {
      docked = listener;
      return () => {
        docked = null;
      };
    },
  };
  return {
    editorWindow,
    calls,
    sent,
    dockBack: (handoff) => {
      waiting = handoff;
      docked?.();
    },
  };
}

/** Opens ThermalControl.sysml in the docked editor and waits for its text. */
async function openThermalControl(): Promise<void> {
  await userEvent.click(
    await screen.findByRole("treeitem", { name: "ThermalControl.sysml, 1 error" }),
  );
  await screen.findByRole("textbox", { name: "model/ThermalControl.sysml" });
}

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
    for (const name of ["Validation", "Version control", "Settings"]) {
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
      render(<Shell services={{ queries, report, editorWindow: NO_EDITOR_WINDOW }} />);
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

  describe("the editor, split beside the views (Phase 5)", () => {
    test("choosing a file opens it, editable, under its path", async () => {
      shell();
      await userEvent.click(
        await screen.findByRole("treeitem", { name: "ThermalControl.sysml, 1 error" }),
      );
      const editor = await screen.findByRole("textbox", { name: "model/ThermalControl.sysml" });
      expect(editor.textContent).toContain("attribute state : HeaterState;");
      expect(editor.getAttribute("contenteditable")).toBe("true");
    });

    test("says plainly that edits are not saved", async () => {
      shell();
      await userEvent.click(
        await screen.findByRole("treeitem", { name: "ThermalControl.sysml, 1 error" }),
      );
      const region = await screen.findByRole("region", { name: "Editor" });
      expect(region.textContent).toContain("Edits are not saved");
    });

    test("a file whose text the core does not have says so", async () => {
      shell();
      await userEvent.click(await screen.findByRole("treeitem", { name: "Interfaces.sysml" }));
      expect(await screen.findByText(/The file is not available/)).toBeDefined();
    });

    test("Close removes the editor", async () => {
      shell();
      await userEvent.click(
        await screen.findByRole("treeitem", { name: "ThermalControl.sysml, 1 error" }),
      );
      await userEvent.click(await screen.findByRole("button", { name: "Close editor" }));
      expect(screen.queryByRole("region", { name: "Editor" })).toBeNull();
    });

    test("a folder row does not open an editor", async () => {
      shell();
      await userEvent.click(await screen.findByRole("treeitem", { name: "model, 1 error" }));
      expect(screen.queryByRole("region", { name: "Editor" })).toBeNull();
    });
  });

  describe("the editor's own window (IX-07)", () => {
    test("outside the desktop app there is no second window, and the button says so", () => {
      shell();
      const button = screen.getByRole("button", { name: "Editor window" });
      expect(button.getAttribute("aria-disabled")).toBe("true");
      expect(button.getAttribute("title")).toContain("needs the sv2 Studio app");
    });

    test("with nothing open there is nothing to move", () => {
      shell(undefined, fakeEditorWindow().editorWindow);
      const button = screen.getByRole("button", { name: "Editor window" });
      expect(button.getAttribute("title")).toContain("Open a file");
    });

    test("moving sends the file as edited, and the main window says where it went", async () => {
      const fake = fakeEditorWindow();
      shell(undefined, fake.editorWindow);
      await openThermalControl();
      await userEvent.click(screen.getByRole("button", { name: "Move to its own window" }));
      expect(fake.calls).toEqual(["undock"]);
      expect(String(fake.sent[0]?.path)).toBe("model/ThermalControl.sysml");
      expect(fake.sent[0]?.text).toContain("attribute state : HeaterState;");
      expect(screen.queryByRole("region", { name: "Editor" })).toBeNull();
      expect(screen.getByRole("contentinfo").textContent).toContain(
        "model/ThermalControl.sysml is open in its own window",
      );
    });

    test("the top bar's button moves it too, and then focuses it", async () => {
      const fake = fakeEditorWindow();
      shell(undefined, fake.editorWindow);
      await openThermalControl();
      await userEvent.click(screen.getByRole("button", { name: "Editor window" }));
      await screen.findByText(/is open in its own window/);
      await userEvent.click(screen.getByRole("button", { name: "Editor window" }));
      expect(fake.calls).toEqual(["undock", "focus"]);
    });

    test("Focus and Dock in the status bar ask the editor window", async () => {
      const fake = fakeEditorWindow();
      shell(undefined, fake.editorWindow);
      await openThermalControl();
      await userEvent.click(screen.getByRole("button", { name: "Move to its own window" }));
      await userEvent.click(await screen.findByRole("button", { name: "Focus" }));
      await userEvent.click(screen.getByRole("button", { name: "Dock" }));
      expect(fake.calls).toEqual(["undock", "focus", "requestDock"]);
    });

    test("choosing a file while the editor is away focuses it, and opens nothing", async () => {
      const fake = fakeEditorWindow();
      shell(undefined, fake.editorWindow);
      await openThermalControl();
      await userEvent.click(screen.getByRole("button", { name: "Move to its own window" }));
      await screen.findByText(/is open in its own window/);
      await userEvent.click(screen.getByRole("treeitem", { name: "Interfaces.sysml" }));
      expect(fake.calls).toEqual(["undock", "focus"]);
      expect(screen.queryByRole("region", { name: "Editor" })).toBeNull();
    });

    test("docking back shows the file as it was edited there, not as it is on disk", async () => {
      const fake = fakeEditorWindow();
      shell(undefined, fake.editorWindow);
      await openThermalControl();
      await userEvent.click(screen.getByRole("button", { name: "Move to its own window" }));
      await screen.findByText(/is open in its own window/);
      const moved = fake.sent[0];
      if (moved === undefined) {
        throw new Error("nothing was undocked");
      }
      fake.dockBack({ ...moved, text: "package Edited;\n", state: null });
      const editor = await screen.findByRole("textbox", { name: "model/ThermalControl.sysml" });
      expect(editor.textContent).toContain("package Edited;");
      expect(screen.queryByText(/is open in its own window/)).toBeNull();
    });

    test("an undock that fails is reported, and the file stays docked", async () => {
      const reported: string[] = [];
      const fake = fakeEditorWindow(() =>
        Promise.resolve(err({ kind: "rejected", command: "editor_undock" } as const)),
      );
      shell(undefined, fake.editorWindow, (what) => {
        reported.push(what);
      });
      await openThermalControl();
      await userEvent.click(screen.getByRole("button", { name: "Move to its own window" }));
      await Promise.resolve();
      expect(reported).toEqual(["opening the editor window failed"]);
      expect(screen.getByRole("region", { name: "Editor" })).toBeDefined();
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
      render(
        <Shell services={{ queries, report: () => undefined, editorWindow: NO_EDITOR_WINDOW }} />,
      );
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
