// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { Shell } from "./Shell";

afterEach(cleanup);

const shell = (): void => {
  render(<Shell report={() => undefined} />);
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
    expect(screen.getByText(/sv2 cannot do listing a workspace's files yet/)).toBeDefined();
    await userEvent.click(screen.getByRole("tab", { name: "Element Source" }));
    expect(screen.getByText(/sv2 cannot do showing an element's source yet/)).toBeDefined();
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
