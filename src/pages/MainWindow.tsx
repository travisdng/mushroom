import { useCallback, useRef, useState } from "react";
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
import { useShell, NOT_AVAILABLE } from "../hooks/useShell";
import { useShortcuts } from "../hooks/useShortcuts";

const SIDEBAR_MIN = 160;
const EDITOR_MIN = 320;
const AI_MIN = 240;
const NOTEBOOK_MIN = 80;

type DialogKind = "about" | "shortcuts" | null;

export default function MainWindow() {
  const shell = useShell();
  const [dialog, setDialog] = useState<DialogKind>(null);
  const { sidebarWidth, notebookHeight, aiWidth } = shell.ui;
  const workAreaRef = useRef<HTMLDivElement>(null);
  const sidebarRef = useRef<HTMLDivElement>(null);

  const notAvailable = useCallback(() => {
    shell.setStatus(NOT_AVAILABLE, 2000);
  }, [shell]);

  useShortcuts(notAvailable, {
    "Ctrl+Shift+F": () => shell.togglePanel("ai"),
    F1: () => setDialog("shortcuts"),
    Escape: () => setDialog(null),
  });

  const exit = useCallback(async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch {
      // Running in a plain browser (npm run dev) — there is no window to close.
      shell.setStatus("Exit is only available in the desktop app", 2500);
    }
  }, [shell]);

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
        notebookHeight: Math.max(
          NOTEBOOK_MIN,
          Math.min(notebookHeight + delta, max),
        ),
      });
    },
    [notebookHeight, shell],
  );

  const dragAi = useCallback(
    (delta: number) => {
      const total = workAreaRef.current?.clientWidth ?? 0;
      const max = Math.max(AI_MIN, total - EDITOR_MIN - sidebarWidth);
      shell.setSizes({
        aiWidth: Math.max(AI_MIN, Math.min(aiWidth - delta, max)),
      });
    },
    [aiWidth, sidebarWidth, shell],
  );

  const menus: MenuDef[] = [
    {
      title: "File",
      mnemonic: "F",
      items: [
        { type: "item", label: "New Note", mnemonic: "N", accel: "Ctrl+N" },
        { type: "item", label: "Open…", mnemonic: "O", accel: "Ctrl+O" },
        { type: "item", label: "Save", mnemonic: "S", accel: "Ctrl+S" },
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
        { type: "item", label: "Undo", mnemonic: "U", accel: "Ctrl+Z" },
        { type: "item", label: "Redo", mnemonic: "R", accel: "Ctrl+Y" },
        { type: "separator" },
        { type: "item", label: "Cut", mnemonic: "t", accel: "Ctrl+X" },
        { type: "item", label: "Copy", mnemonic: "C", accel: "Ctrl+C" },
        { type: "item", label: "Paste", mnemonic: "P", accel: "Ctrl+V" },
        { type: "separator" },
        { type: "item", label: "Select All", mnemonic: "A", accel: "Ctrl+A" },
      ],
    },
    {
      title: "View",
      mnemonic: "V",
      items: [
        {
          type: "item",
          label: "Notes",
          mnemonic: "N",
          checked: shell.ui.showNotesPanel,
          onSelect: () => shell.togglePanel("notes"),
        },
        {
          type: "item",
          label: "Search",
          mnemonic: "S",
          checked: shell.ui.showSearchPanel,
          onSelect: () => shell.togglePanel("search"),
        },
        {
          type: "item",
          label: "AI Search",
          mnemonic: "A",
          checked: shell.ui.showAiPanel,
          onSelect: () => shell.togglePanel("ai"),
        },
        { type: "separator" },
        {
          type: "item",
          label: "Status Bar",
          mnemonic: "B",
          checked: shell.ui.showStatusBar,
          onSelect: shell.toggleStatusBar,
        },
      ],
    },
    {
      title: "Search",
      mnemonic: "S",
      items: [
        { type: "item", label: "Search Notes", mnemonic: "S", accel: "Ctrl+F" },
        { type: "item", label: "Search Everywhere", mnemonic: "E" },
        {
          type: "item",
          label: "AI Search",
          mnemonic: "A",
          accel: "Ctrl+Shift+F",
          onSelect: () => shell.togglePanel("ai"),
        },
      ],
    },
    {
      title: "Note",
      mnemonic: "N",
      items: [
        { type: "item", label: "New Note", mnemonic: "N" },
        { type: "item", label: "Rename", mnemonic: "R" },
        { type: "item", label: "Move", mnemonic: "M" },
        { type: "separator" },
        { type: "item", label: "Delete", mnemonic: "D" },
      ],
    },
    {
      title: "Tools",
      mnemonic: "T",
      items: [
        { type: "item", label: "Rebuild Index", mnemonic: "R" },
        { type: "separator" },
        { type: "item", label: "Settings", mnemonic: "S" },
        { type: "item", label: "Diagnostics", mnemonic: "D" },
      ],
    },
    {
      title: "Help",
      mnemonic: "H",
      items: [
        {
          type: "item",
          label: "Keyboard Shortcuts",
          mnemonic: "K",
          accel: "F1",
          onSelect: () => setDialog("shortcuts"),
        },
        { type: "separator" },
        {
          type: "item",
          label: "About Mushroom",
          mnemonic: "A",
          onSelect: () => setDialog("about"),
        },
      ],
    },
  ];

  const toolbarActions: ToolbarAction[] = [
    { label: "New", icon: "new" },
    { label: "Open", icon: "open" },
    { label: "Save", icon: "save" },
    { label: "Search", icon: "search", separatorBefore: true },
    {
      label: "AI",
      icon: "ai",
      pressed: shell.ui.showAiPanel,
      onClick: () => shell.togglePanel("ai"),
    },
  ];

  return (
    <div className="app">
      <MenuBar menus={menus} />
      <Toolbar actions={toolbarActions} />

      <div className="workarea" ref={workAreaRef}>
        <div className="sidebar" ref={sidebarRef} style={{ width: sidebarWidth }}>
          <Panel title="Notebook" style={{ height: notebookHeight, flex: "none" }}>
            <EmptyState text="No notebooks yet." />
          </Panel>

          <Splitter
            orientation="horizontal"
            onDrag={dragNotebook}
            label="Resize notebook pane"
          />

          {shell.ui.showNotesPanel ? (
            <Panel title="Notes" style={{ flex: "1 1 auto", minHeight: 0 }}>
              <EmptyState text="No notes yet." />
            </Panel>
          ) : null}
        </div>

        <Splitter
          orientation="vertical"
          onDrag={dragSidebar}
          label="Resize sidebar"
        />

        <div className="editor-pane">
          <EmptyState text="No note open." />
        </div>

        {shell.ui.showAiPanel ? (
          <>
            <Splitter
              orientation="vertical"
              onDrag={dragAi}
              label="Resize AI panel"
            />
            <div className="ai-pane" style={{ width: aiWidth }}>
              <Panel title="AI Search" flat>
                <EmptyState text="AI search arrives in a later release." />
              </Panel>
            </div>
          </>
        ) : null}
      </div>

      <StatusBar />

      {dialog === "about" ? (
        <AboutDialog onClose={() => setDialog(null)} />
      ) : null}
      {dialog === "shortcuts" ? (
        <ShortcutsDialog onClose={() => setDialog(null)} />
      ) : null}
    </div>
  );
}
