// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, mock, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Icon } from "./Icon";
import { ENABLED, IconButton } from "./IconButton";
import { Toolbar, type ToolbarItem } from "./Toolbar";

afterEach(cleanup);

describe("IconButton", () => {
  test("is a button named by its label, not by its icon", () => {
    render(
      <IconButton
        label="Toggle navigator"
        icon={<Icon name="navigator" />}
        availability={ENABLED}
        onPress={() => undefined}
      />,
    );
    expect(screen.getByRole("button", { name: "Toggle navigator" })).toBeDefined();
  });

  test("presses when enabled", async () => {
    const onPress = mock(() => undefined);
    render(
      <IconButton
        label="Search"
        icon={<Icon name="search" />}
        availability={ENABLED}
        onPress={onPress}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Search" }));
    expect(onPress).toHaveBeenCalledTimes(1);
  });

  describe("disabled", () => {
    const disabled = { kind: "disabled", reason: "validation is not implemented" } as const;

    test("does not press, and says so to assistive technology", async () => {
      const onPress = mock(() => undefined);
      render(
        <IconButton
          label="Validate"
          icon={<Icon name="validate" />}
          availability={disabled}
          onPress={onPress}
        />,
      );
      const button = screen.getByRole("button", { name: "Validate" });
      await userEvent.click(button);
      expect(onPress).not.toHaveBeenCalled();
      expect(button.getAttribute("aria-disabled")).toBe("true");
    });

    test("stays in the tab order, so its reason can be reached by keyboard", async () => {
      render(
        <IconButton
          label="Validate"
          icon={<Icon name="validate" />}
          availability={disabled}
          onPress={() => undefined}
        />,
      );
      await userEvent.tab();
      expect(document.activeElement).toBe(screen.getByRole("button", { name: "Validate" }));
    });

    test("names the reason in its tooltip", () => {
      render(
        <IconButton
          label="Validate"
          icon={<Icon name="validate" />}
          availability={disabled}
          onPress={() => undefined}
        />,
      );
      expect(screen.getByRole("button", { name: "Validate" }).getAttribute("title")).toContain(
        "validation is not implemented",
      );
    });
  });

  test("a toggle reports whether it is pressed", () => {
    render(
      <IconButton
        label="Navigator"
        icon={<Icon name="navigator" />}
        availability={ENABLED}
        onPress={() => undefined}
        pressed={true}
      />,
    );
    expect(screen.getByRole("button", { name: "Navigator", pressed: true })).toBeDefined();
  });

  test("its icon is hidden from assistive technology", () => {
    const { container } = render(
      <IconButton
        label="Search"
        icon={<Icon name="search" />}
        availability={ENABLED}
        onPress={() => undefined}
      />,
    );
    expect(container.querySelector("svg")?.getAttribute("aria-hidden")).toBe("true");
  });
});

describe("Toolbar", () => {
  function items(onPress: (id: string) => void = () => undefined): ToolbarItem[] {
    return ["Explorer", "Search", "Validation"].map((label) => ({
      id: label.toLowerCase(),
      label,
      icon: "files",
      availability: ENABLED,
      onPress: () => {
        onPress(label);
      },
    }));
  }

  test("is a named toolbar with its orientation", () => {
    render(<Toolbar label="Activity" orientation="vertical" items={items()} />);
    const toolbar = screen.getByRole("toolbar", { name: "Activity" });
    expect(toolbar.getAttribute("aria-orientation")).toBe("vertical");
  });

  test("is one tab stop: Tab enters on the first button and leaves after it", async () => {
    render(
      <>
        <Toolbar label="Activity" orientation="vertical" items={items()} />
        <button type="button">After</button>
      </>,
    );
    await userEvent.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Explorer" }));
    await userEvent.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "After" }));
  });

  test("arrow keys move along its orientation and wrap", async () => {
    render(<Toolbar label="Activity" orientation="vertical" items={items()} />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Search" }));
    await userEvent.keyboard("{ArrowUp}{ArrowUp}");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Validation" }));
  });

  test("the other axis's arrows do nothing", async () => {
    render(<Toolbar label="Activity" orientation="vertical" items={items()} />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowRight}");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Explorer" }));
  });

  test("Home and End jump to the ends", async () => {
    render(<Toolbar label="Tools" orientation="horizontal" items={items()} />);
    await userEvent.tab();
    await userEvent.keyboard("{End}");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Validation" }));
    await userEvent.keyboard("{Home}");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Explorer" }));
  });

  test("the roving tab stop follows focus, so Tab returns to the last button used", async () => {
    render(
      <>
        <Toolbar label="Tools" orientation="horizontal" items={items()} />
        <button type="button">After</button>
      </>,
    );
    await userEvent.tab();
    await userEvent.keyboard("{ArrowRight}");
    await userEvent.tab();
    await userEvent.tab({ shift: true });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Search" }));
  });

  test("pressing a button runs its action", async () => {
    const pressed: string[] = [];
    render(
      <Toolbar label="Tools" orientation="horizontal" items={items((id) => pressed.push(id))} />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Search" }));
    expect(pressed).toEqual(["Search"]);
  });
});
