// Prevents an extra console window from appearing on Windows debug builds of
// this same command layer when it's reused there; harmless on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dotfiledesk_core::backup::RestoreResult;
use dotfiledesk_core::discovery::DiscoveredConfig;
use dotfiledesk_core::history::DiffResult;
use dotfiledesk_core::models::{Category, Configuration, Sensitivity, Snapshot};
use dotfiledesk_core::{
    ConfigurationDetail, ConfigurationView, Core, CoreError, FileContent, SnapshotAllResult,
};
use serde::Serialize;
use std::sync::Mutex;
use tauri::Manager;

struct AppState(Mutex<Core>);

fn with_core<T>(state: tauri::State<AppState>, f: impl FnOnce(&Core) -> Result<T, CoreError>) -> Result<T, CoreError> {
    let core = state.0.lock().expect("core mutex poisoned");
    f(&core)
}

#[tauri::command]
fn scan_configurations(state: tauri::State<AppState>) -> Result<Vec<DiscoveredConfig>, CoreError> {
    with_core(state, |core| Ok(core.scan()))
}

#[tauri::command]
fn list_configurations(state: tauri::State<AppState>) -> Result<Vec<ConfigurationView>, CoreError> {
    with_core(state, |core| core.list_configurations())
}

#[tauri::command]
fn get_configuration_detail(
    state: tauri::State<AppState>,
    id: String,
) -> Result<Option<ConfigurationDetail>, CoreError> {
    with_core(state, |core| core.get_configuration_detail(&id))
}

#[tauri::command]
fn add_discovered(
    state: tauri::State<AppState>,
    definition_id: String,
    confirmed: bool,
) -> Result<Configuration, CoreError> {
    with_core(state, |core| core.add_discovered(&definition_id, confirmed))
}

#[derive(Serialize)]
struct PathPreview {
    exists: bool,
    is_directory: bool,
    is_private_key: bool,
    sensitivity: Sensitivity,
}

#[tauri::command]
fn preview_custom_path(path: String) -> PathPreview {
    let expanded = dotfiledesk_core::expand_home_path(&path);
    let exists = expanded.exists();
    PathPreview {
        exists,
        is_directory: expanded.is_dir(),
        is_private_key: dotfiledesk_core::security::is_private_key(&expanded),
        sensitivity: dotfiledesk_core::security::classify_path(&expanded),
    }
}

#[tauri::command]
fn add_custom(
    state: tauri::State<AppState>,
    name: String,
    path: String,
    category: Category,
    confirmed: bool,
) -> Result<Configuration, CoreError> {
    with_core(state, |core| core.add_custom(&name, &path, category, confirmed))
}

#[tauri::command]
fn remove_configuration(state: tauri::State<AppState>, id: String) -> Result<(), CoreError> {
    with_core(state, |core| core.remove_configuration(&id))
}

#[tauri::command]
fn snapshot_configuration(
    state: tauri::State<AppState>,
    id: String,
    reason: Option<String>,
) -> Result<Option<Snapshot>, CoreError> {
    with_core(state, |core| {
        core.snapshot(&id, reason.as_deref().unwrap_or("Manual snapshot"))
    })
}

#[tauri::command]
fn snapshot_all(state: tauri::State<AppState>) -> Result<SnapshotAllResult, CoreError> {
    with_core(state, |core| core.snapshot_all())
}

#[tauri::command]
fn list_history(state: tauri::State<AppState>, id: String) -> Result<Vec<Snapshot>, CoreError> {
    with_core(state, |core| core.list_history(&id))
}

#[tauri::command]
fn diff_snapshot(state: tauri::State<AppState>, id: String, commit: String) -> Result<DiffResult, CoreError> {
    with_core(state, |core| core.diff_snapshot(&id, &commit))
}

#[tauri::command]
fn diff_working(state: tauri::State<AppState>, id: String) -> Result<DiffResult, CoreError> {
    with_core(state, |core| core.diff_working(&id))
}

#[tauri::command]
fn restore_snapshot(
    state: tauri::State<AppState>,
    id: String,
    commit: String,
) -> Result<RestoreResult, CoreError> {
    with_core(state, |core| core.restore(&id, &commit))
}

#[tauri::command]
fn list_configuration_files(state: tauri::State<AppState>, id: String) -> Result<Vec<String>, CoreError> {
    with_core(state, |core| core.list_configuration_files(&id))
}

#[tauri::command]
fn read_configuration_file(
    state: tauri::State<AppState>,
    id: String,
    relative_path: Option<String>,
) -> Result<FileContent, CoreError> {
    with_core(state, |core| core.read_configuration_file(&id, relative_path.as_deref()))
}

#[tauri::command]
fn write_configuration_file(
    state: tauri::State<AppState>,
    id: String,
    relative_path: Option<String>,
    content: String,
) -> Result<(), CoreError> {
    with_core(state, |core| {
        core.write_configuration_file(&id, relative_path.as_deref(), &content)
    })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("app data dir must be resolvable");
            let core = Core::init(&data_dir).expect("failed to initialize DotfileDesk core");
            app.manage(AppState(Mutex::new(core)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_configurations,
            list_configurations,
            get_configuration_detail,
            add_discovered,
            preview_custom_path,
            add_custom,
            remove_configuration,
            snapshot_configuration,
            snapshot_all,
            list_history,
            diff_snapshot,
            diff_working,
            restore_snapshot,
            list_configuration_files,
            read_configuration_file,
            write_configuration_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DotfileDesk");
}
