//! The note domain types.
//!
//! A note's identity is its path relative to the notes root — no UUIDs, no
//! sidecar files. That keeps the folder openable in any other editor, which is
//! the whole point of local-first. See `.kiro/specs/02-notes-core/design.md`.

use serde::{Deserialize, Serialize};

/// A note's path relative to the notes root, always with `/` separators.
///
/// Constructing one does not prove the path is safe — use
/// [`crate::notes::paths::resolve`] before touching the filesystem.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NoteId(String);

impl NoteId {
    pub fn new(relative: impl Into<String>) -> Self {
        Self(relative.into().replace('\\', "/"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The folder part, or `""` for a note at the root.
    pub fn folder(&self) -> &str {
        match self.0.rfind('/') {
            Some(at) => &self.0[..at],
            None => "",
        }
    }

    /// Filename without the extension — the fallback title.
    pub fn stem(&self) -> &str {
        let name = match self.0.rfind('/') {
            Some(at) => &self.0[at + 1..],
            None => &self.0,
        };
        match name.rfind('.') {
            Some(at) => &name[..at],
            None => name,
        }
    }
}

impl std::fmt::Display for NoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Everything the note list needs, without reading the whole file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMeta {
    pub id: NoteId,
    pub title: String,
    pub folder: String,
    /// Unix seconds. From frontmatter when present.
    pub created: Option<i64>,
    /// Unix seconds, from the filesystem — authoritative for ordering.
    pub modified: i64,
    pub size_bytes: u64,
    pub tags: Vec<String>,
}

/// A note opened for editing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteContent {
    pub meta: NoteMeta,
    /// The body after the frontmatter block, exactly as stored.
    pub body: String,
    /// The frontmatter block verbatim, without the `---` fences.
    pub frontmatter: Option<String>,
    /// Filesystem mtime at read time, echoed back on save to detect an
    /// external edit (R9.3).
    pub disk_modified: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
    /// Path relative to the notes root; `""` is the root itself.
    pub path: String,
    pub name: String,
    /// Notes directly in this folder.
    pub note_count: usize,
    /// Notes in this folder and everything under it.
    pub total_count: usize,
    pub children: Vec<FolderNode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_folder_and_stem() {
        let id = NoteId::new("work/ai-autoqa.md");
        assert_eq!(id.folder(), "work");
        assert_eq!(id.stem(), "ai-autoqa");

        let root = NoteId::new("scratch.md");
        assert_eq!(root.folder(), "");
        assert_eq!(root.stem(), "scratch");
    }

    #[test]
    fn normalises_windows_separators() {
        let id = NoteId::new(r"work\projects\note.md");
        assert_eq!(id.as_str(), "work/projects/note.md");
        assert_eq!(id.folder(), "work/projects");
    }

    #[test]
    fn handles_dots_in_names() {
        let id = NoteId::new("notes/v1.2.release.md");
        assert_eq!(id.stem(), "v1.2.release");
    }
}
