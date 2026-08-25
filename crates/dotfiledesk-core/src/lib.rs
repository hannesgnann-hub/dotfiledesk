//! GUI-independent core of DotfileDesk: discovers dotfiles, tracks them in
//! SQLite, snapshots them into an internal git repository, and restores them
//! safely. No module in this crate knows anything about Tauri or React —
//! it can be driven from a CLI, a test, or any GUI shell.

pub mod backup;
pub mod discovery;
pub mod history;
pub mod models;
pub mod security;
pub mod snippets;
pub mod tracking;

use models::{Category, ConfigKind, Configuration, Platform, Sensitivity, Snapshot, Status};
use serde::{Serialize, Serializer};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("invalid commit reference: {0}")]
    InvalidCommit(String),
    #[error("configuration not found: {0}")]
    NotFound(String),
    #[error("private keys cannot be tracked automatically")]
    PrivateKeyBlocked,
    #[error("this file may contain sensitive data and requires confirmation before tracking")]
    ConfirmationRequired,
    #[error("path does not exist: {0}")]
    PathNotFound(String),
    #[error("couldn't apply this suggestion: {0}")]
    SnippetApplyFailed(String),
}

// Tauri commands need command errors to be Serialize so they reach the
// frontend as plain strings instead of panicking the IPC layer.
impl Serialize for CoreError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A discovered-or-tracked configuration together with its live status, as
/// shown on the dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationView {
    pub configuration: Configuration,
    pub status: Status,
}

/// Everything the config detail page needs.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationDetail {
    pub configuration: Configuration,
    pub status: Status,
    pub size_bytes: Option<u64>,
}

/// Aggregate numbers across every non-archived tracked configuration, shown
/// as the stat-card row at the top of the dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummary {
    pub configuration_count: usize,
    /// Files actually present on disk right now, summed across every
    /// tracked configuration (a directory configuration counts every file
    /// inside it, minus the usual ignore patterns).
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub modified_count: usize,
    pub missing_count: usize,
    pub snapshot_count: usize,
}

/// The content of a single file within a tracked configuration, as shown by
/// the integrated editor. `relative_path` is `None` for a [`ConfigKind::File`]
/// configuration and `Some` (relative to the configuration's root) for a
/// [`ConfigKind::Directory`] one.
#[derive(Debug, Clone, Serialize)]
pub struct FileContent {
    pub relative_path: Option<String>,
    pub content: String,
    pub is_binary: bool,
}

