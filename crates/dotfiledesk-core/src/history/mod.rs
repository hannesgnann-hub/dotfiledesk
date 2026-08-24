use crate::models::{ConfigKind, Configuration, Status};
use crate::security;
use crate::CoreError;
use git2::{IndexAddOption, ObjectType, Repository, Signature};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Wraps the internal, user-invisible git repository that backs all history.
/// One physical repo holds every tracked configuration, each isolated under
/// a `{configuration_id}/` subtree so paths never collide.
pub struct HistoryRepo {
    repo: Repository,
}

impl HistoryRepo {
    pub fn open_or_init(path: &Path) -> Result<Self, CoreError> {
        fs::create_dir_all(path)?;
        let repo = Repository::open(path).or_else(|_| Repository::init(path))?;
        Ok(HistoryRepo { repo })
    }

    fn subtree_dir(&self, config: &Configuration) -> PathBuf {
        self.repo
            .workdir()
            .expect("history repo is never bare")
            .join(&config.id)
    }

    /// Copies the current on-disk state of `config` into the internal repo and
    /// commits it if anything changed. Returns `Ok(None)` when there was
    /// nothing new to record.
    pub fn snapshot(&self, config: &Configuration, reason: &str) -> Result<Option<String>, CoreError> {
        let source = Path::new(&config.path);
        let dest_root = self.subtree_dir(config);

        if dest_root.exists() {
            fs::remove_dir_all(&dest_root)?;
        }

        if source.exists() {
            match config.kind {
                ConfigKind::File => {
                    fs::create_dir_all(&dest_root)?;
                    let file_name = source
                        .file_name()
                        .ok_or_else(|| CoreError::InvalidPath(config.path.clone()))?;
                    fs::copy(source, dest_root.join(file_name))?;
                }
                ConfigKind::Directory => {
                    copy_directory(source, &dest_root)?;
                }
            }
        }
        // If `source` no longer exists we still proceed: the subtree is left
        // empty, so a prior snapshot's files show as deleted in the diff.

        let pathspec = format!("{}/", config.id);
        let mut index = self.repo.index()?;
        index.add_all([&pathspec].iter(), IndexAddOption::DEFAULT, None)?;
        index.update_all([&pathspec].iter(), None)?;
        index.write()?;

        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let parent_commit = self.repo.head().ok().and_then(|h| h.peel_to_commit().ok());

        if let Some(parent) = &parent_commit {
            let mut opts = git2::DiffOptions::new();
            opts.pathspec(&pathspec);
            let diff = self
                .repo
                .diff_tree_to_tree(Some(&parent.tree()?), Some(&tree), Some(&mut opts))?;
            if diff.deltas().len() == 0 {
                return Ok(None);
            }
        }

        let sig = Signature::now("DotfileDesk", "dotfiledesk@localhost")?;
        let parents: Vec<&git2::Commit> = parent_commit.iter().collect();
        let commit_oid = self
            .repo
            .commit(Some("HEAD"), &sig, &sig, reason, &tree, &parents)?;

        Ok(Some(commit_oid.to_string()))
    }

    /// Diffs the subtree of `config` between two commits. `from` is `None` for
    /// the very first snapshot (diffed against an empty tree).
    pub fn diff_commits(
        &self,
        config: &Configuration,
        from: Option<&str>,
        to: &str,
    ) -> Result<DiffResult, CoreError> {
        let old_files = match from {
            Some(oid) => self.read_tree_subtree(config, oid)?,
            None => BTreeMap::new(),
        };
        let new_files = self.read_tree_subtree(config, to)?;
        Ok(diff_file_maps(&old_files, &new_files))
    }

    /// Diffs the latest snapshot's subtree against what's actually on disk
    /// right now, without creating a new snapshot.
    pub fn diff_against_working(
        &self,
        config: &Configuration,
        latest_commit: &str,
    ) -> Result<DiffResult, CoreError> {
        let old_files = self.read_tree_subtree(config, latest_commit)?;
        let new_files = read_disk_files(config)?;
        Ok(diff_file_maps(&old_files, &new_files))
    }

    /// True if the working copy differs from the given snapshot commit.
    pub fn differs_from_working(
        &self,
        config: &Configuration,
        latest_commit: &str,
    ) -> Result<bool, CoreError> {
        let old_files = self.read_tree_subtree(config, latest_commit)?;
        let new_files = read_disk_files(config)?;
        Ok(old_files != new_files)
    }

    /// Writes every file recorded in `commit` back to `config.path` on disk.
    /// Files present in the snapshot are created/overwritten; files on disk
    /// that aren't part of the snapshot are left untouched.
    pub fn restore_files(&self, config: &Configuration, commit: &str) -> Result<(), CoreError> {
        let files = self.read_tree_subtree(config, commit)?;
        match config.kind {
            ConfigKind::File => {
                let dest = Path::new(&config.path);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                if let Some((_, content)) = files.iter().next() {
                    fs::write(dest, content)?;
                }
            }
            ConfigKind::Directory => {
                let root = Path::new(&config.path);
                fs::create_dir_all(root)?;
                for (relative, content) in &files {
                    let dest = root.join(relative);
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(dest, content)?;
                }
            }
        }
        Ok(())
    }

