use serde::{Deserialize, Serialize};

/// A grouping used to organize configurations in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Shell,
    Git,
    Terminal,
    Editor,
    Ssh,
    PackageManagers,
    DeveloperTools,
    Other,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Shell => "Shell",
            Category::Git => "Git",
            Category::Terminal => "Terminal",
            Category::Editor => "Editor",
            Category::Ssh => "SSH",
            Category::PackageManagers => "Package Managers",
            Category::DeveloperTools => "Developer Tools",
            Category::Other => "Other",
        }
    }
}

/// How sensitive the contents of a configuration are. Drives whether DotfileDesk
/// will track a file automatically, warn before tracking, or refuse entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Normal,
    PotentialSecret,
    HighlySensitive,
}

/// Whether a configuration is a single file or an entire directory tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigKind {
    File,
    Directory,
}

/// The platform a [`ConfigDefinition`] applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Macos,
    Linux,
    Windows,
}

impl Platform {
    pub fn current() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::Macos
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else {
            Platform::Linux
        }
    }
}

/// A known configuration location shipped with DotfileDesk (see `/definitions`).
/// This is the read-only catalog entry; a user opts a definition into tracking,
/// which creates a [`Configuration`] row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDefinition {
    pub id: String,
    pub application: String,
    pub category: Category,
    pub kind: ConfigKind,
    /// Candidate paths (may use `~`); the first one that exists on disk is used.
    pub paths: Vec<String>,
    pub platforms: Vec<Platform>,
    pub sensitivity: Sensitivity,
}

/// Live status of a tracked configuration, computed by diffing the file on disk
/// against the most recent snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Matches the latest snapshot exactly.
    Synced,
    /// Differs from the latest snapshot.
    Modified,
    /// No snapshot has been taken yet.
    NotTracked,
    /// The file/directory no longer exists on disk.
    Missing,
    /// Tracked but flagged for user attention (e.g. sensitivity warning pending).
    Warning,
}

/// A configuration DotfileDesk is (or could be) managing. Persisted in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub id: String,
    /// Links back to the catalog entry this came from, if any (`None` for custom entries).
    pub definition_id: Option<String>,
    pub name: String,
    pub path: String,
    pub category: Category,
    pub kind: ConfigKind,
    pub sensitivity: Sensitivity,
    pub added_at: String,
    pub last_snapshot_at: Option<String>,
}

/// One point-in-time snapshot of a [`Configuration`], backed by a git commit in
/// the internal history repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub configuration_id: String,
    pub created_at: String,
    pub git_commit: String,
    pub reason: String,
}
