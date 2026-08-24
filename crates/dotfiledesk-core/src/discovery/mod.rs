use crate::models::{Category, ConfigDefinition, ConfigKind, Platform, Sensitivity};
use crate::security;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

macro_rules! definition_json {
    ($file:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../definitions/", $file))
    };
}

const DEFINITION_SOURCES: &[&str] = &[
    definition_json!("shell.json"),
    definition_json!("git.json"),
    definition_json!("terminals.json"),
    definition_json!("editors.json"),
    definition_json!("tools.json"),
];

/// The catalog of known configuration locations, loaded from `/definitions/*.json`.
pub struct Registry {
    definitions: Vec<ConfigDefinition>,
}

impl Registry {
    /// Loads the catalog embedded in the binary at compile time.
    pub fn load_builtin() -> Self {
        let mut definitions = Vec::new();
        for source in DEFINITION_SOURCES {
            let parsed: Vec<ConfigDefinition> =
                serde_json::from_str(source).expect("built-in definitions must be valid JSON");
            definitions.extend(parsed);
        }
        Registry { definitions }
    }

    pub fn definitions(&self) -> &[ConfigDefinition] {
        &self.definitions
    }
}

/// A configuration found on disk during a scan, not yet tracked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredConfig {
    pub definition_id: String,
    pub application: String,
    pub category: Category,
    pub kind: ConfigKind,
    pub path: String,
    pub sensitivity: Sensitivity,
    /// True when the file/directory name matches a private-key pattern; the
    /// caller must refuse to track these automatically.
    pub is_private_key: bool,
}

fn expand_home(raw: &str, home: &Path) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        home.join(rest)
    } else if raw == "~" {
        home.to_path_buf()
    } else {
        PathBuf::from(raw)
    }
}

/// Scans the filesystem for every definition in `registry` that applies to the
/// current platform and exists on disk. Nothing on disk is modified.
pub fn scan(registry: &Registry) -> Vec<DiscoveredConfig> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    scan_with_home(registry, &home)
}

/// Same as [`scan`] but against an explicit home directory — used by [`crate::Core`]
/// so every path-resolving operation shares one home (the real one in
/// production, an injected one in tests).
pub fn scan_with_home(registry: &Registry, home: &Path) -> Vec<DiscoveredConfig> {
    let current = Platform::current();
    let mut found = Vec::new();

    for def in registry.definitions() {
        if !def.platforms.iter().any(|p| platform_eq(*p, current)) {
            continue;
        }
        for raw_path in &def.paths {
            let expanded = expand_home(raw_path, home);
            let exists = match def.kind {
                ConfigKind::File => expanded.is_file(),
                ConfigKind::Directory => expanded.is_dir(),
            };
            if !exists {
                continue;
            }
            found.push(DiscoveredConfig {
                definition_id: def.id.clone(),
                application: def.application.clone(),
                category: def.category,
                kind: def.kind,
                path: expanded.to_string_lossy().to_string(),
                sensitivity: def.sensitivity,
                is_private_key: security::is_private_key(&expanded),
            });
            // First existing candidate path wins.
            break;
        }
    }

    found
}

fn platform_eq(a: Platform, b: Platform) -> bool {
    matches!(
        (a, b),
        (Platform::Macos, Platform::Macos)
            | (Platform::Linux, Platform::Linux)
            | (Platform::Windows, Platform::Windows)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn loads_builtin_definitions_without_panicking() {
        let registry = Registry::load_builtin();
        assert!(!registry.definitions().is_empty());
    }

    #[test]
    fn scan_only_returns_files_that_exist() {
        let registry = Registry {
            definitions: vec![ConfigDefinition {
                id: "test_zshrc".into(),
                application: "Zsh".into(),
                category: Category::Shell,
                kind: ConfigKind::File,
                paths: vec!["~/.zshrc".into()],
                platforms: vec![Platform::current()],
                sensitivity: Sensitivity::Normal,
            }],
        };
        let home = tempdir().unwrap();
        assert!(scan_with_home(&registry, home.path()).is_empty());

        fs::write(home.path().join(".zshrc"), "export FOO=bar\n").unwrap();
        let found = scan_with_home(&registry, home.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].definition_id, "test_zshrc");
    }

    #[test]
    fn flags_private_keys() {
        let registry = Registry {
            definitions: vec![ConfigDefinition {
                id: "test_key".into(),
                application: "SSH Key".into(),
                category: Category::Ssh,
                kind: ConfigKind::File,
                paths: vec!["~/.ssh/id_rsa".into()],
                platforms: vec![Platform::current()],
                sensitivity: Sensitivity::HighlySensitive,
            }],
        };
        let home = tempdir().unwrap();
        fs::create_dir_all(home.path().join(".ssh")).unwrap();
        fs::write(home.path().join(".ssh/id_rsa"), "-----BEGIN KEY-----").unwrap();
        let found = scan_with_home(&registry, home.path());
        assert_eq!(found.len(), 1);
        assert!(found[0].is_private_key);
    }
}
