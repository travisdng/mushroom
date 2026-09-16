import { Dialog } from "./Dialog";

/**
 * R3.8 — lists only what this build actually supports. Anything shown here
 * that does not work is a bug; anything that works and is missing is too.
 */
const SHORTCUTS: Array<{ combo: string; action: string; works: boolean }> = [
  { combo: "Alt", action: "Focus the menu bar", works: true },
  { combo: "Ctrl+Shift+F", action: "Toggle the AI panel", works: true },
  { combo: "F1", action: "This dialog", works: true },
  { combo: "Escape", action: "Close menu or dialog", works: true },
  { combo: "Ctrl+N", action: "New note", works: false },
  { combo: "Ctrl+O", action: "Open", works: false },
  { combo: "Ctrl+S", action: "Save", works: false },
  { combo: "Ctrl+F", action: "Search notes", works: false },
  { combo: "Ctrl+P", action: "Quick open", works: false },
];

export function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  return (
    <Dialog title="Keyboard Shortcuts" onClose={onClose} acceptOnly width={380}>
      <table style={{ borderCollapse: "collapse", width: "100%" }}>
        <tbody>
          {SHORTCUTS.map((s) => (
            <tr key={s.combo}>
              <td
                style={{
                  width: 110,
                  paddingRight: 10,
                  verticalAlign: "top",
                  fontFamily: "var(--font-mono)",
                  color: s.works ? undefined : "var(--text-disabled)",
                }}
              >
                {s.combo}
              </td>
              <td
                style={{ color: s.works ? undefined : "var(--text-disabled)" }}
              >
                {s.action}
                {s.works ? "" : " — not available yet"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Dialog>
  );
}
