//! Structured, filterable logging for capture/ingest/command handling.

/// Installs the global `tracing` subscriber, honoring `RUST_LOG` (or the
/// crate's default filter if unset) so verbosity is adjustable without a
/// recompile.
pub fn init_subscriber() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "clipd=info".into()))
        .init();
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Layer;

    #[derive(Debug, Default, Clone)]
    struct CapturedEvent {
        message: String,
        fields: HashMap<String, String>,
    }

    struct FieldVisitor<'a> {
        event: &'a mut CapturedEvent,
    }

    impl Visit for FieldVisitor<'_> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            let value_str = format!("{value:?}").trim_matches('"').to_string();
            if field.name() == "message" {
                self.event.message = value_str;
            } else {
                self.event.fields.insert(field.name().to_string(), value_str);
            }
        }
    }

    #[derive(Clone, Default)]
    struct RecordingLayer {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    impl<S: tracing::Subscriber> Layer<S> for RecordingLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
            let mut captured = CapturedEvent::default();
            event.record(&mut FieldVisitor { event: &mut captured });
            self.events.lock().unwrap().push(captured);
        }
    }

    fn capture_with_filter<F: FnOnce()>(filter: &str, f: F) -> Vec<CapturedEvent> {
        let layer = RecordingLayer::default();
        let events = layer.events.clone();
        let filter = tracing_subscriber::EnvFilter::new(filter);
        let subscriber = tracing_subscriber::registry().with(filter).with(layer);
        tracing::subscriber::with_default(subscriber, f);
        let events = events.lock().unwrap().clone();
        events
    }

    fn ingest_one(
        store: &dyn crate::app::Store,
        events: &dyn crate::app::EventPublisher,
        mime: &str,
        text: &str,
        source: Option<clip_core::models::AppContext>,
    ) {
        let snapshot = clip_platform::clipboard::ClipboardSnapshot {
            representations: vec![clip_core::models::ClipRepresentation::new(mime, 0).with_text_value(text)],
        };
        let _ = crate::ingest::ingest(store, events, snapshot, source);
    }

    #[test]
    fn an_ingested_clips_log_event_carries_its_clip_id() {
        let store = crate::app::fakes::FakeStore::new();
        let event_publisher = crate::app::fakes::FakeEventPublisher::new();

        let captured = capture_with_filter("clipd=info", || {
            ingest_one(&store, &event_publisher, "text/plain", "hello", None);
        });

        assert!(captured.iter().any(|e| e.fields.contains_key("clip_id")), "expected a log event carrying clip_id");
    }

    #[test]
    fn an_excluded_captures_log_event_indicates_exclusion_not_silence() {
        let store = crate::app::fakes::FakeStore::new();
        let event_publisher = crate::app::fakes::FakeEventPublisher::new();
        crate::app::Store::save_rule(
            &store,
            &clip_core::models::Rule::new("r1", "1Password", None, None, clip_core::models::RuleAction::Exclude),
        )
        .unwrap();

        let captured = capture_with_filter("clipd=info", || {
            ingest_one(&store, &event_publisher, "text/plain", "secret", Some(clip_core::models::AppContext::new("1Password")));
        });

        assert!(
            captured.iter().any(|e| e.message.to_lowercase().contains("exclud")),
            "expected a log event explicitly indicating exclusion"
        );
    }

    #[test]
    fn a_debug_filter_directive_surfaces_debug_level_ingest_logs() {
        let store = crate::app::fakes::FakeStore::new();
        let event_publisher = crate::app::fakes::FakeEventPublisher::new();

        let captured = capture_with_filter("clipd::ingest=debug", || {
            ingest_one(&store, &event_publisher, "text/plain", "hello", None);
        });

        assert!(
            captured.iter().any(|e| e.message.to_lowercase().contains("normalizing")),
            "expected the debug-level normalization event to be captured under a debug filter"
        );
    }

    #[test]
    fn the_default_filter_suppresses_debug_level_ingest_logs() {
        let store = crate::app::fakes::FakeStore::new();
        let event_publisher = crate::app::fakes::FakeEventPublisher::new();

        let captured = capture_with_filter("clipd::ingest=info", || {
            ingest_one(&store, &event_publisher, "text/plain", "hello", None);
        });

        assert!(
            !captured.iter().any(|e| e.message.to_lowercase().contains("normalizing")),
            "debug-level events should be suppressed at the info level"
        );
    }
}
