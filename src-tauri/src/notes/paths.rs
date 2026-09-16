//! Path safety and filename generation.
//!
//! Every path that arrives from the frontend passes through [`resolve`] before
//! anything touches the filesystem. A note id is user-influenced data, so it is
//! treated as hostile: traversal, absolute paths, UNC paths, and symlinks that
//! point outside the notes root are all rejected.

use std::path::{Component, Path, PathBuf};

use crate::error::AppError;

/// Characters Windows forbids in a filename, plus the path separators.
const ILLEGAL: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Device names Windows reserves, with or without an extension.
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "COM0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "LPT0",
];

pub fn is_reserved_name(stem: &str) -> bool {
    let base = stem.split('.').next().unwrap_or(stem);
    RESERVED.iter().any(|r| r.eq_ignore_ascii_case(base))
}

/// Reject a relative path that could escape the notes root by construction.
///
/// This is the lexical half: no absolute paths, no drive letters, no UNC, no
/// `..`. The symlink half is handled in [`resolve`].
fn check_relative(rel: &str) -> Result<(), AppError> {
    if rel.is_empty() {
        return Err(AppError::PathOutsideRoot { path: rel.into() });
    }

    let normalised = rel.replace('\\', "/");

    // "C:/x", "//server/share", "/etc/passwd"
    let path = Path::new(&normalised);
    if path.is_absolute() || normalised.starts_with("//") {
        return Err(AppError::PathOutsideRoot { path: rel.into() });
    }

    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_string_lossy();
                // A trailing dot or space is silently stripped by Windows,
                // which would make two different ids collide on disk.
                if part.ends_with('.') || part.ends_with(' ') {
                    return Err(AppError::InvalidNoteName { name: part.into() });
                }
                if is_reserved_name(&part) {
                    return Err(AppError::InvalidNoteName { name: part.into() });
                }
            }
            // ".." is the obvious escape; a prefix or root anchor means the
            // path was absolute after all.
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(AppError::PathOutsideRoot { path: rel.into() });
            }
            Component::CurDir => {}
        }
    }

    Ok(())
}

/// Resolve a note id against the notes root, refusing anything that escapes it.
///
/// Works for paths that do not exist yet (creating a note), by canonicalising
/// the nearest existing ancestor — which is what catches a symlinked folder
/// pointing outside the root.
pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf, AppError> {
    check_relative(rel)?;

    let canonical_root =
        dunce::canonicalize(root).map_err(|source| AppError::NotesRootUnavailable {
            path: root.to_path_buf(),
            source,
        })?;

    let joined = canonical_root.join(rel.replace('\\', "/"));

    // Canonicalise as much of the path as exists; a symlink anywhere along it
    // would otherwise let the final path land outside the root.
    let mut existing = joined.as_path();
    loop {
        if existing.exists() {
            break;
        }
        match existing.parent() {
            Some(parent) => existing = parent,
            None => return Err(AppError::PathOutsideRoot { path: rel.into() }),
        }
    }

    let canonical_existing = dunce::canonicalize(existing)
        .map_err(|_| AppError::PathOutsideRoot { path: rel.into() })?;

    if !canonical_existing.starts_with(&canonical_root) {
        return Err(AppError::PathOutsideRoot { path: rel.into() });
    }

    Ok(joined)
}

/// Turn a human title into a safe filename stem.
///
/// Lowercase, spaces to hyphens, illegal and control characters dropped. The
/// result is never empty, never reserved, and never ends in a dot or space.
pub fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut last_dash = false;

    for ch in title.trim().chars() {
        if ch.is_control() || ILLEGAL.contains(&ch) {
            continue;
        }
        if ch.is_whitespace() || ch == '_' {
            if !last_dash && !out.is_empty() {
                out.push('-');
                last_dash = true;
            }
            continue;
        }
        out.extend(ch.to_lowercase());
        last_dash = false;
    }

    // Windows strips these, so a name ending in one would not round-trip.
    while out.ends_with('.') || out.ends_with('-') || out.ends_with(' ') {
        out.pop();
    }

    if out.is_empty() {
        return "untitled".to_string();
    }
    if is_reserved_name(&out) {
        out.push_str("-note");
    }

    // Leave room for a de-duplication suffix within the Windows path limit.
    const MAX_STEM: usize = 80;
    if out.chars().count() > MAX_STEM {
        out = out.chars().take(MAX_STEM).collect();
        while out.ends_with('-') {
            out.pop();
        }
    }

    out
}

