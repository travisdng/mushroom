import { useCallback } from "react";
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
export function MarkdownPreview({ source }: { source: string }) {
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
  const prepared = source.replace(
    /\[\[([^\]]+)\]\]/g,
    (_, name: string) => `[${name}](${name})`,
  );

  return (
    <div className="preview selectable">
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
    </div>
  );
}
