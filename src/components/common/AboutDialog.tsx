import { useEffect, useState } from "react";
import { Dialog } from "./Dialog";
import { Icon } from "./Icon";
import { ping } from "../../services/appService";
import { inDesktopApp } from "../../services/ipc";
import { APP_VERSION } from "../../types/app";

/**
 * R3.7 / R7.3. The version comes from the `ping` command, so this dialog is
 * also the proof that the frontend-to-backend channel works end to end. In a
 * plain browser (npm run dev) it falls back to the build-time version.
 */
export function AboutDialog({ onClose }: { onClose: () => void }) {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    if (!inDesktopApp()) {
      setVersion(APP_VERSION);
      return;
    }
    let cancelled = false;
    ping()
      .then((info) => !cancelled && setVersion(info.version))
      .catch(() => !cancelled && setVersion(APP_VERSION));
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Dialog title="About Mushroom" onClose={onClose} acceptOnly width={340}>
      <div style={{ display: "flex", gap: 12, alignItems: "flex-start" }}>
        <Icon name="mushroom" size={48} title="Mushroom" />
        <div>
          <div style={{ fontWeight: "bold", fontSize: 13 }}>Mushroom</div>
          <div>Personal knowledge and notes.</div>
          <div style={{ marginTop: 8 }}>Version {version ?? "…"}</div>
          <div style={{ marginTop: 8, color: "var(--text-disabled)" }}>
            Licensed under Apache-2.0.
          </div>
        </div>
      </div>
    </Dialog>
  );
}
