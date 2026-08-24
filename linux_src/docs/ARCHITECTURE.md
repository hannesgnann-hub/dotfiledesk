# DotfileDesk Linux — Architecture

Platform-specific notes for the Linux shell. For the shared core (discovery, tracking, history,
security) see [`../../docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md) — this file only covers
what's specific to `linux_src`.

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

## Plugins & capabilities

- `tauri-plugin-dialog` — the native GTK file/folder picker in Add Configuration.
- `tauri-plugin-opener` — `openPath` ("Open") and `revealItemInDir` ("Show in File Manager"),
  called directly from the frontend without a custom command. `revealItemInDir` shells out to
  whatever file manager owns the desktop session (Nautilus, Dolphin, …).

`capabilities/default.json` grants `core:default`, `dialog:default`, `opener:default` to the main
window only.

## Runtime dependencies

Tauri on Linux renders through WebKitGTK, so the machine needs GTK3 + WebKitGTK installed (see the
[README](../README.md#requirements)) — there's no bundled webview like macOS's WKWebView or
Windows' WebView2.

## Window

`tauri.conf.json` intentionally omits `titleBarStyle`/`hiddenTitle` (macOS-only options) and uses
a standard window frame provided by the desktop environment's window manager.

## Data locations

`app.path().app_data_dir()` resolves to `~/.local/share/dotfiledesk/` (respecting `$XDG_DATA_HOME`
if set). `Core::init` creates `dotfiledesk.sqlite` and `repository/` under it on first launch.

## Packaging

`npm run tauri build` produces `.deb`, AppImage, and `.rpm` bundles under
`src-tauri/target/release/bundle/`, per `tauri.conf.json`'s `bundle.targets`, using
`icons/icon.png`.
