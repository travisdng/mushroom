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
import * as notesService from "../services/notesService";
import { inDesktopApp } from "../services/ipc";
import type {
  FolderNode,
  NoteContent,
  NoteMeta,
  NotesStatus,
} from "../types/notes";
import type { AppErrorDto } from "../types/error";

/**
 * Notes state: the folder tree, the note list, the open document, and its
 * dirty/saving lifecycle.
 *
 * The editor buffer lives here rather than in the editor component so that
 * switching notes can flush a pending save before unmounting it.
 */

const AUTOSAVE_MS = 3000;

export type ConflictState = {
  id: string;
  /** What the user has in the editor, which must not be lost. */
  mine: string;
  diskModified: number;
};

type NotesApi = {
  status: NotesStatus | null;
  tree: FolderNode | null;
  notes: NoteMeta[];
  selectedFolder: string | null;
  open: NoteContent | null;
  body: string;
  dirty: boolean;
  saving: boolean;
  lastSavedAt: number | null;
  error: AppErrorDto | null;
  conflict: ConflictState | null;

  selectFolder: (folder: string | null) => void;
  openNote: (id: string, line?: number) => Promise<void>;
  setBody: (value: string) => void;
  save: () => Promise<void>;
  createNote: (folder: string, title: string) => Promise<NoteMeta | null>;
  renameNote: (id: string, title: string) => Promise<void>;
  moveNote: (id: string, folder: string) => Promise<void>;
  deleteNote: (id: string) => Promise<void>;
  createFolder: (parent: string, name: string) => Promise<void>;
  deleteFolder: (folder: string) => Promise<void>;
  refresh: () => Promise<void>;
  resolveConflict: (choice: "mine" | "theirs" | "copy") => Promise<void>;
  dismissError: () => void;
  /** Line the editor should scroll to after opening, then cleared. */
  pendingLine: number | null;
  clearPendingLine: () => void;
};

const NotesContext = createContext<NotesApi | null>(null);

