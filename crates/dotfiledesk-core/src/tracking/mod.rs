use crate::models::{Category, ConfigKind, Configuration, Sensitivity, Snapshot};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

use crate::CoreError;

/// SQLite-backed storage for configuration and snapshot metadata. File
/// contents are never stored here; they live in the internal git repository
/// (see [`crate::history`]).
pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(db_path: &Path) -> Result<Self, CoreError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS configurations (
                id TEXT PRIMARY KEY,
                definition_id TEXT,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                category TEXT NOT NULL,
                kind TEXT NOT NULL,
                sensitivity TEXT NOT NULL,
                added_at TEXT NOT NULL,
                last_snapshot_at TEXT
            );
            CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                configuration_id TEXT NOT NULL REFERENCES configurations(id) ON DELETE CASCADE,
                created_at TEXT NOT NULL,
                git_commit TEXT NOT NULL,
                reason TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;
        Self::ensure_column(&conn, "configurations", "archived", "INTEGER NOT NULL DEFAULT 0")?;
        Self::ensure_column(&conn, "snapshots", "favorite", "INTEGER NOT NULL DEFAULT 0")?;
        Self::ensure_column(&conn, "snapshots", "archived", "INTEGER NOT NULL DEFAULT 0")?;
        Ok(Store { conn })
    }

    /// Adds `column` to `table` if an earlier version of DotfileDesk created
    /// the database before that column existed. Safe to call on every open.
    fn ensure_column(conn: &Connection, table: &str, column: &str, ddl: &str) -> Result<(), CoreError> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let exists = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .any(|name| name == column);
        drop(stmt);
        if !exists {
            conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {ddl}"), [])?;
        }
        Ok(())
    }

    pub fn add_configuration(
        &self,
        definition_id: Option<&str>,
        name: &str,
        path: &str,
        category: Category,
        kind: ConfigKind,
        sensitivity: Sensitivity,
    ) -> Result<Configuration, CoreError> {
        let config = Configuration {
            id: Uuid::new_v4().to_string(),
            definition_id: definition_id.map(|s| s.to_string()),
            name: name.to_string(),
            path: path.to_string(),
            category,
            kind,
            sensitivity,
            added_at: Utc::now().to_rfc3339(),
            last_snapshot_at: None,
            archived: false,
        };
        self.conn.execute(
            "INSERT INTO configurations
                (id, definition_id, name, path, category, kind, sensitivity, added_at, last_snapshot_at, archived)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)",
            params![
                config.id,
                config.definition_id,
                config.name,
                config.path,
                category_str(config.category),
                kind_str(config.kind),
                sensitivity_str(config.sensitivity),
                config.added_at,
                config.last_snapshot_at,
            ],
        )?;
        Ok(config)
    }

    pub fn list_configurations(&self, include_archived: bool) -> Result<Vec<Configuration>, CoreError> {
        let sql = if include_archived {
            "SELECT id, definition_id, name, path, category, kind, sensitivity, added_at, last_snapshot_at, archived
             FROM configurations ORDER BY name ASC"
        } else {
            "SELECT id, definition_id, name, path, category, kind, sensitivity, added_at, last_snapshot_at, archived
             FROM configurations WHERE archived = 0 ORDER BY name ASC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], row_to_configuration)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn list_archived_configurations(&self) -> Result<Vec<Configuration>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, definition_id, name, path, category, kind, sensitivity, added_at, last_snapshot_at, archived
             FROM configurations WHERE archived = 1 ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], row_to_configuration)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get_configuration(&self, id: &str) -> Result<Option<Configuration>, CoreError> {
        self.conn
            .query_row(
                "SELECT id, definition_id, name, path, category, kind, sensitivity, added_at, last_snapshot_at, archived
                 FROM configurations WHERE id = ?1",
                params![id],
                row_to_configuration,
            )
            .optional()
            .map_err(CoreError::from)
    }

    pub fn remove_configuration(&self, id: &str) -> Result<(), CoreError> {
        self.conn
            .execute("DELETE FROM configurations WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_configuration_archived(&self, id: &str, archived: bool) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE configurations SET archived = ?1 WHERE id = ?2",
            params![archived, id],
        )?;
        Ok(())
    }

    pub fn touch_last_snapshot(&self, id: &str, at: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE configurations SET last_snapshot_at = ?1 WHERE id = ?2",
            params![at, id],
        )?;
        Ok(())
    }

    pub fn record_snapshot(
        &self,
        configuration_id: &str,
        git_commit: &str,
        reason: &str,
    ) -> Result<Snapshot, CoreError> {
        let snapshot = Snapshot {
            id: Uuid::new_v4().to_string(),
            configuration_id: configuration_id.to_string(),
            created_at: Utc::now().to_rfc3339(),
            git_commit: git_commit.to_string(),
            reason: reason.to_string(),
            favorite: false,
            archived: false,
        };
        self.conn.execute(
            "INSERT INTO snapshots (id, configuration_id, created_at, git_commit, reason, favorite, archived)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0)",
            params![
                snapshot.id,
                snapshot.configuration_id,
                snapshot.created_at,
                snapshot.git_commit,
                snapshot.reason,
            ],
        )?;
        self.touch_last_snapshot(configuration_id, &snapshot.created_at)?;
        Ok(snapshot)
    }

    pub fn list_snapshots(&self, configuration_id: &str) -> Result<Vec<Snapshot>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, configuration_id, created_at, git_commit, reason, favorite, archived
             FROM snapshots WHERE configuration_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![configuration_id], row_to_snapshot)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<Snapshot>, CoreError> {
        self.conn
            .query_row(
                "SELECT id, configuration_id, created_at, git_commit, reason, favorite, archived
                 FROM snapshots WHERE id = ?1",
                params![snapshot_id],
                row_to_snapshot,
            )
            .optional()
            .map_err(CoreError::from)
    }

    pub fn set_snapshot_favorite(&self, snapshot_id: &str, favorite: bool) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE snapshots SET favorite = ?1 WHERE id = ?2",
            params![favorite, snapshot_id],
        )?;
        Ok(())
    }

    pub fn set_snapshot_archived(&self, snapshot_id: &str, archived: bool) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE snapshots SET archived = ?1 WHERE id = ?2",
            params![archived, snapshot_id],
        )?;
        Ok(())
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), CoreError> {
        self.conn
            .execute("DELETE FROM snapshots WHERE id = ?1", params![snapshot_id])?;
        Ok(())
    }
}

