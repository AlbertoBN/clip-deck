//! Connection pool, pragmas, migrations.

use rusqlite::Connection;

const INIT_SQL: &str = include_str!("../../../migrations/001_init.sql");

/// Applies the full schema. Idempotent: safe to call on an already-migrated
/// database.
pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(INIT_SQL)
}

/// Opens a connection at the given path (or `:memory:`), sets the standard
/// pragmas, and applies migrations.
pub fn open(path: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    run_migrations(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(conn: &rusqlite::Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
            [name],
            |row| row.get::<_, String>(0),
        )
        .is_ok()
    }

    #[test]
    fn fresh_database_gets_the_full_schema() {
        let conn = open(":memory:").unwrap();
        for table in ["clips", "clip_representations", "app_rules", "settings", "events", "clips_fts"] {
            assert!(table_exists(&conn, table), "expected {table} to exist");
        }
    }

    #[test]
    fn rerunning_migrations_on_an_already_migrated_database_is_a_no_op() {
        let conn = open(":memory:").unwrap();
        run_migrations(&conn).expect("second migration run should succeed");
        assert!(table_exists(&conn, "clips"));
    }

    #[test]
    fn new_connection_reports_wal_journal_mode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clipdeck.sqlite3");
        let conn = open(path.to_str().unwrap()).unwrap();
        let mode: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0)).unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn new_connection_enforces_foreign_keys() {
        let conn = open(":memory:").unwrap();
        let result = conn.execute(
            "INSERT INTO clip_representations (clip_id, mime_type) VALUES ('nonexistent-clip', 'text/plain')",
            [],
        );
        assert!(result.is_err());
    }
}
