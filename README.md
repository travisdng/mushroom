# Mushroom

A local-first personal knowledge and notes desktop application with
AI-powered retrieval. One user, one machine, no cloud.

> I write things down in Mushroom, forget where I wrote them, and later ask
> Mushroom to help me find and understand them.

Notes are plain Markdown files in a folder you control. SQLite is only an
index, and it can be rebuilt from those files at any time. The application
looks like a 1990s desktop utility on purpose.

## What it does

- **Notes are Markdown files** in a folder you choose. Edit them here or in
  anything else; Mushroom notices either way.
- **Full-text search** over everything you have written, in a few milliseconds
  on thousands of notes. See [`docs/search-syntax.md`](docs/search-syntax.md).
- **Questions answered from your own notes**, with citations you can click
  through to the source. Optional — point it at OpenAI or any
  OpenAI-compatible gateway, or never turn it on.
- **Nothing leaves the machine** unless you configure AI, and then only the
  passages relevant to the question you asked.

## Installing

Download the `.exe` from [Releases](https://github.com/travisdng/mushroom/releases)
and run it. It installs for the current user, so it needs no administrator
rights, and it fetches the WebView2 runtime if Windows does not already have
it.

The build is unsigned, so SmartScreen will warn that the publisher is unknown
— see [code signing](#code-signing) below.

Uninstalling removes the program and leaves your notes exactly where they are.

## Prerequisites for building (Windows)

| Requirement | Notes |
|---|---|
| **Node 22+** | `node -v` |
| **Rust (stable, MSVC)** | `rustup default stable-x86_64-pc-windows-msvc` |
| **Visual Studio Build Tools** | with the **Desktop development with C++** workload — supplies the MSVC linker and Windows SDK. Requires administrator rights. |
| **WebView2 Runtime** | preinstalled on Windows 11; the installer fetches it otherwise |

Installing the build tools from an elevated prompt:

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override `
  "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

If `cargo build` fails with `LNK1181: cannot open input file 'kernel32.lib'`,
the Windows SDK is missing or still installing — that component is what
provides it.

## Running

```bash
npm install
npm run tauri dev      # desktop app, hot reload for the frontend
```

The frontend also runs standalone in a browser, which is useful for UI work
without a Rust build:

```bash
npm run dev            # then open http://localhost:1420
```

Add `#gallery` to the URL for the control gallery — every button, field,
panel, and dialog state side by side.

## Building

```bash
npm run check          # typecheck, fmt, clippy, and Rust tests
npm run tauri build    # produces an NSIS .exe and an .msi
```

Installers land in `src-tauri/target/release/bundle/`, or under
`$CARGO_TARGET_DIR` if you have set one.

### Measuring

```powershell
node scripts/make-corpus.mjs $env:USERPROFILE\Mushroom\bench-notes
$env:MUSHROOM_BENCH_NOTES = "$env:USERPROFILE\Mushroom\bench-notes"
cargo test --release --test bench -- --ignored --nocapture   # search, open, index
.\scripts\bench.ps1                                          # startup, memory, idle CPU
```

The corpus generator is seeded, so two runs measure the same 5,000 notes. The
numbers for each release are in its release notes.

### Code signing

Builds are unsigned, so Windows SmartScreen warns that the publisher is
unknown. There is no certificate for this project; buying one is the only
fix, and telling people to disable SmartScreen is not.

When a certificate exists, no code change is needed:

```powershell
$env:MUSHROOM_SIGN_PFX = "C:\path\to\certificate.pfx"
$env:MUSHROOM_SIGN_PASSWORD = "..."
npm run tauri build -- --config src-tauri/tauri.signed.conf.json
```

`src-tauri/tauri.signed.conf.json` holds the `signtool` invocation — SHA-256,
with an RFC 3161 timestamp so signatures outlive the certificate. Keep the
`.pfx` out of the repository and out of CI logs.

## Layout

```
src/                  React + TypeScript frontend
├── components/       chrome, common controls, per-feature components
├── pages/            MainWindow, Gallery
├── hooks/            shell state, keyboard shortcuts
├── services/         the only callers of Tauri `invoke`
├── types/            DTOs mirroring the Rust structs
└── styles/           hand-written retro CSS

src-tauri/            Rust backend
├── src/commands/     thin #[tauri::command] wrappers
├── src/config/       settings load/save
├── src/error.rs      AppError → AppErrorDto
├── src/logging.rs    rolling file logs with key redaction
└── src/state.rs      managed application state

docs/                 shortcuts, search syntax, troubleshooting, error audit
scripts/              corpus generator and the performance harness
tools/make_icons.py   regenerates the icon set from pixel grids
```

The frontend never touches the filesystem, the network, or the database —
everything goes through a Tauri command.

## Where your data lives

| What | Where |
|---|---|
| Notes | `%USERPROFILE%\Mushroom\notes`, or wherever you point it |
| Settings | `%APPDATA%\Mushroom\config.json` |
| Index | `%APPDATA%\Mushroom\mushroom.db` |
| Logs | `%APPDATA%\Mushroom\logs\`, one file per day, seven days |
| API keys | Windows Credential Manager — never on disk in a file |

Notes you mark `ai: false` are never sent to an AI endpoint, and everything
that *is* sent is checked for passwords and keys first. See
[docs/troubleshooting.md](docs/troubleshooting.md#a-note-of-mine-should-never-go-to-the-ai)
for what that does and does not promise.

Only the first of those matters. The index is built from your notes and can be
deleted at any time; settings are a small JSON file; the logs never contain
note content or keys.

Uninstalling removes the application, never your notes.

## Documentation

| Page | For |
|---|---|
| [`docs/shortcuts.md`](docs/shortcuts.md) | Every keyboard shortcut (also `F1` in the app) |
| [`docs/search-syntax.md`](docs/search-syntax.md) | Phrases, exclusion, prefix matching |
| [`docs/troubleshooting.md`](docs/troubleshooting.md) | When something is wrong |
| [`docs/error-audit.md`](docs/error-audit.md) | Every failure path, how it was forced, what happened |

## Roadmap

| Version | What |
|---|---|
| 0.1.0 | Shell — window, menus, toolbar, panels, status bar |
| 0.2.0 | Notes — Markdown files, folders, editing, preview |
| 0.3.0 | Local search — SQLite FTS5, snippets, rebuildable index |
| 0.4.0 | AI provider — LiteLLM or OpenAI, settings, connection test |
| 0.5.0 | AI search — grounded answers with source references |
| 1.0.0 | Polish — quick open, diagnostics, installer, performance |

**Not in 1.0:** semantic search (keyword only), a signed installer, and any
platform but Windows.

Release notes for each version are in [`docs/releases/`](docs/releases/), and
the running log of changes is in [`CHANGELOG.md`](CHANGELOG.md).

Specs, steering documents, and the release plan live in `.kiro/`. That
directory is deliberately not committed, so it exists only in a local
checkout.

## Licence

Apache-2.0. See [LICENSE](LICENSE).
