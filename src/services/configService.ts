import { call } from "./ipc";
import type { AppConfig, UiState } from "../types/config";

export function getConfig(): Promise<AppConfig> {
  return call<AppConfig>("get_config");
}

export function setUiState(ui: UiState): Promise<void> {
  return call<void>("set_ui_state", { ui });
}

/** Remember that a note was opened, for Quick Open's recency ranking. */
export function recordRecentNote(id: string): Promise<void> {
  return call<void>("record_recent_note", { id });
}
