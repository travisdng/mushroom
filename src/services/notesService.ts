import { call } from "./ipc";
import type {
  FolderNode,
  ImportReport,
  NoteContent,
  NoteMeta,
  NotesStatus,
  TrashEntry,
} from "../types/notes";

/** Every note operation the UI can perform. No component calls invoke itself. */

export function listNotes(folder?: string | null): Promise<NoteMeta[]> {
  return call<NoteMeta[]>("list_notes", { folder: folder ?? null });
}

export function getFolderTree(): Promise<FolderNode> {
  return call<FolderNode>("get_folder_tree");
}

export function readNote(id: string): Promise<NoteContent> {
  return call<NoteContent>("read_note", { id });
}

export function saveNote(
  id: string,
  body: string,
  expectedModified: number | null,
): Promise<NoteMeta> {
  return call<NoteMeta>("save_note", { id, body, expectedModified });
}

export function createNote(folder: string, title: string): Promise<NoteMeta> {
  return call<NoteMeta>("create_note", { folder, title });
}

export function renameNote(id: string, newTitle: string): Promise<NoteMeta> {
  return call<NoteMeta>("rename_note", { id, newTitle });
}

export function moveNote(id: string, newFolder: string): Promise<NoteMeta> {
  return call<NoteMeta>("move_note", { id, newFolder });
}

export function deleteNote(id: string): Promise<TrashEntry> {
  return call<TrashEntry>("delete_note", { id });
}

export function createFolder(parent: string, name: string): Promise<FolderNode> {
  return call<FolderNode>("create_folder", { parent, name });
}

export function deleteFolder(folder: string): Promise<number> {
  return call<number>("delete_folder", { folder });
}

export function importNotes(
  paths: string[],
  targetFolder: string,
): Promise<ImportReport> {
  return call<ImportReport>("import_notes", { paths, targetFolder });
}

export function exportNotes(ids: string[], targetDir: string): Promise<number> {
  return call<number>("export_notes", { ids, targetDir });
}

export function getNotesStatus(): Promise<NotesStatus> {
  return call<NotesStatus>("get_notes_status");
}

export function refreshNotes(): Promise<number> {
  return call<number>("refresh_notes");
}

export function setNotesRoot(path: string): Promise<number> {
  return call<number>("set_notes_root", { path });
}
