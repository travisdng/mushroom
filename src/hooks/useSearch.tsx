import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import * as searchService from "../services/searchService";
import { inDesktopApp } from "../services/ipc";
import type { IndexProgress, IndexStats, SearchResults } from "../types/search";

/** Queries kept for the classic history drop-down (R3.10). */
const HISTORY_LIMIT = 20;
const DEBOUNCE_MS = 200;

type SearchApi = {
  query: string;
  results: SearchResults | null;
  searching: boolean;
  stats: IndexStats | null;
  progress: IndexProgress | null;
  history: string[];
  scopeFolder: string | null;

  setQuery: (value: string) => void;
  setScopeFolder: (folder: string | null) => void;
  run: (value?: string) => Promise<void>;
  clear: () => void;
  rebuild: () => Promise<void>;
  cancelRebuild: () => Promise<void>;
  refreshStats: () => Promise<void>;
};

const SearchContext = createContext<SearchApi | null>(null);

export function SearchProvider({ children }: { children: ReactNode }) {
  const [query, setQueryState] = useState("");
  const [results, setResults] = useState<SearchResults | null>(null);
  const [searching, setSearching] = useState(false);
  const [stats, setStats] = useState<IndexStats | null>(null);
  const [progress, setProgress] = useState<IndexProgress | null>(null);
  const [history, setHistory] = useState<string[]>([]);
  const [scopeFolder, setScopeFolder] = useState<string | null>(null);

  const debounce = useRef<number | undefined>(undefined);
  /** Only the newest query may write results; older ones are discarded. */
  const generation = useRef(0);

  const refreshStats = useCallback(async () => {
    if (!inDesktopApp()) return;
    try {
      setStats(await searchService.getIndexStats());
    } catch {
      // Stats are decoration; never interrupt the user for them.
    }
  }, []);

  useEffect(() => {
    if (!inDesktopApp()) return;
    void refreshStats();

    let stopReady: (() => void) | undefined;
    let stopProgress: (() => void) | undefined;

    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      stopReady = await listen("index-ready", () => void refreshStats());
      stopProgress = await listen<IndexProgress>("index-progress", (e) => {
        setProgress(e.payload);
      });
    })();

    return () => {
      stopReady?.();
      stopProgress?.();
    };
  }, [refreshStats]);

  const run = useCallback(
    async (value?: string) => {
      const text = (value ?? query).trim();
      if (!text) {
        setResults(null);
        return;
      }

      const mine = ++generation.current;
      setSearching(true);
      try {
        const found = await searchService.searchNotes(text, scopeFolder, null);
        // A slower earlier query must not overwrite a newer one's results.
        if (mine !== generation.current) return;
        setResults(found);
        setHistory((prev) =>
          [text, ...prev.filter((h) => h !== text)].slice(0, HISTORY_LIMIT),
        );
      } catch {
        if (mine === generation.current) setResults(null);
      } finally {
        if (mine === generation.current) setSearching(false);
      }
    },
    [query, scopeFolder],
  );

  const setQuery = useCallback(
    (value: string) => {
      setQueryState(value);
      window.clearTimeout(debounce.current);
      if (!value.trim()) {
        setResults(null);
        return;
      }
      debounce.current = window.setTimeout(() => void run(value), DEBOUNCE_MS);
    },
    [run],
  );

  const clear = useCallback(() => {
    window.clearTimeout(debounce.current);
    generation.current += 1;
    setQueryState("");
    setResults(null);
  }, []);

  const rebuild = useCallback(async () => {
    setProgress({ done: 0, total: 0, skipped: 0 });
    try {
      await searchService.rebuildIndex();
    } finally {
      setProgress(null);
      await refreshStats();
      if (query.trim()) await run();
    }
  }, [query, run, refreshStats]);

  const cancelRebuild = useCallback(async () => {
    await searchService.cancelRebuild();
  }, []);

  const value = useMemo<SearchApi>(
    () => ({
      query,
      results,
      searching,
      stats,
      progress,
      history,
      scopeFolder,
      setQuery,
      setScopeFolder,
      run,
      clear,
      rebuild,
      cancelRebuild,
      refreshStats,
    }),
    [
      query,
      results,
      searching,
      stats,
      progress,
      history,
      scopeFolder,
      setQuery,
      run,
      clear,
      rebuild,
      cancelRebuild,
      refreshStats,
    ],
  );

  return (
    <SearchContext.Provider value={value}>{children}</SearchContext.Provider>
  );
}

export function useSearch(): SearchApi {
  const ctx = useContext(SearchContext);
  if (!ctx) throw new Error("useSearch must be used inside a SearchProvider.");
  return ctx;
}
