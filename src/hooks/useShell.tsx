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
import { getConfig, setUiState } from "../services/configService";
import { inDesktopApp } from "../services/ipc";
import type { UiState } from "../types/config";

/**
 * Shell state: what is visible, how the panes are sized, what the status bar
 * says. Owns the persisted `UiState` so saving lives in exactly one place.
 *
 * Note, search, and AI state live in their own hooks in later milestones.
 */

export type PanelKey = "notes" | "search" | "ai";

const DEFAULT_UI: UiState = {
  window: null,
  sidebarWidth: 240,
  notebookHeight: 220,
  aiWidth: 320,
  showStatusBar: true,
  showNotesPanel: true,
  showSearchPanel: false,
  showAiPanel: false,
};

const PANEL_FIELD: Record<PanelKey, keyof UiState> = {
  notes: "showNotesPanel",
  search: "showSearchPanel",
  ai: "showAiPanel",
};

type ShellApi = {
  context: string;
  status: string;
  ui: UiState;
  setContext: (text: string) => void;
  /** Set the status cell. With `ms`, reverts to the resting status after. */
  setStatus: (text: string, ms?: number) => void;
  togglePanel: (key: PanelKey) => void;
  toggleStatusBar: () => void;
  setSizes: (patch: Partial<UiState>) => void;
};

const RESTING_STATUS = "Ready";
const SAVE_DEBOUNCE_MS = 500;

const ShellContext = createContext<ShellApi | null>(null);

export function ShellProvider({ children }: { children: ReactNode }) {
  const [context, setContext] = useState("0 notes");
  const [status, setStatusRaw] = useState(RESTING_STATUS);
  const [ui, setUi] = useState<UiState>(DEFAULT_UI);

  const revertTimer = useRef<number | undefined>(undefined);
  const saveTimer = useRef<number | undefined>(undefined);
  // Suppresses the save that would otherwise fire from the initial load.
  const loaded = useRef(false);

  // Restore persisted geometry and panel visibility (R1.5, R4.7).
  useEffect(() => {
    if (!inDesktopApp()) {
      loaded.current = true;
      return;
    }
    let cancelled = false;
    getConfig()
      .then((config) => {
        if (!cancelled) setUi({ ...DEFAULT_UI, ...config.ui });
      })
      .catch(() => {
        // Settings are not worth interrupting startup for; defaults are fine.
      })
      .finally(() => {
        loaded.current = true;
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Persist, debounced, so dragging a splitter does not write on every frame.
  useEffect(() => {
    if (!loaded.current || !inDesktopApp()) return;
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      void setUiState(ui).catch(() => {
        // A failed settings write must not disturb the user mid-edit.
      });
    }, SAVE_DEBOUNCE_MS);
    return () => window.clearTimeout(saveTimer.current);
  }, [ui]);

  const setStatus = useCallback((text: string, ms?: number) => {
    window.clearTimeout(revertTimer.current);
    setStatusRaw(text);
    if (ms !== undefined) {
      revertTimer.current = window.setTimeout(
        () => setStatusRaw(RESTING_STATUS),
        ms,
      );
    }
  }, []);

  const togglePanel = useCallback((key: PanelKey) => {
    const field = PANEL_FIELD[key];
    setUi((prev) => ({ ...prev, [field]: !prev[field] }));
  }, []);

  const toggleStatusBar = useCallback(() => {
    setUi((prev) => ({ ...prev, showStatusBar: !prev.showStatusBar }));
  }, []);

  const setSizes = useCallback((patch: Partial<UiState>) => {
    setUi((prev) => ({ ...prev, ...patch }));
  }, []);

  const value = useMemo<ShellApi>(
    () => ({
      context,
      status,
      ui,
      setContext,
      setStatus,
      togglePanel,
      toggleStatusBar,
      setSizes,
    }),
    [context, status, ui, setStatus, togglePanel, toggleStatusBar, setSizes],
  );

  return (
    <ShellContext.Provider value={value}>{children}</ShellContext.Provider>
  );
}

export function useShell(): ShellApi {
  const ctx = useContext(ShellContext);
  if (!ctx) {
    throw new Error("useShell must be used inside a ShellProvider.");
  }
  return ctx;
}

/** Shown when a shortcut or button has no feature behind it yet (R6.2). */
export const NOT_AVAILABLE = "Not available yet";
