//! Which paths this process wrote, and when.
//!
//! Without this, every save triggers a filesystem event, which triggers a
//! reload of the note just saved — the editor flickers, the cursor jumps, and
//! in the worst case a save races a reload of its own output.
//!
//! The window is deliberately generous. NTFS change notifications can arrive
//! well after the write returns, and the cost of ignoring a genuine external
//! edit for two seconds is that the user presses F5; the cost of reloading the
//! user's own save is losing their cursor position mid-sentence.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long after writing a path its events are treated as our own.
pub const SELF_WRITE_WINDOW: Duration = Duration::from_secs(2);

/// The registry every note write reports to and the watcher consults.
///
/// A process-wide value rather than a parameter threaded through every write:
/// there is one notes root and one watcher per process, and passing it through
/// `atomic_write` would push it through every caller of every note operation
/// for no gain in correctness.
static GLOBAL: std::sync::OnceLock<std::sync::Arc<SelfWrites>> = std::sync::OnceLock::new();

pub fn global() -> &'static SelfWrites {
    shared_ref()
}

/// A handle to the same registry, for the watcher thread.
pub fn shared() -> std::sync::Arc<SelfWrites> {
    GLOBAL
        .get_or_init(|| std::sync::Arc::new(SelfWrites::new()))
        .clone()
}

fn shared_ref() -> &'static SelfWrites {
    GLOBAL.get_or_init(|| std::sync::Arc::new(SelfWrites::new()))
}

/// Paths written recently by this process.
#[derive(Debug, Default)]
pub struct SelfWrites {
    recent: Mutex<HashMap<PathBuf, Instant>>,
}

impl SelfWrites {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that we just wrote `path`.
    pub fn record(&self, path: &Path) {
        let Ok(mut recent) = self.recent.lock() else {
            return;
        };

        // Drop anything long expired so a long session does not accumulate an
        // entry per note ever saved.
        let now = Instant::now();
        recent.retain(|_, at| now.duration_since(*at) < SELF_WRITE_WINDOW * 4);
        recent.insert(normalise(path), now);
    }

    /// Whether an event for `path` is this process's own recent write.
    pub fn is_ours(&self, path: &Path) -> bool {
        self.is_ours_at(path, Instant::now())
    }

    fn is_ours_at(&self, path: &Path, now: Instant) -> bool {
        let Ok(recent) = self.recent.lock() else {
            // A poisoned lock must not turn into "reload everything".
            return true;
        };

        recent
            .get(&normalise(path))
            .is_some_and(|at| now.duration_since(*at) < SELF_WRITE_WINDOW)
    }

    #[cfg(test)]
    fn record_at(&self, path: &Path, at: Instant) {
        if let Ok(mut recent) = self.recent.lock() {
            recent.insert(normalise(path), at);
        }
    }
}

/// Compare paths the way Windows does.
///
/// A watcher event and the path we wrote can differ in case and in separator,
/// and comparing them literally means the filter silently never matches.
fn normalise(path: &Path) -> PathBuf {
    let text = path.to_string_lossy().replace('/', "\\").to_lowercase();
    PathBuf::from(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn a_path_we_just_wrote_is_ours() {
        let writes = SelfWrites::new();
        writes.record(&path(r"C:\notes\work\gpu.md"));
        assert!(writes.is_ours(&path(r"C:\notes\work\gpu.md")));
    }

    #[test]
    fn a_path_we_never_wrote_is_not_ours() {
        let writes = SelfWrites::new();
        writes.record(&path(r"C:\notes\work\gpu.md"));
        assert!(!writes.is_ours(&path(r"C:\notes\work\other.md")));
    }

    #[test]
    fn nothing_is_ours_before_any_write() {
        let writes = SelfWrites::new();
        assert!(!writes.is_ours(&path(r"C:\notes\work\gpu.md")));
    }

    #[test]
    fn case_differences_still_match() {
        // The watcher reports whatever case the filesystem hands back, which
        // is not always the case we wrote. Comparing literally would mean the
        // filter never matches and every save reloads itself.
        let writes = SelfWrites::new();
        writes.record(&path(r"C:\Notes\Work\GPU.md"));
        assert!(writes.is_ours(&path(r"c:\notes\work\gpu.md")));
    }

    #[test]
    fn separator_differences_still_match() {
        let writes = SelfWrites::new();
        writes.record(&path("C:/notes/work/gpu.md"));
        assert!(writes.is_ours(&path(r"C:\notes\work\gpu.md")));
    }

    #[test]
    fn the_claim_expires() {
        // After the window, an event for the same path is a genuine external
        // edit and must be acted on.
        let writes = SelfWrites::new();
        let long_ago = Instant::now() - SELF_WRITE_WINDOW - Duration::from_millis(50);
        writes.record_at(&path(r"C:\notes\work\gpu.md"), long_ago);

        assert!(!writes.is_ours(&path(r"C:\notes\work\gpu.md")));
    }

    #[test]
    fn a_write_just_inside_the_window_is_still_ours() {
        let writes = SelfWrites::new();
        let recent = Instant::now() - SELF_WRITE_WINDOW + Duration::from_millis(200);
        writes.record_at(&path(r"C:\notes\work\gpu.md"), recent);

        assert!(writes.is_ours(&path(r"C:\notes\work\gpu.md")));
    }

    #[test]
    fn rewriting_a_path_extends_its_window() {
        let writes = SelfWrites::new();
        let long_ago = Instant::now() - SELF_WRITE_WINDOW - Duration::from_millis(50);
        writes.record_at(&path(r"C:\notes\work\gpu.md"), long_ago);
        assert!(!writes.is_ours(&path(r"C:\notes\work\gpu.md")));

        writes.record(&path(r"C:\notes\work\gpu.md"));
        assert!(writes.is_ours(&path(r"C:\notes\work\gpu.md")));
    }

    #[test]
    fn old_entries_do_not_accumulate_forever() {
        let writes = SelfWrites::new();
        for i in 0..100 {
            let stale = Instant::now() - SELF_WRITE_WINDOW * 10;
            writes.record_at(&path(&format!(r"C:\notes\n{i}.md")), stale);
        }
        // Recording anything prunes what has long expired.
        writes.record(&path(r"C:\notes\fresh.md"));

        let held = writes.recent.lock().unwrap().len();
        assert!(
            held <= 2,
            "kept {held} entries; expired ones should be pruned"
        );
    }
}
