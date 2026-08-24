# DotfileDesk

**GitHub Desktop for your dotfiles.**

DotfileDesk is an open-source, cross-platform desktop app that makes managing developer
configuration files (`~/.zshrc`, `~/.gitconfig`, `~/.config/nvim`, `~/.ssh/config`, …) simple and
visual — without asking you to learn `chezmoi`, GNU Stow, `yadm`, or bare Git repositories.

Repository: [github.com/hannesgnann-hub/dotfiledesk](https://github.com/hannesgnann-hub/dotfiledesk)

## ❤️ Support DotfileDesk

Hi, I'm Hannes, the creator of DotfileDesk.

If DotfileDesk saves you time, consider supporting its development. Your sponsorship helps me fix
bugs, build the features on the [roadmap](#roadmap) below, and keep DotfileDesk free, open source,
and fully local-first.

[Become a GitHub Sponsor](https://github.com/sponsors/hannesgnann-hub)

## Highlights

- discovers configuration files already sitting where your tools expect them — nothing is touched
  until you explicitly choose to manage it
- tracks Shell, Git, Terminal, Editor, SSH, Package Manager, and Developer Tool configs out of the
  box, grouped by category on the dashboard
- snapshots on demand — `Create Snapshot` for one configuration, `Snapshot All` for everything that
  changed — with Git as the invisible history engine underneath
- integrated line-level diff view for every snapshot, and for uncommitted changes against the last
  snapshot
- **integrated editor** — open a tracked file (or browse into a tracked directory like `nvim` or
  `fish`) and edit it right inside the app, no external editor required
- safe restore: the current version is always backed up before any restore, and the restore itself
  becomes a new, visible history entry
- missing-file recovery — if a tracked file disappears, restore it from the last snapshot or stop
  tracking it
- private keys (`id_rsa`, `id_ed25519`, `*.pem`, `*.key`, …) are hard-blocked from tracking, no
  override; anything else sensitive (SSH config, `.npmrc`, AWS credentials, …) requires explicit
  confirmation before DotfileDesk touches it
- **fully offline** — no accounts, no cloud, no telemetry; everything lives in a local SQLite
  database and a local Git repository you never have to interact with directly

## Install

DotfileDesk isn't packaged for distribution yet — for now, build it from source (below). Signed
release bundles for macOS/Linux/Windows are on the [roadmap](#roadmap).

## Quickstart

```bash
cd mac_src      # or linux_src / windows_src
npm install
npm run tauri dev
```

`npm run dev` alone only starts the Vite dev server in a browser — Tauri-only APIs (file
read/write, the native picker, `Open`/`Show in Finder`) won't work there. Use `npm run tauri dev`
for the real app.

## Requirements

| Tool | Purpose |
| --- | --- |
| Node.js + npm | frontend, dev server, build |
| Rust + Cargo | Tauri backend and desktop app |
| Platform build toolchain | Xcode CLT (macOS) · WebKitGTK/GTK3 (Linux) · MSVC + WebView2 (Windows) |

```bash
node -v
npm -v
rustc --version
```

If Rust is missing:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
```

See each platform's own README for exact prerequisites:
[`mac_src/README.md`](mac_src/README.md) · [`linux_src/README.md`](linux_src/README.md) ·
[`windows_src/README.md`](windows_src/README.md).

## Repository layout

```
dotfiledesk/
├── crates/
│   └── dotfiledesk-core/   # GUI-independent Rust core: discovery, tracking, history, backup, security
├── definitions/            # Contributor-editable catalog of known config locations (JSON)
├── docs/
│   └── ARCHITECTURE.md     # Technical architecture (shared core, data flow, security model)
├── mac_src/                # Tauri 2 + React/TypeScript app for macOS
├── linux_src/               # Tauri 2 + React/TypeScript app for Linux
├── windows_src/             # Tauri 2 + React/TypeScript app for Windows
└── LICENSE
```

Each `*_src` directory is a complete, independent Tauri application (its own `package.json` and
`src-tauri/`), mirroring the layout of [easyalias](https://github.com/hannesgnann-hub/easyalias).
All three depend on the same `dotfiledesk-core` crate via a path dependency, so discovery,
snapshotting, diffing, restore, and editor logic is written and tested exactly once. See
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full technical breakdown.

## Core crate

`crates/dotfiledesk-core` has no dependency on Tauri, React, or any GUI toolkit — it can be driven
from a CLI or tested in isolation. Run its test suite with:

```bash
cd crates/dotfiledesk-core
cargo test
```

It's organized by responsibility:

- **`discovery`** — loads the built-in catalog from `/definitions/*.json` and scans the filesystem
  for matches. Read-only; never modifies anything on disk.
- **`tracking`** — SQLite-backed metadata store (which configurations are managed, when they were
  added, when they were last snapshotted).
- **`history`** — the internal Git repository that stores every snapshot, computes structured
  diffs, and restores files back to a chosen version.
- **`backup`** — the restore workflow: back up the current state, restore the target version,
  verify it landed, record the restore itself as a new history entry.
- **`security`** — sensitivity classification and the private-key block list.
- **`models`** — shared data types (`Configuration`, `Snapshot`, `Status`, `Sensitivity`, …).

## How data is stored

Nothing is uploaded anywhere. Locally, DotfileDesk keeps:

- **`dotfiledesk.sqlite`** — which files/directories are tracked, and snapshot metadata
  (id, timestamp, git commit, reason). Never stores file contents.
- **`repository/`** — a plain Git repository, invisible to the user, that stores the actual
  snapshot contents. Each tracked configuration lives under its own `{id}/` subtree so unrelated
  files never collide.

Both live under the platform's standard per-user app-data directory (e.g.
`~/Library/Application Support/dev.hannesgnann.dotfiledesk` on macOS,
`~/.local/share/dotfiledesk` on Linux, `%APPDATA%\dev.hannesgnann.dotfiledesk` on Windows). Your
real dotfiles are **never** replaced with symlinks in v0.1 — DotfileDesk copies their contents into
its internal history and writes back to the same real path on restore or edit.

## Data model

```jsonc
// configurations
{
  "id": "5e1c…",
  "definition_id": "zsh",          // null for a custom-added configuration
  "name": "Zsh",
  "path": "/Users/hannes/.zshrc",
  "category": "shell",
  "kind": "file",                   // "file" | "directory"
  "sensitivity": "normal",          // "normal" | "potential_secret" | "highly_sensitive"
  "added_at": "2026-08-12T09:14:00Z",
  "last_snapshot_at": "2026-08-24T21:41:00Z"
}

// snapshots
{
  "id": "b1e2c85…",
  "configuration_id": "5e1c…",
  "created_at": "2026-08-24T21:41:00Z",
  "git_commit": "b1e2c85f…",
  "reason": "Manual snapshot"        // or "Pre-restore backup", "Restored to <oid>", …
}
```

`status` (`synced` / `modified` / `not_tracked` / `missing` / `warning`) is never stored — it's
recomputed live by diffing the file on disk against the latest snapshot every time the dashboard
or detail page loads.

## Security model

- **Private keys are never tracked automatically.** Filenames matching `id_rsa`, `id_ed25519`,
  `*.pem`, `*.key`, etc. are hard-blocked at the core level — there's no confirmation flow that
  bypasses this.
- **Sensitive files require confirmation.** Anything classified `potential_secret` or
  `highly_sensitive` (SSH config, `.npmrc`, AWS config, …) must be explicitly confirmed by the user
  before DotfileDesk will track it.
- **Nothing leaves the machine.** v0.1 has no network calls related to configuration content —
  no accounts, no cloud sync, no telemetry.

## Extending the catalog

`definitions/*.json` is the source of truth for "known" configurations. Adding support for a new
tool is usually just adding an entry:

```json
{
  "id": "starship",
  "application": "Starship",
  "category": "terminal",
  "kind": "file",
  "paths": ["~/.config/starship.toml"],
  "platforms": ["macos", "linux", "windows"],
  "sensitivity": "normal"
}
```

`category` is one of `shell`, `git`, `terminal`, `editor`, `ssh`, `package_managers`,
`developer_tools`, `other`. `kind` is `file` or `directory`. `sensitivity` is `normal`,
`potential_secret`, or `highly_sensitive`. These definitions are compiled into the app at build
time, so a new entry just needs a PR — no runtime plugin system required for v0.1.

## Development

| Command (run inside `mac_src` / `linux_src` / `windows_src`) | Effect |
| --- | --- |
| `npm run dev` | Vite dev server only (browser preview, no Tauri APIs) |
| `npm run build` | type-checks and builds the frontend |
| `npm run tauri dev` | runs the real desktop app |
| `npm run tauri build` | builds the platform installer/bundle |
| `cargo test` (in `crates/dotfiledesk-core`) | runs the full core test suite |

## v0.1 feature scope

Included: discovery, manual add (catalog + custom), local snapshots, Git-backed history, diffing,
an integrated editor, safe restore (with mandatory pre-restore backup), missing-file recovery,
sensitivity gating, fully offline operation.

## Roadmap

Deliberately **not** in v0.1 — see [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for where these
would plug into the existing design without disturbing it:

- signed, notarized release bundles for all three platforms
- cloud/Git sync (private GitHub repo, GitLab, self-hosted, or encrypted remote storage)
- multi-machine support (common + machine-specific overrides per configuration)
- profiles (Work / Personal / Minimal, e.g. separate `git.email` per profile)
- optional symlink management as an advanced alternative to copy-based snapshots
- Windows registry management
- AI-assisted snapshot summaries
- SSH private key sync (explicitly out of scope even long-term unless it's done with real,
  user-controlled encryption)

## Documentation layout

| Document | Purpose |
| --- | --- |
| `README.md` | shared project overview for all platforms (this file) |
| `docs/ARCHITECTURE.md` | technical architecture: layers, data flow, security model |
| `mac_src/README.md`, `mac_src/docs/ARCHITECTURE.md` | macOS usage and platform-specific architecture |
| `linux_src/README.md`, `linux_src/docs/ARCHITECTURE.md` | Linux usage and platform-specific architecture |
| `windows_src/README.md`, `windows_src/docs/ARCHITECTURE.md` | Windows usage and platform-specific architecture |

## License

DotfileDesk is licensed under the MIT License. See [`LICENSE`](LICENSE).