/// A catalog entry the user hasn't tracked yet and that doesn't exist on
/// disk, offered on the Add Configuration page as something worth creating.
/// Unlike [`discovery::DiscoveredConfig`] (which only ever reports paths that
/// already exist), a suggestion is "created and tracked" from scratch.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogSuggestion {
    pub definition_id: String,
    pub application: String,
    pub category: Category,
    pub kind: ConfigKind,
    pub path: String,
    pub sensitivity: Sensitivity,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotOutcome {
    pub configuration_id: String,
    pub name: String,
    pub snapshot: Option<Snapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotAllResult {
    pub snapshotted: Vec<SnapshotOutcome>,
    pub unchanged_count: usize,
}

/// The single entry point the Tauri command layer talks to.
pub struct Core {
    store: tracking::Store,
    repo: history::HistoryRepo,
    registry: discovery::Registry,
    snippets: snippets::SnippetCatalog,
    home: PathBuf,
}

impl Core {
    /// `app_data_dir` is a per-user, per-app directory (e.g. from
    /// `~/.local/share/dotfiledesk` or the platform's app-data location).
    pub fn init(app_data_dir: &Path) -> Result<Self, CoreError> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Self::init_with_home(app_data_dir, &home)
    }

    /// Same as [`Core::init`] but with an explicit home directory instead of
    /// the process's real one — used by tests so catalog-suggestion path
    /// expansion never touches the developer machine's actual `$HOME`.
    pub fn init_with_home(app_data_dir: &Path, home: &Path) -> Result<Self, CoreError> {
        std::fs::create_dir_all(app_data_dir)?;
        let store = tracking::Store::open(&app_data_dir.join("dotfiledesk.sqlite"))?;
        let repo = history::HistoryRepo::open_or_init(&app_data_dir.join("repository"))?;
        let registry = discovery::Registry::load_builtin();
        let snippets = snippets::SnippetCatalog::load_builtin();
        Ok(Core { store, repo, registry, snippets, home: home.to_path_buf() })
    }

    /// Expands a leading `~` against this `Core`'s home directory (the real
    /// one in production, an injected one in tests).
    fn expand(&self, raw: &str) -> PathBuf {
        if let Some(rest) = raw.strip_prefix("~/") {
            self.home.join(rest)
        } else if raw == "~" {
            self.home.clone()
        } else {
            PathBuf::from(raw)
        }
    }

    pub fn scan(&self) -> Vec<discovery::DiscoveredConfig> {
        discovery::scan_with_home(&self.registry, &self.home)
    }

    pub fn list_configurations(&self) -> Result<Vec<ConfigurationView>, CoreError> {
        let configs = self.store.list_configurations(false)?;
        self.attach_status(configs)
    }

    /// Configurations the user archived — hidden from the dashboard but not
    /// removed; their history is untouched.
    pub fn list_archived_configurations(&self) -> Result<Vec<ConfigurationView>, CoreError> {
        let configs = self.store.list_archived_configurations()?;
        self.attach_status(configs)
    }

    /// Aggregate stats for the dashboard's overview cards, across every
    /// non-archived tracked configuration.
    pub fn dashboard_summary(&self) -> Result<DashboardSummary, CoreError> {
        let configs = self.store.list_configurations(false)?;
        let mut summary = DashboardSummary {
            configuration_count: configs.len(),
            file_count: 0,
            total_size_bytes: 0,
            modified_count: 0,
            missing_count: 0,
            snapshot_count: 0,
        };
        for configuration in &configs {
            let path = Path::new(&configuration.path);
            summary.file_count += path_file_count(path, configuration.kind);
            summary.total_size_bytes += path_size(path).unwrap_or(0);
            summary.snapshot_count += self.store.list_snapshots(&configuration.id)?.len();

            let latest = self.latest_commit(&configuration.id)?;
            match history::compute_status(&self.repo, configuration, latest.as_deref())? {
                Status::Modified => summary.modified_count += 1,
                Status::Missing => summary.missing_count += 1,
                _ => {}
            }
        }
        Ok(summary)
    }

    fn attach_status(&self, configs: Vec<Configuration>) -> Result<Vec<ConfigurationView>, CoreError> {
        configs
            .into_iter()
            .map(|configuration| {
                let latest = self.latest_commit(&configuration.id)?;
                let status = history::compute_status(&self.repo, &configuration, latest.as_deref())?;
                Ok(ConfigurationView { configuration, status })
            })
            .collect()
    }

    pub fn archive_configuration(&self, id: &str) -> Result<(), CoreError> {
        self.store.set_configuration_archived(id, true)
    }

    pub fn unarchive_configuration(&self, id: &str) -> Result<(), CoreError> {
        self.store.set_configuration_archived(id, false)
    }

    pub fn get_configuration_detail(&self, id: &str) -> Result<Option<ConfigurationDetail>, CoreError> {
        let Some(configuration) = self.store.get_configuration(id)? else {
            return Ok(None);
        };
        let latest = self.latest_commit(id)?;
        let status = history::compute_status(&self.repo, &configuration, latest.as_deref())?;
        let size_bytes = path_size(Path::new(&configuration.path));
        Ok(Some(ConfigurationDetail { configuration, status, size_bytes }))
    }

    /// Tracks a discovered catalog entry. `confirmed` must be `true` for
    /// anything other than [`Sensitivity::Normal`]; private keys are always
    /// refused regardless of confirmation.
    pub fn add_discovered(&self, definition_id: &str, confirmed: bool) -> Result<Configuration, CoreError> {
        let discovered = discovery::scan_with_home(&self.registry, &self.home)
            .into_iter()
            .find(|d| d.definition_id == definition_id)
            .ok_or_else(|| CoreError::NotFound(definition_id.to_string()))?;

        if discovered.is_private_key {
            return Err(CoreError::PrivateKeyBlocked);
        }
        if discovered.sensitivity != Sensitivity::Normal && !confirmed {
            return Err(CoreError::ConfirmationRequired);
        }

        self.store.add_configuration(
            Some(&discovered.definition_id),
            &discovered.application,
            &discovered.path,
            discovered.category,
            discovered.kind,
            discovered.sensitivity,
        )
    }

    /// Tracks a user-supplied file or directory outside the built-in catalog.
    pub fn add_custom(
        &self,
        name: &str,
        path: &str,
        category: Category,
        confirmed: bool,
    ) -> Result<Configuration, CoreError> {
        let expanded = self.expand(path);
        if !expanded.exists() {
            return Err(CoreError::PathNotFound(path.to_string()));
        }
        if security::is_private_key(&expanded) {
            return Err(CoreError::PrivateKeyBlocked);
        }
        let sensitivity = security::classify_path(&expanded);
        if sensitivity != Sensitivity::Normal && !confirmed {
            return Err(CoreError::ConfirmationRequired);
        }
        let kind = if expanded.is_dir() { ConfigKind::Directory } else { ConfigKind::File };

        self.store.add_configuration(
            None,
            name,
            &expanded.to_string_lossy(),
            category,
            kind,
            sensitivity,
        )
    }

    pub fn remove_configuration(&self, id: &str) -> Result<(), CoreError> {
        self.store.remove_configuration(id)
    }

    pub fn snapshot(&self, id: &str, reason: &str) -> Result<Option<Snapshot>, CoreError> {
        let configuration = self.require_configuration(id)?;
        match self.repo.snapshot(&configuration, reason)? {
            Some(commit) => Ok(Some(self.store.record_snapshot(id, &commit, reason)?)),
            None => Ok(None),
        }
    }

    pub fn snapshot_all(&self) -> Result<SnapshotAllResult, CoreError> {
        let configs = self.store.list_configurations(false)?;
        let mut snapshotted = Vec::new();
        let mut unchanged_count = 0;
        for configuration in configs {
            let snapshot = self.snapshot(&configuration.id, "Manual snapshot")?;
            if snapshot.is_some() {
                snapshotted.push(SnapshotOutcome {
                    configuration_id: configuration.id,
                    name: configuration.name,
                    snapshot,
                });
            } else {
                unchanged_count += 1;
            }
        }
        Ok(SnapshotAllResult { snapshotted, unchanged_count })
    }

    pub fn list_history(&self, id: &str) -> Result<Vec<Snapshot>, CoreError> {
        self.store.list_snapshots(id)
    }

    /// Diffs a specific snapshot against the one immediately before it.
    pub fn diff_snapshot(&self, id: &str, commit: &str) -> Result<history::DiffResult, CoreError> {
        let configuration = self.require_configuration(id)?;
        let history = self.store.list_snapshots(id)?;
        let position = history
            .iter()
            .position(|s| s.git_commit == commit)
            .ok_or_else(|| CoreError::InvalidCommit(commit.to_string()))?;
        let previous = history.get(position + 1).map(|s| s.git_commit.as_str());
        self.repo.diff_commits(&configuration, previous, commit)
    }

    /// Diffs the latest snapshot against what's currently on disk.
    pub fn diff_working(&self, id: &str) -> Result<history::DiffResult, CoreError> {
        let configuration = self.require_configuration(id)?;
        let latest = self
            .latest_commit(id)?
            .ok_or_else(|| CoreError::NotFound(format!("no snapshot for {id}")))?;
        self.repo.diff_against_working(&configuration, &latest)
    }

    pub fn restore(&self, id: &str, commit: &str) -> Result<backup::RestoreResult, CoreError> {
        let configuration = self.require_configuration(id)?;
        backup::restore(&self.repo, &self.store, &configuration, commit)
    }

    pub fn favorite_snapshot(&self, snapshot_id: &str, favorite: bool) -> Result<(), CoreError> {
        self.store.set_snapshot_favorite(snapshot_id, favorite)
    }

    pub fn archive_snapshot(&self, snapshot_id: &str, archived: bool) -> Result<(), CoreError> {
        self.store.set_snapshot_archived(snapshot_id, archived)
    }

    /// Permanently removes a snapshot's metadata. The underlying git commit
    /// stays in the internal repository (an inert, invisible leftover) — only
    /// the history entry disappears.
    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), CoreError> {
        self.store.delete_snapshot(snapshot_id)
    }

    /// Catalog definitions for the current platform that neither exist on
    /// disk nor are tracked yet — things like Docker or a global `.gitignore`
    /// that make sense to start from scratch. Anything that already exists
    /// surfaces through [`Core::scan`] instead, so the two lists never
    /// overlap. Private-key definitions are excluded outright — creating an
    /// empty file at a private key path would be actively misleading.
    pub fn list_catalog_suggestions(&self) -> Result<Vec<CatalogSuggestion>, CoreError> {
        let tracked: std::collections::HashSet<String> = self
            .store
            .list_configurations(true)?
            .into_iter()
            .filter_map(|c| c.definition_id)
            .collect();
        let current = Platform::current();

        let suggestions = self
            .registry
            .definitions()
            .iter()
            .filter(|def| def.platforms.contains(&current))
            .filter(|def| !tracked.contains(&def.id))
            .filter(|def| !security::is_private_key(&self.expand(def.paths.first().map(String::as_str).unwrap_or(""))))
            .filter_map(|def| {
                let first_path = def.paths.first()?;
                let candidate = def
                    .paths
                    .iter()
                    .map(|p| self.expand(p))
                    .find(|p| match def.kind {
                        ConfigKind::File => p.is_file(),
                        ConfigKind::Directory => p.is_dir(),
                    })
                    .unwrap_or_else(|| self.expand(first_path));
                if candidate.exists() {
                    return None; // already covered by discovery
                }
                Some(CatalogSuggestion {
                    definition_id: def.id.clone(),
                    application: def.application.clone(),
                    category: def.category,
                    kind: def.kind,
                    path: candidate.to_string_lossy().to_string(),
                    sensitivity: def.sensitivity,
                })
            })
            .collect();
        Ok(suggestions)
    }

    /// Tracks a suggestion. If nothing exists at its path yet, an empty
    /// file/directory is created first so the user can fill it in with the
    /// integrated editor right away.
    pub fn add_suggestion(&self, definition_id: &str, confirmed: bool) -> Result<Configuration, CoreError> {
        let def = self
            .registry
            .definitions()
            .iter()
            .find(|d| d.id == definition_id)
            .ok_or_else(|| CoreError::NotFound(definition_id.to_string()))?;

        let first_path = def
            .paths
            .first()
            .ok_or_else(|| CoreError::InvalidPath(definition_id.to_string()))?;
        let path = def
            .paths
            .iter()
            .map(|p| self.expand(p))
            .find(|p| match def.kind {
                ConfigKind::File => p.is_file(),
                ConfigKind::Directory => p.is_dir(),
            })
            .unwrap_or_else(|| self.expand(first_path));

        if security::is_private_key(&path) {
            return Err(CoreError::PrivateKeyBlocked);
        }
        if def.sensitivity != Sensitivity::Normal && !confirmed {
            return Err(CoreError::ConfirmationRequired);
        }

        if !path.exists() {
            match def.kind {
                ConfigKind::File => {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, "")?;
                }
                ConfigKind::Directory => std::fs::create_dir_all(&path)?,
            }
        }

        self.store.add_configuration(
            Some(&def.id),
            &def.application,
            &path.to_string_lossy(),
            def.category,
            def.kind,
            def.sensitivity,
        )
    }

    /// Lists the files inside a directory configuration, relative to its
    /// root, for the integrated editor's file browser. Empty for a
    /// file-kind configuration (there's nothing to browse).
    pub fn list_configuration_files(&self, id: &str) -> Result<Vec<String>, CoreError> {
        let configuration = self.require_configuration(id)?;
        if configuration.kind != ConfigKind::Directory {
            return Ok(Vec::new());
        }
        let root = Path::new(&configuration.path);
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(root).into_iter().filter_entry(|e| {
            e.file_name()
                .to_str()
                .map(|n| !security::is_ignored_entry(n))
                .unwrap_or(true)
        }) {
            let entry = entry.map_err(|e| CoreError::Io(e.into()))?;
            if entry.file_type().is_file() {
                let relative = entry.path().strip_prefix(root).expect("walked under root");
                files.push(relative.to_string_lossy().to_string());
            }
        }
        files.sort();
        Ok(files)
    }

    /// Reads a file for the integrated editor. `relative_path` is required
    /// for directory configurations and ignored for file configurations.
    pub fn read_configuration_file(
        &self,
        id: &str,
        relative_path: Option<&str>,
    ) -> Result<FileContent, CoreError> {
        let configuration = self.require_configuration(id)?;
        let target = self.resolve_editor_path(&configuration, relative_path)?;
        if !target.exists() {
            return Err(CoreError::PathNotFound(target.to_string_lossy().to_string()));
        }
        let bytes = std::fs::read(&target)?;
        match String::from_utf8(bytes) {
            Ok(content) => Ok(FileContent {
                relative_path: relative_path.map(|s| s.to_string()),
                content,
                is_binary: false,
            }),
            Err(_) => Ok(FileContent {
                relative_path: relative_path.map(|s| s.to_string()),
                content: String::new(),
                is_binary: true,
            }),
        }
    }

    /// Writes a file from the integrated editor straight to its real path.
    /// This does not create a snapshot — the file's status simply becomes
    /// `Modified` until the user snapshots it, consistent with the rest of
    /// DotfileDesk never snapshotting automatically.
    pub fn write_configuration_file(
        &self,
        id: &str,
        relative_path: Option<&str>,
        content: &str,
    ) -> Result<(), CoreError> {
        let configuration = self.require_configuration(id)?;
        let target = self.resolve_editor_path(&configuration, relative_path)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, content)?;
        Ok(())
    }

    /// Suggested content to append to a tracked file, based on which catalog
    /// entry it came from. Empty for directory configurations, custom
    /// (non-catalog) configurations, and anything whose file doesn't exist —
    /// there's either no sensible "which file" to append to, or no known
    /// application to suggest content for.
    pub fn list_snippet_suggestions(&self, id: &str) -> Result<Vec<snippets::SnippetSuggestion>, CoreError> {
        let configuration = self.require_configuration(id)?;
        let Some(definition_id) = &configuration.definition_id else {
            return Ok(Vec::new());
        };
        if configuration.kind != ConfigKind::File {
            return Ok(Vec::new());
        }
        let path = Path::new(&configuration.path);
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(path).unwrap_or_default();
        Ok(self.snippets.suggestions_for(definition_id, &content))
    }

    /// Computes the new content after applying a suggestion (found by its
    /// label) from [`Core::list_snippet_suggestions`] on top of
    /// `current_content`. Takes the caller's in-memory buffer rather than
    /// re-reading the file itself, so unsaved edits already in the editor
    /// aren't clobbered by a stale on-disk version. Doesn't write anything —
    /// the editor puts the result straight into its buffer, so the user
    /// still reviews and saves it like any other edit.
    pub fn preview_snippet_insertion(
        &self,
        id: &str,
        label: &str,
        current_content: &str,
    ) -> Result<String, CoreError> {
        let configuration = self.require_configuration(id)?;
        if configuration.kind != ConfigKind::File {
            return Err(CoreError::InvalidPath("suggestions only apply to single files".into()));
        }
        let definition_id = configuration
            .definition_id
            .as_deref()
            .ok_or_else(|| CoreError::NotFound("this configuration has no known suggestions".into()))?;
        let suggestion = self
            .snippets
            .find(definition_id, label)
            .ok_or_else(|| CoreError::NotFound(label.to_string()))?;

        snippets::apply(suggestion, current_content).map_err(CoreError::SnippetApplyFailed)
    }

    fn resolve_editor_path(
        &self,
        configuration: &Configuration,
        relative_path: Option<&str>,
    ) -> Result<PathBuf, CoreError> {
        match configuration.kind {
            ConfigKind::File => Ok(PathBuf::from(&configuration.path)),
            ConfigKind::Directory => {
                let relative = relative_path
                    .ok_or_else(|| CoreError::InvalidPath("a file within the directory must be selected".into()))?;
                Ok(Path::new(&configuration.path).join(relative))
            }
        }
    }

    fn require_configuration(&self, id: &str) -> Result<Configuration, CoreError> {
        self.store
            .get_configuration(id)?
            .ok_or_else(|| CoreError::NotFound(id.to_string()))
    }

    fn latest_commit(&self, id: &str) -> Result<Option<String>, CoreError> {
        Ok(self.store.list_snapshots(id)?.into_iter().next().map(|s| s.git_commit))
    }
}

