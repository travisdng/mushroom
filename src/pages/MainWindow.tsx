import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { MenuBar } from "../components/chrome/MenuBar";
import type { MenuDef } from "../components/chrome/MenuBar";
import { Toolbar } from "../components/chrome/Toolbar";
import type { ToolbarAction } from "../components/chrome/Toolbar";
import { StatusBar } from "../components/chrome/StatusBar";
import { Panel } from "../components/common/Panel";
import { Splitter } from "../components/common/Splitter";
import { EmptyState } from "../components/common/EmptyState";
import { AboutDialog } from "../components/common/AboutDialog";
import { ShortcutsDialog } from "../components/common/ShortcutsDialog";
import { FolderTree } from "../components/notebook/FolderTree";
import { NoteList } from "../components/notebook/NoteList";
import {
  ConfirmDeleteDialog,
  ConflictDialog,
  MoveDialog,
  NoteErrorDialog,
  PromptDialog,
} from "../components/notebook/NoteDialogs";
import { EditorPane } from "../components/editor/EditorPane";
import { SearchPanel } from "../components/search/SearchPanel";
import { RebuildDialog } from "../components/search/RebuildDialog";
import { SettingsDialog } from "../components/settings/SettingsDialog";
import type { ViewMode } from "../components/editor/EditorPane";
import { useShell, NOT_AVAILABLE } from "../hooks/useShell";
import { useNotes } from "../hooks/useNotes";
import { useSearch } from "../hooks/useSearch";
import { useShortcuts } from "../hooks/useShortcuts";

const SIDEBAR_MIN = 160;
const EDITOR_MIN = 320;
const AI_MIN = 240;
const NOTEBOOK_MIN = 80;

type DialogKind =
  | { kind: "about" }
  | { kind: "shortcuts" }
  | { kind: "new-note"; folder: string }
  | { kind: "new-folder"; parent: string }
  | { kind: "rename"; id: string; title: string }
  | { kind: "move"; id: string; title: string }
  | { kind: "delete-note"; id: string; title: string }
  | { kind: "rebuild" }
  | { kind: "settings" }
  | null;