fn row_to_configuration(row: &rusqlite::Row) -> rusqlite::Result<Configuration> {
    Ok(Configuration {
        id: row.get(0)?,
        definition_id: row.get(1)?,
        name: row.get(2)?,
        path: row.get(3)?,
        category: parse_category(&row.get::<_, String>(4)?),
        kind: parse_kind(&row.get::<_, String>(5)?),
        sensitivity: parse_sensitivity(&row.get::<_, String>(6)?),
        added_at: row.get(7)?,
        last_snapshot_at: row.get(8)?,
        archived: row.get(9)?,
    })
}

fn row_to_snapshot(row: &rusqlite::Row) -> rusqlite::Result<Snapshot> {
    Ok(Snapshot {
        id: row.get(0)?,
        configuration_id: row.get(1)?,
        created_at: row.get(2)?,
        git_commit: row.get(3)?,
        reason: row.get(4)?,
        favorite: row.get(5)?,
        archived: row.get(6)?,
    })
}

fn category_str(c: Category) -> &'static str {
    match c {
        Category::Shell => "shell",
        Category::Git => "git",
        Category::Terminal => "terminal",
        Category::Editor => "editor",
        Category::Ssh => "ssh",
        Category::PackageManagers => "package_managers",
        Category::DeveloperTools => "developer_tools",
        Category::Other => "other",
    }
}

