# Changelog

All notable changes to Mushroom. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); versions follow SemVer, and
before 1.0 the minor version tracks the build milestone.

Add entries here as work lands. On release, move the Unreleased entries into a
dated version section and write the matching file in
[`docs/releases/`](docs/releases/).

## [1.0.1] — 2026-09-18

See [docs/releases/v1.0.1.md](docs/releases/v1.0.1.md).

### Fixed
- **The uninstaller could hang.** The "your notes are NOT removed" message
  added in 1.0.0 was shown unconditionally, including under `/S` — where a
  modal dialog does not annoy anyone, it hangs forever with nobody there to
  dismiss it — and during an upgrade, which runs the old uninstaller with
  `/UPDATE` and is not a removal at all. It now appears only for an uninstall
  somebody started and is watching.

## [1.0.0] — 2026-09-18

Milestone 06 — polish and packaging. See
[docs/releases/v1.0.0.md](docs/releases/v1.0.0.md).

### Added
- **Quick Open** (`Ctrl+P`) — type any part of a note's name or folder and go
  straight to it. Matched in the window against the cached list, so there is no
  round trip per keystroke.
- **Mushroom notices edits made outside it.** The notes folder is watched; a
  change from another editor updates the list and the index within a second. An
  open, unmodified note reloads in place; one with unsaved changes asks, with
  `Keep mine` / `Load theirs` / `Save as copy`. Nothing is overwritten silently.
- **Diagnostics** (`Tools → Diagnostics`) — versions, paths, index health, AI
  status and the tail of the log on one screen, with `Copy Diagnostics`,
  `Open Log Folder`, `Rebuild Index…`, `Test Connection…` and `Delete Old Logs`.
  The copied report has API keys removed and home paths rewritten, and says so
  along with what it does still include.
- **Log rotation** — one file per day, seven days, pruned at startup.
- A first-run `Welcome to Mushroom` note, **opened** so it is read rather than
  merely created. It is an ordinary note: edit it, delete it, and it stays
  deleted.
- Documentation: [`docs/search-syntax.md`](docs/search-syntax.md) (linked from
  the search panel), [`docs/troubleshooting.md`](docs/troubleshooting.md), and
  [`docs/error-audit.md`](docs/error-audit.md).
- A reproducible 5,000-note benchmark corpus and the harness that measures
  against it: `scripts/make-corpus.mjs`, `scripts/bench.ps1`, and
  `src-tauri/tests/bench.rs`.
- The uninstaller now says, before removing anything, that your notes and
  settings are kept and where they are.
- `src-tauri/tauri.signed.conf.json`, so a signed build needs a certificate and
  two environment variables rather than a code change.

### Fixed
- **A panic in one operation killed the whole application.**
  `[profile.release]` set `panic = "abort"`, so `panics::guard` — which is
  tested, and which the error audit said turned panics into ordinary errors —
  had nothing to catch in the build people install. It now compiles only under
  unwinding, so this cannot come back quietly.
- **Every control was invisible to `Tab`.** `reset.css` removed the focus
  outline without replacing it, so dialog fields, the question box, Quick Open
  and the toolbar had no visible focus. A dotted focus rectangle is now global.
- **A settings file with a UTF-8 byte-order mark reset every setting.** The
  BOM is now stripped before parsing. Windows PowerShell writes one by default,
  which is how this was found.
- **A window could be restored where it cannot be dragged back.** A saved
  position was accepted if any part of it overlapped a monitor, including one
  whose title bar sat above the screen.
- The keyboard shortcut list claimed `Ctrl+F` and `Ctrl+P` were unavailable
  long after both worked. There is now one list that the dialog, the docs and
  the handler map are all checked against — a documented shortcut with no
  handler is a compile error.
- The folder watcher woke twice a second for the life of the session whether or
  not anything had changed. It now sleeps until the filesystem wakes it.
- The preview re-parsed the entire note on every keystroke despite being
  debounced: it reads the note context, and a context consumer re-renders when
  the context changes whatever its props say. The rendered output is now
  memoised on the text.

### Changed
- The note list is virtualised: 5,000 notes render about forty rows rather than
  five thousand.

## [0.5.0] — 2026-09-17

Milestone 05 — AI search. See
[docs/releases/v0.5.0.md](docs/releases/v0.5.0.md).

### Added
- **Ask questions of your notes.** An `AI SEARCH` panel with a question box,
  `Ask Mushroom` / `Stop`, and a drop-down of your last twenty questions.
- Answers stream in as they are written, with the notes retrieval found shown
  first so there is something to read while the model thinks.
