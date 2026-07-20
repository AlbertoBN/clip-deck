//! CRUD for the key/value `settings` table.

use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::StoreError;

/// Fetches every stored setting as a key -> JSON value map. Absent keys are
/// simply missing from the map; callers fall back to defaults (see
/// `clip_core::config::AppSettings::from_entries`).
pub fn get_all(conn: &Connection) -> Result<HashMap<String, serde_json::Value>, StoreError> {
    let mut stmt = conn.prepare("SELECT key, value_json FROM settings")?;
    let entries = stmt
        .query_map([], |row| {
            let key: String = row.get(0)?;
            let value_json: String = row.get(1)?;
            Ok((key, value_json))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(entries
        .into_iter()
        .filter_map(|(key, value_json)| serde_json::from_str(&value_json).ok().map(|value| (key, value)))
        .collect())
}

/// Inserts or updates a single setting's value.
pub fn set_value(conn: &Connection, key: &str, value: &serde_json::Value) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        params![key, value.to_string(), crate::clips::to_rfc3339(time::OffsetDateTime::now_utc())],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_is_absent_from_get_all() {
        let conn = crate::db::open(":memory:").unwrap();
        let entries = get_all(&conn).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn a_set_value_is_returned_by_get_all() {
        let conn = crate::db::open(":memory:").unwrap();
        set_value(&conn, "retention_window_days", &serde_json::json!(30)).unwrap();
        let entries = get_all(&conn).unwrap();
        assert_eq!(entries.get("retention_window_days"), Some(&serde_json::json!(30)));
    }

    #[test]
    fn setting_the_same_key_twice_updates_it_in_place() {
        let conn = crate::db::open(":memory:").unwrap();
        set_value(&conn, "retention_window_days", &serde_json::json!(30)).unwrap();
        set_value(&conn, "retention_window_days", &serde_json::json!(60)).unwrap();
        let entries = get_all(&conn).unwrap();
        assert_eq!(entries.get("retention_window_days"), Some(&serde_json::json!(60)));
    }
}
