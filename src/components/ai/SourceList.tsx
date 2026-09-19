import { useState } from "react";

import type { Citation, RetrievedPassage } from "../../types/aiSearch";

/**
 * Where the answer came from, and what else was looked at.
 *
 * Each source shows the excerpt **as retrieved from the note** (R5.3), which
 * is the only defence against a model that cites `[1]` and then describes
 * something the note does not say — the citation check cannot catch that, but
 * a reader can.
 *
 * It is deliberately *not* the text that went on the wire. Since spec 08 the
 * two can differ: credentials are replaced with markers, and in `block` mode
 * the excerpt may not be sent at all. Showing the user their own note here is
 * right — it is their note, on their machine — but it means this panel cannot
 * answer "what did Mushroom send?". `Tools → Last AI Request…` answers that,
 * and is the only thing that should be read as a record of what left.
 */
export function SourceList({
  used,
  alsoSearched,
  onOpen,
}: {
  used: Citation[];
  alsoSearched: RetrievedPassage[];
  onOpen: (noteId: string, lineStart: number) => void;
}) {
  const [showAlso, setShowAlso] = useState(false);

  // One entry per note, keeping the first (best) citation for each.
  const byNote = new Map<string, Citation>();
  for (const citation of used) {
    if (!byNote.has(citation.noteId)) byNote.set(citation.noteId, citation);
  }

  const alsoByNote = new Map<string, RetrievedPassage>();
  for (const passage of alsoSearched) {
    if (!byNote.has(passage.noteId) && !alsoByNote.has(passage.noteId)) {
      alsoByNote.set(passage.noteId, passage);
    }
  }

  if (byNote.size === 0 && alsoByNote.size === 0) return null;

  return (
    <div className="sources">
      {byNote.size > 0 ? (
        <>
          <div className="sources__title">Sources:</div>
          <ul className="sources__list">
            {[...byNote.values()].map((citation) => (
              <li key={citation.noteId}>
                <button
                  type="button"
                  className="sources__link"
                  onClick={() => onOpen(citation.noteId, citation.lineStart)}
                >
                  [{citation.number}] {citation.noteTitle}
                </button>
                {citation.folder ? (
                  <span className="sources__folder"> · {citation.folder}</span>
                ) : null}
                <div className="sources__excerpt selectable" title="The excerpt as it is in your note. What was sent may differ — see Tools → Last AI Request…">
                  {citation.text.trim()}
                </div>
              </li>
            ))}
          </ul>
        </>
      ) : null}

      {alsoByNote.size > 0 ? (
        <div className="sources__also">
          <button
            type="button"
            className="sources__toggle"
            aria-expanded={showAlso}
            onClick={() => setShowAlso((open) => !open)}
          >
            {showAlso ? "−" : "+"} Also searched ({alsoByNote.size})
          </button>
          {showAlso ? (
            <ul className="sources__list">
              {[...alsoByNote.values()].map((passage) => (
                <li key={passage.noteId}>
                  <button
                    type="button"
                    className="sources__link"
                    onClick={() => onOpen(passage.noteId, passage.lineStart)}
                  >
                    {passage.noteTitle}
                  </button>
                  {passage.folder ? (
                    <span className="sources__folder"> · {passage.folder}</span>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
