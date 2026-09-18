import type { PrivacyReport } from "../../types/ai";

/**
 * One line saying what the gate changed before sending.
 *
 * Stated as a fact, not a warning: the user configured this. Silence when
 * nothing happened — a notice on every answer is a notice nobody reads.
 */
export function PrivacyNotice({
  report,
  onShowRequest,
}: {
  report: PrivacyReport | null;
  onShowRequest?: () => void;
}) {
  if (!report) return null;

  const redacted = report.redactions.reduce((sum, r) => sum + r.count, 0);
  const parts: string[] = [];

  if (redacted > 0) {
    const kinds = report.redactions.map((r) => `${r.count} ${r.rule}`).join(", ");
    parts.push(
      `${redacted} ${redacted === 1 ? "item was" : "items were"} redacted before sending (${kinds})`,
    );
  }
  if (report.withheld.length > 0) {
    const kinds = [...new Set(report.withheld.map((w) => w.kind))].join(", ");
    parts.push(
      `${report.withheld.length} withheld entirely (${kinds})`,
    );
  }
  if (report.mode === "off") {
    parts.push("privacy mode is off, so nothing was scanned");
  }

  if (parts.length === 0) return null;

  return (
    <p className="privacy-notice">
      {parts.join(". ")}.
      {onShowRequest ? (
        <>
          {" "}
          <button type="button" className="linkish" onClick={onShowRequest}>
            See what was sent
          </button>
        </>
      ) : null}
    </p>
  );
}
