import { Button } from "./Button";

/**
 * Shown wherever an AI feature would be, when no endpoint is configured.
 *
 * Not an error: the user has not done anything wrong, and Mushroom works
 * perfectly well without AI. Plain text and a way forward, no red, no warning
 * icon (R1.6, R7.2).
 */
export function AiUnconfigured({
  onOpenSettings,
  what = "Answers",
}: {
  onOpenSettings: () => void;
  /** What this particular spot would have offered, e.g. "Answers". */
  what?: string;
}) {
  return (
    <div className="ai-unconfigured">
      <p style={{ margin: "0 0 8px" }}>
        {what} need an AI service. Mushroom does not have one yet.
      </p>
      <p style={{ margin: "0 0 10px", color: "var(--text-disabled)" }}>
        Everything else — notes, folders and search — works without it.
      </p>
      <Button onClick={onOpenSettings}>Open Settings</Button>
    </div>
  );
}