export default function MainWindow() {
  const shell = useShell();
  const notes = useNotes();
  const search = useSearch();
  const [dialog, setDialog] = useState<DialogKind>(null);
  const [mode, setMode] = useState<ViewMode>("edit");
  const { sidebarWidth, notebookHeight, aiWidth } = shell.ui;
  const workAreaRef = useRef<HTMLDivElement>(null);
  const sidebarRef = useRef<HTMLDivElement>(null);

  // Depend on the stable setters rather than the whole shell object, so the
  // keydown listener is not re-registered on every status change.
  const setStatus = shell.setStatus;
  const setContext = shell.setContext;
  const togglePanel = shell.togglePanel;

  const notAvailable = useCallback(() => {
    setStatus(NOT_AVAILABLE, 2000);
  }, [setStatus]);

  useEffect(() => {
    const count = notes.notes.length;
    setContext(`${count} ${count === 1 ? "note" : "notes"}`);
  }, [notes.notes.length, setContext]);

  useEffect(() => {
    if (search.progress) {
      const { done, total } = search.progress;
      setStatus(total > 0 ? `Indexing ${done} / ${total}…` : "Indexing…");
    } else if (notes.saving) setStatus("Saving…");
    else if (notes.dirty) setStatus("Modified");
    else if (search.stats?.stale) setStatus("Index out of date");
    else if (notes.lastSavedAt) {
      setStatus(`Saved ${new Date(notes.lastSavedAt).toLocaleTimeString()}`);
    }
  }, [
    notes.saving,
    notes.dirty,
    notes.lastSavedAt,
    search.progress,
    search.stats?.stale,
    setStatus,
  ]);

  // The dirty marker belongs in the window title, as it did (R4.3).
  useEffect(() => {
    const title = notes.open
      ? `${notes.dirty ? "*" : ""}${notes.open.meta.title} — Mushroom`
      : "Mushroom — Personal Knowledge";
    document.title = title;
    void (async () => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().setTitle(title);
      } catch {
        // Browser preview: there is no native window to title.
      }
    })();
  }, [notes.open, notes.dirty]);

  const newNote = useCallback(() => {
    setDialog({ kind: "new-note", folder: notes.selectedFolder ?? "" });
  }, [notes.selectedFolder]);

  const save = notes.save;
  const refresh = notes.refresh;

  const showSearch = useCallback(() => {
    if (!shell.ui.showSearchPanel) togglePanel("search");
  }, [shell.ui.showSearchPanel, togglePanel]);

  const shortcutHandlers = useMemo(
    () => ({
      "Ctrl+N": newNote,
      "Ctrl+S": () => void save(),
      "Ctrl+F": () => showSearch(),
      "Ctrl+Shift+F": () => togglePanel("ai"),
      "Ctrl+Shift+P": () => setMode((m) => (m === "preview" ? "edit" : "preview")),
      F1: () => setDialog({ kind: "shortcuts" }),
      F5: () => void refresh(),
      Escape: () => setDialog(null),
    }),
    [newNote, save, refresh, togglePanel, showSearch],
  );

  useShortcuts(notAvailable, shortcutHandlers);

  const exit = useCallback(async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch {
      setStatus("Exit is only available in the desktop app", 2500);
    }
  }, [setStatus]);

  const dragSidebar = useCallback(
    (delta: number) => {
      const total = workAreaRef.current?.clientWidth ?? 0;
      const reserved = EDITOR_MIN + (shell.ui.showAiPanel ? aiWidth : 0);
      const max = Math.max(SIDEBAR_MIN, total - reserved);
      shell.setSizes({
        sidebarWidth: Math.max(SIDEBAR_MIN, Math.min(sidebarWidth + delta, max)),
      });
    },
    [aiWidth, sidebarWidth, shell],
  );

  const dragNotebook = useCallback(
    (delta: number) => {
      const total = sidebarRef.current?.clientHeight ?? 0;
      const max = Math.max(NOTEBOOK_MIN, total - NOTEBOOK_MIN);
      shell.setSizes({
        notebookHeight: Math.max(NOTEBOOK_MIN, Math.min(notebookHeight + delta, max)),
      });
    },
    [notebookHeight, shell],
  );

  const dragAi = useCallback(
    (delta: number) => {
      const total = workAreaRef.current?.clientWidth ?? 0;
      const max = Math.max(AI_MIN, total - EDITOR_MIN - sidebarWidth);
      shell.setSizes({ aiWidth: Math.max(AI_MIN, Math.min(aiWidth - delta, max)) });
    },
    [aiWidth, sidebarWidth, shell],
  );

  const openId = notes.open?.meta.id;
  const openTitle = notes.open?.meta.title ?? "";

  const menus: MenuDef[] = [
    {
      title: "File",
      mnemonic: "F",
      items: [
        { type: "item", label: "New Note", mnemonic: "N", accel: "Ctrl+N", onSelect: newNote },
        { type: "item", label: "Open…", mnemonic: "O", accel: "Ctrl+O" },
        {
          type: "item",
          label: "Save",
          mnemonic: "S",
          accel: "Ctrl+S",
          onSelect: notes.dirty ? () => void save() : undefined,
        },
        { type: "item", label: "Save As…", mnemonic: "A" },
        { type: "separator" },
        { type: "item", label: "Import…", mnemonic: "I" },
        { type: "item", label: "Export…", mnemonic: "E" },
        { type: "separator" },
        { type: "item", label: "Exit", mnemonic: "x", onSelect: exit },
      ],
    },
    {
      title: "Edit",
      mnemonic: "E",
      items: [
        { type: "item", label: "Undo", mnemonic: "U", accel: "Ctrl+Z", onSelect: () => document.execCommand("undo") },
        { type: "item", label: "Redo", mnemonic: "R", accel: "Ctrl+Y", onSelect: () => document.execCommand("redo") },
        { type: "separator" },
        { type: "item", label: "Cut", mnemonic: "t", accel: "Ctrl+X", onSelect: () => document.execCommand("cut") },
        { type: "item", label: "Copy", mnemonic: "C", accel: "Ctrl+C", onSelect: () => document.execCommand("copy") },
        { type: "item", label: "Paste", mnemonic: "P", accel: "Ctrl+V", onSelect: () => document.execCommand("paste") },
        { type: "separator" },
        { type: "item", label: "Select All", mnemonic: "A", accel: "Ctrl+A", onSelect: () => document.execCommand("selectAll") },
      ],
    },
    {
      title: "View",
      mnemonic: "V",
      items: [
        { type: "item", label: "Edit", mnemonic: "E", checked: mode === "edit", onSelect: () => setMode("edit") },
        { type: "item", label: "Preview", mnemonic: "P", accel: "Ctrl+Shift+P", checked: mode === "preview", onSelect: () => setMode("preview") },
        { type: "item", label: "Split", mnemonic: "S", checked: mode === "split", onSelect: () => setMode("split") },
        { type: "separator" },
        { type: "item", label: "Notes", mnemonic: "N", checked: shell.ui.showNotesPanel, onSelect: () => togglePanel("notes") },
        { type: "item", label: "Search", mnemonic: "h", checked: shell.ui.showSearchPanel, onSelect: () => togglePanel("search") },
        { type: "item", label: "AI Search", mnemonic: "A", checked: shell.ui.showAiPanel, onSelect: () => togglePanel("ai") },
        { type: "separator" },
        { type: "item", label: "Refresh", mnemonic: "R", accel: "F5", onSelect: () => void refresh() },
        { type: "item", label: "Status Bar", mnemonic: "B", checked: shell.ui.showStatusBar, onSelect: shell.toggleStatusBar },
      ],
    },
    {
      title: "Search",
      mnemonic: "S",
      items: [
        {
          type: "item",
          label: "Search Notes",
          mnemonic: "S",
          accel: "Ctrl+F",
          onSelect: showSearch,
        },
        { type: "item", label: "Search Everywhere", mnemonic: "E" },
        { type: "item", label: "AI Search", mnemonic: "A", accel: "Ctrl+Shift+F", onSelect: () => togglePanel("ai") },
      ],
    },
    {
      title: "Note",
      mnemonic: "N",
      items: [
        { type: "item", label: "New Note", mnemonic: "N", onSelect: newNote },
        {
          type: "item",
          label: "Rename…",
          mnemonic: "R",
          onSelect: openId ? () => setDialog({ kind: "rename", id: openId, title: openTitle }) : undefined,
        },
        {
          type: "item",
          label: "Move…",
          mnemonic: "M",
          onSelect: openId ? () => setDialog({ kind: "move", id: openId, title: openTitle }) : undefined,
        },
        { type: "separator" },
        {
          type: "item",
          label: "Delete…",
          mnemonic: "D",
          onSelect: openId ? () => setDialog({ kind: "delete-note", id: openId, title: openTitle }) : undefined,
        },
        { type: "separator" },
        {
          type: "item",
          label: "New Folder…",
          mnemonic: "F",
          onSelect: () => setDialog({ kind: "new-folder", parent: notes.selectedFolder ?? "" }),
        },
      ],
    },
    {
      title: "Tools",
      mnemonic: "T",
      items: [
        {
          type: "item",
          label: "Rebuild Index…",
          mnemonic: "R",
          onSelect: () => setDialog({ kind: "rebuild" }),
        },
        { type: "separator" },
        { type: "item", label: "Settings", mnemonic: "S", onSelect: () => setDialog({ kind: "settings" }) },
        { type: "item", label: "Diagnostics", mnemonic: "D" },
      ],
    },
    {
      title: "Help",
      mnemonic: "H",
      items: [
        { type: "item", label: "Keyboard Shortcuts", mnemonic: "K", accel: "F1", onSelect: () => setDialog({ kind: "shortcuts" }) },
        { type: "separator" },
        { type: "item", label: "About Mushroom", mnemonic: "A", onSelect: () => setDialog({ kind: "about" }) },
      ],
    },
  ];

  const toolbarActions: ToolbarAction[] = [
    { label: "New", icon: "new", onClick: newNote },
    { label: "Open", icon: "open" },
    { label: "Save", icon: "save", onClick: () => void save(), disabled: !notes.dirty },
    {
      label: "Search",
      icon: "search",
      separatorBefore: true,
      pressed: shell.ui.showSearchPanel,
      onClick: showSearch,
    },
    {
      label: "AI",
      icon: "ai",
      pressed: shell.ui.showAiPanel,
      onClick: () => togglePanel("ai"),
    },
  ];

  const rootUnavailable = notes.status != null && !notes.status.available;

  return (
    <div className="app">
      <MenuBar menus={menus} />
      <Toolbar actions={toolbarActions} />

      <div className="workarea" ref={workAreaRef}>
        <div className="sidebar" ref={sidebarRef} style={{ width: sidebarWidth }}>
          <Panel title="Notebook" style={{ height: notebookHeight, flex: "none" }}>
            {rootUnavailable ? (
              <EmptyState text="Your notes folder is not available." />
            ) : (
              <FolderTree />
            )}
          </Panel>

          <Splitter orientation="horizontal" onDrag={dragNotebook} label="Resize notebook pane" />

          {shell.ui.showSearchPanel ? (
            <Panel title="Search" flat style={{ flex: "1 1 auto", minHeight: 0 }}>
              <SearchPanel />
            </Panel>
          ) : shell.ui.showNotesPanel ? (
            <Panel title="Notes" style={{ flex: "1 1 auto", minHeight: 0 }}>
              <NoteList onCreate={newNote} />
            </Panel>
          ) : null}
        </div>

        <Splitter orientation="vertical" onDrag={dragSidebar} label="Resize sidebar" />

        <div className="editor-pane">
          <EditorPane mode={mode} onCreate={newNote} />
        </div>

        {shell.ui.showAiPanel ? (
          <>
            <Splitter orientation="vertical" onDrag={dragAi} label="Resize AI panel" />
            <div className="ai-pane" style={{ width: aiWidth }}>
              <Panel title="AI Search" flat>
                <EmptyState text="AI search arrives in a later release." />
              </Panel>
            </div>
          </>
        ) : null}
      </div>

      <StatusBar />

      {dialog?.kind === "settings" ? <SettingsDialog onClose={() => setDialog(null)} /> : null}
      {dialog?.kind === "about" ? <AboutDialog onClose={() => setDialog(null)} /> : null}
      {dialog?.kind === "shortcuts" ? <ShortcutsDialog onClose={() => setDialog(null)} /> : null}

      {dialog?.kind === "new-note" ? (
        <PromptDialog
          title="New Note"
          label="Title"
          initial="Untitled"
          acceptLabel="Create"
          onAccept={(title) => void notes.createNote(dialog.folder, title)}
          onClose={() => setDialog(null)}
        />
      ) : null}

      {dialog?.kind === "new-folder" ? (
        <PromptDialog
          title="New Folder"
          label="Name"
          acceptLabel="Create"
          onAccept={(name) => void notes.createFolder(dialog.parent, name)}
          onClose={() => setDialog(null)}
        />
      ) : null}

      {dialog?.kind === "rename" ? (
        <PromptDialog
          title="Rename Note"
          label="Title"
          initial={dialog.title}
          acceptLabel="Rename"
          onAccept={(title) => void notes.renameNote(dialog.id, title)}
          onClose={() => setDialog(null)}
        />
      ) : null}

      {dialog?.kind === "move" ? (
        <MoveDialog
          noteTitle={dialog.title}
          onMove={(folder) => void notes.moveNote(dialog.id, folder)}
          onClose={() => setDialog(null)}
        />
      ) : null}

      {dialog?.kind === "delete-note" ? (
        <ConfirmDeleteDialog
          what={dialog.title}
          onConfirm={() => void notes.deleteNote(dialog.id)}
          onClose={() => setDialog(null)}
        />
      ) : null}

      {dialog?.kind === "rebuild" ? (
        <RebuildDialog
          onConfirm={() => void search.rebuild()}
          onClose={() => setDialog(null)}
        />
      ) : null}

      <ConflictDialog />
      <NoteErrorDialog />
    </div>
  );
}
