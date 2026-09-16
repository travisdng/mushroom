import { useCallback, useEffect, useRef, useState } from "react";

import { AiUnconfigured } from "../common/AiUnconfigured";
import { Button } from "../common/Button";
import { TextArea } from "../common/Field";
import { AnswerView } from "./AnswerView";
import { RetrievedNotes } from "./RetrievedNotes";
import { SourceList } from "./SourceList";
import { useAiSearch } from "../../hooks/useAiSearch";
import { useNotes } from "../../hooks/useNotes";
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
}: {
  onOpenSettings: () => void;
}) {
  const ai = useAiSearch();
  const { openNote } = useNotes();
  const [configured, setConfigured] = useState<boolean | null>(null);
  const [showHistory, setShowHistory] = useState(false);
  const fieldRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!inDesktopApp()) {
      setConfigured(false);
      return;
    }
    getAiSettings()
      .then((settings) => setConfigured(settings.config.baseUrl.trim() !== ""))
      // If the settings cannot be read, assume configured rather than showing
      // "set me up" over a working install.
      .catch(() => setConfigured(true));
  }, []);

  const open = useCallback(
    (noteId: string, lineStart: number) => {
      void openNote(noteId, lineStart);
    },
    [openNote],
  );

  const openCitation = useCallback(
    (citation: Citation) => open(citation.noteId, citation.lineStart),
    [open],
  );

  const submit = useCallback(() => {
    void ai.ask();
  }, [ai]);

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
