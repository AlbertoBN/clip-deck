//! CRUD for exclusion and privacy rules.

use clip_core::models::{Rule, RuleAction};
use rusqlite::{params, Connection, OptionalExtension};

use crate::StoreError;

fn action_to_str(action: RuleAction) -> String {
    serde_json::to_value(action).unwrap().as_str().unwrap().to_string()
}

fn action_from_str(s: &str) -> RuleAction {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap()
}

fn row_to_rule(row: &rusqlite::Row) -> rusqlite::Result<Rule> {
    let action_str: String = row.get(4)?;
    Ok(Rule {
        id: row.get(0)?,
        app_match: row.get(1)?,
        window_match: row.get(2)?,
        mime_match: row.get(3)?,
        action: action_from_str(&action_str),
        enabled: row.get::<_, i64>(5)? != 0,
    })
}

const RULE_COLUMNS: &str = "id, app_match, window_match, mime_match, action, enabled";

/// Inserts a new rule.
pub fn insert(conn: &Connection, rule: &Rule) -> Result<(), StoreError> {
    let now = crate::clips::to_rfc3339(time::OffsetDateTime::now_utc());
    conn.execute(
        "INSERT INTO app_rules (id, app_match, window_match, mime_match, action, enabled, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            rule.id,
            rule.app_match,
            rule.window_match,
            rule.mime_match,
            action_to_str(rule.action),
            rule.enabled as i64,
            now,
        ],
    )?;
    Ok(())
}

/// Inserts a new rule, or updates it in place if a rule with the same id
/// already exists.
pub fn upsert(conn: &Connection, rule: &Rule) -> Result<(), StoreError> {
    let now = crate::clips::to_rfc3339(time::OffsetDateTime::now_utc());
    conn.execute(
        "INSERT INTO app_rules (id, app_match, window_match, mime_match, action, enabled, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7) \
         ON CONFLICT(id) DO UPDATE SET \
             app_match = excluded.app_match, \
             window_match = excluded.window_match, \
             mime_match = excluded.mime_match, \
             action = excluded.action, \
             enabled = excluded.enabled, \
             updated_at = excluded.updated_at",
        params![
            rule.id,
            rule.app_match,
            rule.window_match,
            rule.mime_match,
            action_to_str(rule.action),
            rule.enabled as i64,
            now,
        ],
    )?;
    Ok(())
}

/// Deletes a rule by id.
pub fn delete(conn: &Connection, id: &str) -> Result<(), StoreError> {
    conn.execute("DELETE FROM app_rules WHERE id = ?1", [id])?;
    Ok(())
}

/// Fetches a single rule by id.
pub fn get(conn: &Connection, id: &str) -> Result<Option<Rule>, StoreError> {
    let rule = conn
        .query_row(&format!("SELECT {RULE_COLUMNS} FROM app_rules WHERE id = ?1"), [id], row_to_rule)
        .optional()?;
    Ok(rule)
}

/// Enables or disables a rule.
pub fn set_enabled(conn: &Connection, id: &str, enabled: bool) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE app_rules SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
        params![enabled as i64, crate::clips::to_rfc3339(time::OffsetDateTime::now_utc()), id],
    )?;
    Ok(())
}

/// Lists every currently-enabled rule.
pub fn list_enabled(conn: &Connection) -> Result<Vec<Rule>, StoreError> {
    let mut stmt = conn.prepare(&format!("SELECT {RULE_COLUMNS} FROM app_rules WHERE enabled = 1"))?;
    let rules = stmt.query_map([], row_to_rule)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clip_core::models::{Rule, RuleAction};

    #[test]
    fn created_rule_is_fetchable_by_id() {
        let conn = crate::db::open(":memory:").unwrap();
        let rule = Rule::new("r1", "1Password", None, None, RuleAction::Exclude);
        insert(&conn, &rule).unwrap();
        let fetched = get(&conn, "r1").unwrap().unwrap();
        assert_eq!(fetched.app_match, "1Password");
    }

    #[test]
    fn disabling_a_rule_updates_only_its_enabled_flag() {
        let conn = crate::db::open(":memory:").unwrap();
        let rule = Rule::new("r1", "1Password", None, None, RuleAction::Exclude);
        insert(&conn, &rule).unwrap();
        set_enabled(&conn, "r1", false).unwrap();
        let fetched = get(&conn, "r1").unwrap().unwrap();
        assert!(!fetched.enabled);
        assert_eq!(fetched.app_match, "1Password");
    }

    #[test]
    fn upserting_a_new_rule_inserts_it() {
        let conn = crate::db::open(":memory:").unwrap();
        upsert(&conn, &Rule::new("r1", "1Password", None, None, RuleAction::Exclude)).unwrap();
        let fetched = get(&conn, "r1").unwrap().unwrap();
        assert_eq!(fetched.app_match, "1Password");
    }

    #[test]
    fn upserting_an_existing_rule_updates_it_in_place() {
        let conn = crate::db::open(":memory:").unwrap();
        upsert(&conn, &Rule::new("r1", "1Password", None, None, RuleAction::Exclude)).unwrap();
        upsert(&conn, &Rule::new("r1", "Bitwarden", None, None, RuleAction::Exclude)).unwrap();
        let fetched = get(&conn, "r1").unwrap().unwrap();
        assert_eq!(fetched.app_match, "Bitwarden");
    }

    #[test]
    fn deleting_a_rule_removes_it() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &Rule::new("r1", "1Password", None, None, RuleAction::Exclude)).unwrap();
        delete(&conn, "r1").unwrap();
        assert!(get(&conn, "r1").unwrap().is_none());
    }

    #[test]
    fn listing_enabled_rules_excludes_disabled_ones() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &Rule::new("r1", "Enabled", None, None, RuleAction::Exclude)).unwrap();
        insert(&conn, &Rule::new("r2", "Disabled", None, None, RuleAction::Exclude)).unwrap();
        set_enabled(&conn, "r2", false).unwrap();
        let enabled = list_enabled(&conn).unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "r1");
    }

    #[test]
    fn listing_enabled_rules_includes_rules_with_no_window_or_mime_match_set() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &Rule::new("r1", "OnlyApp", None, None, RuleAction::Exclude)).unwrap();
        let enabled = list_enabled(&conn).unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].window_match, None);
        assert_eq!(enabled[0].mime_match, None);
    }
}
