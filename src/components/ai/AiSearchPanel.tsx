import { useCallback, useEffect, useRef, useState } from "react";

import { AiUnconfigured } from "../common/AiUnconfigured";
import { Button } from "../common/Button";
import { TextArea } from "../common/Field";
import { AnswerView } from "./AnswerView";
import { RetrievedNotes } from "./RetrievedNotes";
import { SourceList } from "./SourceList";
import { useAiSearch } from "../../hooks/useAiSearch";
import { useNotes } from "../../hooks/useNotes";
import { useShell } from "../../hooks/useShell";
import { getAiSettings } from "../../services/aiService";
import { inDesktopApp } from "../../services/ipc";
import type { Citation } from "../../types/aiSearch";

/**
 * Ask a question of your notes.
 *
 * A grey utility panel, not a chat window: no bubbles, no avatars, no
 * gradients. The one animation is the block caret while text streams.
 */
export function AiSearchPanel({
  onOpenSettings,
  settingsVersion = 0,
}: {
  onOpenSettings: () => void;
  /**
   * Bumped whenever the Settings dialog closes. Without it the panel would go
   * on saying "not configured" after you had just configured it, until the
   * app was restarted.
   */
  settingsVersion?: number;
}) {
  const ai = useAiSearch();
  const { notes, openNote } = useNotes();
  const { setStatus } = useShell();
  const [configured, setConfigured] = useState<boolean | null>(null);
  const [showHistory, setShowHistory] = useState(false);
  /** A cited note that is no longer where the answer said it was (R5.6). */
  const [unavailable, setUnavailable] = useState<string | null>(null);
  const fieldRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!inDesktopApp()) {
      setConfigured(false);
      return;
    }
    getAiSettings()
      // The endpoint always holds a plausible default, so it cannot say
      // whether anyone chose it. Only the explicit flag can.
      .then((settings) => setConfigured(settings.config.configured))
      // If the settings cannot be read, assume configured rather than showing
      // "set me up" over a working install.
      .catch(() => setConfigured(true));
  }, [settingsVersion]);

  const open = useCallback(
    (noteId: string, lineStart: number) => {
      // Citations are resolved at click time, not when the answer was written.
      // A note renamed or deleted since would otherwise make the link do
      // nothing at all, which reads as a broken button.
      if (!notes.some((note) => note.id === noteId)) {
        setUnavailable(noteId);
        return;
      }
      setUnavailable(null);
      void openNote(noteId, lineStart);
    },
    [notes, openNote],
  );

  const openCitation = useCallback(
    (citation: Citation) => open(citation.noteId, citation.lineStart),
    [open],
  );

  const submit = useCallback(() => {
    setUnavailable(null);
    void ai.ask();
  }, [ai]);

  // Token usage belongs in the status bar, not as a badge on the answer
  // (R8.4). Shown for a while, then the status bar goes back to resting.
  const usage = ai.answer?.usage;
  useEffect(() => {
    if (!usage) return;
    // Kept short: the status cell is narrow, and a truncated number is worse
    // than a smaller complete one. The in/out split is recorded per request
    // for Diagnostics.
    setStatus(`AI: ${usage.totalTokens.toLocaleString()} tokens`, 8000);
  }, [usage, setStatus]);

  if (configured === false) {
    return <AiUnconfigured onOpenSettings={onOpenSettings} what="Answers" />;
  }

  const canAsk = ai.question.trim().length > 0 && !ai.busy;

  return (
    <div className="ai-panel">
      <div className="ai-panel__ask">
        <TextArea
          ref={fieldRef}
          rows={3}
          value={ai.question}
          placeholder="What did I write about…"
          spellCheck={false}
          onChange={(e) => ai.setQuestion(e.target.value)}
          onKeyDown={(e) => {
            // Enter asks; Shift+Enter is a newline, as in every search box of
            // the period that accepted more than one line.
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              if (canAsk) submit();
            }
          }}
        />

        <div className="ai-panel__controls">
          {ai.busy ? (
            <Button onClick={() => void ai.stop()}>Stop</Button>
          ) : (
            <Button onClick={submit} disabled={!canAsk}>
              Ask Mushroom
            </Button>
          )}

          <div className="ai-panel__history">
            <Button
              className="ai-panel__history-toggle"
              aria-label="Recent questions"
              aria-expanded={showHistory}
              disabled={ai.history.length === 0}
              onClick={() => setShowHistory((open) => !open)}
            >
              ▾
            </Button>
            {showHistory ? (
              <div className="ai-panel__history-menu" role="menu">
                {ai.history.map((question) => (
                  <button
                    key={question}
                    type="button"
                    role="menuitem"
                    className="menuitem"
                    onClick={() => {
                      ai.setQuestion(question);
                      setShowHistory(false);
                      fieldRef.current?.focus();
                    }}
                  >
                    <span className="menuitem__label">{question}</span>
                  </button>
                ))}
                <div className="menu__separator" role="separator" />
                <button
                  type="button"
                  role="menuitem"
                  className="menuitem"
                  onClick={() => {
                    void ai.clearHistory();
                    setShowHistory(false);
                  }}
                >
                  <span className="menuitem__label">Clear history</span>
                </button>
              </div>
            ) : null}
          </div>
        </div>
      </div>

      <div className="ai-panel__result">
        <RetrievedNotes
          stage={ai.stage}
          passages={ai.retrieved}
          terms={ai.terms}
          onOpen={open}
        />

        {unavailable ? (
          <div className="ai-panel__unavailable">
            That note is no longer at <code>{unavailable}</code>. It may have
            been renamed, moved or deleted since this answer was written.
          </div>
        ) : null}

        {ai.error ? (
          <div className="ai-panel__error">
            <div className="ai-panel__error-title">{ai.error.title}</div>
            <div>{ai.error.message}</div>
            {ai.error.hint ? (
              <div className="ai-panel__error-hint">{ai.error.hint}</div>
            ) : null}
          </div>
        ) : null}

        <AnswerView
          stage={ai.stage}
          text={ai.text}
          answer={ai.answer}
          incomplete={ai.incomplete}
          onOpenCitation={openCitation}
        />

        {ai.answer && !ai.answer.noResults ? (
          <SourceList
            used={ai.answer.grounding.used}
            alsoSearched={ai.retrieved.filter(
              (p) =>
                !ai.answer!.grounding.used.some((c) => c.noteId === p.noteId),
            )}
            onOpen={open}
          />
        ) : null}
      </div>
    </div>
  );
}
