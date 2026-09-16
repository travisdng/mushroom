/** Mirrors the note types in src-tauri/src/notes/model.rs. */

export type NoteMeta = {
  id: string;
  title: string;
  folder: string;
  /** Unix seconds from frontmatter, when present. */
  created: number | null;
  /** Unix seconds from the filesystem. Drives list ordering. */
  modified: number;
  sizeBytes: number;
  tags: string[];
};

export type NoteContent = {
  meta: NoteMeta;
  body: string;
  frontmatter: string | null;
  /** Echoed back on save so the backend can detect an external edit. */
  diskModified: number;
};

export type FolderNode = {
  path: string;
  name: string;
  noteCount: number;
  totalCount: number;
  children: FolderNode[];
};

export type TrashEntry = {
  id: string;
  trashedPath: string;
};

export type ImportReport = {
  imported: number;
  skipped: number;
  failures: string[];
};

export type NotesStatus = {
  root: string | null;
  count: number;
  skipped: number;
  available: boolean;
};
