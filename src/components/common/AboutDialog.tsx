import { Dialog } from "./Dialog";
import { Icon } from "./Icon";
import { APP_VERSION } from "../../types/app";

/**
 * R3.7. The version currently comes from the build; task 8 replaces it with
 * the value reported by the `ping` command, proving the IPC boundary works.
 */
export function AboutDialog({ onClose }: { onClose: () => void }) {
  return (
    <Dialog title="About Mushroom" onClose={onClose} acceptOnly width={340}>
      <div style={{ display: "flex", gap: 12, alignItems: "flex-start" }}>
        <Icon name="mushroom" size={48} title="Mushroom" />
        <div>
          <div style={{ fontWeight: "bold", fontSize: 13 }}>Mushroom</div>
          <div>Personal knowledge and notes.</div>
          <div style={{ marginTop: 8 }}>Version {APP_VERSION}</div>
          <div style={{ marginTop: 8, color: "var(--text-disabled)" }}>
            Licensed under Apache-2.0.
          </div>
        </div>
      </div>
    </Dialog>
  );
}
