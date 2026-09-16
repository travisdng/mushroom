/** Mirrors `UiState` / `AppConfig` in src-tauri/src/config/mod.rs. */

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
};
