/**
 * Every keyboard shortcut Mushroom has, in one place.
 *
 * The Shortcuts dialog, `docs/shortcuts.md` and the global key handler all
 * derive from this list. They drifted apart once already — the dialog went on
 * calling `Ctrl+F` and `Ctrl+P` "not available yet" for a release after both
 * started working — so the coupling is now enforced rather than remembered:
 * TypeScript checks the handler map against `MANAGED`, and a test checks the
 * documentation.
 */

export type Shortcut = {
  combo: string;
  action: string;
  /** False when the feature is not built yet; shown greyed. */
  available: boolean;
  /**
   * True when something other than the global key handler owns it — the menu
   * bar handles `Alt`, and `Escape` is handled per-dialog as well.
   */
  ownedElsewhere?: boolean;
};

export const SHORTCUTS = [
  { combo: "Ctrl+N", action: "New note", available: true },
  { combo: "Ctrl+S", action: "Save", available: true },
  { combo: "Ctrl+P", action: "Quick Open", available: true },
  { combo: "Ctrl+F", action: "Search notes", available: true },
  { combo: "Ctrl+Shift+F", action: "Toggle the AI panel", available: true },
  { combo: "Ctrl+Shift+P", action: "Toggle preview", available: true },
  { combo: "F1", action: "Keyboard shortcuts", available: true },
  { combo: "F5", action: "Refresh notes", available: true },
  { combo: "Escape", action: "Close menu or dialog", available: true },
  { combo: "Alt", action: "Focus the menu bar", available: true, ownedElsewhere: true },
  { combo: "Ctrl+O", action: "Open a file", available: false },
] as const satisfies readonly Shortcut[];

/**
 * The combos the global handler must implement — everything available that is
 * not owned by another component.
 */
export type ManagedCombo = Extract<
  (typeof SHORTCUTS)[number],
  { available: true; ownedElsewhere?: undefined }
>["combo"];

export const MANAGED: readonly string[] = SHORTCUTS.filter(
  (s) => s.available && !("ownedElsewhere" in s),
).map((s) => s.combo);
