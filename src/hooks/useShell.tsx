import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";

/**
 * Shell state: what is visible, what the status bar says, which menu is open.
 *
 * Deliberately small. Note, search, and AI state live in their own hooks in
 * later milestones — this only owns the window furniture.
 */

export type PanelKey = "notes" | "search" | "ai";

export type ShellState = {
  /** Status bar, left cell: context such as "12 notes". */
  context: string;
  /** Status bar, right cell: "Ready", "Indexing…", "Saved". */
  status: string;
  showStatusBar: boolean;
  panels: Record<PanelKey, boolean>;
};

type ShellApi = ShellState & {
  setContext: (text: string) => void;
  /** Set the status cell. With `ms`, reverts to the resting status after. */
  setStatus: (text: string, ms?: number) => void;
  togglePanel: (key: PanelKey) => void;
  toggleStatusBar: () => void;
};

const RESTING_STATUS = "Ready";

const ShellContext = createContext<ShellApi | null>(null);

export function ShellProvider({
  children,
  initial,
}: {
  children: ReactNode;
  initial?: Partial<ShellState>;
}) {
  const [context, setContext] = useState(initial?.context ?? "0 notes");
  const [status, setStatusRaw] = useState(initial?.status ?? RESTING_STATUS);
  const [showStatusBar, setShowStatusBar] = useState(
    initial?.showStatusBar ?? true,
  );
  const [panels, setPanels] = useState<Record<PanelKey, boolean>>({
    notes: initial?.panels?.notes ?? true,
    search: initial?.panels?.search ?? false,
    ai: initial?.panels?.ai ?? false,
  });

  const revertTimer = useRef<number | undefined>(undefined);

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
    setPanels((prev) => ({ ...prev, [key]: !prev[key] }));
  }, []);

  const toggleStatusBar = useCallback(() => {
    setShowStatusBar((prev) => !prev);
  }, []);

  const value = useMemo<ShellApi>(
    () => ({
      context,
      status,
      showStatusBar,
      panels,
      setContext,
      setStatus,
      togglePanel,
      toggleStatusBar,
    }),
    [
      context,
      status,
      showStatusBar,
      panels,
      setStatus,
      togglePanel,
      toggleStatusBar,
    ],
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
