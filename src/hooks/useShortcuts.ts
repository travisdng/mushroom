import { useEffect } from "react";

/** Normalise a keyboard event into "Ctrl+Shift+F" form. */
function describe(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  parts.push(key);
  return parts.join("+");
}

/** Shortcuts the shell claims. Anything here without a handler flashes R6.2. */
const CLAIMED = [
  "Ctrl+N",
  "Ctrl+O",
  "Ctrl+S",
  "Ctrl+F",
  "Ctrl+Shift+F",
  "Ctrl+P",
  "Escape",
  "F1",
  "F5",
  "Ctrl+Shift+P",
];

/**
 * Browser defaults that make no sense in a desktop application (R6.4).
 * Suppressed in release builds only, so the dev tools stay usable.
 */
const BROWSER_DEFAULTS = ["Ctrl+R", "F5", "F7", "Ctrl+P", "Ctrl+F", "Ctrl+G"];

export function useShortcuts(
  onUnimplemented: (combo: string) => void,
  handlers: Record<string, () => void>,
) {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const combo = describe(e);

      if (!import.meta.env.DEV && BROWSER_DEFAULTS.includes(combo)) {
        e.preventDefault();
      }

      const handler = handlers[combo];
      if (handler) {
        e.preventDefault();
        handler();
        return;
      }

      if (CLAIMED.includes(combo)) {
        e.preventDefault();
        onUnimplemented(combo);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [handlers, onUnimplemented]);
}
