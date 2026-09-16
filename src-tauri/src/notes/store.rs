//! Filesystem primitives for notes.
//!
//! Everything that touches a `.md` file goes through here. The rules that
//! matter: writes are atomic, deletes are recoverable, and the bytes the user
//! typed come back exactly as they were.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppError;
use crate::notes::frontmatter;
use crate::notes::model::{NoteId, NoteMeta};

pub const NOTE_EXTENSIONS: [&str; 2] = ["md", "markdown"];
pub const TRASH_DIR: &str = ".trash";
/// Read only enough of a file during a scan to find its title.
const SCAN_PREFIX_BYTES: usize = 4096;
/// Above this, warn rather than opening silently (R9.5).
pub const LARGE_FILE_BYTES: u64 = 5 * 1024 * 1024;

pub fn is_note_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| NOTE_EXTENSIONS.iter().any(|n| e.eq_ignore_ascii_case(n)))
        .unwrap_or(false)
}

fn mtime_seconds(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn parse_rfc3339(value: &str) -> Option<i64> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|t| t.unix_timestamp())
}

/// Derive a title: frontmatter `title`, else the first level-1 heading, else
/// the filename (R2.2).
pub fn derive_title(id: &NoteId, fm: &frontmatter::Frontmatter, body: &str) -> String {
    if let Some(title) = fm.title.as_ref().filter(|t| !t.trim().is_empty()) {
        return title.trim().to_string();
    }
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            if !heading.trim().is_empty() {
                return heading.trim().to_string();
            }
        }
    }
    id.stem().to_string()
}

/// Write bytes so that a crash leaves either the old file or the new one.
///
/// Temp file in the same directory (so the rename stays on one volume), flushed
/// and fsynced, then renamed over the target. Never a truncate-in-place.
pub fn atomic_write(path: &Path, contents: &str) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| AppError::NoteWrite {
        path: path.to_path_buf(),
        source: std::io::Error::other("note path has no parent directory"),
    })?;

    std::fs::create_dir_all(parent).map_err(|source| AppError::NoteWrite {
        path: path.to_path_buf(),
        source,
    })?;

    let tmp = path.with_extension("md.tmp");
    {
        let mut file = std::fs::File::create(&tmp).map_err(|source| AppError::NoteWrite {
            path: tmp.clone(),
            source,
        })?;
        file.write_all(contents.as_bytes())
            .map_err(|source| AppError::NoteWrite {
                path: tmp.clone(),
                source,
            })?;
        // Without this the rename can land before the data does.
        file.sync_all().map_err(|source| AppError::NoteWrite {
            path: tmp.clone(),
            source,
        })?;
    }

    std::fs::rename(&tmp, path).map_err(|source| {
        // Do not leave a stray .md.tmp behind if the rename failed.
        let _ = std::fs::remove_file(&tmp);
        AppError::NoteWrite {
            path: path.to_path_buf(),
            source,
        }
    })
}

pub struct ReadNote {
    pub meta: NoteMeta,
    pub body: String,
    pub frontmatter: Option<String>,
    pub disk_modified: i64,
}

pub fn read(root: &Path, id: &NoteId) -> Result<ReadNote, AppError> {
    let path = crate::notes::paths::resolve(root, id.as_str())?;

    let fs_meta = std::fs::metadata(&path).map_err(|source| AppError::NoteRead {
        path: path.clone(),
        source,
    })?;

    let bytes = std::fs::read(&path).map_err(|source| AppError::NoteRead {
        path: path.clone(),
        source,
    })?;

    // Refuse rather than lossily convert: saving afterwards would corrupt the
    // file the user already had (R9.2).
    let content =
        String::from_utf8(bytes).map_err(|_| AppError::NoteNotText { path: path.clone() })?;

    let split = frontmatter::split(&content);
    let title = derive_title(id, &split.frontmatter, &split.body);

    Ok(ReadNote {
        meta: NoteMeta {
            id: id.clone(),
            title,
            folder: id.folder().to_string(),
            created: split.frontmatter.created.as_deref().and_then(parse_rfc3339),
            modified: mtime_seconds(&fs_meta),
            size_bytes: fs_meta.len(),
            tags: split.frontmatter.tags.clone(),
        },
        body: split.body,
        frontmatter: split.frontmatter.raw,
        disk_modified: mtime_seconds(&fs_meta),
    })
}

