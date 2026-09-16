import ReactMarkdown, { defaultUrlTransform } from "react-markdown";
import remarkGfm from "remark-gfm";

import type { AiAnswer, AiStage, Citation } from "../../types/aiSearch";

/** The link scheme a citation marker is rewritten to before parsing. */
const CITATION_SCHEME = "mushroom-citation:";

/**
 * The answer area.
 *
 * Streaming text renders as plain text with a block caret. Running each delta
 * through a Markdown parser would reflow the paragraph on every token and
 * flicker; once the answer is complete it renders properly, with the `[n]`
 * markers as buttons.
 *
 * Deliberately not `MarkdownPreview`: that one resolves `[[wikilinks]]` against
 * the note list, which is right for a note and wrong for model output.
 */
export function AnswerView({
  stage,
  text,
  answer,
  incomplete,
  onOpenCitation,
}: {
  stage: AiStage;
  text: string;
  answer: AiAnswer | null;
  incomplete: boolean;
  onOpenCitation: (citation: Citation) => void;
}) {
  if (!text) return null;

  const grounding = answer?.grounding;
  const streaming = stage === "streaming" || stage === "asking";

  return (
    <div className="answer">
      {grounding?.uncited && !answer?.noResults ? (
        <div className="answer__header">
          Unverified — the model did not cite any of your notes.
        </div>
      ) : null}

      {grounding && grounding.unmatched.length > 0 ? (
        <div className="answer__warning">
          {grounding.unmatched.length === 1
            ? "One citation could not be matched to a note."
            : `${grounding.unmatched.length} citations could not be matched to your notes.`}
        </div>
      ) : null}

      <div className="answer__body selectable">
        {streaming ? (
          <span className="answer__streaming">
            {text}
            <span className="answer__caret" aria-hidden="true">
              █
            </span>
          </span>
        ) : (
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            // react-markdown sanitises unknown URL schemes to an empty href,
            // which silently turned every citation into plain text. Let our own
            // scheme through; everything else still goes through the default.
            urlTransform={(url) =>
              url.startsWith(CITATION_SCHEME) ? url : defaultUrlTransform(url)
            }
            components={{
              a({ href, children }) {
                if (!href?.startsWith(CITATION_SCHEME)) {
                  // The model was told to answer from the notes, not to link
                  // out. Anything else it emits renders as plain text.
                  return <>{children}</>;
                }
                const number = Number(href.slice(CITATION_SCHEME.length));
                const citation = answer?.grounding.used.find(
                  (c) => c.number === number,
                );

                if (!citation) {
                  return (
                    <span
                      className="citation citation--unmatched"
                      title="This is not one of the notes that was sent"
                    >
                      [{number}]
                    </span>
                  );
                }
                return (
                  <button
                    type="button"
                    className="citation"
                    title={`${citation.noteTitle} — open this note`}
                    onClick={() => onOpenCitation(citation)}
                  >
                    [{number}]
                  </button>
                );
              },
              img({ alt }) {
                return <span>[image: {alt ?? "untitled"}]</span>;
              },
            }}
          >
            {linkCitations(text)}
          </ReactMarkdown>
        )}
      </div>

      {incomplete ? (
        <div className="answer__incomplete">
          This answer is incomplete — it stopped before the model finished.
        </div>
      ) : null}

      {answer && !answer.noResults ? <AnswerFooter answer={answer} /> : null}
    </div>
  );
}

function AnswerFooter({ answer }: { answer: AiAnswer }) {
  const noteCount = new Set(answer.grounding.used.map((c) => c.noteId)).size;

  return (
    <div className="answer__footer">
      {answer.model && noteCount > 0
        ? `Answered by ${answer.model} from ${noteCount} of your notes.`
        : answer.model
          ? `Answered by ${answer.model}.`
          : null}
      {answer.dropped > 0
        ? ` ${answer.dropped} ${
            answer.dropped === 1 ? "excerpt was" : "excerpts were"
          } left out to fit the context limit.`
        : null}
      {answer.oversized
        ? " One excerpt was larger than the whole context budget."
        : null}
    </div>
  );
}

/**
 * Rewrite `[1]` and `[1, 2]` into inline links so the parser hands them to the
 * `a` renderer. Bare `[1]` would otherwise be read as a reference link.
 *
 * Code is left alone: `items[0]` in a sample is an index, not a citation.
 */
export function linkCitations(markdown: string): string {
  return outsideCode(markdown)
    .map((segment) =>
      segment.isCode
        ? segment.text
        : segment.text.replace(
            /\[(\d+(?:\s*,\s*\d+)*)\]/g,
            (_whole, inner: string) =>
              inner
                .split(",")
                .map((n) => n.trim())
                .map((n) => `[${n}](${CITATION_SCHEME}${n})`)
                .join(""),
          ),
    )
    .join("");
}

type Segment = { text: string; isCode: boolean };

/** Split into code and non-code runs, preserving every character. */
function outsideCode(markdown: string): Segment[] {
  const segments: Segment[] = [];
  // Fenced blocks, then inline spans. The alternation order matters: a fence
  // must win over the single backtick that starts it.
  const pattern = /(```[\s\S]*?```|```[\s\S]*$|`[^`\n]*`)/g;

  let last = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(markdown)) !== null) {
    if (match.index > last) {
      segments.push({ text: markdown.slice(last, match.index), isCode: false });
    }
    segments.push({ text: match[0], isCode: true });
    last = match.index + match[0].length;
  }

  if (last < markdown.length) {
    segments.push({ text: markdown.slice(last), isCode: false });
  }
  return segments;
}
