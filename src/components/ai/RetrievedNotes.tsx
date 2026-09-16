import type { AiStage, RetrievedPassage } from "../../types/aiSearch";

/**
 * Progress, and the notes retrieval found.
 *
 * The list appears as soon as retrieval finishes, while the model is still
 * thinking (R1.4) — so a wait has something to read, and a wrong answer can be
 * spotted from the wrong notes before it even arrives.
 *
 * The progress bar is a segmented indeterminate bar, not a spinner: this is a
 * 1990s utility panel.
 */
export function RetrievedNotes({
  stage,
  passages,
  terms,
  onOpen,
}: {
  stage: AiStage;
  passages: RetrievedPassage[];
  terms: string[];
  onOpen: (noteId: string, lineStart: number) => void;
}) {
  const working = stage === "retrieving" || stage === "asking";
  if (stage === "idle") return null;

  const label =
    stage === "retrieving" ? "Searching your notes…" : "Asking the model…";

  // One line per note, not per passage.
  const seen = new Set<string>();
  const notes = passages.filter((p) => {
    if (seen.has(p.noteId)) return false;
    seen.add(p.noteId);
    return true;
  });

  return (
    <div className="retrieved">
      {working ? (
        <div className="retrieved__progress">
          <span className="retrieved__label">{label}</span>
          <div
            className="progress progress--indeterminate"
            role="progressbar"
            aria-label={label}
          >
            <span />
            <span />
            <span />
            <span />
            <span />
            <span />
          </div>
        </div>
      ) : null}

      {notes.length > 0 ? (
        <div className="retrieved__notes">
          <div className="retrieved__title">
            Found {notes.length} {notes.length === 1 ? "note" : "notes"}
            {terms.length > 0 ? ` for: ${terms.join(" ")}` : ""}
          </div>
          <ul className="retrieved__list">
            {notes.map((passage) => (
              <li key={passage.noteId}>
                <button
                  type="button"
                  className="retrieved__link"
                  onClick={() => onOpen(passage.noteId, passage.lineStart)}
                >
                  {passage.noteTitle}
                </button>
                {passage.folder ? (
                  <span className="retrieved__folder"> · {passage.folder}</span>
                ) : null}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