export function NotesProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<NotesStatus | null>(null);
  const [tree, setTree] = useState<FolderNode | null>(null);
  const [notes, setNotes] = useState<NoteMeta[]>([]);
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
  const [open, setOpen] = useState<NoteContent | null>(null);
  const [body, setBodyState] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [lastSavedAt, setLastSavedAt] = useState<number | null>(null);
  const [error, setError] = useState<AppErrorDto | null>(null);
  const [conflict, setConflict] = useState<ConflictState | null>(null);
  const [pendingLine, setPendingLine] = useState<number | null>(null);

  const saveTimer = useRef<number | undefined>(undefined);
  // Read inside async callbacks that must not close over stale state.
  const latest = useRef({ open, body, dirty });
  latest.current = { open, body, dirty };

  const reportError = useCallback((e: unknown) => {
    setError(e as AppErrorDto);
  }, []);

  const loadList = useCallback(
    async (folder: string | null) => {
      try {
        const [list, folders, st] = await Promise.all([
          notesService.listNotes(folder),
          notesService.getFolderTree(),
          notesService.getNotesStatus(),
        ]);
        setNotes(list);
        setTree(folders);
        setStatus(st);
      } catch (e) {
        reportError(e);
      }
    },
    [reportError],
  );

  // Initial load, and again when the backend finishes its startup scan.
  useEffect(() => {
    if (!inDesktopApp()) return;
    void loadList(null);

    let unlisten: (() => void) | undefined;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const stop = await listen("notes-ready", () => {
        void loadList(null);
      });
      unlisten = stop;
    })();

    return () => unlisten?.();
  }, [loadList]);

  /** Write the buffer if it differs from disk. Returns false on conflict. */
  const flush = useCallback(async (): Promise<boolean> => {
    const current = latest.current;
    if (!current.open || !current.dirty) return true;

    setSaving(true);
    try {
      const meta = await notesService.saveNote(
        current.open.meta.id,
        current.body,
        current.open.diskModified,
      );
      // Re-read to pick up the new mtime, otherwise the next save looks like
      // an external edit and trips the conflict check against ourselves.
      const fresh = await notesService.readNote(meta.id);
      setOpen(fresh);
      setDirty(false);
      setLastSavedAt(Date.now());
      setNotes((prev) => prev.map((n) => (n.id === meta.id ? meta : n)));
      return true;
    } catch (e) {
      const err = e as AppErrorDto;
      if (err.code === "NOTE_CHANGED_ON_DISK") {
        setConflict({
          id: current.open.meta.id,
          mine: current.body,
          diskModified: current.open.diskModified,
        });
      } else {
        reportError(err);
      }
      return false;
    } finally {
      setSaving(false);
    }
  }, [reportError]);

  // Debounced autosave (R4.5).
  useEffect(() => {
    if (!dirty) return;
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => void flush(), AUTOSAVE_MS);
    return () => window.clearTimeout(saveTimer.current);
  }, [dirty, body, flush]);

  // Also flush when the window loses focus or is closing, so work in progress
  // is not lost to a crash or a distracted user.
  useEffect(() => {
    const onBlur = () => void flush();
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, [flush]);

  const selectFolder = useCallback(
    (folder: string | null) => {
      setSelectedFolder(folder);
      void loadList(folder);
    },
    [loadList],
  );

  const openNote = useCallback(
    async (id: string, line?: number) => {
      // Never switch away from unsaved work without writing it first (R4.7).
      if (!(await flush())) return;
      try {
        const content = await notesService.readNote(id);
        setOpen(content);
        setBodyState(content.body);
        setDirty(false);
        setPendingLine(line ?? null);
      } catch (e) {
        reportError(e);
      }
    },
    [flush, reportError],
  );

  const setBody = useCallback((value: string) => {
    setBodyState(value);
    setDirty(true);
  }, []);

  const save = useCallback(async () => {
    await flush();
  }, [flush]);

  const createNote = useCallback(
    async (folder: string, title: string) => {
      if (!(await flush())) return null;
      try {
        const meta = await notesService.createNote(folder, title);
        await loadList(selectedFolder);
        const content = await notesService.readNote(meta.id);
        setOpen(content);
        setBodyState(content.body);
        setDirty(false);
        return meta;
      } catch (e) {
        reportError(e);
        return null;
      }
    },
    [flush, loadList, reportError, selectedFolder],
  );

  const renameNote = useCallback(
    async (id: string, title: string) => {
      try {
        const meta = await notesService.renameNote(id, title);
        await loadList(selectedFolder);
        if (latest.current.open?.meta.id === id) {
          const content = await notesService.readNote(meta.id);
          setOpen(content);
          setBodyState(content.body);
          setDirty(false);
        }
      } catch (e) {
        reportError(e);
      }
    },
    [loadList, reportError, selectedFolder],
  );

  const moveNote = useCallback(
    async (id: string, folder: string) => {
      try {
        const meta = await notesService.moveNote(id, folder);
        await loadList(selectedFolder);
        if (latest.current.open?.meta.id === id) {
          setOpen(await notesService.readNote(meta.id));
        }
      } catch (e) {
        reportError(e);
      }
    },
    [loadList, reportError, selectedFolder],
  );

  const deleteNote = useCallback(
    async (id: string) => {
      try {
        await notesService.deleteNote(id);
        if (latest.current.open?.meta.id === id) {
          setOpen(null);
          setBodyState("");
          setDirty(false);
        }
        await loadList(selectedFolder);
      } catch (e) {
        reportError(e);
      }
    },
    [loadList, reportError, selectedFolder],
  );

  const createFolder = useCallback(
    async (parent: string, name: string) => {
      try {
        await notesService.createFolder(parent, name);
        await loadList(selectedFolder);
      } catch (e) {
        reportError(e);
      }
    },
    [loadList, reportError, selectedFolder],
  );

  const deleteFolder = useCallback(
    async (folder: string) => {
      try {
        await notesService.deleteFolder(folder);
        setSelectedFolder(null);
        await loadList(null);
      } catch (e) {
        reportError(e);
      }
    },
    [loadList, reportError],
  );

  const refresh = useCallback(async () => {
    try {
      await notesService.refreshNotes();
      await loadList(selectedFolder);
    } catch (e) {
      reportError(e);
    }
  }, [loadList, reportError, selectedFolder]);

  /** Keep mine / load theirs / save a copy (R9.3). */
  const resolveConflict = useCallback(
    async (choice: "mine" | "theirs" | "copy") => {
      const pending = conflict;
      if (!pending) return;
      setConflict(null);

      try {
        if (choice === "theirs") {
          const fresh = await notesService.readNote(pending.id);
          setOpen(fresh);
          setBodyState(fresh.body);
          setDirty(false);
          return;
        }

        if (choice === "mine") {
          // Pass null to skip the mtime check: the user has been shown the
          // conflict and chosen deliberately.
          const meta = await notesService.saveNote(pending.id, pending.mine, null);
          setOpen(await notesService.readNote(meta.id));
          setDirty(false);
          setLastSavedAt(Date.now());
          return;
        }

        const folder = pending.id.includes("/")
          ? pending.id.slice(0, pending.id.lastIndexOf("/"))
          : "";
        const base = pending.id.split("/").pop()?.replace(/\.md$/, "") ?? "note";
        const copy = await notesService.createNote(folder, `${base} (my copy)`);
        await notesService.saveNote(copy.id, pending.mine, null);
        await loadList(selectedFolder);
        setOpen(await notesService.readNote(copy.id));
        setDirty(false);
      } catch (e) {
        reportError(e);
      }
    },
    [conflict, loadList, reportError, selectedFolder],
  );

  const value = useMemo<NotesApi>(
    () => ({
      status,
      tree,
      notes,
      selectedFolder,
      open,
      body,
      dirty,
      saving,
      lastSavedAt,
      error,
      conflict,
      selectFolder,
      openNote,
      setBody,
      save,
      createNote,
      renameNote,
      moveNote,
      deleteNote,
      createFolder,
      deleteFolder,
      refresh,
      resolveConflict,
      dismissError: () => setError(null),
      pendingLine,
      clearPendingLine: () => setPendingLine(null),
    }),
    [
      status,
      tree,
      notes,
      selectedFolder,
      open,
      body,
      dirty,
      saving,
      lastSavedAt,
      error,
      conflict,
      pendingLine,
      selectFolder,
      openNote,
      setBody,
      save,
      createNote,
      renameNote,
      moveNote,
      deleteNote,
      createFolder,
      deleteFolder,
      refresh,
      resolveConflict,
    ],
  );

  return (
    <NotesContext.Provider value={value}>{children}</NotesContext.Provider>
  );
}

export function useNotes(): NotesApi {
  const ctx = useContext(NotesContext);
  if (!ctx) throw new Error("useNotes must be used inside a NotesProvider.");
  return ctx;
}
