//! Note use cases: the operations the commands expose.
//!
//! Holds the notes root and the cache, and is the only place that composes
//! path resolution, the store, and frontmatter. Testable without Tauri.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::error::AppError;
use crate::notes::cache::NotesCache;
use crate::notes::frontmatter::{self, Frontmatter};
use crate::notes::model::{FolderNode, NoteContent, NoteId, NoteMeta};
use crate::notes::paths;
use crate::notes::store;

pub struct NotesService {
    root: RwLock<Option<PathBuf>>,
    cache: RwLock<NotesCache>,
}

/// What a delete moved, so the UI can offer an undo path.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    pub id: NoteId,
    pub trashed_path: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub failures: Vec<String>,
}

impl NotesService {
    pub fn new() -> Self {
        Self {
            root: RwLock::new(None),
            cache: RwLock::new(NotesCache::default()),
        }
    }

    fn lock_err() -> AppError {
        AppError::Internal("notes state lock poisoned".into())
    }

    pub fn root(&self) -> Result<PathBuf, AppError> {
        self.root
            .read()
            .map_err(|_| Self::lock_err())?
            .clone()
            .ok_or(AppError::NotesRootMissing)
    }

    pub fn set_root(&self, root: PathBuf) -> Result<(), AppError> {
        *self.root.write().map_err(|_| Self::lock_err())? = Some(root);
        Ok(())
    }

    /// Walk the notes root and replace the cache. Call from a blocking task.
    pub fn rescan(&self) -> Result<usize, AppError> {
        let root = self.root()?;
        let result = store::scan(&root);
        let count = result.notes.len();
        let skipped = result.skipped.len();

        *self.cache.write().map_err(|_| Self::lock_err())? = NotesCache::from_scan(result);

        if skipped > 0 {
            tracing::warn!(target: "files", skipped, "some files could not be indexed");
        }
        tracing::info!(target: "files", count, "notes scanned");
        Ok(count)
    }

    pub fn list(&self, folder: Option<&str>) -> Result<Vec<NoteMeta>, AppError> {
        Ok(self
            .cache
            .read()
            .map_err(|_| Self::lock_err())?
            .list(folder))
    }

