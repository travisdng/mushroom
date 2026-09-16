import { Dialog } from "../common/Dialog";
import { useSearch } from "../../hooks/useSearch";

/**
 * R2.4 — the dialog states plainly that rebuilding does not modify Markdown.
 * That promise is the whole reason the index can be treated as disposable.
 */
export function RebuildDialog({
  onConfirm,
  onClose,
}: {
  onConfirm: () => void;
  onClose: () => void;
}) {
  const { stats } = useSearch();

  return (
    <Dialog
      title="Rebuild Index"
      onClose={onClose}
      onAccept={() => {
        onConfirm();
        onClose();
      }}
      acceptLabel="Rebuild"
      width={420}
    >
      <p style={{ margin: "0 0 8px" }}>
        Mushroom will read every note again and rebuild its search index.
      </p>
      <p style={{ margin: "0 0 8px", fontWeight: "bold" }}>
        Your notes are not modified. The index is only a cache.
      </p>
      {stats ? (
        <p style={{ margin: 0, color: "var(--text-disabled)" }}>
          {stats.noteCount} notes, {stats.passageCount} passages indexed
          {stats.lastFullRebuild
            ? `, last rebuilt ${new Date(stats.lastFullRebuild).toLocaleString()}`
            : ""}
          .
        </p>
      ) : null}
    </Dialog>
  );
}