- Inline `[1]` citations that open the cited note at the passage they came
  from, a `Sources:` list showing the excerpt each answer was actually given,
  and a collapsed `Also searched` list of notes that were retrieved but not
  cited.
- Grounding checks: an answer citing nothing is marked unverified, and a
  citation to an excerpt that was never sent is called out.
- A question with no matching notes never reaches the model at all.
- `Tools → Settings` gained a streaming switch; a service that refuses
  streaming falls back to a single request automatically.
- Per-request usage records and totals for a future Diagnostics panel.

### Fixed
- Question retrieval required *every* word, so a question of five or six terms
  found nothing. Questions now match any term, ranked; the search box is
  unchanged.
- `cargo test` deleted the developer's own stored API key: the credential tests
  used the application's Credential Manager service name.
- A `400 Bad Request` was retried, because every unclassified status became
  `ServerError` and all of those were treated as transient.
- A stream that broke part-way was re-requested in full, discarding the partial
  answer already on screen.
- The `AI is not configured` state could never appear, because the endpoint
  always holds a plausible default — and once it could, configuring AI did not
  clear it until the app was restarted.

### Known issues
- Retrieval is still keyword-based. A question worded differently from the note
  will miss it; that is what v1.1 is for.
- `Tools → Diagnostics` is still greyed out.
- The installer is unsigned.

## [0.4.0] — 2026-09-16

Milestone 04 — AI provider and settings. See
[docs/releases/v0.4.0.md](docs/releases/v0.4.0.md).

### Added
- `Tools → Settings`: a period-styled dialog with AI Service, Notes and
  Advanced sections, OK / Cancel / Apply, and validation shown beside the
  offending field.
- Two provider presets — LiteLLM and OpenAI — and any other
  OpenAI-compatible endpoint by typing a URL.
- API keys stored in Windows Credential Manager, one per provider, never in
  `config.json` and never returned across the IPC boundary.
- `Test Connection`, which makes one minimal real request and reports the
  endpoint, model and latency, or a specific reason it failed.
- Model list from `GET /models`, with a free-text fallback for endpoints that
  do not offer one.
- An HTTP client with a 10 s connect timeout, a configurable overall timeout,
  one retry on connect errors and 5xx, and cancellation that aborts the
  in-flight request.
- Server-sent-events streaming, ready for milestone 05.
- Log redaction applied at the writer, so every line — including lines from
  dependencies — is scrubbed of anything resembling a credential.
- The notes folder can be changed from Settings, with a native folder picker.

### Fixed
- `redact()` scanned for credential prefixes in list order rather than by
  position, so an earlier secret could be copied out verbatim on the way to a
  later match. It had also never been wired into `tracing` at all.
- Settings files written before the AI section kept reporting `"version": 1`,
  because the schema constant was bumped and nothing ever wrote it.
- The model combo filtered its options by what was already typed, so `Refresh`
  appeared to return only the model already in the box.
- `Refresh` and `Test Connection` built their requests from the saved settings
  rather than the ones on screen. Since a key is stored as soon as it is
  entered, that could send a newly-entered key to the previously-saved
  endpoint.

### Known issues
- No AI panel yet — asking questions of your notes is milestone 05. Settings
  configures the service; nothing calls it except Test Connection.
- `Tools → Diagnostics` is still greyed out.
- The installer is unsigned.

## [0.3.0] — 2026-09-16

Milestone 03 — local search. See
[docs/releases/v0.3.0.md](docs/releases/v0.3.0.md).

### Added
- Full-text search over every note with `Ctrl+F`: ranked results with marked
  snippets, opening at the matching line.
- Quoted phrases, `-exclusion`, `prefix*` matching, and word stemming.
- Folder scoping and a drop-down of the last twenty queries.
- `Tools → Rebuild Index` with progress, stating plainly that it does not
  modify any Markdown.
- Status bar reports `Indexing n / total…` and `Index out of date`.
- SQLite FTS5 index at `%APPDATA%\Mushroom\mushroom.db`, storing
  heading-delimited passages with their line ranges.
- `Retriever` trait with normalised scores, so semantic and hybrid retrieval
  can be added later without changing callers.

### Fixed
- Notes could not be found by their title when the body never used those
  words; the title index was built but never queried.
- A heading ran into the following body text when indexed, producing a single
  token and making the first body word after every heading unsearchable.

### Known issues
- Search is keyword-based; differently-worded queries will miss. Semantic
  search is deliberately post-1.0.
- No AI yet — milestones 04 and 05.
- No date-range filter in the UI, though the backend supports one.
- The installer is unsigned.