    pub fn count(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    pub fn skipped(&self) -> usize {
        self.cache.read().map(|c| c.skipped.len()).unwrap_or(0)
    }

    pub fn folder_tree(&self) -> Result<FolderNode, AppError> {
        let root = self.root()?;
        Ok(self
            .cache
            .read()
            .map_err(|_| Self::lock_err())?
            .folder_tree(&root))
    }

    pub fn read(&self, id: &NoteId) -> Result<NoteContent, AppError> {
        let root = self.root()?;
        let note = store::read(&root, id)?;

        if note.meta.size_bytes > store::LARGE_FILE_BYTES {
            tracing::warn!(
                target: "files",
                size = note.meta.size_bytes,
                "opening a large note"
            );
        }

        Ok(NoteContent {
            meta: note.meta,
            body: note.body,
            frontmatter: note.frontmatter,
            disk_modified: note.disk_modified,
        })
    }

    /// Save a note, refusing if it changed on disk since it was loaded.
    ///
    /// `expected_modified` is the mtime the editor was given. A mismatch means
    /// something else wrote the file, and overwriting would destroy that edit
    /// silently (R9.3).
    pub fn save(
        &self,
        id: &NoteId,
        body: &str,
        expected_modified: Option<i64>,
    ) -> Result<NoteMeta, AppError> {
        let root = self.root()?;
        let path = paths::resolve(&root, id.as_str())?;

        let existing = std::fs::read_to_string(&path).ok();

        if let (Some(expected), true) = (expected_modified, path.exists()) {
            let actual = store::file_mtime(&path);
            if actual != expected {
                return Err(AppError::NoteChangedOnDisk {
                    path: path.clone(),
                    disk_modified: actual,
                });
            }
        }

        // Keep the user's frontmatter; update only the fields we own.
        let mut fm = existing
            .as_deref()
            .map(|content| frontmatter::split(content).frontmatter)
            .unwrap_or_default();
        fm.updated = Some(store::now_rfc3339());
        if fm.created.is_none() {
            fm.created = Some(store::now_rfc3339());
        }

        let rendered = frontmatter::render(&fm, body);
        store::atomic_write(&path, &rendered)?;

        let meta = store::read(&root, id)?.meta;
        self.cache
            .write()
            .map_err(|_| Self::lock_err())?
            .upsert(meta.clone());
        Ok(meta)
    }

    pub fn create(&self, folder: &str, title: &str) -> Result<NoteMeta, AppError> {
        let root = self.root()?;

        let folder_path = if folder.is_empty() {
            root.clone()
        } else {
            paths::resolve(&root, folder)?
        };
        std::fs::create_dir_all(&folder_path).map_err(|source| AppError::NoteWrite {
            path: folder_path.clone(),
            source,
        })?;

        let stem = paths::slugify(title);
        let path = paths::deduplicate(&folder_path, &stem, "md");
        let id = relative_id(&root, &path)?;

        let now = store::now_rfc3339();
        let fm = Frontmatter {
            raw: Some(String::new()),
            title: Some(title.trim().to_string()).filter(|t| !t.is_empty()),
            created: Some(now.clone()),
            updated: Some(now),
            tags: vec![],
            // A new note is not excluded. Exclusion is something the user
            // writes, and Mushroom never writes it for them.
            ai_excluded: false,
        };
        let content = frontmatter::render(&fm, "");
        store::atomic_write(&path, &content)?;

        let meta = store::read(&root, &id)?.meta;
        self.cache
            .write()
            .map_err(|_| Self::lock_err())?
            .upsert(meta.clone());
        Ok(meta)
    }

    pub fn rename(&self, id: &NoteId, new_title: &str) -> Result<NoteMeta, AppError> {
        let root = self.root()?;
        let from = paths::resolve(&root, id.as_str())?;

        let stem = paths::slugify(new_title);
        let folder = from.parent().unwrap_or(&root).to_path_buf();
        let to = folder.join(format!("{stem}.md"));

        if to != from && to.exists() {
            return Err(AppError::NoteExists { path: to.clone() });
        }

        // Update the title inside the file before moving it, so a failure
        // leaves the note where it was rather than half-renamed.
        let content = std::fs::read_to_string(&from).map_err(|source| AppError::NoteRead {
            path: from.clone(),
            source,
        })?;
        let mut split = frontmatter::split(&content);
        split.frontmatter.title = Some(new_title.trim().to_string());
        split.frontmatter.updated = Some(store::now_rfc3339());
        store::atomic_write(&from, &frontmatter::render(&split.frontmatter, &split.body))?;

        if to != from {
            std::fs::rename(&from, &to).map_err(|source| AppError::NoteWrite {
                path: from.clone(),
                source,
            })?;
        }

        let new_id = relative_id(&root, &to)?;
        let meta = store::read(&root, &new_id)?.meta;
        self.cache
            .write()
            .map_err(|_| Self::lock_err())?
            .rename(id, meta.clone());
        Ok(meta)
    }

    pub fn move_note(&self, id: &NoteId, new_folder: &str) -> Result<NoteMeta, AppError> {
        let root = self.root()?;
        let from = paths::resolve(&root, id.as_str())?;

        let folder_path = if new_folder.is_empty() {
            root.clone()
        } else {
            paths::resolve(&root, new_folder)?
        };
        std::fs::create_dir_all(&folder_path).map_err(|source| AppError::NoteWrite {
            path: folder_path.clone(),
            source,
        })?;

        let name = from
            .file_name()
            .ok_or_else(|| AppError::Internal("note has no file name".into()))?;
        let to = folder_path.join(name);

        if to == from {
            return store::read(&root, id).map(|n| n.meta);
        }
        if to.exists() {
            return Err(AppError::NoteExists { path: to });
        }

        std::fs::rename(&from, &to).map_err(|source| AppError::NoteWrite {
            path: from.clone(),
            source,
        })?;

        let new_id = relative_id(&root, &to)?;
        let meta = store::read(&root, &new_id)?.meta;
        self.cache
            .write()
            .map_err(|_| Self::lock_err())?
            .rename(id, meta.clone());
        Ok(meta)
    }

    pub fn delete(&self, id: &NoteId) -> Result<TrashEntry, AppError> {
        let root = self.root()?;
        let trashed = store::move_to_trash(&root, id)?;

        self.cache.write().map_err(|_| Self::lock_err())?.remove(id);

        tracing::info!(target: "files", note = %id, "note moved to trash");
        Ok(TrashEntry {
            id: id.clone(),
            trashed_path: trashed.to_string_lossy().to_string(),
        })
    }

    pub fn create_folder(&self, parent: &str, name: &str) -> Result<FolderNode, AppError> {
        let root = self.root()?;
        let safe = paths::slugify(name);
        let relative = if parent.is_empty() {
            safe
        } else {
            format!("{parent}/{safe}")
        };
        let path = paths::resolve(&root, &relative)?;

        if path.exists() {
            return Err(AppError::NoteExists { path });
        }
        std::fs::create_dir_all(&path).map_err(|source| AppError::NoteWrite {
            path: path.clone(),
            source,
        })?;

        self.folder_tree()
    }

    /// Move a whole folder to the trash, as one unit (R7.5).
    pub fn delete_folder(&self, folder: &str) -> Result<usize, AppError> {
        let root = self.root()?;
        if folder.is_empty() {
            return Err(AppError::InvalidNoteName {
                name: "the notes root".into(),
            });
        }
        let path = paths::resolve(&root, folder)?;

        let affected = self
            .cache
            .read()
            .map_err(|_| Self::lock_err())?
            .list(None)
            .iter()
            .filter(|n| n.folder == folder || n.folder.starts_with(&format!("{folder}/")))
            .count();

        let stamp = store::now_rfc3339().replace([':', '-'], "");
        let target = root
            .join(store::TRASH_DIR)
            .join(format!("{}.{stamp}", folder.replace('/', "_")));
        std::fs::create_dir_all(target.parent().unwrap_or(&root)).map_err(|source| {
            AppError::NoteWrite {
                path: target.clone(),
                source,
            }
        })?;
        std::fs::rename(&path, &target).map_err(|source| AppError::NoteWrite {
            path: path.clone(),
            source,
        })?;

        self.cache
            .write()
            .map_err(|_| Self::lock_err())?
            .remove_folder(folder);
        Ok(affected)
    }

    /// Copy Markdown files into the notes root, never overwriting (R8.4).
    pub fn import(
        &self,
        sources: &[PathBuf],
        target_folder: &str,
    ) -> Result<ImportReport, AppError> {
        let root = self.root()?;
        let folder_path = if target_folder.is_empty() {
            root.clone()
        } else {
            paths::resolve(&root, target_folder)?
        };
        std::fs::create_dir_all(&folder_path).map_err(|source| AppError::NoteWrite {
            path: folder_path.clone(),
            source,
        })?;

        let mut report = ImportReport::default();

        for source in sources {
            let files: Vec<PathBuf> = if source.is_dir() {
                walkdir::WalkDir::new(source)
                    .follow_links(false)
                    .into_iter()
                    .flatten()
                    .filter(|e| e.file_type().is_file() && store::is_note_file(e.path()))
                    .map(|e| e.path().to_path_buf())
                    .collect()
            } else if store::is_note_file(source) {
                vec![source.clone()]
            } else {
                report.skipped += 1;
                continue;
            };

            for file in files {
                let stem = file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(paths::slugify)
                    .unwrap_or_else(|| "imported".to_string());
                let target = paths::deduplicate(&folder_path, &stem, "md");

                match std::fs::copy(&file, &target) {
                    Ok(_) => report.imported += 1,
                    Err(_) => {
                        report.skipped += 1;
                        report.failures.push(file.to_string_lossy().to_string());
                    }
                }
            }
        }

        self.rescan()?;
        Ok(report)
    }

    pub fn export(&self, ids: &[NoteId], target_dir: &Path) -> Result<usize, AppError> {
        let root = self.root()?;
        let mut exported = 0;

        for id in ids {
            let source = paths::resolve(&root, id.as_str())?;
            let destination = target_dir.join(id.as_str());
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|source| AppError::NoteWrite {
                    path: destination.clone(),
                    source,
                })?;
            }
            std::fs::copy(&source, &destination).map_err(|src| AppError::NoteWrite {
                path: destination.clone(),
                source: src,
            })?;
            exported += 1;
        }

