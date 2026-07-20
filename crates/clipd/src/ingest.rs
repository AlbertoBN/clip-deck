//! Normalize, rule-filter, dedup, persist.

use crate::app::{EventPublisher, Store};
use clip_core::models::{AppContext, Clip, RuleAction};
use clip_ipc::protocol::Event;
use clip_platform::clipboard::ClipboardSnapshot;

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error(transparent)]
    Store(#[from] clip_store::StoreError),
    #[error(transparent)]
    Core(#[from] clip_core::errors::CoreError),
}

/// Normalizes, rule-filters, dedups, and persists a captured clipboard
/// snapshot. `source` is the app that most likely owned the clipboard at
/// capture time (best-effort; queried via the backend's `focused_app` at or
/// near the moment of capture).
pub fn ingest(
    store: &dyn Store,
    events: &dyn EventPublisher,
    snapshot: ClipboardSnapshot,
    source: Option<AppContext>,
) -> Result<(), IngestError> {
    if snapshot.representations.is_empty() {
        return Ok(());
    }

    let mut representations = Vec::with_capacity(snapshot.representations.len());
    for (ordinal, mut repr) in snapshot.representations.into_iter().enumerate() {
        let normalized = clip_core::mime::normalize_mime(&repr.mime_type)?;
        tracing::debug!(from = %repr.mime_type, to = %normalized, "normalizing representation mime type");
        repr.mime_type = normalized;
        repr.ordinal = ordinal as i64;
        representations.push(repr);
    }

    let rules = store.list_enabled_rules()?;
    if let Some(ctx) = &source {
        let excluded = representations
            .iter()
            .any(|repr| rules.iter().any(|rule| rule.action == RuleAction::Exclude && rule.matches(ctx, &repr.mime_type)));
        if excluded {
            tracing::info!(app = %ctx.app, "capture excluded by rule");
            return Ok(());
        }
    }

    let primary = &representations[0];
    let primary_text = primary.text_value.as_deref().unwrap_or("");
    let content_hash = clip_core::hashing::hash_content(primary_text.as_bytes());
    let primary_mime = primary.mime_type.clone();
    let display_text = primary.text_value.clone();
    let byte_size = representations.iter().map(|r| r.byte_size).sum();

    let clip_id = uuid::Uuid::new_v4().to_string();
    let mut clip = Clip::new(&clip_id, content_hash.clone(), primary_mime.clone(), representations);
    clip.display_text = display_text;
    clip.byte_size = byte_size;
    clip.source_app = source.as_ref().map(|c| c.app.clone());
    clip.source_window = source.as_ref().and_then(|c| c.window.clone());

    match store.insert_clip(&clip) {
        Ok(()) => {
            tracing::info!(clip_id = %clip.id, "clip ingested");
            events.publish(Event::ClipCaptured { clip_id: clip.id });
            Ok(())
        }
        Err(clip_store::StoreError::DedupConflict) => {
            if let Some(existing) = store.find_active_by_hash(&content_hash, &primary_mime)? {
                tracing::info!(clip_id = %existing.id, "duplicate capture reused existing clip");
                store.touch_last_used(&existing.id)?;
            }
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::app::fakes::{FakeEventPublisher, FakeStore};
    use crate::app::Store;
    use clip_core::models::{AppContext, ClipRepresentation, Rule, RuleAction};
    use clip_platform::clipboard::ClipboardSnapshot;

    fn snapshot_with(mime: &str, text: &str) -> ClipboardSnapshot {
        ClipboardSnapshot {
            representations: vec![ClipRepresentation::new(mime, 0).with_text_value(text).with_byte_size(text.len() as u64)],
        }
    }

    #[test]
    fn mixed_case_mime_type_is_normalized_before_storage() {
        let store = FakeStore::new();
        let events = FakeEventPublisher::new();

        super::ingest(&store, &events, snapshot_with("TEXT/PLAIN", "hello"), None).unwrap();

        let clips = store.search("", &Default::default()).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].primary_mime, "text/plain");
        assert_eq!(clips[0].representations[0].mime_type, "text/plain");
    }

    #[test]
    fn excluded_apps_copy_is_not_persisted() {
        let store = FakeStore::new();
        let events = FakeEventPublisher::new();
        store.save_rule(&Rule::new("r1", "1Password", None, None, RuleAction::Exclude)).unwrap();

        super::ingest(&store, &events, snapshot_with("text/plain", "secret"), Some(AppContext::new("1Password"))).unwrap();

        assert!(store.search("", &Default::default()).unwrap().is_empty());
        assert!(events.events().is_empty());
    }

    #[test]
    fn non_matching_apps_copy_persists_normally() {
        let store = FakeStore::new();
        let events = FakeEventPublisher::new();
        store.save_rule(&Rule::new("r1", "1Password", None, None, RuleAction::Exclude)).unwrap();

        super::ingest(&store, &events, snapshot_with("text/plain", "ssh cmd"), Some(AppContext::new("gnome-terminal"))).unwrap();

        assert_eq!(store.search("", &Default::default()).unwrap().len(), 1);
    }

    #[test]
    fn recopying_identical_content_updates_last_used_at_instead_of_erroring() {
        let store = FakeStore::new();
        let events = FakeEventPublisher::new();

        super::ingest(&store, &events, snapshot_with("text/plain", "same"), None).unwrap();
        let first = store.search("", &Default::default()).unwrap();
        assert_eq!(first.len(), 1);
        assert!(first[0].last_used_at.is_none());

        let result = super::ingest(&store, &events, snapshot_with("text/plain", "same"), None);

        assert!(result.is_ok());
        let after = store.search("", &Default::default()).unwrap();
        assert_eq!(after.len(), 1, "no duplicate clip should be created");
        assert!(after[0].last_used_at.is_some());
    }

    #[test]
    fn plain_text_and_html_from_one_copy_become_one_clip() {
        let store = FakeStore::new();
        let events = FakeEventPublisher::new();
        let snapshot = ClipboardSnapshot {
            representations: vec![
                ClipRepresentation::new("text/plain", 0).with_text_value("hi"),
                ClipRepresentation::new("text/html", 1).with_text_value("<b>hi</b>"),
            ],
        };

        super::ingest(&store, &events, snapshot, None).unwrap();

        let clips = store.search("", &Default::default()).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].representations.len(), 2);
    }

    #[test]
    fn persisting_a_new_clip_publishes_clip_captured() {
        let store = FakeStore::new();
        let events = FakeEventPublisher::new();

        super::ingest(&store, &events, snapshot_with("text/plain", "new content"), None).unwrap();

        let clip_id = store.search("", &Default::default()).unwrap()[0].id.clone();
        assert_eq!(events.events(), vec![clip_ipc::protocol::Event::ClipCaptured { clip_id }]);
    }
}
