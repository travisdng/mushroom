/** Mirrors src-tauri/src/diagnostics/mod.rs. */

export type LogLine = {
  timestamp: string;
  level: string;
  category: string;
  message: string;
};

export type AppInfo = {
  version: string;
  platform: string;
  dataDir: string;
  logDir: string;
  notesRoot: string | null;
  notesCount: number;
  notesSkipped: number;
  /** False when the notes folder could not be watched; F5 still refreshes. */
  watching: boolean;
  configVersion: number;
};

export type IndexInfo = {
  databasePath: string | null;
  noteCount: number;
  passageCount: number;
  stale: boolean;
  indexing: boolean;
  skipped: number;
  lastFullRebuild: string | null;
};

export type AiInfo = {
  provider: string;
  endpoint: string;
  model: string;
  keyStatus: string;
  configured: boolean;
  streaming: boolean;
  lastConnection: string | null;
  requests: number;
  totalTokens: number;
  failures: number;
};

export type Diagnostics = {
  app: AppInfo;
  index: IndexInfo;
  ai: AiInfo;
};

/** The log categories the backend uses as tracing targets. */
export const LOG_CATEGORIES = ["app", "files", "db", "search", "ai", "net"] as const;
export const LOG_LEVELS = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] as const;
