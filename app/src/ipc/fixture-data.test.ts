// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The generated fixture module matches the data it was generated from.
 *
 * The data is authoritative (§11 rule 7) and the module is a copy the bundle
 * can import. This compares the two directly — each JSON file on disk against
 * the corresponding reply in the module — without going through the
 * generator, so a generator defect cannot hide a mismatch. When it fails, run
 * `bun run fixtures`.
 */

import { describe, expect, test } from "bun:test";
import { readdir } from "node:fs/promises";
import path from "node:path";

import { THERMAL_CONTROL } from "./generated/thermal-control";

const DATA = path.resolve(import.meta.dir, "../../test-data/thermal-control");

async function readJson(file: string): Promise<unknown> {
  const value: unknown = await Bun.file(path.join(DATA, file)).json();
  return value;
}

describe("the generated ThermalControl module is current", () => {
  test("workspace.json", async () => {
    expect(THERMAL_CONTROL.workspace).toEqual(await readJson("workspace.json"));
  });

  test("views.json", async () => {
    expect(THERMAL_CONTROL.views).toEqual(await readJson("views.json"));
  });

  test("elements/: the same petnames, and each reply equal", async () => {
    const files = (await readdir(path.join(DATA, "elements")))
      .filter((name) => name.endsWith(".json"))
      .sort();
    expect(Object.keys(THERMAL_CONTROL.elements).sort()).toEqual(
      files.map((name) => name.slice(0, -".json".length)),
    );
    for (const name of files) {
      const id = name.slice(0, -".json".length);
      expect(THERMAL_CONTROL.elements[id]).toEqual(await readJson(`elements/${name}`));
    }
  });
});
