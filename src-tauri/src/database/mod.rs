//! SQLite: the search index.
//!
//! A cache, not storage. Deleting `mushroom.db` must be a safe, boring act —
//! everything here is rebuilt from the Markdown on the next launch.

pub mod migrations;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::AppError;

pub struct Db {
    /// One connection behind a mutex. A single user generates no meaningful
    /// contention, and it removes a whole class of WAL and locking surprises.
    conn: Mutex<Connection>,
    pub path: PathBuf,
}

impl Db {
    /// Open the index, applying pragmas and migrations.
    ///
    /// A database that cannot be opened or understood is moved aside rather
    /// than blocking startup; the caller then triggers a rebuild (R2.6).
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = match Self::open_and_prepare(path) {
            Ok(conn) => conn,
            Err(err) => {
                tracing::warn!(
                    target: "db",
                    error = %err,
                    "index could not be opened; moving it aside and starting fresh"
                );
                Self::quarantine(path)?;
                Self::open_and_prepare(path)?
            }
        };

        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }

    /// An in-memory index, for tests.
    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory().map_err(|source| AppError::Database {
            what: "opening an in-memory index".into(),
            source,
        })?;
        Self::probe_fts5(&conn)?;
        migrations::apply(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        })
    }

    fn open_and_prepare(path: &Path) -> Result<Connection, AppError> {
        let conn = Connection::open(path).map_err(|source| AppError::Database {
            what: "opening the index".into(),
            source,
        })?;

        // WAL keeps readers from blocking on the writer; NORMAL is the right
        // durability trade for a cache that can be rebuilt from Markdown.
        for (pragma, value) in [
            ("journal_mode", "WAL"),
            ("synchronous", "NORMAL"),
            ("foreign_keys", "ON"),
        ] {
            conn.pragma_update(None, pragma, value)
                .map_err(|source| AppError::Database {
                    what: format!("setting {pragma}"),
                    source,
                })?;
        }

        Self::probe_fts5(&conn)?;
        migrations::apply(&conn)?;
        Ok(conn)
    }

    /// Fail loudly at startup if this build of SQLite has no FTS5, rather than
    /// at the first search.
    fn probe_fts5(conn: &Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "CREATE VIRTUAL TABLE temp.fts5_probe USING fts5(x);
             DROP TABLE temp.fts5_probe;",
        )
        .map_err(|source| AppError::Fts5Unavailable { source })
    }

    fn quarantine(path: &Path) -> Result<(), AppError> {
        if !path.exists() {
            return Ok(());
        }
        let stamp = crate::notes::store::now_rfc3339().replace([':', '-'], "");
        let aside = path.with_extension(format!("db.corrupt.{stamp}"));
        std::fs::rename(path, &aside).map_err(|source| AppError::IndexUnavailable {
            path: path.to_path_buf(),
            source,
        })?;
        // The -wal and -shm siblings belong to the old database too.
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(PathBuf::from(format!("{}{suffix}", path.display())));
        }
        tracing::warn!(target: "db", moved_to = %aside.display(), "index quarantined");
        Ok(())
    }

    /// Run a closure with the connection. Callers are already on a blocking
    /// thread; this never awaits.
    pub fn with<T>(
        &self,
        what: &str,
        f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, AppError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| AppError::Internal("index lock poisoned".into()))?;
        f(&conn).map_err(|source| AppError::Database {
            what: what.to_string(),
            source,
        })
    }

    /// Run a closure inside a transaction, rolling back on error.
    pub fn transaction<T>(
        &self,
        what: &str,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> rusqlite::Result<T>,
    ) -> Result<T, AppError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| AppError::Internal("index lock poisoned".into()))?;
        let tx = conn.transaction().map_err(|source| AppError::Database {
            what: what.to_string(),
            source,
        })?;
        let value = f(&tx).map_err(|source| AppError::Database {
            what: what.to_string(),
            source,
        })?;
        tx.commit().map_err(|source| AppError::Database {
            what: what.to_string(),
            source,
        })?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts5_is_available_in_this_build() {
        // If this fails, the bundled SQLite was built without FTS5 and every
        // search would break at runtime instead of here.
        let db = Db::open_in_memory().unwrap();
        db.with("probe", |conn| {
            conn.execute_batch(
                "CREATE VIRTUAL TABLE temp.t USING fts5(x);
                 INSERT INTO temp.t(x) VALUES ('hello world');",
            )?;
            let hits: i64 = conn.query_row(
                "SELECT count(*) FROM temp.t WHERE t MATCH 'hello'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(hits, 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn migrations_create_the_schema_and_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mushroom.db");

        let db = Db::open(&path).unwrap();
        let version = db.with("version", migrations::current_version).unwrap();
        assert_eq!(version, migrations::latest_version());
        drop(db);

        // Re-opening must not re-apply anything.
        let db = Db::open(&path).unwrap();
        assert_eq!(
            db.with("version", migrations::current_version).unwrap(),
            migrations::latest_version()
        );

        for table in [
            "notes",
            "passages",
            "notes_fts",
            "passages_fts",
            "index_state",
        ] {
            let found: i64 = db
                .with("check table", |conn| {
                    conn.query_row(
                        "SELECT count(*) FROM sqlite_master WHERE name = ?1",
                        [table],
                        |r| r.get(0),
                    )
                })
                .unwrap();
            assert_eq!(found, 1, "{table} missing");
        }
    }

    #[test]
    fn a_corrupt_index_is_moved_aside_rather_than_blocking_startup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mushroom.db");
        std::fs::write(&path, b"this is definitely not a sqlite database").unwrap();

        // Opening must succeed by starting over, not fail.
        let db = Db::open(&path).unwrap();
        assert_eq!(
            db.with("version", migrations::current_version).unwrap(),
            migrations::latest_version()
        );

        let quarantined: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
            .collect();
        assert_eq!(quarantined.len(), 1, "the bad file should have been kept");
    }

    #[test]
    fn a_future_schema_is_refused_rather_than_downgraded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mushroom.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", 99).unwrap();
        }

        // Db::open recovers by quarantining, so check the migration layer
        // directly: it must refuse rather than silently downgrade.
        let conn = Connection::open(&path).unwrap();
        assert!(matches!(
            migrations::apply(&conn),
            Err(AppError::IndexTooNew { .. })
        ));
    }
}
