//! Noticing that a note changed underneath us.
//!
//! Added last on purpose. A watcher touches the note cache, the editor buffer
//! and the search index at once; building it while any of those was still
//! moving would have meant debugging races against a shifting target.
//!
//! Raw filesystem events are far noisier than "a note changed": a single save
//! from another editor can produce a create, several writes and a rename. They
//! are coalesced per path over a window, filtered against this process's own
//! writes, and only then reported.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use crate::notes::selfwrites::SelfWrites;

/// How long to wait for a path to stop changing before reporting it.
///
/// Long enough to collapse one editor's save into a single event, short enough
/// that the list does not feel stale.
pub const DEBOUNCE: Duration = Duration::from_millis(500);

/// What happened to a note, after coalescing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteChange {
    /// Created or modified.
    Written(PathBuf),
    Removed(PathBuf),
}

/// Whether a path is a note we care about.
///
/// The temp file is the important one: `atomic_write` creates `x.md.tmp` and
/// renames it, so without this every save announces a phantom note.
pub fn is_watchable(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };

    // The trash is ours, and dotfiles belong to other tools.
    for part in relative.components() {
        let name = part.as_os_str().to_string_lossy();
        if name == ".trash" || name.starts_with('.') {
            return false;
        }
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if name.ends_with(".md.tmp") {
        return false;
    }

    name.ends_with(".md")
}

/// Collapses a stream of raw events into one report per path.
///
/// Its own type so the coalescing can be tested without a filesystem: the
/// interesting behaviour is *when* a path is released, not how `notify`
/// reports it.
#[derive(Debug, Default)]
pub struct Debouncer {
    pending: HashMap<PathBuf, (Instant, bool)>,
}

