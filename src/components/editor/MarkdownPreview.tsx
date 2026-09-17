import { memo, useCallback, useMemo } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useNotes } from "../../hooks/useNotes";

/**
 * Rendered Markdown.
 *
 * Deliberately no `rehype-raw`: raw HTML in a note is escaped and shown as
 * text. A note is a local file, but it can arrive by import or sync, and
 * executing HTML from it would make Mushroom a way to run someone else's
 * markup (R5.4).
 */
function MarkdownPreviewInner({ source }: { source: string }) {
  const { notes, openNote } = useNotes();

  const followLink = useCallback(
    async (href: string) => {
      if (/^https?:\/\//i.test(href)) {
        // External links belong in the browser, not in the app window (R5.5).
        try {
          const { openUrl } = await import("@tauri-apps/plugin-opener");
          await openUrl(href);
        } catch {
          window.open(href, "_blank", "noopener");
        }
        return;
      }

      // A relative .md path or a [[wikilink]] resolves against the note list.
      const target = href.replace(/^\.\//, "");
      const wanted = target.replace(/\.md$/i, "").toLowerCase();

      const match =
        notes.find((n) => n.id.toLowerCase() === target.toLowerCase()) ??
        notes.find((n) => n.id.replace(/\.md$/i, "").toLowerCase() === wanted) ??
        notes.find((n) => n.title.toLowerCase() === wanted);

      if (match) void openNote(match.id);
    },
    [notes, openNote],
  );

  // [[wikilinks]] are not Markdown, so rewrite them to links before parsing.
  const prepared = useMemo(
    () => source.replace(/\[\[([^\]]+)\]\]/g, (_, name: string) => `[${name}](${name})`),
    [source],
  );

  // Memoised on the text, not on the render.
  //
  // The 150 ms debounce upstream stops the *text* changing on every keystroke,
  // but not the rendering: this component reads `useNotes()`, the note body
  // lives in that context, and a context consumer re-renders whenever the
  // context changes — every keystroke — whatever `memo` says about its props.
  // So react-markdown re-parsed the whole note on each key for a result
  // identical to the last one, and the debounce was doing a third of the job
  // it was credited with. Returning the same element reference is what
  // actually lets React skip the subtree.
  const rendered = useMemo(
    () => (
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a({ href, children }) {
            return (
              <a
                href={href ?? "#"}
                onClick={(e) => {
                  e.preventDefault();
                  if (href) void followLink(href);
                }}
              >
                {children}
              </a>
            );
          },
          // Images could reference remote hosts; show the alt text instead of
          // silently fetching from the network.
          img({ alt }) {
            return <span className="preview-image">[image: {alt ?? "untitled"}]</span>;
          },
        }}
      >
        {prepared}
      </ReactMarkdown>
    ),
    // `followLink` changes only when the note *list* does, which is not
    // something typing causes.
    [prepared, followLink],
  );

  return <div className="preview selectable">{rendered}</div>;
}

/** Also skips the render entirely when a parent re-renders for its own reasons. */
export const MarkdownPreview = memo(MarkdownPreviewInner);