        Ok(exported)
    }
}

impl Default for NotesService {
    fn default() -> Self {
        Self::new()
    }
}

fn relative_id(root: &Path, path: &Path) -> Result<NoteId, AppError> {
    let canonical_root = dunce::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    // The note may not exist yet (it is about to be created), and canonicalising
    // a missing path fails. Canonicalise the parent — which does exist — and
    // re-attach the file name. Without this a path carrying a Windows 8.3 short
    // name ("TRAVIS~1.DUO") would not match the long-form root, and creating a
    // note at the root would look like an attempt to escape it.
    let canonical = match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => dunce::canonicalize(parent)
            .map(|p| p.join(name))
            .unwrap_or_else(|_| path.to_path_buf()),
        _ => dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
    };

    let relative =
        canonical
            .strip_prefix(&canonical_root)
            .map_err(|_| AppError::PathOutsideRoot {
                path: path.to_string_lossy().to_string(),
            })?;
    Ok(NoteId::new(relative.to_string_lossy().replace('\\', "/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> (NotesService, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        crate::notes::cache::bootstrap(&root).unwrap();
        let svc = NotesService::new();
        svc.set_root(root).unwrap();
        svc.rescan().unwrap();
        (svc, dir)
    }

    #[test]
    fn creates_reads_and_saves() {
        let (svc, _dir) = service();

        let meta = svc.create("work", "GPU issue").unwrap();
        assert_eq!(meta.id.as_str(), "work/gpu-issue.md");
        assert_eq!(meta.title, "GPU issue");

        let opened = svc.read(&meta.id).unwrap();
        assert_eq!(opened.body, "");

        let saved = svc
            .save(
                &meta.id,
                "The orchestrator stops early.\n",
                Some(opened.disk_modified),
            )
            .unwrap();
        assert_eq!(saved.title, "GPU issue");

        let reopened = svc.read(&meta.id).unwrap();
        assert_eq!(reopened.body, "The orchestrator stops early.\n");
    }

    #[test]
    fn save_refuses_when_the_file_changed_underneath() {
        let (svc, _dir) = service();
        let meta = svc.create("", "Note").unwrap();
        let opened = svc.read(&meta.id).unwrap();

        // Something else writes the file after we loaded it.
        let path = svc.root().unwrap().join(meta.id.as_str());
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&path, "written by another program\n").unwrap();

        let result = svc.save(&meta.id, "our edit\n", Some(opened.disk_modified));
        assert!(
            matches!(result, Err(AppError::NoteChangedOnDisk { .. })),
            "must not overwrite an external edit"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "written by another program\n",
            "the other program's content must survive"
        );
    }

    #[test]
    fn creates_notes_at_the_notes_root() {
        // Regression: the id was computed from a non-canonical path, so a note
        // at the root looked like it was outside the root on Windows.
        let (svc, _dir) = service();
        let meta = svc.create("", "Root Level").unwrap();
        assert_eq!(meta.id.as_str(), "root-level.md");
        assert_eq!(meta.folder, "");
        assert!(svc.read(&meta.id).is_ok());
    }

    #[test]
    fn create_deduplicates_names() {
        let (svc, _dir) = service();
        let a = svc.create("ideas", "Same Title").unwrap();
        let b = svc.create("ideas", "Same Title").unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(b.id.as_str(), "ideas/same-title-2.md");
    }

    #[test]
    fn renames_file_and_title() {
        let (svc, _dir) = service();
        let meta = svc.create("work", "Old Name").unwrap();
        let renamed = svc.rename(&meta.id, "New Name").unwrap();

        assert_eq!(renamed.id.as_str(), "work/new-name.md");
        assert_eq!(renamed.title, "New Name");
        assert!(!svc.root().unwrap().join("work/old-name.md").exists());
    }

    #[test]
    fn rename_refuses_to_overwrite() {
        let (svc, _dir) = service();
        svc.create("work", "Taken").unwrap();
        let other = svc.create("work", "Other").unwrap();

        assert!(matches!(
            svc.rename(&other.id, "Taken"),
            Err(AppError::NoteExists { .. })
        ));
    }

    #[test]
    fn moves_between_folders_preserving_content() {
        let (svc, _dir) = service();
        let meta = svc.create("work", "Movable").unwrap();
        svc.save(&meta.id, "body text\n", None).unwrap();

        let moved = svc.move_note(&meta.id, "ideas").unwrap();
        assert_eq!(moved.id.as_str(), "ideas/movable.md");
        assert_eq!(svc.read(&moved.id).unwrap().body, "body text\n");
    }

    #[test]
    fn delete_moves_to_trash_and_keeps_content() {
        let (svc, _dir) = service();
        let meta = svc.create("work", "Doomed").unwrap();
        svc.save(&meta.id, "still here\n", None).unwrap();

        let entry = svc.delete(&meta.id).unwrap();
        assert!(svc.read(&meta.id).is_err(), "note should be gone");

        let trashed = std::fs::read_to_string(&entry.trashed_path).unwrap();
        assert!(trashed.contains("still here"), "trash must keep content");
        assert_eq!(svc.count(), 0);
    }

    #[test]
    fn delete_folder_takes_its_notes_and_reports_the_count() {
        let (svc, _dir) = service();
        svc.create("work", "One").unwrap();
        svc.create("work", "Two").unwrap();
        svc.create("ideas", "Untouched").unwrap();

        let affected = svc.delete_folder("work").unwrap();
        assert_eq!(affected, 2);
        assert_eq!(svc.count(), 1);
        assert!(!svc.root().unwrap().join("work").exists());
    }

    #[test]
    fn refuses_to_delete_the_notes_root() {
        let (svc, _dir) = service();
        assert!(svc.delete_folder("").is_err());
    }

    #[test]
    fn import_copies_without_overwriting() {
        let (svc, dir) = service();
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("note.md"), "imported one\n").unwrap();
        std::fs::write(outside.join("other.txt"), "not markdown").unwrap();

        svc.create("work", "note").unwrap();

        let report = svc.import(std::slice::from_ref(&outside), "work").unwrap();
        assert_eq!(report.imported, 1);

        // The existing note.md must not have been clobbered.
        assert!(svc.root().unwrap().join("work/note.md").exists());
        assert!(svc.root().unwrap().join("work/note-2.md").exists());
    }

    #[test]
    fn export_preserves_structure() {
        let (svc, dir) = service();
        let meta = svc.create("work", "Exported").unwrap();
        svc.save(&meta.id, "content\n", None).unwrap();

        let out = dir.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        assert_eq!(svc.export(std::slice::from_ref(&meta.id), &out).unwrap(), 1);
        assert!(out.join("work/exported.md").exists());
    }

    #[test]
    fn save_preserves_unknown_frontmatter() {
        let (svc, _dir) = service();
        let meta = svc.create("work", "Keeper").unwrap();
        let path = svc.root().unwrap().join(meta.id.as_str());

        std::fs::write(
            &path,
            "---\ntitle: Keeper\ncustom_field: do not lose me\n---\noriginal\n",
        )
        .unwrap();

        svc.save(&meta.id, "edited\n", None).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("custom_field: do not lose me"));
        assert!(content.contains("edited"));
    }
}
