import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { MANAGED, SHORTCUTS } from "./shortcuts";

/** The combos `docs/shortcuts.md` lists in its table. */
function documentedCombos(): string[] {
  const doc = readFileSync("docs/shortcuts.md", "utf8");
  const table = doc.slice(doc.indexOf("| Shortcut | Action |"));
  const rows = table.split("\n").filter((line) => line.startsWith("| `"));
  return rows.map((row) => row.split("|")[1]!.trim().split("`").join(""));
}

describe("the shortcut list", () => {
  it("has no duplicate combos", () => {
    const combos = SHORTCUTS.map((s) => s.combo);
    expect(new Set(combos).size).toBe(combos.length);
  });

  it("every shortcut says what it does", () => {
    for (const shortcut of SHORTCUTS) {
      expect(shortcut.action.trim()).not.toBe("");
    }
  });

  it("MANAGED excludes what another component owns", () => {
    // The menu bar owns Alt; the global handler must not claim it.
    expect(MANAGED).not.toContain("Alt");
    expect(MANAGED).toContain("Ctrl+N");
  });

  it("MANAGED excludes shortcuts that are not built yet", () => {
    expect(MANAGED).not.toContain("Ctrl+O");
  });
});

describe("docs/shortcuts.md", () => {
  it("lists exactly the shortcuts the application has", () => {
    // The dialog and this page drifted apart once: the dialog called Ctrl+F
    // and Ctrl+P "not available yet" for a release after both worked.
    expect(documentedCombos().sort()).toEqual(
      SHORTCUTS.map((s) => s.combo).sort(),
    );
  });

  it("marks the unavailable ones as unavailable", () => {
    const doc = readFileSync("docs/shortcuts.md", "utf8");
    for (const shortcut of SHORTCUTS.filter((s) => !s.available)) {
      const row = doc
        .split("\n")
        .find((line) => line.startsWith(`| \`${shortcut.combo}\``));
      expect(row, `no row for ${shortcut.combo}`).toBeDefined();
      expect(row).toContain("not available yet");
    }
  });

  it("does not claim an available shortcut is unavailable", () => {
    const doc = readFileSync("docs/shortcuts.md", "utf8");
    for (const shortcut of SHORTCUTS.filter((s) => s.available)) {
      const row = doc
        .split("\n")
        .find((line) => line.startsWith(`| \`${shortcut.combo}\``));
      expect(row, `no row for ${shortcut.combo}`).toBeDefined();
      expect(row).not.toContain("not available yet");
    }
  });
});
