//! FTS5 synchronization and search queries.
//!
//! FTS sync itself happens via the `clips_ai`/`clips_au`/`clips_ad` triggers defined in
//! `migrations/001_init.sql`; this module only builds and runs search queries against
//! the resulting `clips_fts` index.

use clip_core::models::Clip;
use clip_core::search::{parse_query, ParsedQuery, SearchFilters};
use rusqlite::{params, Connection};

use crate::clips::{get_representations, row_to_clip, CLIP_COLUMNS};
use crate::StoreError;

fn build_match_query(parsed: &ParsedQuery) -> String {
    parsed
        .terms
        .iter()
        .map(|t| if t.is_prefix { format!("{}*", t.text) } else { t.text.clone() })
        .collect::<Vec<_>>()
        .join(" ")
}

fn matches_filters(clip: &Clip, filters: &SearchFilters) -> bool {
    if filters.pinned_only && !clip.is_pinned {
        return false;
    }
    if filters.favorite_only && !clip.is_favorite {
        return false;
    }
    if let Some(group_id) = &filters.group_id {
        if clip.group_id.as_deref() != Some(group_id.as_str()) {
            return false;
        }
    }
    if let Some(source_app) = &filters.source_app {
        if clip.source_app.as_deref() != Some(source_app.as_str()) {
            return false;
        }
    }
    if let Some(family) = filters.mime_family {
        match clip_core::mime::mime_family(&clip.primary_mime) {
            Ok(f) if f == family => {}
            _ => return false,
        }
    }
    true
}

/// Searches clips by free-text query (empty falls back to pinned-first, then
/// recency), narrowing by `filters`.
pub fn search(conn: &Connection, raw_query: &str, filters: &SearchFilters) -> Result<Vec<Clip>, StoreError> {
    let parsed = parse_query(raw_query);
    let mut clips = if parsed.is_empty() {
        let mut stmt = conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS} FROM clips WHERE is_deleted = 0 ORDER BY is_pinned DESC, created_at DESC"
        ))?;
        let rows = stmt.query_map([], row_to_clip)?.collect::<Result<Vec<_>, _>>()?;
        rows
    } else {
        let match_query = build_match_query(&parsed);
        let sql = format!(
            "SELECT {cols} FROM clips c JOIN clips_fts ON clips_fts.clip_id = c.id \
             WHERE clips_fts MATCH ?1 AND c.is_deleted = 0 \
             ORDER BY c.is_pinned DESC, bm25(clips_fts) ASC",
            cols = CLIP_COLUMNS
                .split(", ")
                .map(|c| format!("c.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![match_query], row_to_clip)?.collect::<Result<Vec<_>, _>>()?;
        rows
    };

    for clip in clips.iter_mut() {
        clip.representations = get_representations(conn, &clip.id)?;
    }

    clips.retain(|c| matches_filters(c, filters));
    Ok(clips)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clip_core::models::Clip;
    use clip_core::search::SearchFilters;

    fn clip_with_text(id: &str, hash: &str, text: &str) -> Clip {
        let mut clip = Clip::new(id, hash, "text/plain", vec![]);
        clip.display_text = Some(text.to_string());
        clip
    }

    #[test]
    fn inserting_a_clip_makes_it_searchable() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_with_text("c1", "h1", "deploy staging via ssh")).unwrap();
        let results = search(&conn, "deploy", &SearchFilters::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "c1");
    }

    #[test]
    fn updating_display_text_updates_search_results() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_with_text("c1", "h1", "foo")).unwrap();
        conn.execute("UPDATE clips SET display_text = 'bar' WHERE id = 'c1'", []).unwrap();
        assert!(search(&conn, "foo", &SearchFilters::default()).unwrap().is_empty());
        assert_eq!(search(&conn, "bar", &SearchFilters::default()).unwrap().len(), 1);
    }

    #[test]
    fn deleting_a_clip_removes_it_from_search_results() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_with_text("c1", "h1", "deploy staging")).unwrap();
        crate::clips::soft_delete(&conn, "c1").unwrap();
        assert!(search(&conn, "deploy", &SearchFilters::default()).unwrap().is_empty());
    }

    #[test]
    fn prefix_of_a_word_matches_the_full_word() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_with_text("c1", "h1", "deploy staging")).unwrap();
        let results = search(&conn, "depl", &SearchFilters::default()).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn empty_query_returns_pinned_clips_before_unpinned_ones() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_with_text("old", "h1", "one")).unwrap();
        crate::clips::insert(&conn, &clip_with_text("new", "h2", "two")).unwrap();
        crate::clips::set_pinned(&conn, "old", true).unwrap();
        let results = search(&conn, "", &SearchFilters::default()).unwrap();
        assert_eq!(results[0].id, "old");
    }

    #[test]
    fn empty_query_orders_unpinned_clips_by_recency() {
        let conn = crate::db::open(":memory:").unwrap();
        let mut earlier = clip_with_text("earlier", "h1", "one");
        earlier.created_at = time::OffsetDateTime::now_utc() - time::Duration::minutes(5);
        earlier.updated_at = earlier.created_at;
        let later = clip_with_text("later", "h2", "two");
        crate::clips::insert(&conn, &earlier).unwrap();
        crate::clips::insert(&conn, &later).unwrap();
        let results = search(&conn, "", &SearchFilters::default()).unwrap();
        assert_eq!(results[0].id, "later");
    }

    #[test]
    fn filtering_by_group_excludes_clips_outside_that_group() {
        let conn = crate::db::open(":memory:").unwrap();
        conn.execute(
            "INSERT INTO groups (id, name, created_at) VALUES ('g1', 'Work', '2024-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let mut in_group = clip_with_text("c1", "h1", "one");
        in_group.group_id = Some("g1".to_string());
        crate::clips::insert(&conn, &in_group).unwrap();
        crate::clips::insert(&conn, &clip_with_text("c2", "h2", "two")).unwrap();
        let filters = SearchFilters { group_id: Some("g1".to_string()), ..Default::default() };
        let results = search(&conn, "", &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "c1");
    }

    #[test]
    fn filtering_by_pinned_only_excludes_unpinned_clips() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_with_text("c1", "h1", "one")).unwrap();
        crate::clips::insert(&conn, &clip_with_text("c2", "h2", "two")).unwrap();
        crate::clips::set_pinned(&conn, "c1", true).unwrap();
        let filters = SearchFilters { pinned_only: true, ..Default::default() };
        let results = search(&conn, "", &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "c1");
    }

    #[test]
    fn pinned_clip_ranks_above_an_equally_relevant_unpinned_clip() {
        let conn = crate::db::open(":memory:").unwrap();
        crate::clips::insert(&conn, &clip_with_text("unpinned", "h1", "deploy staging")).unwrap();
        crate::clips::insert(&conn, &clip_with_text("pinned", "h2", "deploy staging")).unwrap();
        crate::clips::set_pinned(&conn, "pinned", true).unwrap();
        let results = search(&conn, "deploy", &SearchFilters::default()).unwrap();
        assert_eq!(results[0].id, "pinned");
    }
}
