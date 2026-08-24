use crate::history::HistoryRepo;
use crate::models::Configuration;
use crate::tracking::Store;
use crate::CoreError;
use serde::{Deserialize, Serialize};

/// Result of a restore operation, surfaced to the UI so it can show exactly
/// what happened (backup taken, restore verified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    /// Commit created to preserve the pre-restore state, if it differed from
    /// the last snapshot.
    pub backup_commit: Option<String>,
    /// Commit representing the restored state, recorded as a new history entry.
    pub restore_commit: String,
    /// Whether the file/directory was confirmed present on disk afterwards.
    pub verified: bool,
}

/// Restores `config` to `target_commit`, following the mandatory safety
/// workflow: back up the current state first, write the restored files,
/// verify they landed, then record the restore itself as a new snapshot.
pub fn restore(
    repo: &HistoryRepo,
    store: &Store,
    config: &Configuration,
    target_commit: &str,
) -> Result<RestoreResult, CoreError> {
    // 1. Snapshot current version so it's never silently lost.
    let backup_commit = repo.snapshot(config, "Pre-restore backup")?;
    if let Some(commit) = &backup_commit {
        store.record_snapshot(&config.id, commit, "Pre-restore backup")?;
    }

    // 2. Restore the selected version onto the real path.
    repo.restore_files(config, target_commit)?;

    // 3. Verify the file/directory now exists.
    let verified = std::path::Path::new(&config.path).exists();

    // 4. Record the restore itself as a new history entry.
    let short = &target_commit[..target_commit.len().min(7)];
    let reason = format!("Restored to {short}");
    let restore_commit = match repo.snapshot(config, &reason)? {
        Some(commit) => commit,
        // Working copy already matched the target exactly (e.g. restoring
        // straight after a backup of identical content); still log the event.
        None => target_commit.to_string(),
    };
    store.record_snapshot(&config.id, &restore_commit, &reason)?;

    Ok(RestoreResult {
        backup_commit,
        restore_commit,
        verified,
    })
}
