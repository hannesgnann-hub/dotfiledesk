# DotfileDesk Windows — Architecture

Platform-specific notes for the Windows shell. For the shared core (discovery, tracking, history,
security) see [`../../docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) — this file only covers
what's specific to `windows_src`.

## Command layer

`src-tauri/src/main.rs` holds one `Mutex<Core>` in Tauri's managed state and exposes it as thin
`#[tauri::command]` wrappers, all listed in `invoke_handler!`:

| Command | Core method | Used by |
| --- | --- | --- |
| `scan_configurations` | `Core::scan` | Onboarding, Add Configuration |
| `list_configurations` | `Core::list_configurations` | Dashboard |
| `get_configuration_detail` | `Core::get_configuration_detail` | Detail, History, Editor pages |
| `add_discovered` | `Core::add_discovered` | Onboarding, Add Configuration |
| `preview_custom_path` | `security::classify_path` / `is_private_key` | Add Configuration (custom form) |
| `add_custom` | `Core::add_custom` | Add Configuration (custom form) |
| `remove_configuration` | `Core::remove_configuration` | Detail page |
| `snapshot_configuration` | `Core::snapshot` | Detail page ("Create Snapshot") |
| `snapshot_all` | `Core::snapshot_all` | Dashboard ("Snapshot All") |
| `list_history` | `Core::list_history` | History page |
| `diff_snapshot` | `Core::diff_snapshot` | History → snapshot diff |
| `diff_working` | `Core::diff_working` | Detail page ("View Changes") |
| `restore_snapshot` | `Core::restore` (via `backup::restore`) | Snapshot diff ("Restore this Version") |
| `list_configuration_files` | `Core::list_configuration_files` | Editor page (directory configs) |
| `read_configuration_file` | `Core::read_configuration_file` | Editor page |
| `write_configuration_file` | `Core::write_configuration_file` | Editor page ("Save") |

`CoreError` implements `Serialize` by writing its `Display` string, so every command that fails
rejects the frontend promise with a plain, readable message — no separate error-code mapping layer.

The `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` attribute at the top of
`main.rs` suppresses the console window on release builds; it's a no-op on the other two platforms
that share this same file.

## Plugins & capabilities

- `tauri-plugin-dialog` — the native Explorer file/folder picker in Add Configuration.
- `tauri-plugin-opener` — `openPath` ("Open") and `revealItemInDir` ("Show in Explorer"), called
  directly from the frontend without a custom command.

`capabilities/default.json` grants `core:default`, `dialog:default`, `opener:default` to the main
window only.

## Runtime dependencies

Tauri renders through the WebView2 runtime, preinstalled on current Windows 10/11 but required as
a redistributable on older installs. The Rust MSVC toolchain additionally needs the Visual Studio
Build Tools C++ workload — see the [README](../README.md#requirements).

## Window

`tauri.conf.json` uses the standard Windows title bar (no `titleBarStyle`/`hiddenTitle` — those
are macOS-only options).

## Data locations

`app.path().app_data_dir()` resolves to `%APPDATA%\dev.hannesgnann.dotfiledesk\`. `Core::init`
creates `dotfiledesk.sqlite` and `repository\` under it on first launch.

## Packaging

`npm run tauri build` produces MSI and NSIS installers under
`src-tauri\target\release\bundle\`, per `tauri.conf.json`'s `bundle.targets`, using
`icons/icon.ico` + `icons/icon.png`.
