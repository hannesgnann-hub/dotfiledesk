# DotfileDesk Windows

The Windows build of DotfileDesk: a Tauri 2 + React/TypeScript app for discovering, snapshotting,
diffing, editing, and safely restoring your dotfiles — see the [project overview](../README.md)
for what DotfileDesk does and why.

### ❤️ Support DotfileDesk

If DotfileDesk is useful to you, consider [becoming a GitHub Sponsor](https://github.com/sponsors/hannesgnann-hub) — see the [root README](../README.md#-support-dotfiledesk) for details.

## Quickstart

```powershell
npm install
npm run tauri dev
```

`npm run dev` alone starts just the Vite dev server in a browser; Tauri-only APIs (`invoke`,
`openPath`, `revealItemInDir`, the native file picker) won't work there. Use `npm run tauri dev`
for the real app.

## Requirements

| Tool | Purpose |
| --- | --- |
| Node.js + npm | frontend, dev server, build |
| Rust + Cargo (MSVC toolchain) | Tauri backend and desktop app |
| WebView2 Runtime | Tauri's Windows webview (preinstalled on modern Windows 10/11) |
| Visual Studio Build Tools (C++ workload) | required by the Rust MSVC toolchain |

See the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows) for the
current install steps.

```powershell
node -v
npm -v
rustc --version
```

## Where DotfileDesk keeps its data

`%APPDATA%\dev.hannesgnann.dotfiledesk\` — a SQLite database (`dotfiledesk.sqlite`) for metadata,
and an internal Git repository (`repository\`) that holds every snapshot. Your real dotfiles are
untouched except when you explicitly create a snapshot or restore a version.

## Development

| Command | Effect |
| --- | --- |
| `npm run dev` | Vite dev server only (browser preview, no Tauri APIs) |
| `npm run build` | type-checks and builds the frontend |
| `npm run tauri dev` | runs the real Windows app |
| `npm run tauri build` | builds MSI / NSIS installers |

## Project structure

```
windows_src/
├── docs/
│   └── ARCHITECTURE.md   Windows-specific architecture (command layer, plugins, packaging)
├── src/                  React + TypeScript UI
│   ├── components/       Shared UI pieces (rows, diff view, dialogs)
│   ├── pages/             Onboarding, Dashboard, Detail, History, Diff, Editor, Add Configuration
│   ├── services/          Tauri `invoke` wrappers and formatting helpers
│   └── types/              Types mirroring the Rust core's models
└── src-tauri/
    └── src/main.rs         Tauri commands wrapping `dotfiledesk-core`
```

The actual discovery/tracking/history/security logic lives in
[`../crates/dotfiledesk-core`](../crates/dotfiledesk-core) and is shared with `mac_src` and
`linux_src`. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full command reference
and [`../docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) for the shared core's architecture.

## Documentation layout

| Document | Purpose |
| --- | --- |
| [`../README.md`](../README.md) | shared project overview for all platforms |
| `README.md` | Windows usage, requirements, and quickstart (this file) |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Windows-specific technical architecture |

## License

MIT — see [`../LICENSE`](../LICENSE).