/// Expands a leading `~` in a user-supplied path to the current home
/// directory. Exposed for the GUI layer to preview custom paths before adding
/// them (see `preview_custom_path`).
pub fn expand_home_path(raw: &str) -> PathBuf {
    expand_home(raw)
}

fn expand_home(raw: &str) -> PathBuf {
    let Some(home) = dirs::home_dir() else {
        return PathBuf::from(raw);
    };
    if let Some(rest) = raw.strip_prefix("~/") {
        home.join(rest)
    } else if raw == "~" {
        home
    } else {
        PathBuf::from(raw)
    }
}

fn path_size(path: &Path) -> Option<u64> {
    if path.is_file() {
        return std::fs::metadata(path).ok().map(|m| m.len());
    }
    if path.is_dir() {
        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(path).into_iter().flatten() {
            if entry.file_type().is_file() {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }
        return Some(total);
    }
    None
}

/// How many files a configuration actually contributes right now — 1 for an
/// existing plain file, 0 if it's missing, or a directory's file count minus
/// the usual ignore patterns (`.git`, `node_modules`, …).
fn path_file_count(path: &Path, kind: ConfigKind) -> usize {
    match kind {
        ConfigKind::File => usize::from(path.is_file()),
        ConfigKind::Directory => {
            if !path.is_dir() {
                return 0;
            }
            walkdir::WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| !security::is_ignored_entry(n))
                        .unwrap_or(true)
                })
                .flatten()
                .filter(|e| e.file_type().is_file())
                .count()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// The returned `TempDir` doubles as both the app-data dir and the "home"
    /// directory Core resolves `~` against, so tests that exercise catalog
    /// suggestions never touch the real developer machine's `$HOME`.
    fn make_core() -> (Core, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let core = Core::init_with_home(dir.path(), dir.path()).unwrap();
        (core, dir)
    }

    #[test]
    fn full_lifecycle_add_snapshot_modify_diff_restore() {
        let (core, data_dir) = make_core();
        let home = tempdir().unwrap();
        let file = home.path().join("myrc");
        std::fs::write(&file, "line one\n").unwrap();

        let config = core
            .add_custom("My Tool", &file.to_string_lossy(), Category::Other, false)
            .unwrap();
        assert_eq!(config.sensitivity, Sensitivity::Normal);

        let snap = core.snapshot(&config.id, "Initial snapshot").unwrap();
        assert!(snap.is_some());

        let detail = core.get_configuration_detail(&config.id).unwrap().unwrap();
        assert_eq!(detail.status, Status::Synced);

        std::fs::write(&file, "line one\nline two\n").unwrap();
        let detail = core.get_configuration_detail(&config.id).unwrap().unwrap();
        assert_eq!(detail.status, Status::Modified);

        let working_diff = core.diff_working(&config.id).unwrap();
        assert_eq!(working_diff.files.len(), 1);

        let second = core.snapshot(&config.id, "Added line two").unwrap().unwrap();
        let snap_diff = core.diff_snapshot(&config.id, &second.git_commit).unwrap();
        assert_eq!(snap_diff.files.len(), 1);

        let history = core.list_history(&config.id).unwrap();
        assert_eq!(history.len(), 2);
        let first_commit = history.last().unwrap().git_commit.clone();

        let restore_result = core.restore(&config.id, &first_commit).unwrap();
        assert!(restore_result.verified);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "line one\n");

        let history_after = core.list_history(&config.id).unwrap();
        assert!(history_after.len() >= 3); // + pre-restore backup + restore event

        drop(core);
        drop(data_dir);
    }

    #[test]
    fn private_keys_are_never_tracked() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".ssh")).unwrap();
        let key = home.path().join(".ssh/id_rsa");
        std::fs::write(&key, "-----BEGIN KEY-----").unwrap();

        let result = core.add_custom("SSH Key", &key.to_string_lossy(), Category::Ssh, true);
        assert!(matches!(result, Err(CoreError::PrivateKeyBlocked)));
    }

    #[test]
    fn sensitive_paths_require_confirmation() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();
        let npmrc = home.path().join(".npmrc");
        std::fs::write(&npmrc, "//registry.npmjs.org/:_authToken=abc123\n").unwrap();

        let unconfirmed = core.add_custom("npm", &npmrc.to_string_lossy(), Category::PackageManagers, false);
        assert!(matches!(unconfirmed, Err(CoreError::ConfirmationRequired)));

        let confirmed = core
            .add_custom("npm", &npmrc.to_string_lossy(), Category::PackageManagers, true)
            .unwrap();
        assert_eq!(confirmed.sensitivity, Sensitivity::PotentialSecret);
    }

    #[test]
    fn snapshot_all_only_records_changed_configs() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();
        let a = home.path().join("a.conf");
        let b = home.path().join("b.conf");
        std::fs::write(&a, "a\n").unwrap();
        std::fs::write(&b, "b\n").unwrap();
        let ca = core.add_custom("A", &a.to_string_lossy(), Category::Other, false).unwrap();
        let cb = core.add_custom("B", &b.to_string_lossy(), Category::Other, false).unwrap();

        let first = core.snapshot_all().unwrap();
        assert_eq!(first.snapshotted.len(), 2);
        assert_eq!(first.unchanged_count, 0);

        std::fs::write(&a, "a changed\n").unwrap();
        let second = core.snapshot_all().unwrap();
        assert_eq!(second.snapshotted.len(), 1);
        assert_eq!(second.snapshotted[0].configuration_id, ca.id);
        assert_eq!(second.unchanged_count, 1);
        let _ = cb;
    }

    #[test]
    fn dashboard_summary_aggregates_across_configurations() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();

        let file = home.path().join("solo.conf");
        std::fs::write(&file, "hello\n").unwrap();
        let file_config = core.add_custom("Solo", &file.to_string_lossy(), Category::Other, false).unwrap();

        let dir = home.path().join("nvim");
        std::fs::create_dir_all(dir.join("lua")).unwrap();
        std::fs::write(dir.join("init.lua"), "-- init\n").unwrap();
        std::fs::write(dir.join("lua/plugins.lua"), "-- plugins\n").unwrap();
        let dir_config = core.add_custom("Neovim", &dir.to_string_lossy(), Category::Editor, false).unwrap();

        let empty = core.dashboard_summary().unwrap();
        assert_eq!(empty.configuration_count, 2);
        assert_eq!(empty.file_count, 3); // solo.conf + init.lua + lua/plugins.lua
        assert_eq!(empty.snapshot_count, 0);
        assert_eq!(empty.modified_count, 0);
        assert_eq!(empty.missing_count, 0);
        assert!(empty.total_size_bytes > 0);

        core.snapshot(&file_config.id, "Initial snapshot").unwrap();
        core.snapshot(&dir_config.id, "Initial snapshot").unwrap();
        std::fs::write(&file, "hello again\n").unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        let after = core.dashboard_summary().unwrap();
        assert_eq!(after.file_count, 1); // the directory config is now gone
        assert_eq!(after.snapshot_count, 2);
        assert_eq!(after.modified_count, 1); // solo.conf changed since its snapshot
        assert_eq!(after.missing_count, 1); // nvim directory no longer exists
    }

    #[test]
    fn edits_file_configuration_in_place() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();
        let file = home.path().join("editable.conf");
        std::fs::write(&file, "original\n").unwrap();
        let config = core.add_custom("Editable", &file.to_string_lossy(), Category::Other, false).unwrap();

        let content = core.read_configuration_file(&config.id, None).unwrap();
        assert!(!content.is_binary);
        assert_eq!(content.content, "original\n");

        core.write_configuration_file(&config.id, None, "edited\n").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "edited\n");

        let detail = core.get_configuration_detail(&config.id).unwrap().unwrap();
        assert_eq!(detail.status, Status::NotTracked); // no snapshot taken yet
    }

    #[test]
    fn edits_file_within_directory_configuration() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();
        let dir = home.path().join("nvim");
        std::fs::create_dir_all(dir.join("lua")).unwrap();
        std::fs::write(dir.join("init.lua"), "-- init\n").unwrap();
        std::fs::write(dir.join("lua/plugins.lua"), "-- plugins\n").unwrap();

        let config = core.add_custom("Neovim", &dir.to_string_lossy(), Category::Editor, false).unwrap();

        let mut files = core.list_configuration_files(&config.id).unwrap();
        files.sort();
        assert_eq!(files, vec!["init.lua".to_string(), "lua/plugins.lua".to_string()]);

        let content = core.read_configuration_file(&config.id, Some("lua/plugins.lua")).unwrap();
        assert_eq!(content.content, "-- plugins\n");

        core
            .write_configuration_file(&config.id, Some("lua/plugins.lua"), "-- plugins updated\n")
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("lua/plugins.lua")).unwrap(),
            "-- plugins updated\n"
        );

        let missing = core.read_configuration_file(&config.id, None);
        assert!(matches!(missing, Err(CoreError::InvalidPath(_))));
    }

    #[test]
    fn archiving_configuration_hides_it_from_dashboard_but_keeps_history() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();
        let file = home.path().join("archived.conf");
        std::fs::write(&file, "content\n").unwrap();
        let config = core.add_custom("Archived", &file.to_string_lossy(), Category::Other, false).unwrap();
        core.snapshot(&config.id, "Initial snapshot").unwrap();

        core.archive_configuration(&config.id).unwrap();
        assert!(core.list_configurations().unwrap().is_empty());
        assert_eq!(core.list_archived_configurations().unwrap().len(), 1);
        // History and detail still work while archived.
        assert_eq!(core.list_history(&config.id).unwrap().len(), 1);
        assert!(core.get_configuration_detail(&config.id).unwrap().is_some());

        core.unarchive_configuration(&config.id).unwrap();
        assert_eq!(core.list_configurations().unwrap().len(), 1);
    }

    #[test]
    fn snapshot_favorite_archive_and_delete_via_core() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();
        let file = home.path().join("starred.conf");
        std::fs::write(&file, "content\n").unwrap();
        let config = core.add_custom("Starred", &file.to_string_lossy(), Category::Other, false).unwrap();
        let snap = core.snapshot(&config.id, "Initial snapshot").unwrap().unwrap();

        core.favorite_snapshot(&snap.id, true).unwrap();
        let history = core.list_history(&config.id).unwrap();
        assert!(history[0].favorite);

        core.archive_snapshot(&snap.id, true).unwrap();
        let history = core.list_history(&config.id).unwrap();
        assert!(history[0].archived);

        core.delete_snapshot(&snap.id).unwrap();
        assert!(core.list_history(&config.id).unwrap().is_empty());
    }

    #[test]
    fn suggestions_exclude_tracked_and_private_keys_and_can_create_missing_files() {
        let (core, _data) = make_core();

        let suggestions = core.list_catalog_suggestions().unwrap();
        assert!(!suggestions.is_empty());
        assert!(
            suggestions.iter().all(|s| !s.definition_id.starts_with("ssh_key_")),
            "private key definitions must never be suggested"
        );

        let gitconfig_suggestion = suggestions.iter().find(|s| s.definition_id == "gitconfig").unwrap();
        assert_eq!(gitconfig_suggestion.category, Category::Git);

        // Suggested definitions get created on disk when tracked.
        assert!(suggestions.iter().any(|s| s.definition_id == "ripgrep"));
        let tracked = core.add_suggestion("ripgrep", false).unwrap();
        assert!(std::path::Path::new(&tracked.path).exists());

        let still_suggested = core
            .list_catalog_suggestions()
            .unwrap()
            .iter()
            .any(|s| s.definition_id == "ripgrep");
        assert!(!still_suggested, "tracked suggestions must disappear from the list");
    }

    #[test]
    fn suggestions_never_include_paths_that_already_exist() {
        let dir = tempdir().unwrap();
        let core = Core::init_with_home(dir.path(), dir.path()).unwrap();
        // ~/.gitconfig now exists in the sandboxed home used by this Core.
        std::fs::write(dir.path().join(".gitconfig"), "[user]\n").unwrap();

        let suggestions = core.list_catalog_suggestions().unwrap();
        assert!(
            !suggestions.iter().any(|s| s.definition_id == "gitconfig"),
            "an existing file should surface via discovery, not suggestions"
        );

        let discovered = core.scan();
        assert!(discovered.iter().any(|d| d.definition_id == "gitconfig"));
    }

    #[test]
    fn snippet_suggestions_are_scoped_to_catalog_files_and_disappear_once_inserted() {
        // Core's home and app-data dir are the same sandboxed tempdir here,
        // so writing `.zshrc` into it is what `add_discovered`'s internal
        // scan will find (mirrors `suggestions_never_include_paths_...`).
        let (core, data_dir) = make_core();

        // Custom (non-catalog) configuration: no definition_id, so no snippets.
        let custom_file = data_dir.path().join("custom.conf");
        std::fs::write(&custom_file, "content\n").unwrap();
        let custom = core
            .add_custom("Custom", &custom_file.to_string_lossy(), Category::Other, false)
            .unwrap();
        assert!(core.list_snippet_suggestions(&custom.id).unwrap().is_empty());

        // A catalog-linked file (zsh) gets real suggestions.
        let zshrc = data_dir.path().join(".zshrc");
        std::fs::write(&zshrc, "export FOO=bar\n").unwrap();
        let zsh = core.add_discovered("zsh", false).unwrap();

        let suggestions = core.list_snippet_suggestions(&zsh.id).unwrap();
        assert!(!suggestions.is_empty());

        // Inserting one (simulating the editor's "Insert" button) removes it
        // from the next call's suggestions.
        let first = suggestions[0].clone();
        let appended = format!("export FOO=bar\n{}", first.snippet);
        std::fs::write(&zshrc, &appended).unwrap();
        let after = core.list_snippet_suggestions(&zsh.id).unwrap();
        assert!(after.iter().all(|s| s.label != first.label));
    }

    #[test]
    fn previews_a_json_snippet_insertion_without_writing_to_disk() {
        let (core, data_dir) = make_core();
        // Matches the macOS candidate path for the "vscode_settings"
        // definition, so `add_discovered` (which does a real filesystem
        // scan against Core's sandboxed home) picks it up.
        let settings = data_dir.path().join("Library/Application Support/Code/User/settings.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        let original = "{\n  \"editor.tabSize\": 4\n}\n";
        std::fs::write(&settings, original).unwrap();

        let config = core.add_discovered("vscode_settings", false).unwrap();

        let preview = core
            .preview_snippet_insertion(&config.id, "Format on save", original)
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&preview).unwrap();
        assert_eq!(parsed["editor.tabSize"], 4);
        assert_eq!(parsed["editor.formatOnSave"], true);

        // Preview never writes — the file on disk is exactly as before.
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), original);

        // Unsaved edits already in the buffer are preserved, not clobbered
        // by re-reading the (stale) version still on disk.
        let unsaved = "{\n  \"editor.tabSize\": 4,\n  \"editor.wordWrap\": \"on\"\n}";
        let preview_over_unsaved = core
            .preview_snippet_insertion(&config.id, "Format on save", unsaved)
            .unwrap();
        let parsed_unsaved: serde_json::Value = serde_json::from_str(&preview_over_unsaved).unwrap();
        assert_eq!(parsed_unsaved["editor.wordWrap"], "on");
        assert_eq!(parsed_unsaved["editor.formatOnSave"], true);

        // A custom (non-catalog) configuration has no definition_id to look
        // suggestions up by, so it's correctly refused instead of guessing.
        let custom_file = data_dir.path().join("custom.conf");
        std::fs::write(&custom_file, "content\n").unwrap();
        let custom = core
            .add_custom("Custom", &custom_file.to_string_lossy(), Category::Other, false)
            .unwrap();
        assert!(core.preview_snippet_insertion(&custom.id, "Format on save", "").is_err());
    }

    #[test]
    fn missing_file_is_reported_and_restorable() {
        let (core, _data) = make_core();
        let home = tempdir().unwrap();
        let file = home.path().join("gone.conf");
        std::fs::write(&file, "content\n").unwrap();
        let config = core.add_custom("Gone", &file.to_string_lossy(), Category::Other, false).unwrap();
        core.snapshot(&config.id, "Initial snapshot").unwrap();

        std::fs::remove_file(&file).unwrap();
        let detail = core.get_configuration_detail(&config.id).unwrap().unwrap();
        assert_eq!(detail.status, Status::Missing);

        let history = core.list_history(&config.id).unwrap();
        let latest = history.first().unwrap().git_commit.clone();
        let result = core.restore(&config.id, &latest).unwrap();
        assert!(result.verified);
        assert!(file.exists());
    }
}