/// Move a note into `.trash/`, keeping its relative structure and a timestamp.
///
/// Deleting is never `remove_file` in the normal path (R7.2, R7.3).
pub fn move_to_trash(root: &Path, id: &NoteId) -> Result<PathBuf, AppError> {
    let source = crate::notes::paths::resolve(root, id.as_str())?;

    let stamp = time::OffsetDateTime::now_utc()
        .format(
            &time::format_description::parse_borrowed::<2>(
                "[year][month][day]-[hour][minute][second]",
            )
            .expect("static format"),
        )
        .unwrap_or_else(|_| "unknown".into());

    let relative = Path::new(id.as_str());
    let stem = relative
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("note");
    let ext = relative
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("md");

    let target_dir = match relative.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => root.join(TRASH_DIR).join(parent),
        _ => root.join(TRASH_DIR),
    };
    std::fs::create_dir_all(&target_dir).map_err(|source| AppError::NoteWrite {
        path: target_dir.clone(),
        source,
    })?;

    let target = target_dir.join(format!("{stem}.{stamp}.{ext}"));
    std::fs::rename(&source, &target).map_err(|src| AppError::NoteWrite {
        path: source.clone(),
        source: src,
    })?;

    Ok(target)
}

/// Result of walking the notes root.
pub struct ScanResult {
    pub notes: Vec<NoteMeta>,
    /// Files that could not be read. One bad file never aborts a scan (R9.4).
    pub skipped: Vec<String>,
}

/// Walk the notes root, reading metadata and just enough of each file to find a
/// title. Bodies are read on open, not here.
pub fn scan(root: &Path) -> ScanResult {
    let mut notes = Vec::new();
    let mut skipped = Vec::new();

    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            // Skip the trash and anything hidden, at any depth.
            !(name.starts_with('.') && entry.depth() > 0)
        });

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                skipped.push(err.to_string());
                continue;
            }
        };

        if !entry.file_type().is_file() || !is_note_file(entry.path()) {
            continue;
        }

        let relative = match entry.path().strip_prefix(root) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        let id = NoteId::new(relative);

        match scan_one(entry.path(), &id) {
            Ok(meta) => notes.push(meta),
            Err(_) => skipped.push(id.to_string()),
        }
    }

    // Newest first — what the note list wants (R3.2).
    notes.sort_by_key(|n| std::cmp::Reverse(n.modified));
    ScanResult { notes, skipped }
}

fn scan_one(path: &Path, id: &NoteId) -> Result<NoteMeta, std::io::Error> {
    let fs_meta = std::fs::metadata(path)?;

    // Only the head of the file is needed for a title and frontmatter.
    let prefix = read_prefix(path, SCAN_PREFIX_BYTES)?;
    let split = frontmatter::split(&prefix);
    let title = derive_title(id, &split.frontmatter, &split.body);

    Ok(NoteMeta {
        id: id.clone(),
        title,
        folder: id.folder().to_string(),
        created: split.frontmatter.created.as_deref().and_then(parse_rfc3339),
        modified: mtime_seconds(&fs_meta),
        size_bytes: fs_meta.len(),
        tags: split.frontmatter.tags,
    })
}

fn read_prefix(path: &Path, limit: usize) -> Result<String, std::io::Error> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0u8; limit];
    let read = file.read(&mut buffer)?;
    buffer.truncate(read);
    // A cut in the middle of a multi-byte character is fine here: this text is
    // only used to find a title, never written back.
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

