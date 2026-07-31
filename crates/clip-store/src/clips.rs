//! Insert, update, delete, list, get.

use clip_core::models::{Clip, ClipRepresentation, PasteMode};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::StoreError;

pub(crate) fn to_rfc3339(dt: time::OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339).expect("valid datetime")
}

pub(crate) fn from_rfc3339(s: &str) -> time::OffsetDateTime {
    time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .expect("stored timestamps are always valid RFC3339")
}

fn paste_mode_to_str(mode: PasteMode) -> String {
    serde_json::to_value(mode).unwrap().as_str().unwrap().to_string()
}

fn paste_mode_from_str(s: &str) -> PasteMode {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap()
}

fn metadata_to_string(value: &Option<serde_json::Value>) -> Option<String> {
    value.as_ref().map(|v| v.to_string())
}

fn metadata_from_string(value: Option<String>) -> Option<serde_json::Value> {
    value.and_then(|s| serde_json::from_str(&s).ok())
}

fn is_unique_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(ffi_err, _) if ffi_err.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

pub(crate) fn row_to_clip(row: &Row) -> rusqlite::Result<Clip> {
    let paste_mode_str: String = row.get(13)?;
    let metadata_str: Option<String> = row.get(14)?;
    Ok(Clip {
        id: row.get(0)?,
        created_at: from_rfc3339(&row.get::<_, String>(1)?),
        updated_at: from_rfc3339(&row.get::<_, String>(2)?),
        last_used_at: row.get::<_, Option<String>>(3)?.map(|s| from_rfc3339(&s)),
        source_app: row.get(4)?,
        source_window: row.get(5)?,
        primary_mime: row.get(6)?,
        display_text: row.get(7)?,
        content_hash: row.get(8)?,
        byte_size: row.get::<_, i64>(9)? as u64,
        is_favorite: row.get::<_, i64>(10)? != 0,
        is_pinned: row.get::<_, i64>(11)? != 0,
        is_deleted: row.get::<_, i64>(12)? != 0,
        paste_mode_default: paste_mode_from_str(&paste_mode_str),
        metadata: metadata_from_string(metadata_str),
        representations: vec![],
    })
}

pub(crate) const CLIP_COLUMNS: &str = "id, created_at, updated_at, last_used_at, source_app, source_window, \
     primary_mime, display_text, content_hash, byte_size, is_favorite, is_pinned, is_deleted, \
     paste_mode_default, metadata_json";

