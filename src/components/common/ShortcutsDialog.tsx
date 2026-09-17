import { Dialog } from "./Dialog";
import { SHORTCUTS } from "../../services/shortcuts";

/**
 * R3.8 — lists only what this build actually supports.
 *
 * Rendered from the single shortcut list, so it cannot drift from what the
 * keys actually do. It did drift once, which is why the list exists.
 */
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
                  color: s.available ? undefined : "var(--text-disabled)",
                }}
              >
                {s.combo}
              </td>
              <td
                style={{
                  color: s.available ? undefined : "var(--text-disabled)",
                }}
              >
                {s.action}
                {s.available ? "" : " — not available yet"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Dialog>
  );
}
