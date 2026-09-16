/** Mirrors `UiState` / `AppConfig` in src-tauri/src/config/mod.rs. */

import type { AiConfig } from "./ai";

export type WindowRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type UiState = {
  window: WindowRect | null;
  sidebarWidth: number;
  notebookHeight: number;
  aiWidth: number;
  showStatusBar: boolean;
  showNotesPanel: boolean;
  showSearchPanel: boolean;
  showAiPanel: boolean;
};

export type AppConfig = {
  version: number;
  ui: UiState;
  /** Null until first run resolves the default. */
  notesRoot: string | null;
  ai: AiConfig;
  aiHistory: string[];
  /** The last 20 notes opened, most recent first. */
  recentNotes: string[];
};