impl Debouncer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Note an event. `removed` marks a delete.
    pub fn push(&mut self, path: PathBuf, removed: bool, now: Instant) {
        // A later event wins: a file written then deleted is deleted, and one
        // deleted then rewritten (which is what a rename looks like) is
        // written.
        self.pending.insert(path, (now, removed));
    }

    /// Take everything that has been quiet for `DEBOUNCE`.
    pub fn take_settled(&mut self, now: Instant) -> Vec<NoteChange> {
        let settled: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (at, _))| now.duration_since(*at) >= DEBOUNCE)
            .map(|(path, _)| path.clone())
            .collect();

        settled
            .into_iter()
            .filter_map(|path| {
                let (_, removed) = self.pending.remove(&path)?;
                Some(if removed {
                    NoteChange::Removed(path)
                } else {
                    NoteChange::Written(path)
                })
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// A running watcher. Dropping it stops watching.
pub struct NotesWatcher {
    _watcher: notify::RecommendedWatcher,
}

/// Start watching `root`, calling `on_change` for each settled change.
///
/// Returns an error rather than panicking when the platform refuses — a
/// network share or a permissions problem must leave the application working
/// with manual refresh (R2.7).
pub fn watch<F>(
    root: &Path,
    self_writes: Arc<SelfWrites>,
    mut on_change: F,
) -> Result<NotesWatcher, notify::Error>
where
    F: FnMut(Vec<NoteChange>) + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = notify::recommended_watcher(move |event| {
        // A send failure means the receiving thread is gone, which happens on
        // shutdown; there is nothing useful to do about it.
        let _ = tx.send(event);
    })?;
    watcher.watch(root, RecursiveMode::Recursive)?;

    let root = root.to_path_buf();

    std::thread::Builder::new()
        .name("mushroom-watcher".into())
        .spawn(move || {
            let mut debouncer = Debouncer::new();

            loop {
                // With nothing pending there is nothing to release, so block
                // until an event arrives rather than waking twice a second for
                // the rest of the session — an idle app runs no timers (R5.5).
                // With something pending, wake in time to release it.
                let received = if debouncer.is_empty() {
                    rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
                } else {
                    rx.recv_timeout(DEBOUNCE)
                };

                match received {
                    Ok(Ok(event)) => {
                        let removed = matches!(event.kind, EventKind::Remove(_));
                        for path in event.paths {
                            if !is_watchable(&root, &path) {
                                continue;
                            }
                            if self_writes.is_ours(&path) {
                                continue;
                            }
                            debouncer.push(path, removed, Instant::now());
                        }
                    }
                    Ok(Err(err)) => {
                        tracing::warn!(target: "files", error = %err, "watch event error");
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }

                let settled = debouncer.take_settled(Instant::now());
                if !settled.is_empty() {
                    on_change(settled);
                }
            }

            tracing::debug!(target: "files", "watcher thread stopped");
        })
        .map_err(|e| notify::Error::io(std::io::Error::other(e.to_string())))?;

    Ok(NotesWatcher { _watcher: watcher })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(r"C:\notes")
    }

    fn under(rel: &str) -> PathBuf {
        root().join(rel)
    }

    #[test]
    fn markdown_notes_are_watched() {
        assert!(is_watchable(&root(), &under("gpu.md")));
        assert!(is_watchable(&root(), &under(r"work\gpu.md")));
        assert!(is_watchable(&root(), &under(r"work\deep\nested.md")));
    }

    #[test]
    fn the_temp_file_of_our_own_save_is_ignored() {
        // atomic_write creates `x.md.tmp` and renames it. Without this every
        // save announces a note that never existed.
        assert!(!is_watchable(&root(), &under("gpu.md.tmp")));
        assert!(!is_watchable(&root(), &under(r"work\gpu.md.tmp")));
    }

    #[test]
    fn the_trash_is_ignored() {
        assert!(!is_watchable(&root(), &under(r".trash\gpu.md")));
        assert!(!is_watchable(&root(), &under(r".trash\deep\gpu.md")));
    }

    #[test]
    fn dotfiles_and_dot_directories_are_ignored() {
        assert!(!is_watchable(&root(), &under(".hidden.md")));
        assert!(!is_watchable(&root(), &under(r".git\config.md")));
        assert!(!is_watchable(&root(), &under(r".obsidian\workspace.md")));
    }

    #[test]
    fn non_markdown_is_ignored() {
        assert!(!is_watchable(&root(), &under("notes.txt")));
        assert!(!is_watchable(&root(), &under("image.png")));
        assert!(!is_watchable(&root(), &under("README")));
    }

    #[test]
    fn extension_matching_is_case_insensitive() {
        assert!(is_watchable(&root(), &under("GPU.MD")));
        assert!(!is_watchable(&root(), &under("GPU.MD.TMP")));
    }

    #[test]
    fn anything_outside_the_root_is_ignored() {
        assert!(!is_watchable(&root(), Path::new(r"C:\elsewhere\gpu.md")));
    }

    #[test]
    fn a_change_is_held_until_it_settles() {
        let mut d = Debouncer::new();
        let start = Instant::now();
        d.push(under("gpu.md"), false, start);

        assert!(d.take_settled(start).is_empty(), "reported immediately");
        assert!(
            d.take_settled(start + DEBOUNCE - Duration::from_millis(1))
                .is_empty(),
            "reported before the window closed"
        );
        assert_eq!(
            d.take_settled(start + DEBOUNCE),
            vec![NoteChange::Written(under("gpu.md"))]
        );
    }

    #[test]
    fn a_burst_of_writes_becomes_one_change() {
        // One save from another editor is a create plus several writes.
        let mut d = Debouncer::new();
        let start = Instant::now();
        for i in 0..5 {
            d.push(
                under("gpu.md"),
                false,
                start + Duration::from_millis(i * 50),
            );
        }

        let settled = d.take_settled(start + Duration::from_millis(250) + DEBOUNCE);
        assert_eq!(settled.len(), 1, "{settled:?}");
    }

    #[test]
    fn continued_activity_keeps_extending_the_window() {
        // A file still being written must not be reported half-written.
        let mut d = Debouncer::new();
        let start = Instant::now();
        d.push(under("gpu.md"), false, start);

        let mut at = start;
        for _ in 0..5 {
            at += DEBOUNCE - Duration::from_millis(50);
            assert!(d.take_settled(at).is_empty(), "released while still busy");
            d.push(under("gpu.md"), false, at);
        }

        assert_eq!(d.take_settled(at + DEBOUNCE).len(), 1);
    }

    #[test]
    fn different_paths_settle_independently() {
        let mut d = Debouncer::new();
        let start = Instant::now();
        d.push(under("first.md"), false, start);
        d.push(
            under("second.md"),
            false,
            start + Duration::from_millis(400),
        );

        let settled = d.take_settled(start + DEBOUNCE);
        assert_eq!(settled, vec![NoteChange::Written(under("first.md"))]);
        assert!(!d.is_empty(), "the second is still pending");

        let later = d.take_settled(start + Duration::from_millis(400) + DEBOUNCE);
        assert_eq!(later, vec![NoteChange::Written(under("second.md"))]);
    }

    #[test]
    fn a_delete_after_a_write_reports_a_delete() {
        let mut d = Debouncer::new();
        let start = Instant::now();
        d.push(under("gpu.md"), false, start);
        d.push(under("gpu.md"), true, start + Duration::from_millis(10));

        assert_eq!(
            d.take_settled(start + Duration::from_millis(10) + DEBOUNCE),
            vec![NoteChange::Removed(under("gpu.md"))]
        );
    }

    #[test]
    fn a_write_after_a_delete_reports_a_write() {
        // What a rename into place looks like from outside.
        let mut d = Debouncer::new();
        let start = Instant::now();
        d.push(under("gpu.md"), true, start);
        d.push(under("gpu.md"), false, start + Duration::from_millis(10));

        assert_eq!(
            d.take_settled(start + Duration::from_millis(10) + DEBOUNCE),
            vec![NoteChange::Written(under("gpu.md"))]
        );
    }

    #[test]
    fn watching_a_path_that_does_not_exist_fails_rather_than_panicking() {
        // R2.7: a network share or a permissions problem must leave the
        // application running on manual refresh, not take it down.
        let missing = std::env::temp_dir().join("mushroom-no-such-folder-cafe");
        assert!(!missing.exists(), "precondition: the path must not exist");

        let outcome = watch(&missing, crate::notes::selfwrites::shared(), |_| {});
        assert!(outcome.is_err(), "watching a missing folder should fail");
    }

    #[test]
    fn taking_settled_changes_empties_them() {
        let mut d = Debouncer::new();
        let start = Instant::now();
        d.push(under("gpu.md"), false, start);

        assert_eq!(d.take_settled(start + DEBOUNCE).len(), 1);
        assert!(d.is_empty());
        assert!(d.take_settled(start + DEBOUNCE * 2).is_empty());
    }
}
