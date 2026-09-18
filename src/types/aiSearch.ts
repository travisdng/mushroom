/** Mirrors the AI search types in src-tauri/src/ai/search.rs. */

import type { AppErrorDto } from "./error";
import type { PrivacyReport } from "./ai";

export type RetrievedPassage = {
  noteId: string;
  noteTitle: string;
  folder: string;
  headingPath: string;
  lineStart: number;
  lineEnd: number;
  text: string;
  /** Normalised 0..1 within this result set. */
  score: number;
  source: "keyword" | "semantic" | "hybrid";
};

/** What a `[n]` marker resolves to. */
export type Citation = {
  number: number;
  noteId: string;
  noteTitle: string;
  folder: string;
  headingPath: string;
  lineStart: number;
  lineEnd: number;
  /** The excerpt exactly as it was sent to the model. */
  text: string;
};

export type Grounding = {
  used: Citation[];
  /** Numbers the model cited that were never sent. */
  unmatched: number[];
  /** The model cited nothing at all. */
  uncited: boolean;
  /** Sent but not cited — the "Also searched" list. */
  unused: Citation[];
};

export type Usage = {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
};

export type AiAnswer = {
  text: string;
  model: string;
  usage: Usage | null;
  grounding: Grounding;
  dropped: number;
  oversized: boolean;
  terms: string[];
  noResults: boolean;
  /** The model was asked and said it could not answer from the excerpts. */
  declined: boolean;
  estimatedPromptTokens: number;
  latencyMs: number;
  /** Notes that matched but are excluded from AI by the user. */
  excludedNotes: number;
  /** What the privacy gate changed before sending. Null if nothing was sent. */
  privacy: PrivacyReport | null;
};

/** Serde tags these with `kind`. */
export type AiDelta =
  | {
      kind: "retrieved";
      passages: RetrievedPassage[];
      terms: string[];
      excludedNotes: number;
    }
  | { kind: "started"; model: string }
  | { kind: "text"; delta: string }
  | { kind: "done"; answer: AiAnswer }
  | { kind: "failed"; error: AppErrorDto };

/**
 * The panel's explicit stages. Named rather than inferred from a handful of
 * booleans, because that is how spinners get stuck on.
 */
export type AiStage =
  | "idle"
  | "retrieving"
  | "asking"
  | "streaming"
  | "done"
  | "cancelled"
  | "error";

export type UsageRecord = {
  at: string;
  model: string;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  latencyMs: number;
  outcome: string;
};

export type UsageStats = {
  requests: number;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  failures: number;
  recent: UsageRecord[];
};