fn parse_category(s: &str) -> Category {
    match s {
        "shell" => Category::Shell,
        "git" => Category::Git,
        "terminal" => Category::Terminal,
        "editor" => Category::Editor,
        "ssh" => Category::Ssh,
        "package_managers" => Category::PackageManagers,
        "developer_tools" => Category::DeveloperTools,
        _ => Category::Other,
    }
}

fn kind_str(k: ConfigKind) -> &'static str {
    match k {
        ConfigKind::File => "file",
        ConfigKind::Directory => "directory",
    }
}

fn parse_kind(s: &str) -> ConfigKind {
    match s {
        "directory" => ConfigKind::Directory,
        _ => ConfigKind::File,
    }
}

fn sensitivity_str(s: Sensitivity) -> &'static str {
    match s {
        Sensitivity::Normal => "normal",
        Sensitivity::PotentialSecret => "potential_secret",
        Sensitivity::HighlySensitive => "highly_sensitive",
    }
}

fn parse_sensitivity(s: &str) -> Sensitivity {
    match s {
        "potential_secret" => Sensitivity::PotentialSecret,
        "highly_sensitive" => Sensitivity::HighlySensitive,
        _ => Sensitivity::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_remove_roundtrip() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let config = store
            .add_configuration(
                Some("zsh"),
                "Zsh",
                "/home/u/.zshrc",
                Category::Shell,
                ConfigKind::File,
                Sensitivity::Normal,
            )
            .unwrap();

        let listed = store.list_configurations(true).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, config.id);

        let fetched = store.get_configuration(&config.id).unwrap().unwrap();
        assert_eq!(fetched.path, "/home/u/.zshrc");

        store.remove_configuration(&config.id).unwrap();
        assert!(store.list_configurations(true).unwrap().is_empty());
    }

    #[test]
    fn records_snapshots_and_updates_last_snapshot_at() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let config = store
            .add_configuration(
                None,
                "Custom",
                "/home/u/.customrc",
                Category::Other,
                ConfigKind::File,
                Sensitivity::Normal,
            )
            .unwrap();

        store
            .record_snapshot(&config.id, "abc123", "Initial snapshot")
            .unwrap();
        let snaps = store.list_snapshots(&config.id).unwrap();
        assert_eq!(snaps.len(), 1);

        let refreshed = store.get_configuration(&config.id).unwrap().unwrap();
        assert!(refreshed.last_snapshot_at.is_some());
    }

    #[test]
    fn archiving_hides_configuration_from_default_list() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let config = store
            .add_configuration(None, "Custom", "/home/u/.customrc", Category::Other, ConfigKind::File, Sensitivity::Normal)
            .unwrap();

        store.set_configuration_archived(&config.id, true).unwrap();
        assert!(store.list_configurations(false).unwrap().is_empty());
        assert_eq!(store.list_configurations(true).unwrap().len(), 1);
        assert_eq!(store.list_archived_configurations().unwrap().len(), 1);

        store.set_configuration_archived(&config.id, false).unwrap();
        assert_eq!(store.list_configurations(false).unwrap().len(), 1);
    }

    #[test]
    fn snapshot_favorite_archive_and_delete() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let config = store
            .add_configuration(None, "Custom", "/home/u/.customrc", Category::Other, ConfigKind::File, Sensitivity::Normal)
            .unwrap();
        let snap = store.record_snapshot(&config.id, "abc123", "Initial snapshot").unwrap();
        assert!(!snap.favorite);
        assert!(!snap.archived);

        store.set_snapshot_favorite(&snap.id, true).unwrap();
        let fetched = store.get_snapshot(&snap.id).unwrap().unwrap();
        assert!(fetched.favorite);

        store.set_snapshot_archived(&snap.id, true).unwrap();
        let fetched = store.get_snapshot(&snap.id).unwrap().unwrap();
        assert!(fetched.archived);

        store.delete_snapshot(&snap.id).unwrap();
        assert!(store.get_snapshot(&snap.id).unwrap().is_none());
        assert!(store.list_snapshots(&config.id).unwrap().is_empty());
    }
}
