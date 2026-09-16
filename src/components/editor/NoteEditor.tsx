import { useEffect, useRef } from "react";
import { useNotes } from "../../hooks/useNotes";

/**
 * A plain textarea, on purpose.
 *
 * It gives correct native undo/redo, IME, spellcheck, and accessibility for
 * free, weighs nothing, and matches the period. CodeMirror can replace exactly
 * this component later if line numbers or highlighting are ever wanted.
 */
export function NoteEditor() {
  const { body, setBody, open, pendingLine, clearPendingLine } = useNotes();
  const ref = useRef<HTMLTextAreaElement>(null);

  // Scroll to the line a search result or link asked for.
  useEffect(() => {
    if (pendingLine == null || !ref.current) return;
    const el = ref.current;
    const lines = el.value.split("\n");
    const offset = lines
      .slice(0, Math.max(0, pendingLine - 1))
      .reduce((sum, l) => sum + l.length + 1, 0);
    el.focus();
    el.setSelectionRange(offset, offset);
    // Approximate: scroll so the target line sits near the top.
    const lineHeight = 18;
    el.scrollTop = Math.max(0, (pendingLine - 2) * lineHeight);
    clearPendingLine();
  }, [pendingLine, clearPendingLine, open?.meta.id]);

  return (
    <textarea
      ref={ref}
      className="editor selectable"
      value={body}
      spellCheck
      wrap="soft"
      aria-label="Note body"
      onChange={(e) => setBody(e.target.value)}
      onKeyDown={(e) => {
        // Tab indents rather than leaving the editor.
        if (e.key === "Tab") {
          e.preventDefault();
          const el = e.currentTarget;
          const { selectionStart: s, selectionEnd: t, value } = el;
          const next = `${value.slice(0, s)}\t${value.slice(t)}`;
          setBody(next);
          requestAnimationFrame(() => el.setSelectionRange(s + 1, s + 1));
        }
      }}
    />
  );
}
