import { Dialog } from "../common/Dialog";

/**
 * What is about to start leaving this machine, said once and said plainly.
 *
 * Shown when an AI endpoint is first configured, and again whenever it
 * changes — because consent was given for a particular destination, not for
 * whatever the box says next month.
 *
 * The hard part of this dialog is what it does **not** say. It does not claim
 * Mushroom removes secrets. A person who believes that stops marking their
 * notes, and marking them is the only control that actually works.
 */
export function EndpointConsentDialog({
  endpoint,
  onAccept,
  onClose,
}: {
  endpoint: string;
  onAccept: () => void;
  onClose: () => void;
}) {
  return (
    <Dialog
      title="Where your notes will be sent"
      onClose={onClose}
      onAccept={onAccept}
      acceptLabel="I understand"
      width={430}
    >
      <p>
        When you ask Mushroom a question, the parts of your notes that match
        it are sent to:
      </p>

      <p className="consent__endpoint">{endpoint}</p>

      <p>Before anything is sent, Mushroom:</p>
      <ul className="consent__list">
        <li>
          <strong>never sends</strong> a note marked <code>ai: false</code>, or
          one in a folder you have excluded
        </li>
        <li>
          replaces passwords and keys it <em>recognises</em> with a marker
        </li>
      </ul>

      <p className="consent__caveat">
        Recognising credentials is best-effort and will miss some. If a note
        must never leave this machine, mark it — that works every time, and is
        not a guess.
      </p>

      <p className="consent__where">
        You can change this in <strong>Tools → Settings → Privacy</strong>, and
        see exactly what was sent in <strong>Tools → Last AI Request</strong>.
      </p>
    </Dialog>
  );
}
