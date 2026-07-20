//! Auto-delete and pruning jobs.

use rusqlite::{params, Connection};

use crate::StoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearScope {
    All,
    ExcludingPinned,
}

/// Permanently removes non-pinned clips older than `retention_days`. A `None`
/// window is a no-op regardless of clip age.
pub fn prune(conn: &Connection, retention_days: Option<u32>) -> Result<usize, StoreError> {
    let Some(days) = retention_days else {
        return Ok(0);
    };
    let cutoff = crate::clips::to_rfc3339(time::OffsetDateTime::now_utc() - time::Duration::days(days as i64));
    let deleted = conn.execute(
        "DELETE FROM clips WHERE is_pinned = 0 AND created_at < ?1",
        params![cutoff],
    )?;
    Ok(deleted)
}

/// Clears history per the given scope.
pub fn clear(conn: &Connection, scope: ClearScope) -> Result<usize, StoreError> {
    let deleted = match scope {
        ClearScope::All => conn.execute("DELETE FROM clips", [])?,
        ClearScope::ExcludingPinned => conn.execute("DELETE FROM clips WHERE is_pinned = 0", [])?,
    };
    Ok(deleted)
}

/// Clears history per the given scope, like `clear`, but returns the ids of
/// the clips actually removed (so callers can publish a per-clip event).
pub fn clear_with_ids(conn: &Connection, scope: ClearScope) -> Result<Vec<String>, StoreError> {
    let where_clause = match scope {
        ClearScope::All => "",
        ClearScope::ExcludingPinned => " WHERE is_pinned = 0",
    };
    let mut stmt = conn.prepare(&format!("SELECT id FROM clips{where_clause}"))?;
    let ids: Vec<String> = stmt.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;
    conn.execute(&format!("DELETE FROM clips{where_clause}"), [])?;
    Ok(ids)
}

/// Deletes a single clip by id, independent of retention or bulk clear.
pub fn delete_clip(conn: &Connection, id: &str) -> Result<(), StoreError> {
    crate::clips::soft_delete(conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clip_core::models::Clip;

    fn clip_created_days_ago(id: &str, days: i64) -> Clip {
        let mut clip = Clip::new(id, format!("hash-{id}"), "text/plain", vec![]);
        clip.created_at = time::OffsetDateTime::now_utc() - time::Duration::days(days);
        clip.updated_at = clip.created_at;
        clip
    }

    #[test]
    fn old_unpinned_clip_is_pruned() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_created_days_ago("c1", 40)).unwrap();
        prune(&conn, Some(30)).unwrap();
        assert!(crate::clips::get(&conn, "c1").unwrap().is_none());
    }

    #[test]
    fn old_pinned_clip_survives_pruning() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_created_days_ago("c1", 40)).unwrap();
        crate::clips::set_pinned(&conn, "c1", true).unwrap();
        prune(&conn, Some(30)).unwrap();
        assert!(crate::clips::get(&conn, "c1").unwrap().is_some());
    }

    #[test]
    fn no_retention_window_configured_means_prune_is_a_no_op() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_created_days_ago("c1", 400)).unwrap();
        prune(&conn, None).unwrap();
        assert!(crate::clips::get(&conn, "c1").unwrap().is_some());
    }

    #[test]
    fn clearing_with_all_scope_removes_pinned_clips_too() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &Clip::new("pinned", "h1", "text/plain", vec![])).unwrap();
        crate::clips::insert(&conn, &Clip::new("unpinned", "h2", "text/plain", vec![])).unwrap();
        crate::clips::set_pinned(&conn, "pinned", true).unwrap();
        clear(&conn, ClearScope::All).unwrap();
        assert!(crate::clips::get(&conn, "pinned").unwrap().is_none());
        assert!(crate::clips::get(&conn, "unpinned").unwrap().is_none());
    }

    #[test]
    fn clearing_with_excluding_pinned_scope_keeps_pinned_clips() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &Clip::new("pinned", "h1", "text/plain", vec![])).unwrap();
        crate::clips::insert(&conn, &Clip::new("unpinned", "h2", "text/plain", vec![])).unwrap();
        crate::clips::set_pinned(&conn, "pinned", true).unwrap();
        clear(&conn, ClearScope::ExcludingPinned).unwrap();
        assert!(crate::clips::get(&conn, "pinned").unwrap().is_some());
        assert!(crate::clips::get(&conn, "unpinned").unwrap().is_none());
    }

    #[test]
    fn clear_with_ids_returns_the_ids_of_removed_clips_only() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &Clip::new("pinned", "h1", "text/plain", vec![])).unwrap();
        crate::clips::insert(&conn, &Clip::new("unpinned", "h2", "text/plain", vec![])).unwrap();
        crate::clips::set_pinned(&conn, "pinned", true).unwrap();
        let removed = clear_with_ids(&conn, ClearScope::ExcludingPinned).unwrap();
        assert_eq!(removed, vec!["unpinned".to_string()]);
    }

    #[test]
    fn deleting_one_clip_does_not_affect_others() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &Clip::new("c1", "h1", "text/plain", vec![])).unwrap();
        crate::clips::insert(&conn, &Clip::new("c2", "h2", "text/plain", vec![])).unwrap();
        delete_clip(&conn, "c1").unwrap();
        let listed = crate::clips::list(&conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "c2");
    }
}
