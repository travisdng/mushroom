import { Channel } from "@tauri-apps/api/core";

import { call } from "./ipc";
import type { AiDelta, UsageStats } from "../types/aiSearch";

/**
 * Ask a question of the notes.
 *
 * Progress arrives on a channel created per request, not on a global event, so
 * two panels — or a question the user has already replaced — cannot write into
 * each other's answer.
 */
export function askQuestion(
  question: string,
  folder: string | null,
  requestId: string,
  onDelta: (delta: AiDelta) => void,
): Promise<void> {
  const channel = new Channel<AiDelta>();
  channel.onmessage = onDelta;

  return call<void>("ai_search", {
    question,
    folder,
    requestId,
    channel,
  });
}

export function cancelQuestion(requestId: string): Promise<boolean> {
  return call<boolean>("ai_search_cancel", { requestId });
}

export function getHistory(): Promise<string[]> {
  return call<string[]>("get_ai_history");
}

export function clearHistory(): Promise<void> {
  return call<void>("clear_ai_history");
}

export function getUsageStats(): Promise<UsageStats> {
  return call<UsageStats>("get_ai_usage_stats");
}