/// Remove `*.md.tmp` files left behind by a crash mid-write.
pub fn sweep_temp_files(root: &Path) -> usize {
    let mut removed = 0;
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file()
            && entry
                .path()
                .to_string_lossy()
                .to_lowercase()
                .ends_with(".md.tmp")
            && std::fs::remove_file(entry.path()).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

pub fn file_mtime(path: &Path) -> i64 {
    std::fs::metadata(path)
        .as_ref()
        .map(mtime_seconds)
        .unwrap_or(0)
}

pub fn system_time_now() -> SystemTime {
    SystemTime::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn atomic_write_replaces_content_and_leaves_no_temp_file() {
        let dir = root();
        let path = dir.path().join("note.md");

        atomic_write(&path, "first").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");

        atomic_write(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");

        let strays: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(strays.is_empty(), "a .tmp file was left behind");
    }

    #[test]
    fn atomic_write_preserves_bytes_exactly() {
        let dir = root();
        let path = dir.path().join("note.md");
        // CRLF, tabs, trailing spaces, unicode: all must survive untouched.
        let awkward = "# Title\r\n\r\n\tindented\ttabs   \r\nCafé 日本語\r\n\r\n";
        atomic_write(&path, awkward).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), awkward);
    }

    #[test]
    fn reads_a_note_with_frontmatter() {
        let dir = root();
        std::fs::create_dir_all(dir.path().join("work")).unwrap();
        std::fs::write(
            dir.path().join("work/n.md"),
            "---\ntitle: From Frontmatter\ncreated: 2026-01-02T03:04:05Z\ntags: [a, b]\n---\nBody\n",
        )
        .unwrap();

        let note = read(dir.path(), &NoteId::new("work/n.md")).unwrap();
        assert_eq!(note.meta.title, "From Frontmatter");
        assert_eq!(note.meta.folder, "work");
        assert_eq!(note.meta.tags, vec!["a", "b"]);
        assert_eq!(note.body, "Body\n");
        assert!(note.meta.created.is_some());
    }

    #[test]
    fn title_falls_back_to_heading_then_filename() {
        let dir = root();
        std::fs::write(dir.path().join("a.md"), "# Heading Title\n\ntext").unwrap();
        std::fs::write(dir.path().join("b.md"), "no heading here").unwrap();

        assert_eq!(
            read(dir.path(), &NoteId::new("a.md")).unwrap().meta.title,
            "Heading Title"
        );
        assert_eq!(
            read(dir.path(), &NoteId::new("b.md")).unwrap().meta.title,
            "b"
        );
    }

    #[test]
    fn refuses_non_utf8_files() {
        let dir = root();
        std::fs::write(dir.path().join("bin.md"), [0xff, 0xfe, 0x00, 0x01]).unwrap();
        assert!(matches!(
            read(dir.path(), &NoteId::new("bin.md")),
            Err(AppError::NoteNotText { .. })
        ));
    }

    #[test]
    fn trash_keeps_structure_and_content() {
        let dir = root();
        std::fs::create_dir_all(dir.path().join("work")).unwrap();
        let original = "important content\n";
        std::fs::write(dir.path().join("work/n.md"), original).unwrap();

        let trashed = move_to_trash(dir.path(), &NoteId::new("work/n.md")).unwrap();

        assert!(!dir.path().join("work/n.md").exists());
        assert!(trashed.starts_with(dir.path().join(TRASH_DIR).join("work")));
        assert_eq!(std::fs::read_to_string(&trashed).unwrap(), original);
    }

    #[test]
    fn scan_finds_notes_and_skips_noise() {
        let dir = root();
        std::fs::create_dir_all(dir.path().join("work")).unwrap();
        std::fs::create_dir_all(dir.path().join(TRASH_DIR)).unwrap();
        std::fs::write(dir.path().join("work/a.md"), "# A").unwrap();
        std::fs::write(dir.path().join("b.markdown"), "# B").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "not a note").unwrap();
        std::fs::write(dir.path().join(".hidden.md"), "# hidden").unwrap();
        std::fs::write(dir.path().join(TRASH_DIR).join("gone.md"), "# gone").unwrap();
        std::fs::write(dir.path().join("half.md.tmp"), "# partial").unwrap();

        let result = scan(dir.path());
        let ids: Vec<_> = result.notes.iter().map(|n| n.id.to_string()).collect();

        assert!(ids.contains(&"work/a.md".to_string()));
        assert!(ids.contains(&"b.markdown".to_string()));
        assert_eq!(ids.len(), 2, "got {ids:?}");
    }

    #[test]
    fn sweeps_temp_files_left_by_a_crash() {
        let dir = root();
        std::fs::write(dir.path().join("note.md.tmp"), "partial").unwrap();
        std::fs::write(dir.path().join("keep.md"), "real").unwrap();

        assert_eq!(sweep_temp_files(dir.path()), 1);
        assert!(!dir.path().join("note.md.tmp").exists());
        assert!(dir.path().join("keep.md").exists());
    }
}