pub(crate) fn get_representations(conn: &Connection, clip_id: &str) -> Result<Vec<ClipRepresentation>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT mime_type, text_value, blob_path, preview_text, width, height, byte_size, ordinal, is_preview \
         FROM clip_representations WHERE clip_id = ?1 ORDER BY ordinal",
    )?;
    let reprs = stmt
        .query_map([clip_id], |row| {
            Ok(ClipRepresentation {
                mime_type: row.get(0)?,
                text_value: row.get(1)?,
                blob_path: row.get(2)?,
                preview_text: row.get(3)?,
                width: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
                height: row.get::<_, Option<i64>>(5)?.map(|v| v as u32),
                byte_size: row.get::<_, i64>(6)? as u64,
                ordinal: row.get(7)?,
                is_preview: row.get::<_, i64>(8)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(reprs)
}

/// Inserts a clip and its representations. Fails with `StoreError::DedupConflict`
/// if a non-deleted clip with the same content hash + MIME already exists.
pub fn insert(conn: &Connection, clip: &Clip) -> Result<(), StoreError> {
    let tx = conn.unchecked_transaction()?;
    let result = tx.execute(
        &format!("INSERT INTO clips ({CLIP_COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)"),
        params![
            clip.id,
            to_rfc3339(clip.created_at),
            to_rfc3339(clip.updated_at),
            clip.last_used_at.map(to_rfc3339),
            clip.source_app,
            clip.source_window,
            clip.primary_mime,
            clip.display_text,
            clip.content_hash,
            clip.byte_size as i64,
            clip.is_favorite as i64,
            clip.is_pinned as i64,
            clip.is_deleted as i64,
            paste_mode_to_str(clip.paste_mode_default),
            metadata_to_string(&clip.metadata),
        ],
    );
    if let Err(e) = result {
        return Err(if is_unique_violation(&e) { StoreError::DedupConflict } else { StoreError::Sqlite(e) });
    }
    for repr in &clip.representations {
        tx.execute(
            "INSERT INTO clip_representations (clip_id, mime_type, text_value, blob_path, preview_text, width, height, byte_size, ordinal, is_preview) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                clip.id,
                repr.mime_type,
                repr.text_value,
                repr.blob_path,
                repr.preview_text,
                repr.width,
                repr.height,
                repr.byte_size as i64,
                repr.ordinal,
                repr.is_preview as i64,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Fetches a single clip (deleted or not) by id, with its representations.
pub fn get(conn: &Connection, id: &str) -> Result<Option<Clip>, StoreError> {
    let clip = conn
        .query_row(&format!("SELECT {CLIP_COLUMNS} FROM clips WHERE id = ?1"), [id], row_to_clip)
        .optional()?;
    let Some(mut clip) = clip else {
        return Ok(None);
    };
    clip.representations = get_representations(conn, id)?;
    Ok(Some(clip))
}

/// Lists non-deleted clips ordered by `created_at DESC`.
pub fn list(conn: &Connection) -> Result<Vec<Clip>, StoreError> {
    let mut stmt =
        conn.prepare(&format!("SELECT {CLIP_COLUMNS} FROM clips WHERE is_deleted = 0 ORDER BY created_at DESC"))?;
    let clips = stmt.query_map([], row_to_clip)?.collect::<Result<Vec<_>, _>>()?;
    let mut result = Vec::with_capacity(clips.len());
    for mut clip in clips {
        clip.representations = get_representations(conn, &clip.id)?;
        result.push(clip);
    }
    Ok(result)
}

/// Updates only a clip's pinned flag (and `updated_at`).
pub fn set_pinned(conn: &Connection, id: &str, pinned: bool) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE clips SET is_pinned = ?1, updated_at = ?2 WHERE id = ?3",
        params![pinned as i64, to_rfc3339(time::OffsetDateTime::now_utc()), id],
    )?;
    Ok(())
}

/// Fetches the active (non-deleted) clip with the given content hash and
/// MIME type, if one exists - the counterpart lookup for a dedup conflict.
pub fn get_by_hash(conn: &Connection, content_hash: &str, primary_mime: &str) -> Result<Option<Clip>, StoreError> {
    let clip = conn
        .query_row(
            &format!("SELECT {CLIP_COLUMNS} FROM clips WHERE content_hash = ?1 AND primary_mime = ?2 AND is_deleted = 0"),
            params![content_hash, primary_mime],
            row_to_clip,
        )
        .optional()?;
    let Some(mut clip) = clip else {
        return Ok(None);
    };
    clip.representations = get_representations(conn, &clip.id)?;
    Ok(Some(clip))
}

/// Updates a clip's `last_used_at` to the current time (e.g. on paste, or
/// when a dedup conflict means an existing clip was "re-copied").
pub fn touch_last_used(conn: &Connection, id: &str) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE clips SET last_used_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![to_rfc3339(time::OffsetDateTime::now_utc()), id],
    )?;
    Ok(())
}

/// Soft-deletes a clip by setting `is_deleted = 1`.
pub fn soft_delete(conn: &Connection, id: &str) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE clips SET is_deleted = 1, updated_at = ?1 WHERE id = ?2",
        params![to_rfc3339(time::OffsetDateTime::now_utc()), id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clip_core::models::{Clip, ClipRepresentation};

    fn new_clip(id: &str, hash: &str, mime: &str) -> Clip {
        Clip::new(id, hash, mime, vec![])
    }

    #[test]
    fn inserting_a_duplicate_clip_is_rejected() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &new_clip("c1", "abc", "text/plain")).unwrap();
        let result = insert(&conn, &new_clip("c2", "abc", "text/plain"));
        assert!(matches!(result, Err(crate::StoreError::DedupConflict)));
    }

    #[test]
    fn reinsertion_after_soft_delete_succeeds() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &new_clip("c1", "abc", "text/plain")).unwrap();
        soft_delete(&conn, "c1").unwrap();
        let result = insert(&conn, &new_clip("c2", "abc", "text/plain"));
        assert!(result.is_ok());
    }

    #[test]
    fn get_returns_a_previously_inserted_clip() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &new_clip("c1", "abc", "text/plain")).unwrap();
        let fetched = get(&conn, "c1").unwrap().unwrap();
        assert_eq!(fetched.id, "c1");
        assert_eq!(fetched.content_hash, "abc");
        assert_eq!(fetched.primary_mime, "text/plain");
    }

    #[test]
    fn soft_delete_hides_the_clip_from_default_listing() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &new_clip("c1", "abc", "text/plain")).unwrap();
        soft_delete(&conn, "c1").unwrap();
        let listed = list(&conn).unwrap();
        assert!(listed.is_empty());
        assert!(get(&conn, "c1").unwrap().is_some());
    }

    #[test]
    fn get_by_hash_finds_the_active_clip_with_that_content_hash_and_mime() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &new_clip("c1", "abc", "text/plain")).unwrap();
        let fetched = get_by_hash(&conn, "abc", "text/plain").unwrap().unwrap();
        assert_eq!(fetched.id, "c1");
    }

    #[test]
    fn get_by_hash_ignores_soft_deleted_clips() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &new_clip("c1", "abc", "text/plain")).unwrap();
        soft_delete(&conn, "c1").unwrap();
        assert!(get_by_hash(&conn, "abc", "text/plain").unwrap().is_none());
    }

    #[test]
    fn touching_last_used_sets_it_to_a_recent_time() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &new_clip("c1", "abc", "text/plain")).unwrap();
        touch_last_used(&conn, "c1").unwrap();
        let fetched = get(&conn, "c1").unwrap().unwrap();
        let last_used = fetched.last_used_at.expect("last_used_at should be set");
        assert!(time::OffsetDateTime::now_utc() - last_used < time::Duration::seconds(5));
    }

    #[test]
    fn pinning_a_clip_updates_only_the_pin_flag() {
        let conn = crate::db::open(":memory:").unwrap();
        insert(&conn, &new_clip("c1", "abc", "text/plain")).unwrap();
        set_pinned(&conn, "c1", true).unwrap();
        let fetched = get(&conn, "c1").unwrap().unwrap();
        assert!(fetched.is_pinned);
        assert_eq!(fetched.content_hash, "abc");
    }

    #[test]
    fn two_representations_round_trip_in_order() {
        let conn = crate::db::open(":memory:").unwrap();
        let text = ClipRepresentation::new("text/plain", 0).with_text_value("hi");
        let html = ClipRepresentation::new("text/html", 1).with_text_value("<b>hi</b>");
        let clip = Clip::new("c1", "abc", "text/plain", vec![text, html]);
        insert(&conn, &clip).unwrap();
        let fetched = get(&conn, "c1").unwrap().unwrap();
        assert_eq!(fetched.representations.len(), 2);
        assert_eq!(fetched.representations[0].mime_type, "text/plain");
        assert_eq!(fetched.representations[1].mime_type, "text/html");
    }
}
