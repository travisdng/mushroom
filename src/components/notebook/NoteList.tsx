import { useMemo } from "react";

import { useNotes } from "../../hooks/useNotes";
import { useVirtualRows } from "../../hooks/useVirtualRows";
import { EmptyState } from "../common/EmptyState";
import { Button } from "../common/Button";
import type { NoteMeta } from "../../types/notes";

/** Both note rows and date headers are exactly this tall — see notes.css. */
const ROW_HEIGHT = 18;

/** Date header in the classic short form: "Sep 15". */
function dayLabel(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

type Row =
  | { kind: "group"; key: string; label: string }
  | { kind: "note"; key: string; note: NoteMeta };

/**
 * Flatten the day grouping into one list of equal-height rows.
 *
 * Virtualising a nested structure means measuring; virtualising a flat one of
 * uniform height is arithmetic.
 */
function toRows(notes: NoteMeta[]): Row[] {
  const rows: Row[] = [];
  let currentDay: string | null = null;

  for (const note of notes) {
    const day = dayLabel(note.modified);
    if (day !== currentDay) {
      rows.push({ kind: "group", key: `day-${day}`, label: day });
      currentDay = day;
    }
    rows.push({ kind: "note", key: note.id, note });
  }

  return rows;
}

export function NoteList({
  onContextMenu,
  onCreate,
}: {
  onContextMenu?: (id: string, x: number, y: number) => void;
  onCreate?: () => void;
}) {
  const { notes, open, openNote } = useNotes();
  const rows = useMemo(() => toRows(notes), [notes]);
  const view = useVirtualRows(rows.length, ROW_HEIGHT);

  if (notes.length === 0) {
    return (
      <EmptyState
        text="No notes yet."
        action={
          onCreate ? (
            <Button onClick={onCreate}>Create your first note</Button>
          ) : undefined
        }
      />
    );
  }

  return (
    <div
      ref={view.ref}
      role="listbox"
      aria-label="Notes"
      className="note-list"
    >
      <div style={{ height: view.paddingTop }} />

      {rows.slice(view.start, view.end).map((row) =>
        row.kind === "group" ? (
          <div key={row.key} className="list-group">
            {row.label}
          </div>
        ) : (
          <div
            key={row.key}
            className="list-row"
            role="option"
            aria-selected={open?.meta.id === row.note.id}
            data-selected={open?.meta.id === row.note.id ? "true" : undefined}
            tabIndex={0}
            title={row.note.id}
            onClick={() => void openNote(row.note.id)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                void openNote(row.note.id);
              }
            }}
            onContextMenu={(e) => {
              if (!onContextMenu) return;
              e.preventDefault();
              onContextMenu(row.note.id, e.clientX, e.clientY);
            }}
          >
            <span className="list-title">{row.note.title}</span>
            {row.note.folder ? (
              <span className="list-folder">{row.note.folder}</span>
            ) : null}
          </div>
        ),
      )}

      <div style={{ height: view.paddingBottom }} />
    </div>
  );
}
