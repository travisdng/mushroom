import { useShell } from "../../hooks/useShell";

/**
 * Two sunken cells: context on the left, state on the right (R2.5).
 * More cells arrive with the features that need them.
 */
export function StatusBar() {
  const { context, status, ui } = useShell();

  if (!ui.showStatusBar) return null;

  return (
    <div className="statusbar" role="status" aria-live="polite">
      <div className="statusbar__cell statusbar__cell--grow">{context}</div>
      <div className="statusbar__cell statusbar__cell--fixed">{status}</div>
    </div>
  );
}
