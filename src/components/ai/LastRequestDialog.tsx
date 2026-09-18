import { useEffect, useState } from "react";

import { Dialog } from "../common/Dialog";
import { getLastAiRequest } from "../../services/aiService";
import type { LastRequest } from "../../types/ai";

/**
 * What Mushroom actually sent, markers and all.
 *
 * The point of this screen is that it lets somebody *check* rather than trust.
 * Any assurance in a settings dialog is worth less than being able to read the
 * bytes — so this shows the request verbatim, including the `[redacted: …]`
 * markers where credentials used to be.
 *
 * Read-only, local, and nothing here is ever sent anywhere.
 */
export function LastRequestDialog({ onClose }: { onClose: () => void }) {
  const [request, setRequest] = useState<LastRequest | null | "loading">(
    "loading",
  );

  useEffect(() => {
    let cancelled = false;
    getLastAiRequest()
      .then((found) => {
        if (!cancelled) setRequest(found);
      })
      .catch(() => {
        if (!cancelled) setRequest(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Dialog title="Last AI Request" onClose={onClose} acceptOnly width={620}>
      {request === "loading" ? (
        <p>Reading…</p>
      ) : request === null ? (
        <p>
          Nothing has been sent yet. Ask a question and this will show exactly
          what left your machine.
        </p>
      ) : (
        <>
          <p className="last-request__summary">
            Sent to <strong>{request.model}</strong> · {summarise(request)}
          </p>

          <div className="last-request__body" role="document">
            {request.messages.map((message, index) => (
              <div key={index} className="last-request__message">
                <div className="last-request__role">{message.role}</div>
                <pre className="last-request__text">{message.content}</pre>
              </div>
            ))}
          </div>

          <p className="last-request__note">
            This is the text as sent. It is kept in memory only, is replaced by
            the next request, and is never written to disk.
          </p>
        </>
      )}
    </Dialog>
  );
}

/** One line describing what the gate did, in plain words. */
function summarise(request: LastRequest): string {
  const { report } = request;
  const parts: string[] = [];

  const redacted = report.redactions.reduce((sum, r) => sum + r.count, 0);
  if (redacted > 0) {
    const kinds = report.redactions
      .map((r) => `${r.count} ${r.rule}`)
      .join(", ");
    parts.push(`${redacted} redacted (${kinds})`);
  }
  if (report.withheld.length > 0) {
    parts.push(`${report.withheld.length} withheld`);
  }
  if (report.excludedNotes > 0) {
    parts.push(`${report.excludedNotes} notes excluded`);
  }
  if (report.mode === "off") {
    parts.push("privacy mode was off");
  }

  return parts.length > 0 ? parts.join(" · ") : "nothing was redacted";
}
