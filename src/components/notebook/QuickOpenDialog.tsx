import { useEffect, useMemo, useRef, useState } from "react";

import { Dialog } from "../common/Dialog";
import { listNotes } from "../../services/notesService";
import { rankNotes } from "../../services/quickOpen";
import type { NoteMeta } from "../../types/notes";

const VISIBLE_ROWS = 10;

/**
 * `Ctrl+P` — open a note by typing part of its name.
 *
 * Matching runs against the cached note list, so there is no round trip per
 * keystroke. A plain dialog with a sunken list and navy full-row selection,
 * not a floating command palette.
 */
export function QuickOpenDialog({
  notes,
  recent,
  onOpen,
  onClose,
}: {
  /** The list already on screen, used until the full one arrives. */
  notes: NoteMeta[];
  /** Most recently opened note ids, newest first. */
  recent: string[];
  onOpen: (id: string) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [all, setAll] = useState<NoteMeta[] | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // The list on screen is filtered to the selected folder, and Quick Open that
  // only finds notes in the folder you are already looking at is no use. Fetch
  // the unfiltered list once, when the dialog opens — not per keystroke, which
  // is what the local matching is for.
  useEffect(() => {
    let live = true;
    listNotes(null)
      .then((full) => {
        if (live) setAll(full);
      })
      .catch(() => {
        // Fall back to what is already on screen rather than showing nothing.
      });
    return () => {
      live = false;
    };
  }, []);

  const searchable = all ?? notes;

  const hits = useMemo(
    () => rankNotes(searchable, query, recent),
    [searchable, query, recent],
  );

  // A new query invalidates the old selection.
  useEffect(() => setActive(0), [query]);

  // Keep the selected row in view when arrowing past the visible window.
  useEffect(() => {
    const row = listRef.current?.children[active];
    if (row instanceof HTMLElement) row.scrollIntoView({ block: "nearest" });
  }, [active]);

  const choose = (index: number) => {
    const hit = hits[index];
    if (!hit) return;
    onOpen(hit.note.id);
    onClose();
  };

  return (
    <Dialog
      title="Quick Open"
      onClose={onClose}
      onAccept={() => choose(active)}
      acceptLabel="Open"
      acceptDisabled={hits.length === 0}
      width={460}
    >
      <input
        className="field"
        value={query}
        placeholder="Type part of a note name"
        spellCheck={false}
        autoFocus
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setActive((i) => Math.min(i + 1, hits.length - 1));
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setActive((i) => Math.max(i - 1, 0));
          } else if (e.key === "PageDown") {
            e.preventDefault();
            setActive((i) => Math.min(i + VISIBLE_ROWS, hits.length - 1));
          } else if (e.key === "PageUp") {
            e.preventDefault();
            setActive((i) => Math.max(i - VISIBLE_ROWS, 0));
          }
          // Enter and Escape are the Dialog's; letting them through keeps one
          // definition of what those keys do.
        }}
      />

      <div className="quick-open__list" ref={listRef} role="listbox">
        {hits.length === 0 ? (
          <div className="quick-open__empty">
            {query.trim()
              ? "No note matches that."
              : "Nothing opened yet — type to search."}
          </div>
        ) : (
          hits.map((hit, index) => (
            <div
              key={hit.note.id}
              role="option"
              aria-selected={index === active}
              className="quick-open__row"
              data-active={index === active ? "true" : undefined}
              onMouseEnter={() => setActive(index)}
              onClick={() => choose(index)}
            >
              <span className="quick-open__title">{hit.note.title}</span>
              <span className="quick-open__folder">{hit.note.folder}</span>
            </div>
          ))
        )}
      </div>
    </Dialog>
  );
}
