import { useNotes } from "../../hooks/useNotes";
import { EmptyState } from "../common/EmptyState";
import { Button } from "../common/Button";
import type { NoteMeta } from "../../types/notes";

/** Date header in the classic short form: "Sep 15". */
function dayLabel(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function groupByDay(notes: NoteMeta[]): Array<[string, NoteMeta[]]> {
  const groups = new Map<string, NoteMeta[]>();
  for (const note of notes) {
    const key = dayLabel(note.modified);
    const bucket = groups.get(key);
    if (bucket) bucket.push(note);
    else groups.set(key, [note]);
  }
  return [...groups.entries()];
}

export function NoteList({
  onContextMenu,
  onCreate,
}: {
  onContextMenu?: (id: string, x: number, y: number) => void;
  onCreate?: () => void;
}) {
  const { notes, open, openNote } = useNotes();

  if (notes.length === 0) {
    return (
      <EmptyState
        text="No notes yet."
        action={onCreate ? <Button onClick={onCreate}>Create your first note</Button> : undefined}
      />
    );
  }

  return (
    <div role="listbox" aria-label="Notes" style={{ padding: 2 }}>
      {groupByDay(notes).map(([day, items]) => (
        <div key={day}>
          <div className="list-group">{day}</div>
          {items.map((note) => (
            <div
              key={note.id}
              className="list-row"
              role="option"
              aria-selected={open?.meta.id === note.id}
              data-selected={open?.meta.id === note.id ? "true" : undefined}
              tabIndex={0}
              title={note.id}
              onClick={() => void openNote(note.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void openNote(note.id);
                }
              }}
              onContextMenu={(e) => {
                if (!onContextMenu) return;
                e.preventDefault();
                onContextMenu(note.id, e.clientX, e.clientY);
              }}
            >
              <span className="list-title">{note.title}</span>
              {note.folder ? <span className="list-folder">{note.folder}</span> : null}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
