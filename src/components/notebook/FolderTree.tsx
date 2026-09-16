import { useState } from "react";
import type { FolderNode } from "../../types/notes";
import { useNotes } from "../../hooks/useNotes";

/** Classic expand/collapse tree with per-folder counts (R3.1). */
export function FolderTree({
  onContextMenu,
}: {
  onContextMenu?: (folder: string, x: number, y: number) => void;
}) {
  const { tree, selectedFolder, selectFolder } = useNotes();
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  if (!tree) return null;

  const toggle = (path: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const render = (node: FolderNode, depth: number) => {
    const isRoot = node.path === "";
    const selected = isRoot ? selectedFolder === null : selectedFolder === node.path;
    const hasChildren = node.children.length > 0;
    const isCollapsed = collapsed.has(node.path);

    return (
      <div key={node.path || "__root__"}>
        <div
          className="tree-row"
          data-selected={selected ? "true" : undefined}
          style={{ paddingLeft: 2 + depth * 12 }}
          onClick={() => selectFolder(isRoot ? null : node.path)}
          onContextMenu={(e) => {
            if (!onContextMenu) return;
            e.preventDefault();
            onContextMenu(node.path, e.clientX, e.clientY);
          }}
          role="treeitem"
          aria-selected={selected}
          aria-expanded={hasChildren ? !isCollapsed : undefined}
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              selectFolder(isRoot ? null : node.path);
            }
            if (e.key === "ArrowRight" && hasChildren && isCollapsed) toggle(node.path);
            if (e.key === "ArrowLeft" && hasChildren && !isCollapsed) toggle(node.path);
          }}
        >
          <span
            className="tree-twisty"
            onClick={(e) => {
              e.stopPropagation();
              if (hasChildren) toggle(node.path);
            }}
            aria-hidden="true"
          >
            {hasChildren ? (isCollapsed ? "+" : "−") : ""}
          </span>
          <span className="tree-label">{node.name}</span>
          <span className="tree-count">{node.totalCount || ""}</span>
        </div>
        {hasChildren && !isCollapsed
          ? node.children.map((child) => render(child, depth + 1))
          : null}
      </div>
    );
  };

  return (
    <div role="tree" aria-label="Notebook" style={{ padding: 2 }}>
      {render(tree, 0)}
    </div>
  );
}
