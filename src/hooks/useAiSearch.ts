import { useCallback, useEffect, useRef, useState } from "react";

import * as ai from "../services/aiSearchService";
import { inDesktopApp, toAppError } from "../services/ipc";
import type {
  AiAnswer,
  AiDelta,
  AiStage,
  RetrievedPassage,
} from "../types/aiSearch";
import type { AppErrorDto } from "../types/error";

/**
 * The question panel's state.
 *
 * The stage is explicit rather than derived from a handful of booleans — a
 * spinner that never stops is almost always two flags disagreeing.
 */
export type AiSearchApi = {
  stage: AiStage;
  question: string;
  /** The text streamed so far. Kept on failure, so a partial answer survives. */
  text: string;
  answer: AiAnswer | null;
  retrieved: RetrievedPassage[];
  terms: string[];
  model: string | null;
  error: AppErrorDto | null;
  /** True when the stream stopped before the answer finished. */
  incomplete: boolean;
  history: string[];
  busy: boolean;

  setQuestion: (value: string) => void;
  ask: (folder?: string | null) => Promise<void>;
  stop: () => Promise<void>;
  reset: () => void;
  clearHistory: () => Promise<void>;
};

function newRequestId(): string {
  return `q-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function useAiSearch(): AiSearchApi {
  const [stage, setStage] = useState<AiStage>("idle");
  const [question, setQuestion] = useState("");
  const [text, setText] = useState("");
  const [answer, setAnswer] = useState<AiAnswer | null>(null);
  const [retrieved, setRetrieved] = useState<RetrievedPassage[]>([]);
  const [terms, setTerms] = useState<string[]>([]);
  const [model, setModel] = useState<string | null>(null);
  const [error, setError] = useState<AppErrorDto | null>(null);
  const [incomplete, setIncomplete] = useState(false);
  const [history, setHistory] = useState<string[]>([]);

  /** The request whose deltas we are still willing to accept. */
  const currentId = useRef<string | null>(null);
  /** Set when the user pressed Stop, to tell cancellation from failure. */
  const stopped = useRef(false);
  /**
   * Whether any text arrived for the current request.
   *
   * A ref, not the `text` state: the catch below runs in a closure created
   * when the question was asked, where `text` was still empty — so reading the
   * state there always said "nothing arrived" and a broken-off answer was
   * never marked incomplete.
   */
  const receivedText = useRef(false);

  useEffect(() => {
    if (!inDesktopApp()) return;
    ai.getHistory().then(setHistory).catch(() => {
      // A missing history is not worth reporting; the field still works.
    });
  }, []);

  const busy =
    stage === "retrieving" || stage === "asking" || stage === "streaming";

  const ask = useCallback(
    async (folder: string | null = null) => {
      const asked = question.trim();
      if (!asked || busy) return;

      const id = newRequestId();
      currentId.current = id;
      stopped.current = false;
      receivedText.current = false;

      setStage("retrieving");
      setText("");
      setAnswer(null);
      setRetrieved([]);
      setTerms([]);
      setModel(null);
      setError(null);
      setIncomplete(false);

      const onDelta = (delta: AiDelta) => {
        // A delta from a superseded request must not touch this answer.
        if (currentId.current !== id) return;

        switch (delta.kind) {
          case "retrieved":
            setRetrieved(delta.passages);
            setTerms(delta.terms);
            setStage("asking");
            break;
          case "started":
            setModel(delta.model);
            setStage("asking");
            break;
          case "text":
            receivedText.current = true;
            setStage("streaming");
            setText((current) => current + delta.delta);
            break;
          case "done":
            setAnswer(delta.answer);
            // The assembled answer is authoritative; the deltas were a preview.
            setText(delta.answer.text);
            setStage("done");
            break;
          case "failed":
            // Belt and braces: the backend does not send this for a
            // cancellation, and if it ever did it would still not be an error
            // the user needs telling about.
            if (stopped.current) break;
            setError(delta.error);
            setStage("error");
            break;
        }
      };

      try {
        await ai.askQuestion(asked, folder, id, onDelta);
        setHistory((current) => [
          asked,
          ...current.filter((q) => q !== asked),
        ].slice(0, 20));
      } catch (raw) {
        if (currentId.current !== id) return;

        if (stopped.current) {
          // Whatever arrived before Stop is kept and marked (R6.2).
          setStage("cancelled");
          setIncomplete(true);
        } else {
          setError(toAppError(raw));
          setStage("error");
          // Text already on screen stays; it is real, just unfinished.
          setIncomplete(receivedText.current);
        }
      }
    },
    [busy, question],
  );

  const stop = useCallback(async () => {
    const id = currentId.current;
    if (!id) return;
    stopped.current = true;
    setIncomplete(true);
    setStage("cancelled");
    await ai.cancelQuestion(id).catch(() => {
      // Already finished; nothing to stop.
    });
  }, []);

  const reset = useCallback(() => {
    // Abandon any in-flight request rather than letting it write here later.
    currentId.current = null;
    setStage("idle");
    setText("");
    setAnswer(null);
    setRetrieved([]);
    setTerms([]);
    setModel(null);
    setError(null);
    setIncomplete(false);
  }, []);

  const clearHistory = useCallback(async () => {
    await ai.clearHistory();
    setHistory([]);
  }, []);

  return {
    stage,
    question,
    text,
    answer,
    retrieved,
    terms,
    model,
    error,
    incomplete,
    history,
    busy,
    setQuestion,
    ask,
    stop,
    reset,
    clearHistory,
  };
}
