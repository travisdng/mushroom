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

  const content =
    mode === "preview" ? (
      <MarkdownPreview source={debouncedBody} />
    ) : mode === "edit" ? (
      <NoteEditor />
    ) : (
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

  if (!open.meta.aiExcluded) return content;

  // Shown in every mode, and shown as a fact rather than a warning: excluding
  // a note is a thing the user did on purpose, not a problem to fix.
  return (
    <div className="editor-with-notice">
      <div className="editor-notice">
        This note is not sent to AI. Remove <code>ai: false</code> from its
        frontmatter to include it.
      </div>
      {content}
    </div>
  );
}
