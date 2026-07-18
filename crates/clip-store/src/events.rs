//! Audit/event log.

use rusqlite::{params, Connection};

use crate::StoreError;

#[derive(Debug, Clone, PartialEq)]
pub struct StoredEvent {
    pub id: i64,
    pub clip_id: Option<String>,
    pub event_type: String,
    pub payload: Option<serde_json::Value>,
    pub created_at: time::OffsetDateTime,
}

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<StoredEvent> {
    let payload_str: Option<String> = row.get(3)?;
    let created_at: String = row.get(4)?;
    Ok(StoredEvent {
        id: row.get(0)?,
        clip_id: row.get(1)?,
        event_type: row.get(2)?,
        payload: payload_str.and_then(|s| serde_json::from_str(&s).ok()),
        created_at: crate::clips::from_rfc3339(&created_at),
    })
}

const EVENT_COLUMNS: &str = "id, clip_id, event_type, payload_json, created_at";

/// Records an event, optionally tied to a clip.
pub fn record(
    conn: &Connection,
    clip_id: Option<&str>,
    event_type: &str,
    payload: Option<serde_json::Value>,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO events (clip_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![
            clip_id,
            event_type,
            payload.map(|p| p.to_string()),
            crate::clips::to_rfc3339(time::OffsetDateTime::now_utc()),
        ],
    )?;
    Ok(())
}

/// Lists events for a given clip, newest first.
pub fn list_for_clip(conn: &Connection, clip_id: &str) -> Result<Vec<StoredEvent>, StoreError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {EVENT_COLUMNS} FROM events WHERE clip_id = ?1 ORDER BY created_at DESC, id DESC"
    ))?;
    let events = stmt.query_map([clip_id], row_to_event)?.collect::<Result<Vec<_>, _>>()?;
    Ok(events)
}

/// Lists events of a given type, newest first.
pub fn list_by_type(conn: &Connection, event_type: &str) -> Result<Vec<StoredEvent>, StoreError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {EVENT_COLUMNS} FROM events WHERE event_type = ?1 ORDER BY created_at DESC, id DESC"
    ))?;
    let events = stmt.query_map([event_type], row_to_event)?.collect::<Result<Vec<_>, _>>()?;
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_clip(conn: &rusqlite::Connection, id: &str) {
        let clip = clip_core::models::Clip::new(id, format!("hash-{id}"), "text/plain", vec![]);
        crate::clips::insert(conn, &clip).unwrap();
    }

    #[test]
    fn recording_an_event_tied_to_a_clip() {
        let conn = crate::db::open(":memory:").unwrap();
        insert_clip(&conn, "c1");
        record(&conn, Some("c1"), "ClipCaptured", None).unwrap();
        let events = list_for_clip(&conn, "c1").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "ClipCaptured");
    }

    #[test]
    fn recording_an_event_with_no_clip_association() {
        let conn = crate::db::open(":memory:").unwrap();
        record(&conn, None, "CapturePaused", None).unwrap();
        let events = list_by_type(&conn, "CapturePaused").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].clip_id, None);
    }

    #[test]
    fn listing_events_for_a_clip_returns_only_its_events_newest_first() {
        let conn = crate::db::open(":memory:").unwrap();
        insert_clip(&conn, "c1");
        insert_clip(&conn, "c2");
        record(&conn, Some("c1"), "ClipCaptured", None).unwrap();
        record(&conn, Some("c1"), "ClipUpdated", None).unwrap();
        record(&conn, Some("c2"), "ClipCaptured", None).unwrap();
        let events = list_for_clip(&conn, "c1").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "ClipUpdated");
        assert_eq!(events[1].event_type, "ClipCaptured");
    }

    #[test]
    fn listing_events_by_type_returns_only_matching_events() {
        let conn = crate::db::open(":memory:").unwrap();
        insert_clip(&conn, "c1");
        record(&conn, Some("c1"), "ClipCaptured", None).unwrap();
        record(&conn, Some("c1"), "ClipDeleted", None).unwrap();
        let deleted_events = list_by_type(&conn, "ClipDeleted").unwrap();
        assert_eq!(deleted_events.len(), 1);
        assert_eq!(deleted_events[0].event_type, "ClipDeleted");
    }

    #[test]
    fn hard_deleting_a_clip_nulls_out_its_events_clip_id() {
        let conn = crate::db::open(":memory:").unwrap();
        insert_clip(&conn, "c1");
        record(&conn, Some("c1"), "ClipCaptured", None).unwrap();
        conn.execute("DELETE FROM clips WHERE id = 'c1'", []).unwrap();
        let events = list_by_type(&conn, "ClipCaptured").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].clip_id, None);
    }
}