    fn read_tree_subtree(
        &self,
        config: &Configuration,
        commit_oid: &str,
    ) -> Result<BTreeMap<String, Vec<u8>>, CoreError> {
        let oid = git2::Oid::from_str(commit_oid).map_err(|_| CoreError::InvalidCommit(commit_oid.to_string()))?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let mut files = BTreeMap::new();
        let entry = match tree.get_path(Path::new(&config.id)) {
            Ok(e) => e,
            Err(_) => return Ok(files), // subtree absent (e.g. deleted before this commit)
        };
        let object = entry.to_object(&self.repo)?;
        if let Some(subtree) = object.as_tree() {
            collect_tree_files(&self.repo, subtree, &PathBuf::new(), &mut files)?;
        }
        Ok(files)
    }
}

fn collect_tree_files(
    repo: &Repository,
    tree: &git2::Tree,
    prefix: &Path,
    out: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), CoreError> {
    for entry in tree.iter() {
        let name = entry.name().unwrap_or_default();
        let relative = prefix.join(name);
        let object = entry.to_object(repo)?;
        match entry.kind() {
            Some(ObjectType::Tree) => {
                collect_tree_files(repo, object.as_tree().unwrap(), &relative, out)?;
            }
            Some(ObjectType::Blob) => {
                let blob = object.as_blob().unwrap();
                out.insert(relative.to_string_lossy().to_string(), blob.content().to_vec());
            }
            _ => {}
        }
    }
    Ok(())
}

fn read_disk_files(config: &Configuration) -> Result<BTreeMap<String, Vec<u8>>, CoreError> {
    let mut files = BTreeMap::new();
    let source = Path::new(&config.path);
    if !source.exists() {
        return Ok(files);
    }
    match config.kind {
        ConfigKind::File => {
            let name = source
                .file_name()
                .ok_or_else(|| CoreError::InvalidPath(config.path.clone()))?
                .to_string_lossy()
                .to_string();
            files.insert(name, fs::read(source)?);
        }
        ConfigKind::Directory => {
            for entry in WalkDir::new(source).into_iter().filter_entry(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| !security::is_ignored_entry(n))
                    .unwrap_or(true)
            }) {
                let entry = entry.map_err(|e| CoreError::Io(e.into()))?;
                if entry.file_type().is_file() {
                    let relative = entry.path().strip_prefix(source).unwrap();
                    files.insert(
                        relative.to_string_lossy().to_string(),
                        fs::read(entry.path())?,
                    );
                }
            }
        }
    }
    Ok(files)
}

fn copy_directory(source: &Path, dest: &Path) -> Result<(), CoreError> {
    for entry in WalkDir::new(source).into_iter().filter_entry(|e| {
        e.file_name()
            .to_str()
            .map(|n| !security::is_ignored_entry(n))
            .unwrap_or(true)
    }) {
        let entry = entry.map_err(|e| CoreError::Io(e.into()))?;
        let relative = entry.path().strip_prefix(source).unwrap();
        let target = dest.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineTag {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub tag: LineTag,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub change: FileChangeKind,
    pub binary: bool,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffResult {
    pub files: Vec<FileDiff>,
}

fn diff_file_maps(old: &BTreeMap<String, Vec<u8>>, new: &BTreeMap<String, Vec<u8>>) -> DiffResult {
    let mut paths: Vec<&String> = old.keys().chain(new.keys()).collect();
    paths.sort();
    paths.dedup();

    let mut files = Vec::new();
    for path in paths {
        let old_bytes = old.get(path);
        let new_bytes = new.get(path);
        if old_bytes == new_bytes {
            continue;
        }
        let change = match (old_bytes, new_bytes) {
            (None, Some(_)) => FileChangeKind::Added,
            (Some(_), None) => FileChangeKind::Deleted,
            _ => FileChangeKind::Modified,
        };

        let old_text = old_bytes.and_then(|b| std::str::from_utf8(b).ok());
        let new_text = new_bytes.and_then(|b| std::str::from_utf8(b).ok());
        let binary = (old_bytes.is_some() && old_text.is_none())
            || (new_bytes.is_some() && new_text.is_none());

        let lines = if binary {
            Vec::new()
        } else {
            build_diff_lines(old_text.unwrap_or(""), new_text.unwrap_or(""))
        };

        files.push(FileDiff {
            path: path.clone(),
            change,
            binary,
            lines,
        });
    }
    DiffResult { files }
}

fn build_diff_lines(old_text: &str, new_text: &str) -> Vec<DiffLine> {
    use similar::{ChangeTag, TextDiff};
    let diff = TextDiff::from_lines(old_text, new_text);
    diff.iter_all_changes()
        .map(|change| {
            let tag = match change.tag() {
                ChangeTag::Delete => LineTag::Removed,
                ChangeTag::Insert => LineTag::Added,
                ChangeTag::Equal => LineTag::Context,
            };
            DiffLine {
                tag,
                content: change.to_string().trim_end_matches('\n').to_string(),
            }
        })
        .collect()
}

/// Computes the current status of `config` against `latest_commit`, if any.
pub fn compute_status(
    repo: &HistoryRepo,
    config: &Configuration,
    latest_commit: Option<&str>,
) -> Result<Status, CoreError> {
    let exists = Path::new(&config.path).exists();
    match (exists, latest_commit) {
        (false, Some(_)) => Ok(Status::Missing),
        (false, None) => Ok(Status::NotTracked),
        (true, None) => Ok(Status::NotTracked),
        (true, Some(commit)) => {
            if repo.differs_from_working(config, commit)? {
                Ok(Status::Modified)
            } else {
                Ok(Status::Synced)
            }
        }
    }
}
