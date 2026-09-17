# Troubleshooting

Start with **Tools → Diagnostics**. It shows where your notes, index, settings
and logs are, how many notes have been indexed, whether the folder watcher is
running, and the last few hundred log lines. Most of the questions below are
answered by looking at it.

`Copy Diagnostics` puts the whole report on the clipboard with API keys
removed and your home folder rewritten to `%USERPROFILE%`. It **does** include
your configured AI endpoint and your folder names, so read it before pasting
it anywhere public.

---

## Mushroom cannot connect to the AI service

The message names which of these it is. They need different fixes.

| Message | What it means | Fix |
|---|---|---|
| "could not connect to `<url>`" | Nothing is listening there | Check the service is running and the port is right. A local gateway on `http://localhost:4000` has to actually be up. |
| "The address `<host>` could not be resolved" | The hostname is wrong or DNS cannot see it | Check the spelling. An internal hostname needs you to be on the VPN. |
| "could not establish a secure connection" | TLS failed — usually an expired or self-signed certificate | If it is your own gateway, fix the certificate. Mushroom will not skip verification. |
| "refused the key (401)" | The key is wrong, expired, or revoked | Re-enter it in **Tools → Settings → AI**. Keys are stored in Windows Credential Manager, never in a file. |
| "(404)" plus a hint about `/v1` | The endpoint is missing its path | OpenAI-compatible endpoints end in `/v1` — `https://api.openai.com/v1`, not `https://api.openai.com`. |
| "(429)" | Rate limited by the provider | Wait, or check your plan's limits. |

**Test Connection** in Settings is the fastest way to tell these apart: it
reports the model it reached and how long it took, or exactly which of the
above went wrong.

### The answer stopped part-way

"This answer is incomplete — it stopped before the model finished" means the
connection dropped mid-stream. What you got is kept. Ask again.

If you pressed **Stop**, there is no error strip: stopping is not a failure,
and the partial answer is still yours.

### The answer says it cannot find anything

The AI only sees notes Mushroom retrieved for it. If your notes genuinely do
not cover the question, that is the honest answer. If you know they do, search
for the same words with `Ctrl+F` — if search finds nothing either, the index
is the problem, not the model. See below.

---

## Search results are out of date, or missing notes

Mushroom watches the notes folder, so an edit from another editor normally
appears within a second.

1. **`F5`** rescans the folder. Do this first.
2. **Tools → Rebuild Index** rebuilds the whole index from your Markdown. On
   5,000 notes this takes a few seconds and shows progress. It reads your
   notes and never writes to them — there is no way for a rebuild to lose
   anything.
3. If Diagnostics shows **watching: false**, the folder watcher could not
   start. This happens on network shares and some permission setups.
   Everything still works; you just have to press `F5` after external edits.

Notes Mushroom could not read are reported as a **skipped** count in
Diagnostics, with the reason in the log — usually a file that is not valid
UTF-8, or one locked by another program.

---

## The notes folder moved, or is on a drive that is not there

Mushroom says which folder it wanted and keeps running.

- **Tools → Settings → Notes folder** points it somewhere else. Changing it
  rescans; nothing is copied or deleted.
- If the folder is on a USB disk or a network share that is temporarily gone,
  plug it back in and press `F5`.
- Moving your notes yourself is safe: move the folder, then point Mushroom at
  the new location. The index rebuilds itself.

There is no import step. Any folder of Markdown files works — your notes are
whatever is in that folder.

---

## The search index is corrupt

You would see this as an error mentioning the database, or a rebuild that
fails immediately.

Mushroom handles it on its own: the damaged file is **renamed** to
`mushroom.db.corrupt.<timestamp>` — kept, not deleted — and a fresh index is
built from your notes. Your notes are never involved.

To force it by hand, close Mushroom and delete:

```
%APPDATA%\Mushroom\mushroom.db
```

It will be rebuilt on the next launch. The index is disposable by design; the
Markdown is the only thing that matters.

An index written by a **newer version** of Mushroom is refused rather than
downgraded — install the newer version again, or delete the file as above.

---

## Settings went back to their defaults

The settings file could not be parsed, and Mushroom carried on with defaults
rather than refusing to start. The log line is `settings could not be loaded`.

The file is `%APPDATA%\Mushroom\config.json`. If you have edited it by hand,
note that it must be UTF-8 **without** a byte-order mark — PowerShell's
`Set-Content -Encoding utf8` adds one on Windows PowerShell 5.1. Use
`-Encoding utf8NoBOM` on PowerShell 7, or just change the setting in the app.

Your API keys are in Windows Credential Manager, not in this file, so they are
unaffected.

---

## A note would not save

The message says which file and, importantly, that **your changes are still in
the editor**. Nothing is lost while the window is open.

- **Read-only file** — clear the read-only attribute, then save again.
- **Disk full** — free some space. The save is atomic, so the original note on
  disk is intact.
- **Changed on disk while you were editing** — you are offered `Keep mine`,
  `Load theirs`, or `Save as copy`. Nothing is overwritten without you
  choosing.

If you cannot resolve it, select the text and copy it out before closing.

---

## Windows says the publisher is unknown

Mushroom's installer is not code-signed, so SmartScreen warns about it. Choose
**More info → Run anyway** if you trust the download.

Do not turn SmartScreen off. Check the file against the SHA-256 published with
the release instead — that tells you the download is the one that was built,
which is what the signature would have told you.

---

## Where the logs are

```
%APPDATA%\Mushroom\logs\mushroom.log.<date>
```

One file per day, kept for seven days, plain text. API keys are stripped
before anything is written, and note content is never logged — queries and
answers do not appear, only counts and timings.

**Tools → Diagnostics** shows the tail of the current log without leaving the
app.

---

## Reporting a problem

Include the output of `Copy Diagnostics` (read it first — see the top of this
page), what you did, and what you expected instead. The version and your
Windows build are already in the report.
