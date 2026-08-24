use crate::models::Sensitivity;
use std::path::Path;

/// Filename stems that are always treated as private keys and refused for
/// automatic tracking, regardless of what the catalog says about the path.
const PRIVATE_KEY_STEMS: &[&str] = &["id_rsa", "id_ed25519", "id_ecdsa", "id_dsa"];
const PRIVATE_KEY_EXTENSIONS: &[&str] = &["pem", "key", "ppk"];

/// True if `path` looks like a private key file. Used to hard-block tracking
/// even if a user tries to add it as a custom configuration.
pub fn is_private_key(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        // Note: `id_rsa.pub` etc. are public keys and intentionally excluded —
        // only an exact stem match (no `.pub` suffix) counts as private.
        if PRIVATE_KEY_STEMS.contains(&name) {
            return true;
        }
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if PRIVATE_KEY_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
            return true;
        }
    }
    false
}

/// Filename fragments that mark a path as likely to hold credentials, used
/// only as a fallback when a custom path isn't in the catalog.
const SECRET_HINTS: &[&str] = &[
    "credentials", "credential", ".env", "token", "secret", ".npmrc", ".netrc",
];

/// Best-effort sensitivity classification for a path that has no catalog entry
/// (i.e. a user-added custom configuration).
pub fn classify_path(path: &Path) -> Sensitivity {
    if is_private_key(path) {
        return Sensitivity::HighlySensitive;
    }
    let lower = path.to_string_lossy().to_ascii_lowercase();
    if lower.contains("/.ssh/") || SECRET_HINTS.iter().any(|hint| lower.contains(hint)) {
        return Sensitivity::PotentialSecret;
    }
    Sensitivity::Normal
}

/// Directory/file name patterns skipped when snapshotting a directory
/// configuration (build artifacts, VCS metadata, OS cruft).
pub const DIRECTORY_IGNORE_PATTERNS: &[&str] = &[
    ".git",
    ".DS_Store",
    "node_modules",
    "__pycache__",
    ".cache",
    "*.log",
    "*.swp",
    "*.tmp",
];

/// Whether a directory entry name should be skipped during a directory snapshot.
pub fn is_ignored_entry(name: &str) -> bool {
    DIRECTORY_IGNORE_PATTERNS.iter().any(|pattern| {
        if let Some(suffix) = pattern.strip_prefix('*') {
            name.ends_with(suffix)
        } else {
            name == *pattern
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_private_keys() {
        assert!(is_private_key(&PathBuf::from("/home/u/.ssh/id_rsa")));
        assert!(is_private_key(&PathBuf::from("/home/u/.ssh/id_ed25519")));
        assert!(is_private_key(&PathBuf::from("/home/u/certs/server.pem")));
        assert!(!is_private_key(&PathBuf::from("/home/u/.ssh/id_rsa.pub")));
        assert!(!is_private_key(&PathBuf::from("/home/u/.zshrc")));
    }

    #[test]
    fn classifies_secret_hints() {
        assert_eq!(
            classify_path(&PathBuf::from("/home/u/.npmrc")),
            Sensitivity::PotentialSecret
        );
        assert_eq!(
            classify_path(&PathBuf::from("/home/u/.ssh/config")),
            Sensitivity::PotentialSecret
        );
        assert_eq!(
            classify_path(&PathBuf::from("/home/u/.zshrc")),
            Sensitivity::Normal
        );
    }

    #[test]
    fn ignores_common_noise() {
        assert!(is_ignored_entry(".git"));
        assert!(is_ignored_entry("debug.log"));
        assert!(!is_ignored_entry("config"));
    }
}
