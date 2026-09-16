/** Mirrors the search types in src-tauri/src/search/service.rs. */

export type SearchHit = {
  id: string;
  title: string;
  folder: string;
  modified: number;
  snippet: string;
  headingPath: string;
  /** 1-based line in the original Markdown, for open-to-passage. */
  lineStart: number;
  score: number;
};

export type SearchResults = {
  hits: SearchHit[];
  /** What was actually searched for, after stop words and operators. */
  terms: string[];
  stale: boolean;
};

export type IndexStats = {
  noteCount: number;
  passageCount: number;
  databaseBytes: number;
  lastFullRebuild: string | null;
  skipped: number;
  stale: boolean;
  indexing: boolean;
};

export type IndexProgress = {
  done: number;
  total: number;
  skipped: number;
};