/// Pick a filename that does not already exist, appending `-2`, `-3`, …
pub fn deduplicate(folder: &Path, stem: &str, extension: &str) -> PathBuf {
    let first = folder.join(format!("{stem}.{extension}"));
    if !first.exists() {
        return first;
    }
    for n in 2..10_000 {
        let candidate = folder.join(format!("{stem}-{n}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    // Absurd, but better than looping forever or overwriting someone's note.
    folder.join(format!("{stem}-{}.{extension}", nanos()))
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("work")).unwrap();
        std::fs::write(dir.path().join("work/note.md"), "x").unwrap();
        dir
    }

    #[test]
    fn accepts_paths_inside_the_root() {
        let dir = root();
        assert!(resolve(dir.path(), "work/note.md").is_ok());
        // A note that does not exist yet must still resolve, for creation.
        assert!(resolve(dir.path(), "work/new-note.md").is_ok());
        assert!(resolve(dir.path(), "scratch.md").is_ok());
    }

    #[test]
    fn rejects_parent_traversal() {
        let dir = root();
        for evil in [
            "../outside.md",
            "work/../../outside.md",
            r"..\..\Windows\System32\drivers\etc\hosts",
            "work/../../../secrets.md",
        ] {
            assert!(
                matches!(
                    resolve(dir.path(), evil),
                    Err(AppError::PathOutsideRoot { .. })
                ),
                "should have rejected {evil:?}"
            );
        }
    }

    #[test]
    fn rejects_absolute_and_unc_paths() {
        let dir = root();
        for evil in [
            r"C:\Windows\System32\config\SAM",
            "/etc/passwd",
            r"\\server\share\note.md",
            "//server/share/note.md",
            r"\\?\C:\Windows",
        ] {
            assert!(
                resolve(dir.path(), evil).is_err(),
                "should have rejected {evil:?}"
            );
        }
    }

    #[test]
    fn rejects_windows_reserved_names() {
        let dir = root();
        for evil in ["CON.md", "nul.md", "work/COM1.md", "work/LPT9.md", "aux.md"] {
            assert!(
                matches!(
                    resolve(dir.path(), evil),
                    Err(AppError::InvalidNoteName { .. })
                ),
                "should have rejected {evil:?}"
            );
        }
        // A name that merely starts with a reserved word is fine.
        assert!(resolve(dir.path(), "console-notes.md").is_ok());
    }

    #[test]
    fn rejects_trailing_dot_or_space() {
        let dir = root();
        assert!(resolve(dir.path(), "note .md").is_ok(), "space before ext");
        assert!(resolve(dir.path(), "trailing. ").is_err());
        assert!(resolve(dir.path(), "trailing.").is_err());
        assert!(resolve(dir.path(), "folder /note.md").is_err());
    }

    #[test]
    fn rejects_empty() {
        let dir = root();
        assert!(resolve(dir.path(), "").is_err());
    }

    #[test]
    fn slugifies_titles() {
        assert_eq!(slugify("GPU issue"), "gpu-issue");
        assert_eq!(slugify("  Lots   of   space  "), "lots-of-space");
        assert_eq!(
            slugify("AI AutoQA / Batch Incident"),
            "ai-autoqa-batch-incident"
        );
        assert_eq!(slugify(r#"bad<>:"/\|?*chars"#), "badchars");
        assert_eq!(slugify(""), "untitled");
        assert_eq!(slugify("..."), "untitled");
        assert_eq!(slugify("CON"), "con-note");
        assert_eq!(slugify("trailing dot."), "trailing-dot");
    }

    #[test]
    fn slugify_keeps_unicode() {
        assert_eq!(slugify("Café notes"), "café-notes");
        assert_eq!(slugify("日本語のノート"), "日本語のノート");
    }

    #[test]
    fn slugify_bounds_length() {
        let long = "a".repeat(500);
        assert!(slugify(&long).chars().count() <= 80);
    }

    #[test]
    fn deduplicates_colliding_names() {
        let dir = tempfile::tempdir().unwrap();
        let first = deduplicate(dir.path(), "note", "md");
        assert!(first.ends_with("note.md"));
        std::fs::write(&first, "x").unwrap();

        let second = deduplicate(dir.path(), "note", "md");
        assert!(second.ends_with("note-2.md"));
        std::fs::write(&second, "x").unwrap();

        let third = deduplicate(dir.path(), "note", "md");
        assert!(third.ends_with("note-3.md"));
    }

    #[cfg(windows)]
    #[test]
    fn rejects_a_directory_symlink_that_escapes_the_root() {
        let dir = root();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.md"), "secret").unwrap();

        // Needs developer mode or elevation; skip rather than fail if denied.
        let link = dir.path().join("escape");
        if std::os::windows::fs::symlink_dir(outside.path(), &link).is_err() {
            eprintln!("skipping: symlink creation not permitted");
            return;
        }

        assert!(
            matches!(
                resolve(dir.path(), "escape/secret.md"),
                Err(AppError::PathOutsideRoot { .. })
            ),
            "a symlinked folder must not be a way out of the notes root"
        );
    }
}
