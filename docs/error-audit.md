# Error audit

Every failure path Mushroom is expected to survive, how it was forced, and
what actually happened. The point of this page is the **Observed** column: a
path listed here with nothing observed has not been tested, and says so.

Rules each row is judged against, from `.kiro/steering/error-handling.md`:

- the message names the thing that failed, in plain words,
- it says what to do next where there is anything to do,
- nothing the user typed is lost,
- no raw Rust text reaches the screen.

## Forced and observed

| Path | How it was forced | Observed |
|---|---|---|
| **Read-only note** | `attrib +R` on the file, then edit and `Ctrl+S` | "Mushroom could not save that note — The file at `…\realtime-guidance.md` could not be written. **Your changes are still in the editor.**" Hint: "Check the disk is not full and the file is not read-only." `Details…` present, status bar stayed **Modified**. Nothing lost. |
| **Note changed on disk while open and unmodified** | append to the file from PowerShell | Reloaded in place, `Reloaded from disk` in the status bar. |
| **Note changed on disk with unsaved edits** | type in the editor, then append externally | "This note changed on disk … Nothing has been saved yet, so neither version is lost." with `Keep mine` / `Load theirs` / `Save as copy`. `Keep mine` wrote the unsaved text intact; `Save as copy` produced a second note with both kept. |
| **Concurrent save (same note, two writers)** | integration test `concurrent_save_is_refused_and_loses_nothing` | Save refused, both versions intact. |
| **Cited note renamed after an answer** | rename the file, then click its source | "Mushroom could not open that note — The file at `…` could not be read." The answer and its sources stayed intact. |
| **Corrupt search index** | scribble over `mushroom.db` | Renamed to `mushroom.db.corrupt.<timestamp>` — kept, not deleted — and a fresh index built. Notes untouched. |
| **Index from a newer version** | write a higher `schema_version` | Refused rather than downgraded. |
| **AI endpoint refusing connections** | point at `http://localhost:4000` with nothing listening | "Mushroom could not connect to http://localhost:4000." |
| **AI host that does not resolve** | `http://no-such-host.invalid/v1` | "The address no-such-host.invalid could not be resolved." — distinct from the above. |
| **Expired TLS certificate** | `https://expired.badssl.com/v1` | "Mushroom could not establish a secure connection to …" |
| **Endpoint missing `/v1`** | `https://api.openai.com` against the real API | Real 404, plus the hint naming `https://api.openai.com/v1`. |
| **Rejected API key** | a bogus key against the real OpenAI API | "The AI service refused the key (401). Check the key in Tools → Settings." Retrieved notes stayed listed and clickable. |
| **Stream broken mid-answer** | a stub server that destroys the socket part-way | One request only, partial answer kept, "This answer is incomplete — it stopped before the model finished." |
| **User stops an answer** | `Stop` mid-stream | Partial kept and marked; **no** error strip — stopping is not a failure. |
| **Panic inside a command** | unit tests in `panics.rs` | Becomes `AppError::Internal`; payload and backtrace logged; the next command still works. |
| **Watcher cannot start** | unit test against a non-existent folder | Returns an error rather than panicking; `watching: false` surfaces in Diagnostics and `F5` still refreshes. |

## Not forced

Recorded rather than quietly skipped.

| Path | Why not | Risk if wrong |
|---|---|---|
| **Disk full during save** | needs a small VHD mounted as the notes root | The save path is the same one the read-only test exercised, and it is atomic — a failed write leaves the original intact. The *message* would differ. |
| **File locked by another process** | attempted three times; driving the exact note through the UI proved unreliable, and the attempts kept landing on the wrong note | Same write path and the same `NoteWrite` error as read-only, so the handling is shared — but the specific message has not been seen. |
| **Database locked by another writer** | needs a second SQLite process holding a write lock | Search would degrade; notes are a separate path and unaffected. |
| **Panic surfaced through IPC** | the guard is unit-tested, but no deliberate panicking command is wired | The wrapper is on the shared `blocking` helper every note command uses. |
| **Network lost mid-answer (real adapter)** | a stub socket drop stood in for it | Believed equivalent; the reqwest error class may differ. |

## Fixed because of this audit

- **`Copy Diagnostics` did not say what it kept.** It correctly removed keys
  and rewrote paths, but silently included the configured endpoint — which can
  be an internal hostname. The report now states plainly that it includes the
  endpoint and folder structure, so nobody pastes it into a public issue
  assuming otherwise.
