//! Reading the log back, and not keeping too much of it.
//!
//! The tail is read from the file rather than kept in a ring buffer in memory:
//! the log is already on disk, already scrubbed, and a buffer would be a
//! second copy of it that has to be kept in step.

use std::path::{Path, PathBuf};

use crate::diagnostics::LogLine;

/// How many lines the Diagnostics window shows.
pub const TAIL_LINES: usize = 200;
/// How many daily log files to keep.
pub const KEEP_FILES: usize = 7;

/// Parse one formatted log line.
///
/// The format is `tracing-subscriber`'s default:
/// `2026-09-17T04:11:04.841560Z  INFO files: watching the notes folder`.
/// Anything that does not match is still shown — a line we cannot parse is
/// usually the interesting one, and dropping it would hide a panic.
pub fn parse_line(line: &str) -> LogLine {
    let mut parts = line.splitn(2, char::is_whitespace);
    let timestamp = parts.next().unwrap_or_default().to_string();
    let rest = parts.next().unwrap_or("").trim_start();

    let mut words = rest.splitn(2, char::is_whitespace);
    let level = words.next().unwrap_or_default().to_string();
    let after_level = words.next().unwrap_or("").trim_start();

    // `files: message` — the target, then the message.
    let (category, message) = match after_level.split_once(": ") {
        Some((category, message)) if !category.contains(' ') => {
            (category.to_string(), message.to_string())
        }
        _ => (String::new(), after_level.to_string()),
    };

    let looks_parsed = timestamp.starts_with("20") && is_level(&level);

    if looks_parsed {
        LogLine {
            timestamp,
            level,
            category,
            message,
        }
    } else {
        LogLine {
            timestamp: String::new(),
            level: String::new(),
            category: String::new(),
            message: line.to_string(),
        }
    }
}

fn is_level(word: &str) -> bool {
    matches!(word, "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR")
}

/// The newest log file in `log_dir`, if any.
pub fn current_log(log_dir: &Path) -> Option<PathBuf> {
    daily_logs(log_dir).pop()
}

/// Every `mushroom.log.*` in the folder, oldest first.
///
/// The names carry a date, so a lexicographic sort is a chronological one.
pub fn daily_logs(log_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return Vec::new();
    };

    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with("mushroom.log"))
                .unwrap_or(false)
        })
        .collect();

    files.sort();
    files
}

/// Delete all but the newest [`KEEP_FILES`] logs.
///
/// Enforced at startup rather than on a timer: a desktop app that is open for
/// a week should not accumulate a week of files, and there is no reason to
/// wake up to find that out.
pub fn prune(log_dir: &Path) -> usize {
    let files = daily_logs(log_dir);
    if files.len() <= KEEP_FILES {
        return 0;
    }

    let mut removed = 0;
    for path in &files[..files.len() - KEEP_FILES] {
        match std::fs::remove_file(path) {
            Ok(()) => removed += 1,
            Err(err) => {
                tracing::debug!(target: "app", error = %err, "an old log could not be removed");
            }
        }
    }
    removed
}

