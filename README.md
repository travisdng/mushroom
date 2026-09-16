# Mushroom

A local-first personal knowledge and notes desktop application with
AI-powered retrieval. One user, one machine, no cloud.

> I write things down in Mushroom, forget where I wrote them, and later ask
> Mushroom to help me find and understand them.

Notes are plain Markdown files in a folder you control. SQLite is only an
index, and it can be rebuilt from those files at any time. The application
looks like a 1990s desktop utility on purpose.

## Status

Milestone 1 — the application shell. No notes, search, or AI yet; see the
roadmap below.

## Prerequisites (Windows)

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

Installers land in `src-tauri/target/release/bundle/`.

Builds are currently unsigned, so Windows SmartScreen will warn that the
publisher is unknown.

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

tools/make_icons.py   regenerates the icon set from pixel grids
```

The frontend never touches the filesystem, the network, or the database —
everything goes through a Tauri command.

## Where your data lives

| What | Where |
|---|---|
| Notes | `%USERPROFILE%\Mushroom\notes` (from milestone 2) |
| Settings | `%APPDATA%\Mushroom\config.json` |
| Index | `%APPDATA%\Mushroom\mushroom.db` (from milestone 3) |
| Logs | `%APPDATA%\Mushroom\logs\` |

Uninstalling removes the application, never your notes.

## Roadmap

| Version | What |
|---|---|
| 0.1.0 | Shell — window, menus, toolbar, panels, status bar |
| 0.2.0 | Notes — Markdown files, folders, editing, preview |
| 0.3.0 | Local search — SQLite FTS5, snippets, rebuildable index |
| 0.4.0 | AI provider — LiteLLM or OpenAI, settings, connection test |
| 0.5.0 | AI search — grounded answers with source references |
| 1.0.0 | Polish — quick open, diagnostics, installer, performance |

Release notes for each version are in [`docs/releases/`](docs/releases/), and
the running log of changes is in [`CHANGELOG.md`](CHANGELOG.md).

Specs, steering documents, and the release plan live in `.kiro/`. That
directory is deliberately not committed, so it exists only in a local
checkout.

## Licence

Apache-2.0. See [LICENSE](LICENSE).
