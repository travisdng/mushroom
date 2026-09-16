//! Schema migrations, applied in order inside a transaction.
//!
//! Embedded with `include_str!` so the binary carries its own schema and a
//! missing file cannot produce a half-migrated database.

use rusqlite::Connection;

use crate::error::AppError;

/// Every migration, in order. Append only — never edit a shipped one.
const MIGRATIONS: &[(u32, &str)] = &[(1, include_str!("../../migrations/0001_init.sql"))];

pub fn current_version(conn: &Connection) -> rusqlite::Result<u32> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
}

pub fn apply(conn: &Connection) -> Result<(), AppError> {
    let version = current_version(conn).map_err(|source| AppError::Database {
        what: "reading the schema version".into(),
        source,
    })?;

    let latest = MIGRATIONS.last().map(|(v, _)| *v).unwrap_or(0);

    // A database written by a newer Mushroom must not be downgraded in place:
    // it may contain columns this build would drop on the next write.
    if version > latest {
        return Err(AppError::IndexTooNew {
            found: version,
            supported: latest,
        });
    }

    for (number, sql) in MIGRATIONS {
        if *number <= version {
            continue;
        }
        tracing::info!(target: "db", migration = number, "applying migration");
        conn.execute_batch(&format!(
            "BEGIN; {sql} PRAGMA user_version = {number}; COMMIT;"
        ))
        .map_err(|source| AppError::Database {
            what: format!("applying migration {number}"),
            source,
        })?;
    }

    Ok(())
}