/// The last [`TAIL_LINES`] lines of `path`, parsed.
pub fn tail(path: &Path, limit: usize) -> Vec<LogLine> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = lines.len().saturating_sub(limit);
    lines[start..].iter().map(|l| parse_line(l)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_normal_line_is_split_into_its_parts() {
        let line = "2026-09-17T04:11:04.841560Z  INFO files: watching the notes folder";
        let parsed = parse_line(line);

        assert_eq!(parsed.level, "INFO");
        assert_eq!(parsed.category, "files");
        assert_eq!(parsed.message, "watching the notes folder");
        assert!(parsed.timestamp.starts_with("2026-09-17"));
    }

    #[test]
    fn every_level_is_recognised() {
        for level in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] {
            let parsed = parse_line(&format!("2026-09-17T04:11:04.8Z  {level} db: something"));
            assert_eq!(parsed.level, level);
            assert_eq!(parsed.category, "db");
        }
    }

    #[test]
    fn a_message_containing_a_colon_keeps_all_of_it() {
        let parsed = parse_line("2026-09-17T04:11:04.8Z  WARN ai: failed: the key was rejected");
        assert_eq!(parsed.category, "ai");
        assert_eq!(parsed.message, "failed: the key was rejected");
    }

    #[test]
    fn an_unparseable_line_is_kept_whole() {
        // A panic backtrace is exactly the line worth not dropping.
        let line = "thread 'main' panicked at src/lib.rs:42:";
        let parsed = parse_line(line);
        assert_eq!(parsed.message, line);
        assert_eq!(parsed.level, "");
    }

    #[test]
    fn an_empty_line_does_not_panic() {
        assert_eq!(parse_line("").message, "");
    }

    #[test]
    fn the_tail_returns_the_last_lines_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mushroom.log.2026-09-17");
        let body: String = (0..500)
            .map(|i| format!("2026-09-17T04:11:04.8Z  INFO app: line {i}\n"))
            .collect();
        std::fs::write(&path, body).unwrap();

        let lines = tail(&path, TAIL_LINES);
        assert_eq!(lines.len(), TAIL_LINES);
        assert_eq!(lines.first().unwrap().message, "line 300");
        assert_eq!(lines.last().unwrap().message, "line 499");
    }

    #[test]
    fn a_short_log_returns_everything_it_has() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mushroom.log.2026-09-17");
        std::fs::write(&path, "2026-09-17T04:11:04.8Z  INFO app: only line\n").unwrap();

        assert_eq!(tail(&path, TAIL_LINES).len(), 1);
    }

    #[test]
    fn a_missing_log_is_empty_rather_than_an_error() {
        assert!(tail(Path::new("no-such-file.log"), TAIL_LINES).is_empty());
    }

    #[test]
    fn pruning_keeps_the_newest_seven() {
        let dir = tempfile::tempdir().unwrap();
        for day in 1..=12 {
            std::fs::write(
                dir.path().join(format!("mushroom.log.2026-09-{day:02}")),
                "x",
            )
            .unwrap();
        }

        assert_eq!(prune(dir.path()), 5);

        let left = daily_logs(dir.path());
        assert_eq!(left.len(), KEEP_FILES);
        // The newest survive; the oldest are gone.
        assert!(left.last().unwrap().to_string_lossy().ends_with("09-12"));
        assert!(left.first().unwrap().to_string_lossy().ends_with("09-06"));
    }

    #[test]
    fn pruning_a_folder_within_the_limit_removes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        for day in 1..=3 {
            std::fs::write(
                dir.path().join(format!("mushroom.log.2026-09-{day:02}")),
                "x",
            )
            .unwrap();
        }
        assert_eq!(prune(dir.path()), 0);
        assert_eq!(daily_logs(dir.path()).len(), 3);
    }

    #[test]
    fn pruning_leaves_files_that_are_not_ours_alone() {
        let dir = tempfile::tempdir().unwrap();
        for day in 1..=10 {
            std::fs::write(
                dir.path().join(format!("mushroom.log.2026-09-{day:02}")),
                "x",
            )
            .unwrap();
        }
        std::fs::write(dir.path().join("something-else.txt"), "keep me").unwrap();

        prune(dir.path());
        assert!(dir.path().join("something-else.txt").exists());
    }

    #[test]
    fn the_current_log_is_the_newest() {
        let dir = tempfile::tempdir().unwrap();
        for day in [3, 1, 2] {
            std::fs::write(dir.path().join(format!("mushroom.log.2026-09-0{day}")), "x").unwrap();
        }
        let current = current_log(dir.path()).unwrap();
        assert!(current.to_string_lossy().ends_with("09-03"));
    }

    #[test]
    fn a_missing_log_folder_is_not_an_error() {
        assert!(daily_logs(Path::new("no-such-folder")).is_empty());
        assert_eq!(prune(Path::new("no-such-folder")), 0);
        assert!(current_log(Path::new("no-such-folder")).is_none());
    }
}
