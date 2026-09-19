// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";

import { Splitter, type SplitterProps } from "./Splitter";

afterEach(cleanup);

/** A splitter whose value is real state. The mockup's navigator is 280 px. */
function Harness(props: Readonly<{ side: SplitterProps["side"] }>): React.JSX.Element {
  const [width, setWidth] = useState(280);
  return (
    <>
      <div id="navigator">navigator</div>
      <Splitter
        label="Resize navigator"
        controls="navigator"
        side={props.side}
        value={width}
        min={200}
        max={480}
        onChange={setWidth}
      />
    </>
  );
}

const splitter = (): HTMLElement => screen.getByRole("separator", { name: "Resize navigator" });
const value = (): string | null => splitter().getAttribute("aria-valuenow");

describe("Splitter", () => {
  test("is a focusable separator stating its value, limits and panel", async () => {
    render(<Harness side="start" />);
    expect(value()).toBe("280");
    expect(splitter().getAttribute("aria-valuemin")).toBe("200");
    expect(splitter().getAttribute("aria-valuemax")).toBe("480");
    expect(splitter().getAttribute("aria-controls")).toBe("navigator");
    await userEvent.tab();
    expect(document.activeElement).toBe(splitter());
  });

  test("arrows step it, and a start-side panel grows to the right", async () => {
    render(<Harness side="start" />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowRight}");
    expect(value()).toBe("296");
    await userEvent.keyboard("{ArrowLeft}{ArrowLeft}");
    expect(value()).toBe("264");
  });

  test("an end-side panel, like the sidebar, grows to the left", async () => {
    render(<Harness side="end" />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowLeft}");
    expect(value()).toBe("296");
  });

  test("Home and End jump to the limits", async () => {
    render(<Harness side="start" />);
    await userEvent.tab();
    await userEvent.keyboard("{End}");
    expect(value()).toBe("480");
    await userEvent.keyboard("{Home}");
    expect(value()).toBe("200");
  });

  test("it never reports a value outside its limits", async () => {
    render(<Harness side="start" />);
    await userEvent.tab();
    await userEvent.keyboard("{End}{ArrowRight}{ArrowRight}");
    expect(value()).toBe("480");
  });

  test("dragging with a pointer moves it, clamped", () => {
    render(<Harness side="start" />);
    fireEvent.pointerDown(splitter(), { pointerId: 1, clientX: 300 });
    fireEvent.pointerMove(splitter(), { pointerId: 1, clientX: 340 });
    expect(value()).toBe("320");
    fireEvent.pointerMove(splitter(), { pointerId: 1, clientX: 900 });
    expect(value()).toBe("480");
  });

  test("releasing the pointer ends the drag", () => {
    render(<Harness side="start" />);
    fireEvent.pointerDown(splitter(), { pointerId: 1, clientX: 300 });
    fireEvent.pointerUp(splitter(), { pointerId: 1, clientX: 300 });
    fireEvent.pointerMove(splitter(), { pointerId: 1, clientX: 400 });
    expect(value()).toBe("280");
  });
});
