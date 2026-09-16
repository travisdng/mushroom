import { useEffect, useState } from "react";
import { useNotes } from "../../hooks/useNotes";
import { NoteEditor } from "./NoteEditor";
import { MarkdownPreview } from "./MarkdownPreview";
import { EmptyState } from "../common/EmptyState";
import { Button } from "../common/Button";

export type ViewMode = "edit" | "preview" | "split";

/** Preview re-renders are debounced so typing stays smooth in Split mode. */
function useDebounced<T>(value: T, ms: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = window.setTimeout(() => setDebounced(value), ms);
    return () => window.clearTimeout(id);
  }, [value, ms]);
  return debounced;
}

export function EditorPane({
  mode,
  onCreate,
}: {
  mode: ViewMode;
  onCreate?: () => void;
}) {
  const { open, body } = useNotes();
  const debouncedBody = useDebounced(body, 150);

  if (!open) {
    return (
      <EmptyState
        text="No note open."
        action={onCreate ? <Button onClick={onCreate}>New note</Button> : undefined}
      />
    );
  }

  if (mode === "preview") return <MarkdownPreview source={debouncedBody} />;
  if (mode === "edit") return <NoteEditor />;

  return (
    <div className="split-view">
      <div className="split-half">
        <NoteEditor />
      </div>
      <div className="split-divider" aria-hidden="true" />
      <div className="split-half">
        <MarkdownPreview source={debouncedBody} />
      </div>
    </div>
  );
}
