import { useState } from "react";
import { Dialog } from "../common/Dialog";
import { Field } from "../common/Field";
import { Button } from "../common/Button";
import { useNotes } from "../../hooks/useNotes";
import type { FolderNode } from "../../types/notes";

export function PromptDialog({
  title,
  label,
  initial,
  acceptLabel,
  onAccept,
  onClose,
}: {
  title: string;
  label: string;
  initial?: string;
  acceptLabel?: string;
  onAccept: (value: string) => void;
  onClose: () => void;
}) {
  const [value, setValue] = useState(initial ?? "");
  const trimmed = value.trim();

  return (
    <Dialog
      title={title}
      onClose={onClose}
      onAccept={() => {
        onAccept(trimmed);
        onClose();
      }}
      acceptLabel={acceptLabel ?? "OK"}
      acceptDisabled={trimmed.length === 0}
      width={360}
    >
      <Field
        label={label}
        value={value}
        autoFocus
        onChange={(e) => setValue(e.target.value)}
      />
    </Dialog>
  );
}

/** Delete confirmation. Cancel is the default action (R7.1). */
export function ConfirmDeleteDialog({
  what,
  detail,
  onConfirm,
  onClose,
}: {
  what: string;
  detail?: string;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <Dialog
      title="Delete"
      onClose={onClose}
      onAccept={() => {
        onConfirm();
        onClose();
      }}
      acceptLabel="Delete"
      cancelLabel="Cancel"
      width={380}
    >
      <p style={{ margin: "0 0 8px" }}>
        Delete <strong>{what}</strong>?
      </p>
      {detail ? <p style={{ margin: "0 0 8px" }}>{detail}</p> : null}
      <p style={{ margin: 0, color: "var(--text-disabled)" }}>
        It will be moved to the .trash folder inside your notes, so you can get
        it back.
      </p>
    </Dialog>
  );
}

function flatten(node: FolderNode, into: string[] = []): string[] {
  if (node.path !== "") into.push(node.path);
  node.children.forEach((child) => flatten(child, into));
  return into;
}

export function MoveDialog({
  noteTitle,
  onMove,
  onClose,
}: {
  noteTitle: string;
  onMove: (folder: string) => void;
  onClose: () => void;
}) {
  const { tree } = useNotes();
  const [folder, setFolder] = useState("");
  const folders = tree ? ["", ...flatten(tree)] : [""];

  return (
    <Dialog
      title="Move Note"
      onClose={onClose}
      onAccept={() => {
        onMove(folder);
        onClose();
      }}
      acceptLabel="Move"
      width={360}
    >
      <p style={{ margin: "0 0 8px" }}>
        Move <strong>{noteTitle}</strong> to:
      </p>
      <div className="bevel-sunken" style={{ background: "var(--field-bg)", maxHeight: 180, overflow: "auto" }}>
        {folders.map((f) => (
          <div
            key={f || "__root__"}
            className="list-row"
            data-selected={folder === f ? "true" : undefined}
            onClick={() => setFolder(f)}
            role="option"
            aria-selected={folder === f}
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                setFolder(f);
              }
            }}
          >
            <span className="list-title">{f === "" ? "(notes root)" : f}</span>
          </div>
        ))}
      </div>
    </Dialog>
  );
}

/**
 * The three-way conflict prompt (R9.3).
 *
 * There is no "overwrite silently" option on purpose: both versions exist and
 * the user is the only one who can decide which matters.
 */
export function ConflictDialog() {
  const { conflict, resolveConflict } = useNotes();
  if (!conflict) return null;

  return (
    <Dialog
      title="This note changed on disk"
      onClose={() => void resolveConflict("copy")}
      acceptOnly
      acceptLabel="Save as copy"
      onAccept={() => void resolveConflict("copy")}
      width={440}
    >
      <p style={{ margin: "0 0 10px" }}>
        <strong>{conflict.id}</strong> was modified by another program after you
        opened it. Nothing has been saved yet, so neither version is lost.
      </p>
      <div style={{ display: "flex", gap: 6, marginBottom: 10 }}>
        <Button onClick={() => void resolveConflict("mine")}>Keep mine</Button>
        <Button onClick={() => void resolveConflict("theirs")}>Load theirs</Button>
      </div>
      <p style={{ margin: 0, color: "var(--text-disabled)" }}>
        Keep mine overwrites the file on disk. Load theirs discards your edits
        in the editor. Save as copy keeps both.
      </p>
    </Dialog>
  );
}

/** Any note error, in the standard shape, with Details for the technical text. */
export function NoteErrorDialog() {
  const { error, dismissError } = useNotes();
  const [showDetail, setShowDetail] = useState(false);
  if (!error) return null;

  return (
    <Dialog
      title={error.title}
      onClose={() => {
        setShowDetail(false);
        dismissError();
      }}
      acceptOnly
      width={420}
    >
      <p style={{ margin: "0 0 8px" }}>{error.message}</p>
      {error.hint ? (
        <p style={{ margin: "0 0 8px", color: "var(--text-disabled)" }}>{error.hint}</p>
      ) : null}
      {error.detail ? (
        <>
          <Button onClick={() => setShowDetail((v) => !v)}>
            {showDetail ? "Hide details" : "Details…"}
          </Button>
          {showDetail ? (
            <pre
              className="bevel-sunken selectable"
              style={{
                marginTop: 8,
                padding: 6,
                maxHeight: 140,
                overflow: "auto",
                background: "var(--field-bg)",
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                whiteSpace: "pre-wrap",
              }}
            >
              {error.detail}
            </pre>
          ) : null}
        </>
      ) : null}
    </Dialog>
  );
}
