//! What Mushroom knows about itself.
//!
//! Everything here is for someone trying to work out why something went wrong
//! — theirs, or ours. It is therefore all safe to hand over: the report goes
//! through [`redact`] on the way out, and the log view shows lines that were
//! already scrubbed when they were written.

pub mod logs;
pub mod redact;

use serde::Serialize;

/// One line of the log, parsed enough to filter it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub timestamp: String,
    pub level: String,
    /// `app`, `files`, `db`, `search`, `ai`, `net`.
    pub category: String,
    pub message: String,
}

/// Everything the Diagnostics window shows, gathered in one call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub app: AppInfo,
    pub index: IndexInfo,
    pub ai: AiInfo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub data_dir: String,
    pub log_dir: String,
    pub notes_root: Option<String>,
    pub notes_count: usize,
    pub notes_skipped: usize,
    /// False when the notes folder could not be watched (R2.7).
    pub watching: bool,
    pub config_version: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub database_path: Option<String>,
    pub note_count: i64,
    pub passage_count: i64,
    pub stale: bool,
    pub indexing: bool,
    pub skipped: usize,
    pub last_full_rebuild: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiInfo {
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    pub key_status: String,
    pub configured: bool,
    pub streaming: bool,
    pub last_connection: Option<String>,
    pub requests: u32,
    pub total_tokens: u64,
    pub failures: u32,
}

/// Render the report a person can paste into a message.
///
/// Plain text on purpose: it has to survive being pasted into a chat window,
/// an email, and a GitHub issue without any of them mangling it.
pub fn render(diagnostics: &Diagnostics) -> String {
    let app = &diagnostics.app;
    let index = &diagnostics.index;
    let ai = &diagnostics.ai;

    let mut out = String::new();
    out.push_str("Mushroom diagnostics\n");
    out.push_str("====================\n\n");

    out.push_str("Application\n");
    out.push_str(&format!("  Version:        {}\n", app.version));
    out.push_str(&format!("  Platform:       {}\n", app.platform));
    out.push_str(&format!("  Config version: {}\n", app.config_version));
    out.push_str(&format!("  Data folder:    {}\n", app.data_dir));
    out.push_str(&format!("  Log folder:     {}\n", app.log_dir));
    out.push_str(&format!(
        "  Notes folder:   {}\n",
        app.notes_root.as_deref().unwrap_or("(none)")
    ));
    out.push_str(&format!("  Notes:          {}\n", app.notes_count));
    out.push_str(&format!("  Skipped files:  {}\n", app.notes_skipped));
    out.push_str(&format!(
        "  Watching:       {}\n",
        if app.watching {
            "yes"
        } else {
            "no (use F5 to refresh)"
        }
    ));

    out.push_str("\nIndex\n");
    out.push_str(&format!(
        "  Database:       {}\n",
        index.database_path.as_deref().unwrap_or("(not open)")
    ));
    out.push_str(&format!("  Notes indexed:  {}\n", index.note_count));
    out.push_str(&format!("  Passages:       {}\n", index.passage_count));
    out.push_str(&format!("  Skipped:        {}\n", index.skipped));
    out.push_str(&format!("  Out of date:    {}\n", yes_no(index.stale)));
    out.push_str(&format!("  Indexing now:   {}\n", yes_no(index.indexing)));
    out.push_str(&format!(
        "  Last rebuild:   {}\n",
        index.last_full_rebuild.as_deref().unwrap_or("(never)")
    ));

    out.push_str("\nAI service\n");
    out.push_str(&format!("  Configured:     {}\n", yes_no(ai.configured)));
    out.push_str(&format!("  Provider:       {}\n", ai.provider));
    out.push_str(&format!("  Endpoint:       {}\n", ai.endpoint));
    out.push_str(&format!("  Model:          {}\n", ai.model));
    out.push_str(&format!("  API key:        {}\n", ai.key_status));
    out.push_str(&format!("  Streaming:      {}\n", yes_no(ai.streaming)));
    out.push_str(&format!(
        "  Last test:      {}\n",
        ai.last_connection.as_deref().unwrap_or("(not tested)")
    ));
    out.push_str(&format!("  Requests:       {}\n", ai.requests));
    out.push_str(&format!("  Tokens:         {}\n", ai.total_tokens));
    out.push_str(&format!("  Failures:       {}\n", ai.failures));

    out.push_str("\nThis report contains no note titles, note content, or API keys.\n");
    out
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn sample() -> Diagnostics {
        Diagnostics {
            app: AppInfo {
                version: "1.0.0".into(),
                platform: "windows".into(),
                data_dir: r"C:\Users\alice\AppData\Roaming\Mushroom".into(),
                log_dir: r"C:\Users\alice\AppData\Roaming\Mushroom\logs".into(),
                notes_root: Some(r"C:\Users\alice\Mushroom\notes".into()),
                notes_count: 412,
                notes_skipped: 1,
                watching: true,
                config_version: 3,
            },
            index: IndexInfo {
                database_path: Some(r"C:\Users\alice\AppData\Roaming\Mushroom\mushroom.db".into()),
                note_count: 412,
                passage_count: 980,
                stale: false,
                indexing: false,
                skipped: 0,
                last_full_rebuild: Some("2026-09-17T09:00:00Z".into()),
            },
            ai: AiInfo {
                provider: "liteLlm".into(),
                endpoint: "http://localhost:4000/v1".into(),
                model: "gpt-4.1-mini".into(),
                key_status: "set".into(),
                configured: true,
                streaming: true,
                last_connection: Some("Connected in 381 ms".into()),
                requests: 7,
                total_tokens: 4200,
                failures: 1,
            },
        }
    }

    #[test]
    fn the_report_carries_what_someone_debugging_would_ask_for() {
        let report = render(&sample());
        for expected in [
            "1.0.0",
            "windows",
            "412",
            "gpt-4.1-mini",
            "Watching",
            "Passages",
        ] {
            assert!(report.contains(expected), "missing {expected}:\n{report}");
        }
    }

    #[test]
    fn a_report_carries_no_key_no_paths_and_no_note_content() {
        // The whole promise of the button: paste it anywhere without reading
        // it first.
        let notes = Path::new(r"C:\Users\alice\Mushroom\notes");
        let home = Path::new(r"C:\Users\alice");

        let mut diagnostics = sample();
        diagnostics.ai.last_connection = Some("failed with key sk-live-SHOULDNOTAPPEAR".into());

        let report = redact::redact_report(&render(&diagnostics), Some(notes), Some(home));

        assert!(!report.contains("SHOULDNOTAPPEAR"), "{report}");
        assert!(!report.contains("alice"), "{report}");
        assert!(report.contains("<notes>"), "{report}");
        assert!(report.contains("<user>"), "{report}");
    }

    #[test]
    fn an_unwatched_folder_says_what_to_do_instead() {
        let mut diagnostics = sample();
        diagnostics.app.watching = false;
        let report = render(&diagnostics);
        assert!(report.contains("F5"), "{report}");
    }

    #[test]
    fn the_report_states_what_it_does_not_contain() {
        let report = render(&sample());
        assert!(report.contains("no note titles"), "{report}");
    }
}
