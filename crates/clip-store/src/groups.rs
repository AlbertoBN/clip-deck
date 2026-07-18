//! CRUD and hierarchy.

use clip_core::models::Group;
use rusqlite::{params, Connection, OptionalExtension};

use crate::StoreError;

fn row_to_group(row: &rusqlite::Row) -> rusqlite::Result<Group> {
    Ok(Group {
        id: row.get(0)?,
        name: row.get(1)?,
        parent_group_id: row.get(2)?,
        sort_order: row.get(3)?,
    })
}

/// Inserts a new group.
pub fn insert(conn: &Connection, group: &Group) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO groups (id, name, parent_group_id, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            group.id,
            group.name,
            group.parent_group_id,
            group.sort_order,
            crate::clips::to_rfc3339(time::OffsetDateTime::now_utc()),
        ],
    )?;
    Ok(())
}

/// Fetches a single group by id.
pub fn get(conn: &Connection, id: &str) -> Result<Option<Group>, StoreError> {
    let group = conn
        .query_row(
            "SELECT id, name, parent_group_id, sort_order FROM groups WHERE id = ?1",
            [id],
            row_to_group,
        )
        .optional()?;
    Ok(group)
}

/// Renames a group.
pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<(), StoreError> {
    conn.execute("UPDATE groups SET name = ?1 WHERE id = ?2", params![name, id])?;
    Ok(())
}

/// Lists the direct children of `parent_id`, or top-level groups when `None`.
pub fn list_children(conn: &Connection, parent_id: Option<&str>) -> Result<Vec<Group>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, parent_group_id, sort_order FROM groups WHERE parent_group_id IS ?1",
    )?;
    let groups = stmt.query_map(params![parent_id], row_to_group)?.collect::<Result<Vec<_>, _>>()?;
    Ok(groups)
}

/// Deletes a group. Child groups cascade-delete; clips referencing this group
/// (or its deleted descendants) have their `group_id` set to `NULL` by the
/// database's foreign-key actions.
pub fn delete(conn: &Connection, id: &str) -> Result<(), StoreError> {
    conn.execute("DELETE FROM groups WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clip_core::models::Group;

    #[test]
    fn created_group_is_fetchable_by_id() {
        let conn = crate::db::open(":memory:").unwrap();
        let group = Group::new("g1", "SSH commands", None).unwrap();
        insert(&conn, &group).unwrap();
        let fetched = get(&conn, "g1").unwrap().unwrap();
        assert_eq!(fetched.name, "SSH commands");
    }

    #[test]
    fn renaming_a_group_updates_its_name_only() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &Group::new("g1", "SSH commands", None).unwrap()).unwrap();
        rename(&conn, "g1", "Ops snippets").unwrap();
        let fetched = get(&conn, "g1").unwrap().unwrap();
        assert_eq!(fetched.name, "Ops snippets");
        assert_eq!(fetched.id, "g1");
    }

    #[test]
    fn listing_children_returns_only_direct_children() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &Group::new("work", "Work", None).unwrap()).unwrap();
        insert(&conn, &Group::new("ssh", "SSH", Some("work".to_string())).unwrap()).unwrap();
        insert(&conn, &Group::new("sql", "SQL", Some("work".to_string())).unwrap()).unwrap();
        insert(&conn, &Group::new("prod", "Prod", Some("ssh".to_string())).unwrap()).unwrap();
        let children = list_children(&conn, Some("work")).unwrap();
        let ids: Vec<_> = children.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"ssh"));
        assert!(ids.contains(&"sql"));
        assert!(!ids.contains(&"prod"));
    }

    #[test]
    fn listing_top_level_groups_excludes_nested_groups() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &Group::new("work", "Work", None).unwrap()).unwrap();
        insert(&conn, &Group::new("ssh", "SSH", Some("work".to_string())).unwrap()).unwrap();
        let top_level = list_children(&conn, None).unwrap();
        let ids: Vec<_> = top_level.iter().map(|g| g.id.as_str()).collect();
        assert!(ids.contains(&"work"));
        assert!(!ids.contains(&"ssh"));
    }

    #[test]
    fn deleting_a_parent_group_deletes_its_child_group() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &Group::new("work", "Work", None).unwrap()).unwrap();
        insert(&conn, &Group::new("ssh", "SSH", Some("work".to_string())).unwrap()).unwrap();
        delete(&conn, "work").unwrap();
        assert!(get(&conn, "ssh").unwrap().is_none());
    }

    #[test]
    fn deleting_a_group_detaches_its_clips_instead_of_deleting_them() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &Group::new("work", "Work", None).unwrap()).unwrap();
        let mut clip = clip_core::models::Clip::new("c1", "h1", "text/plain", vec![]);
        clip.group_id = Some("work".to_string());
        crate::clips::insert(&conn, &clip).unwrap();
        delete(&conn, "work").unwrap();
        let fetched = crate::clips::get(&conn, "c1").unwrap().unwrap();
        assert_eq!(fetched.group_id, None);
    }
}
