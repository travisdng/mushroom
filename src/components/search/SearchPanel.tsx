import { useEffect, useRef, useState } from "react";
import { Button } from "../common/Button";
import { useSearch } from "../../hooks/useSearch";
import { useNotes } from "../../hooks/useNotes";
import { openDocs } from "../../services/docs";
import { DOCS_BASE } from "../../types/app";
import type { SearchHit } from "../../types/search";

/** Highlight the matched terms in a snippet, inverse-video as it was. */
function Snippet({ text, terms }: { text: string; terms: string[] }) {
  if (terms.length === 0) return <>{text}</>;

  // Longest first, so "orchestrator" wins over "orch".
  const pattern = terms
    .filter((t) => t.length > 1)
    .sort((a, b) => b.length - a.length)
    .map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("|");

  if (!pattern) return <>{text}</>;

  const parts = text.split(new RegExp(`(${pattern})`, "gi"));
  const lower = terms.map((t) => t.toLowerCase());

  return (
    <>
      {parts.map((part, i) =>
        lower.includes(part.toLowerCase()) ? (
          <mark key={i} className="hit">
            {part}
          </mark>
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </>
  );
}

function dayLabel(unixSeconds: number): string {
  if (!unixSeconds) return "";
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}

export function SearchPanel() {
  const search = useSearch();
  const notes = useNotes();
  const inputRef = useRef<HTMLInputElement>(null);
  const [showHistory, setShowHistory] = useState(false);
  const [active, setActive] = useState(0);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  useEffect(() => setActive(0), [search.results]);

  const hits = search.results?.hits ?? [];

  const open = (hit: SearchHit) => {
    // lineStart is where the passage begins in the original Markdown, which
    // is what makes a result land on the right part of a long note.
    void notes.openNote(hit.id, hit.lineStart);
  };

  return (
    <div className="search-panel">
      <div className="search-controls">
        <div className="search-input-row">
          <input
            ref={inputRef}
            className="field"
            type="text"
            value={search.query}
            placeholder="Search your notes"
            aria-label="Search notes"
            onChange={(e) => search.setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                void search.run();
              } else if (e.key === "ArrowDown" && hits.length > 0) {
                e.preventDefault();
                setActive((i) => Math.min(i + 1, hits.length - 1));
              } else if (e.key === "ArrowUp" && hits.length > 0) {
                e.preventDefault();
                setActive((i) => Math.max(i - 1, 0));
              }
            }}
          />
          <Button
            className="search-history-button"
            onClick={() => setShowHistory((v) => !v)}
            disabled={search.history.length === 0}
            aria-label="Recent searches"
            title="Recent searches"
          >
            ▾
          </Button>
        </div>

        {showHistory && search.history.length > 0 ? (
          <div className="search-history">
            {search.history.map((entry) => (
              <div
                key={entry}
                className="list-row"
                onClick={() => {
                  setShowHistory(false);
                  search.setQuery(entry);
                  void search.run(entry);
                }}
                role="option"
                aria-selected={false}
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    setShowHistory(false);
                    search.setQuery(entry);
                    void search.run(entry);
                  }
                }}
              >
                <span className="list-title">{entry}</span>
              </div>
            ))}
          </div>
        ) : null}

        <div className="search-scope">
          <label htmlFor="search-scope">In:</label>
          <select
            id="search-scope"
            className="field"
            value={search.scopeFolder ?? ""}
            onChange={(e) => search.setScopeFolder(e.target.value || null)}
          >
            <option value="">All notes</option>
            {notes.tree?.children.map((child) => (
              <option key={child.path} value={child.path}>
                {child.name}
              </option>
            ))}
          </select>
          {/* R8.4. Beside the box rather than behind a menu: the moment you
              want to know about quotes and `-` is the moment you are typing. */}
          <a
            className="search-syntax-link"
            href={`${DOCS_BASE}/search-syntax.md`}
            title="Quoted phrases, exclusion, prefix matching"
            onClick={(e) => {
              e.preventDefault();
              void openDocs("search-syntax.md");
            }}
          >
            Syntax
          </a>
        </div>
      </div>

      {search.results?.stale ? (
        <div className="search-notice">
          The index may be out of date.{" "}
          <Button onClick={() => void search.rebuild()}>Rebuild</Button>
        </div>
      ) : null}

      <div className="search-count">
        {search.searching
          ? "Searching…"
          : search.results
            ? `${hits.length} ${hits.length === 1 ? "note" : "notes"} found`
            : ""}
      </div>

      <div className="search-results" role="listbox" aria-label="Search results">
        {search.results && hits.length === 0 && !search.searching ? (
          <div className="search-empty">
            <div>No notes found for “{search.query}”.</div>
            {search.results.terms.length > 0 ? (
              <div className="search-empty-terms">
                Searched for: {search.results.terms.join(", ")}
              </div>
            ) : null}
          </div>
        ) : null}

        {hits.map((hit, i) => (
          <div
            key={`${hit.id}-${hit.lineStart}`}
            className="search-hit"
            data-selected={i === active ? "true" : undefined}
            role="option"
            aria-selected={i === active}
            tabIndex={0}
            onClick={() => {
              setActive(i);
              open(hit);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                open(hit);
              }
            }}
          >
            <div className="search-hit-head">
              <span className="search-hit-title">{hit.title}</span>
              <span className="search-hit-date">{dayLabel(hit.modified)}</span>
            </div>
            <div className="search-hit-meta">
              {hit.folder || "(root)"}
              {hit.headingPath ? ` · ${hit.headingPath}` : ""}
            </div>
            <div className="search-hit-snippet">
              <Snippet text={hit.snippet} terms={search.results?.terms ?? []} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