## [0.2.0] — 2026-09-16

Milestone 02 — notes. See [docs/releases/v0.2.0.md](docs/releases/v0.2.0.md).

### Added
- Markdown notes on disk: create, edit, save, rename, move, and delete, with
  folders. Notes live in `%USERPROFILE%\Mushroom
otes` as ordinary `.md`
  files; the first run creates `work`, `projects`, `ideas`, and `personal`.
- Notebook pane with a folder tree and counts; notes list grouped by date,
  newest first.
- Markdown preview (GFM tables, code, quotes, lists) with Edit / Preview /
  Split modes, on the View menu or `Ctrl+Shift+P`.
- Autosave three seconds after typing stops, and on note switch and window
  blur. Manual save on `Ctrl+S`.
- Deletes move to `.trash` inside the notes folder and ask for confirmation,
  with Cancel as the default.
- Links between notes: relative `.md` paths and `[[wikilinks]]`. External
  links open in the system browser.
- `F5` re-reads the notes folder after external changes.
- CI workflow running the same `npm run check` gate on every push and pull
  request, and a release workflow that builds both installers from a `v*` tag
  and publishes them to a GitHub release.

### Changed
- `npm run check` now runs the real `vite build` rather than `tsc --noEmit`,
  so the gate catches build-time failures that typechecking alone misses.
- Keyboard shortcut handlers are memoised, so the global keydown listener is
  registered once instead of on every render.
- The folder scan no longer re-stats files the directory walk already
  described, and reads file heads across threads. 5,000 notes scan in 1.3s.

### Fixed
- A window closed while maximised reopened screen-sized but not maximised, and
  un-maximising did nothing visible. The maximised state is now stored
  separately and the normal-state size is preserved, so un-maximising returns
  to the size you last used.
- Creating a note at the notes root could be rejected as a path escape, because
  the note id was derived from a path that had not been canonicalised and could
  carry a Windows 8.3 short name.
- Frontmatter timestamps were quoted unnecessarily; YAML only needs a scalar
  quoted when a colon is followed by a space.

### Known issues
- No search or AI yet — milestones 03 through 05.
- External edits are detected on save, not live; there is no file watcher until
  milestone 06.
- `File → Open`, `Import`, `Export`, and `Save As` are present in the menus but
  not yet wired to a file picker, though the backend commands exist.
- The installer is unsigned.

## [0.1.0] — 2026-09-16

Milestone 01 — the application shell. See
[docs/releases/v0.1.0.md](docs/releases/v0.1.0.md).

### Added
- Tauri v2 + React 19 + TypeScript desktop application, 1100x720, titled
  "Mushroom — Personal Knowledge".
- Classic window chrome: menu bar with seven menus, toolbar, NOTEBOOK/NOTES
  sidebar, editor pane, optional AI panel, status bar.
- Menus driven by mouse and keyboard: `Alt` focus, mnemonics, arrow-key
  navigation, right-aligned accelerators, separators, disabled items for
  features that do not exist yet.
- Draggable splitters with minimum pane sizes.
- About and Keyboard Shortcuts dialogs. About sources its version from the
  Rust backend, proving the IPC boundary.
- Window position, size, and splitter positions persist across restarts; an
  off-screen saved position is replaced by centring.
- Hand-drawn pixel mushroom icon set (16-256px PNGs plus a 7-size `.ico`),
  generated by `tools/make_icons.py`.
- `AppError` → `AppErrorDto` error contract; nothing raw reaches the UI.
- Rolling file logging with API-key redaction, at
  `%APPDATA%\Mushroom\logs\`.
- `npm run check` — typecheck, `cargo fmt --check`, clippy with
  `-D warnings`, and `cargo test`.

### Fixed
- Window grew by the decoration size (22x56px) on every launch, because
  geometry was saved as the outer size but restored as the inner size.
- A window reporting an implausible size mid-creation (height 32767) could be
  persisted and restored, leaving an unusable window.
- Application data was written to `%APPDATA%\com.mushroom.app` (Tauri's
  identifier-based default) instead of `%APPDATA%\Mushroom`.

### Known issues
- No notes, search, or AI — milestones 02 through 05.
- The installer is unsigned; SmartScreen warns that the publisher is unknown.
- Windows only.
- Not tested on a separate Windows profile, or on a machine without the
  WebView2 runtime already installed.

---

<!--
Template for a released version — copy, do not edit this comment.

## [X.Y.Z] — YYYY-MM-DD

### Added
### Changed
### Fixed
### Known issues
-->
