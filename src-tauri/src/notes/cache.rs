//! The in-memory note list and folder tree.
//!
//! Scanned once at startup and mutated in place afterwards. The alternative —
//! re-walking the filesystem on every click — is what makes note apps feel
//! slow, and it would put disk I/O on the render path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::notes::model::{FolderNode, NoteId, NoteMeta};
use crate::notes::store;

#[derive(Debug, Default)]
pub struct NotesCache {
    notes: Vec<NoteMeta>,
    /// Files the last scan could not read, reported to the user as a count.
    pub skipped: Vec<String>,
}

impl NotesCache {
    pub fn from_scan(result: store::ScanResult) -> Self {
        Self {
            notes: result.notes,
            skipped: result.skipped,
        }
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Notes in a folder, or all of them when `folder` is `None`.
    ///
    /// Already newest-first, because the scan sorted them and every mutation
    /// keeps that order.
    pub fn list(&self, folder: Option<&str>) -> Vec<NoteMeta> {
        match folder {
            None => self.notes.clone(),
            Some(f) => self
                .notes
                .iter()
                .filter(|n| n.folder == f)
                .cloned()
                .collect(),
        }
    }

    pub fn get(&self, id: &NoteId) -> Option<&NoteMeta> {
        self.notes.iter().find(|n| &n.id == id)
    }

    /// Insert or replace a note, keeping the newest-first ordering.
    pub fn upsert(&mut self, meta: NoteMeta) {
        match self.notes.iter_mut().find(|n| n.id == meta.id) {
            Some(existing) => *existing = meta,
            None => self.notes.push(meta),
        }
        self.resort();
    }

    pub fn remove(&mut self, id: &NoteId) {
        self.notes.retain(|n| &n.id != id);
    }

    /// Re-key a note after a rename or move, preserving list position rules.
    pub fn rename(&mut self, from: &NoteId, meta: NoteMeta) {
        self.notes.retain(|n| &n.id != from);
        self.notes.push(meta);
        self.resort();
    }

    /// Drop every note under a folder prefix, for a folder delete or move.
    pub fn remove_folder(&mut self, folder: &str) -> usize {
        let before = self.notes.len();
        let prefix = format!("{folder}/");
        self.notes
            .retain(|n| n.folder != folder && !n.folder.starts_with(&prefix));
        before - self.notes.len()
    }

    fn resort(&mut self) {
        self.notes.sort_by_key(|n| std::cmp::Reverse(n.modified));
    }

    /// Build the folder tree from the note paths plus the directories on disk,
    /// so an empty folder the user just created still appears.
    pub fn folder_tree(&self, root: &Path) -> FolderNode {
        let mut direct: BTreeMap<String, usize> = BTreeMap::new();
        for note in &self.notes {
            *direct.entry(note.folder.clone()).or_insert(0) += 1;
        }

        let mut folders: Vec<String> = collect_dirs(root);
        for folder in direct.keys() {
            if !folder.is_empty() {
                folders.push(folder.clone());
            }
        }
        // Every ancestor must exist as a node, even with no notes of its own.
        let mut all: Vec<String> = Vec::new();
        for folder in &folders {
            let mut parts: Vec<&str> = Vec::new();
            for part in folder.split('/') {
                parts.push(part);
                all.push(parts.join("/"));
            }
        }
        all.sort();
        all.dedup();

        build_node("", "", &all, &direct)
    }
}

/// Directories under the root, excluding hidden ones and the trash.
fn collect_dirs(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !(name.starts_with('.') && e.depth() > 0)
        });

    for entry in walker.flatten() {
        if !entry.file_type().is_dir() || entry.depth() == 0 {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    out
}

fn build_node(
    path: &str,
    name: &str,
    all: &[String],
    direct: &BTreeMap<String, usize>,
) -> FolderNode {
    let prefix = if path.is_empty() {
        String::new()
    } else {
        format!("{path}/")
    };

    let depth = if path.is_empty() {
        0
    } else {
        path.matches('/').count() + 1
    };

    let children: Vec<FolderNode> = all
        .iter()
        .filter(|candidate| {
            candidate.starts_with(&prefix)
                && candidate.matches('/').count() == depth
                && candidate.as_str() != path
        })
        .map(|child| {
            let child_name = child.rsplit('/').next().unwrap_or(child);
            build_node(child, child_name, all, direct)
        })
        .collect();

    let note_count = direct.get(path).copied().unwrap_or(0);
    let total_count = note_count + children.iter().map(|c| c.total_count).sum::<usize>();

    FolderNode {
        path: path.to_string(),
        name: if path.is_empty() {
            "All Notes".to_string()
        } else {
            name.to_string()
        },
        note_count,
        total_count,
        children,
    }
}

/// Create the notes root and its starter folders on first run (R1.1).
pub fn bootstrap(root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(root)?;
    for starter in ["work", "projects", "ideas", "personal"] {
        std::fs::create_dir_all(root.join(starter))?;
    }
    Ok(())
}

/// Bootstrap, plus the welcome note when the folder did not exist at all.
///
/// Separate from [`bootstrap`] on purpose. Making the folder usable and
/// greeting a new user are different jobs with different triggers: pointing
/// Mushroom at an existing folder of notes should create nothing, and every
/// test that builds a corpus calls `bootstrap` and would otherwise find an
/// extra note in it.
///
/// Returns the welcome note so the caller can open it. A note nobody reads
/// explains nothing, and "wrote a file somewhere" is not a first-run
/// experience (R7.1).
pub fn bootstrap_with_welcome(root: &Path) -> std::io::Result<Option<NoteId>> {
    // Decided before anything is created.
    let fresh = !root.exists();
    bootstrap(root)?;

    if !fresh {
        return Ok(None);
    }

    // Only ever on a genuinely new folder. The welcome note is an ordinary
    // note — the user can edit or delete it — and an app that quietly
    // recreated it on the next launch would be overriding that (R7.2).
    std::fs::write(root.join(WELCOME_NAME), WELCOME_BODY)?;
    Ok(Some(NoteId::new(WELCOME_NAME)))
}

pub const WELCOME_NAME: &str = "Welcome to Mushroom.md";

/// Deliberately short. A first-run note that fills the screen is one nobody
/// reads; this one fits without scrolling and every claim in it is something
/// the reader can try immediately.
const WELCOME_BODY: &str = "\
---
title: Welcome to Mushroom
tags: [welcome]
---

# Welcome to Mushroom

This is an ordinary note. Edit it, or delete it - it will not come back.

## Your notes are files

Everything you write here is a Markdown file in your notes folder. Open them
in any editor, put them in Dropbox or Git, or walk away from Mushroom
entirely and keep every word. There is no database to be locked out of: the
search index is built from your files and can be deleted at any time.

## Getting around

- Ctrl+N makes a new note.
- Ctrl+P is Quick Open: type any part of a note's name and go straight to it.
- Ctrl+F searches everything you have written.
- Ctrl+S saves, though Mushroom also saves on its own.
- F1 lists every shortcut.
- View > Preview renders a note instead of showing the Markdown, and
  View > Split shows both at once.

## Asking questions

The AI button answers questions from your own notes and cites which ones it
used, so you can check it. It needs an API key first, under Tools > Settings.
Mushroom works entirely without one - search, notes and everything else are
local and always will be.

## If something looks wrong

Tools > Diagnostics shows where your notes and index live and what the app has
been doing. Copy Diagnostics puts that on the clipboard with your API keys
removed.
";

/// Resolve the notes root, creating the default one if none is configured.
///
/// Returns the path even when it could not be created, so the UI can say which
/// folder it wanted rather than failing silently (R1.3).
pub fn resolve_root(configured: Option<&PathBuf>) -> Option<PathBuf> {
    match configured {
        Some(path) => Some(path.clone()),
        None => crate::config::default_notes_root(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(id: &str, modified: i64) -> NoteMeta {
        let id = NoteId::new(id);
        NoteMeta {
            folder: id.folder().to_string(),
            title: id.stem().to_string(),
            id,
            created: None,
            modified,
            size_bytes: 0,
            tags: vec![],
        }
    }

    fn cache(ids: &[(&str, i64)]) -> NotesCache {
        NotesCache {
            notes: ids.iter().map(|(id, m)| meta(id, *m)).collect(),
            skipped: vec![],
        }
    }

    #[test]
    fn lists_all_or_by_folder() {
        let c = cache(&[("work/a.md", 3), ("ideas/b.md", 2), ("work/c.md", 1)]);
        assert_eq!(c.list(None).len(), 3);
        assert_eq!(c.list(Some("work")).len(), 2);
        assert_eq!(c.list(Some("ideas")).len(), 1);
        assert_eq!(c.list(Some("nope")).len(), 0);
    }

    #[test]
    fn upsert_replaces_and_keeps_newest_first() {
        let mut c = cache(&[("a.md", 1), ("b.md", 2)]);
        c.upsert(meta("a.md", 99));

        assert_eq!(c.len(), 2, "upsert must replace, not duplicate");
        assert_eq!(c.list(None)[0].id.as_str(), "a.md");
    }

    #[test]
    fn rename_moves_the_entry() {
        let mut c = cache(&[("old.md", 1)]);
        c.rename(&NoteId::new("old.md"), meta("work/new.md", 5));

        assert!(c.get(&NoteId::new("old.md")).is_none());
        assert!(c.get(&NoteId::new("work/new.md")).is_some());
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn remove_folder_takes_nested_notes_too() {
        let mut c = cache(&[("work/a.md", 1), ("work/deep/b.md", 2), ("ideas/c.md", 3)]);
        assert_eq!(c.remove_folder("work"), 2);
        assert_eq!(c.len(), 1);
        assert_eq!(c.list(None)[0].id.as_str(), "ideas/c.md");
    }

    #[test]
    fn folder_tree_counts_directly_and_recursively() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("work/deep")).unwrap();
        std::fs::create_dir_all(dir.path().join("empty")).unwrap();

        let c = cache(&[("work/a.md", 1), ("work/deep/b.md", 2), ("root.md", 3)]);
        let tree = c.folder_tree(dir.path());

        assert_eq!(tree.path, "");
        assert_eq!(tree.note_count, 1, "root.md sits at the root");
        assert_eq!(tree.total_count, 3);

        let work = tree.children.iter().find(|c| c.path == "work").unwrap();
        assert_eq!(work.note_count, 1);
        assert_eq!(work.total_count, 2, "includes work/deep");

        // A folder with no notes must still be listed.
        assert!(
            tree.children.iter().any(|c| c.path == "empty"),
            "an empty folder the user created must still appear"
        );
    }

    #[test]
    fn bootstrap_creates_the_starter_folders() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        bootstrap(&root).unwrap();

        for starter in ["work", "projects", "ideas", "personal"] {
            assert!(root.join(starter).is_dir(), "{starter} missing");
        }
    }

    #[test]
    fn bootstrap_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        bootstrap(&root).unwrap();
        std::fs::write(root.join("work/keep.md"), "content").unwrap();

        bootstrap(&root).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("work/keep.md")).unwrap(),
            "content",
            "re-running bootstrap must not disturb existing notes"
        );
    }

    #[test]
    fn a_first_run_writes_a_welcome_note() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        let welcome = bootstrap_with_welcome(&root).unwrap();
        assert_eq!(
            welcome.unwrap().as_str(),
            WELCOME_NAME,
            "must be returned to be opened"
        );

        let body = std::fs::read_to_string(root.join(WELCOME_NAME)).unwrap();
        assert!(body.starts_with("---\ntitle: Welcome to Mushroom"));
        // It has to be a note the scanner finds, not a special case.
        let found = store::scan(&root);
        assert_eq!(found.notes.len(), 1);
        assert_eq!(found.notes[0].title, "Welcome to Mushroom");
    }

    #[test]
    fn a_deleted_welcome_note_stays_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        bootstrap_with_welcome(&root).unwrap();
        std::fs::remove_file(root.join(WELCOME_NAME)).unwrap();

        // Every launch runs this. Recreating the note the user threw away
        // would be the app arguing with them.
        assert!(bootstrap_with_welcome(&root).unwrap().is_none());
        assert!(bootstrap_with_welcome(&root).unwrap().is_none());
        assert!(!root.join(WELCOME_NAME).exists());
    }

    #[test]
    fn an_edited_welcome_note_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        bootstrap(&root).unwrap();
        std::fs::write(root.join(WELCOME_NAME), "# Mine now\n").unwrap();

        bootstrap(&root).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join(WELCOME_NAME)).unwrap(),
            "# Mine now\n"
        );
    }

    #[test]
    fn an_existing_folder_gains_no_welcome_note() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        // Someone pointing Mushroom at a folder of notes they already have.
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("existing.md"), "# Existing\n").unwrap();

        assert!(bootstrap_with_welcome(&root).unwrap().is_none());
        assert!(!root.join(WELCOME_NAME).exists());
    }

    #[test]
    fn plain_bootstrap_never_writes_a_welcome_note() {
        // Every test corpus in the suite is built with `bootstrap`; a stray
        // note in it would shift counts in tests that have nothing to do with
        // first runs, and the failures would point anywhere but here.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        bootstrap(&root).unwrap();
        assert!(!root.join(WELCOME_NAME).exists());
        assert_eq!(store::scan(&root).notes.len(), 0);
    }
}
