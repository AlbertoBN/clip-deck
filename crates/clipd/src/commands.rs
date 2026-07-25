//! IPC command handlers.

use std::sync::Arc;

use crate::app::{Backend, EventPublisher, Store};
use crate::watch_loop::WatchLoop;
use clip_ipc::protocol::{ClearScope as IpcClearScope, Command, Event};
use clip_store::retention::ClearScope as StoreClearScope;

fn to_store_scope(scope: IpcClearScope) -> StoreClearScope {
    match scope {
        IpcClearScope::All => StoreClearScope::All,
        IpcClearScope::ExcludingPinned => StoreClearScope::ExcludingPinned,
    }
}

/// Translates each IPC `Command` into `clip-store`/`clip-platform` calls and
/// the correct response payload/event, per the PRD's IPC contract.
pub struct CommandHandler {
    store: Arc<dyn Store>,
    backend: Arc<dyn Backend>,
    events: Arc<dyn EventPublisher>,
    watch_loop: Arc<WatchLoop>,
    backend_name: String,
}

impl CommandHandler {
    pub fn new(
        store: Arc<dyn Store>,
        backend: Arc<dyn Backend>,
        events: Arc<dyn EventPublisher>,
        watch_loop: Arc<WatchLoop>,
        backend_name: String,
    ) -> Self {
        Self { store, backend, events, watch_loop, backend_name }
    }

    pub fn handle(&self, command: Command) -> Result<serde_json::Value, String> {
        match command {
            Command::SearchClips { query, filters, .. } => {
                let clips = self.store.search(&query, &filters).map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(clips).expect("clips always serialize"))
            }
            Command::GetClip { id } => {
                let clip = self.store.get_clip(&id).map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(clip).expect("clip always serializes"))
            }
            Command::ListGroups => {
                let groups = self.store.list_groups().map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(groups).expect("groups always serialize"))
            }
            Command::ListRules => {
                let rules = self.store.list_rules().map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(rules).expect("rules always serialize"))
            }
            Command::GetSettings => {
                let settings = self.store.get_settings().map_err(|e| e.to_string())?;
                Ok(serde_json::to_value(settings).expect("settings always serialize"))
            }
            Command::GetDiagnostics => {
                let capabilities = self.backend.capabilities();
                Ok(serde_json::json!({ "backend": self.backend_name, "capabilities": capabilities }))
            }
            Command::PasteClip { id, mode } => {
                let clip = self.store.get_clip(&id).map_err(|e| e.to_string())?.ok_or("clip not found")?;
                self.backend.simulate_paste(&clip.representations, mode).map_err(|e| e.to_string())?;
                self.store.touch_last_used(&id).map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"ok": true}))
            }
            Command::PinClip { id, pinned } => {
                self.store.set_pinned(&id, pinned).map_err(|e| e.to_string())?;
                self.events.publish(Event::ClipUpdated { clip_id: id });
                Ok(serde_json::json!({"ok": true}))
            }
            Command::AssignGroup { id, group_id } => {
                self.store.set_group(&id, group_id.as_deref()).map_err(|e| e.to_string())?;
                self.events.publish(Event::ClipUpdated { clip_id: id });
                Ok(serde_json::json!({"ok": true}))
            }
            Command::DeleteClip { id } => {
                self.store.delete_clip(&id).map_err(|e| e.to_string())?;
                self.events.publish(Event::ClipDeleted { clip_id: id });
                Ok(serde_json::json!({"ok": true}))
            }
            Command::ClearHistory { scope } => {
                let removed = self.store.clear_history(to_store_scope(scope)).map_err(|e| e.to_string())?;
                for clip_id in removed {
                    self.events.publish(Event::ClipDeleted { clip_id });
                }
                Ok(serde_json::json!({"ok": true}))
            }
            Command::SaveRule { rule } => {
                self.store.save_rule(&rule).map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"ok": true}))
            }
            Command::DeleteRule { id } => {
                self.store.delete_rule(&id).map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"ok": true}))
            }
            Command::UpdateSettings { settings } => {
                clip_platform::hotkeys::parse_binding(&settings.hotkey_binding).map_err(|e| e.to_string())?;
                self.store.update_settings(&settings).map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"ok": true}))
            }
            Command::PauseCapture { paused } => {
                self.watch_loop.set_paused(paused);
                self.events.publish(Event::CapturePaused { paused });
                Ok(serde_json::json!({"ok": true}))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::fakes::{FakeBackend, FakeEventPublisher, FakeStore};
    use crate::app::Store;
    use crate::commands::CommandHandler;
    use crate::watch_loop::WatchLoop;
    use clip_core::models::{Clip, Group, PasteMode, Rule, RuleAction};
    use clip_ipc::protocol::{ClearScope, Command, Event};
    use std::sync::Arc;

    fn handler() -> (CommandHandler, Arc<FakeStore>, Arc<FakeBackend>, Arc<FakeEventPublisher>) {
        let store = Arc::new(FakeStore::new());
        let backend = Arc::new(FakeBackend::new());
        let events = Arc::new(FakeEventPublisher::new());
        let watch_loop = Arc::new(WatchLoop::new());
        let handler = CommandHandler::new(
            store.clone(),
            backend.clone(),
            events.clone(),
            watch_loop,
            "x11".to_string(),
        );
        (handler, store, backend, events)
    }

    fn clip_with_text(id: &str, text: &str) -> Clip {
        let mut clip = Clip::new(id, format!("hash-{id}"), "text/plain", vec![]);
        clip.display_text = Some(text.to_string());
        clip
    }

    #[test]
    fn search_clips_reflects_a_just_ingested_clip() {
        let (handler, store, _backend, _events) = handler();
        store.insert_clip(&clip_with_text("c1", "deploy staging")).unwrap();

        let response = handler
            .handle(Command::SearchClips { query: "deploy".to_string(), filters: Default::default(), limit: 20, offset: 0 })
            .unwrap();

        let clips: Vec<Clip> = serde_json::from_value(response).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].id, "c1");
    }

    #[test]
    fn get_clip_does_not_mutate_last_used_at() {
        let (handler, store, _backend, _events) = handler();
        store.insert_clip(&clip_with_text("c1", "hello")).unwrap();

        handler.handle(Command::GetClip { id: "c1".to_string() }).unwrap();

        let clip = store.get_clip("c1").unwrap().unwrap();
        assert!(clip.last_used_at.is_none());
    }

    #[test]
    fn successful_paste_updates_last_used_at() {
        let (handler, store, _backend, _events) = handler();
        store.insert_clip(&clip_with_text("c1", "hello")).unwrap();

        let response = handler.handle(Command::PasteClip { id: "c1".to_string(), mode: PasteMode::Auto }).unwrap();

        assert_eq!(response, serde_json::json!({"ok": true}));
        assert!(store.get_clip("c1").unwrap().unwrap().last_used_at.is_some());
    }

    #[test]
    fn failed_paste_does_not_update_last_used_at_and_returns_an_error() {
        let (handler, store, backend, _events) = handler();
        store.insert_clip(&clip_with_text("c1", "hello")).unwrap();
        backend.set_fail_paste(true);

        let result = handler.handle(Command::PasteClip { id: "c1".to_string(), mode: PasteMode::Auto });

        assert!(result.is_err());
        assert!(store.get_clip("c1").unwrap().unwrap().last_used_at.is_none());
    }

    #[test]
    fn pin_clip_publishes_clip_updated() {
        let (handler, store, _backend, events) = handler();
        store.insert_clip(&clip_with_text("c1", "hello")).unwrap();

        handler.handle(Command::PinClip { id: "c1".to_string(), pinned: true }).unwrap();

        assert!(store.get_clip("c1").unwrap().unwrap().is_pinned);
        assert_eq!(events.events(), vec![Event::ClipUpdated { clip_id: "c1".to_string() }]);
    }

    #[test]
    fn delete_clip_publishes_clip_deleted() {
        let (handler, store, _backend, events) = handler();
        store.insert_clip(&clip_with_text("c1", "hello")).unwrap();

        handler.handle(Command::DeleteClip { id: "c1".to_string() }).unwrap();

        assert!(store.get_clip("c1").unwrap().unwrap().is_deleted);
        assert_eq!(events.events(), vec![Event::ClipDeleted { clip_id: "c1".to_string() }]);
    }

    #[test]
    fn assign_group_publishes_clip_updated() {
        let (handler, store, _backend, events) = handler();
        store.insert_clip(&clip_with_text("c1", "hello")).unwrap();

        handler.handle(Command::AssignGroup { id: "c1".to_string(), group_id: Some("g1".to_string()) }).unwrap();

        assert_eq!(store.get_clip("c1").unwrap().unwrap().group_id, Some("g1".to_string()));
        assert_eq!(events.events(), vec![Event::ClipUpdated { clip_id: "c1".to_string() }]);
    }

    #[test]
    fn clear_history_excluding_pinned_leaves_pinned_clip_and_its_event_unpublished() {
        let (handler, store, _backend, events) = handler();
        let mut pinned = clip_with_text("pinned", "keep");
        pinned.is_pinned = true;
        store.insert_clip(&pinned).unwrap();
        store.insert_clip(&clip_with_text("unpinned", "drop")).unwrap();

        handler.handle(Command::ClearHistory { scope: ClearScope::ExcludingPinned }).unwrap();

        assert!(!store.get_clip("pinned").unwrap().unwrap().is_deleted);
        assert!(store.get_clip("unpinned").unwrap().unwrap().is_deleted);
        assert_eq!(events.events(), vec![Event::ClipDeleted { clip_id: "unpinned".to_string() }]);
    }

    #[test]
    fn a_newly_saved_exclusion_rule_applies_to_the_next_capture() {
        let (handler, store, _backend, _events) = handler();

        handler
            .handle(Command::SaveRule { rule: Rule::new("r1", "1Password", None, None, RuleAction::Exclude) })
            .unwrap();

        // SaveRule persisted via the same `store` the handler was built
        // with; the very next ingest against that store should see it, with
        // no daemon restart in between.
        let events = crate::app::fakes::FakeEventPublisher::new();
        crate::ingest::ingest(
            store.as_ref(),
            &events,
            clip_platform::clipboard::ClipboardSnapshot {
                representations: vec![clip_core::models::ClipRepresentation::new("text/plain", 0).with_text_value("secret")],
            },
            Some(clip_core::models::AppContext::new("1Password")),
        )
        .unwrap();

        assert!(store.search("", &Default::default()).unwrap().is_empty());
    }

    #[test]
    fn delete_rule_removes_it_from_enabled_rules() {
        let (handler, store, _backend, _events) = handler();
        handler
            .handle(Command::SaveRule { rule: Rule::new("r1", "1Password", None, None, RuleAction::Exclude) })
            .unwrap();

        handler.handle(Command::DeleteRule { id: "r1".to_string() }).unwrap();

        assert!(store.list_enabled_rules().unwrap().is_empty());
    }

    #[test]
    fn pause_capture_publishes_capture_paused_and_stops_the_watch_loop() {
        let store = Arc::new(FakeStore::new());
        let backend = Arc::new(FakeBackend::new());
        let events = Arc::new(FakeEventPublisher::new());
        let watch_loop = Arc::new(WatchLoop::new());
        watch_loop.start(backend.clone(), store.clone(), events.clone()).unwrap();
        let handler =
            CommandHandler::new(store.clone(), backend.clone(), events.clone(), watch_loop.clone(), "x11".to_string());

        handler.handle(Command::PauseCapture { paused: true }).unwrap();

        assert!(watch_loop.is_paused());
        assert_eq!(events.events(), vec![Event::CapturePaused { paused: true }]);
    }

    #[test]
    fn update_settings_then_get_settings_round_trips() {
        let (handler, _store, _backend, _events) = handler();
        let settings =
            clip_core::config::AppSettings { retention_window_days: Some(45), ..clip_core::config::AppSettings::default() };

        handler.handle(Command::UpdateSettings { settings: settings.clone() }).unwrap();
        let response = handler.handle(Command::GetSettings).unwrap();

        let fetched: clip_core::config::AppSettings = serde_json::from_value(response).unwrap();
        assert_eq!(fetched.retention_window_days, Some(45));
    }

    #[test]
    fn update_settings_persists_a_valid_hotkey_binding() {
        let (handler, _store, _backend, _events) = handler();
        let settings = clip_core::config::AppSettings {
            hotkey_binding: "Ctrl+Shift+V".to_string(),
            ..clip_core::config::AppSettings::default()
        };

        handler.handle(Command::UpdateSettings { settings }).unwrap();

        let response = handler.handle(Command::GetSettings).unwrap();
        let fetched: clip_core::config::AppSettings = serde_json::from_value(response).unwrap();
        assert_eq!(fetched.hotkey_binding, "Ctrl+Shift+V");
    }

    #[test]
    fn update_settings_rejects_an_invalid_hotkey_binding_without_persisting_it() {
        let (handler, _store, _backend, _events) = handler();
        let settings = clip_core::config::AppSettings {
            hotkey_binding: "NotAKey+++".to_string(),
            ..clip_core::config::AppSettings::default()
        };

        let result = handler.handle(Command::UpdateSettings { settings });

        assert!(result.is_err());
        let response = handler.handle(Command::GetSettings).unwrap();
        let fetched: clip_core::config::AppSettings = serde_json::from_value(response).unwrap();
        assert_eq!(fetched.hotkey_binding, clip_core::config::AppSettings::default().hotkey_binding);
    }

    #[test]
    fn get_diagnostics_mirrors_the_fake_backends_capabilities() {
        let (handler, _store, backend, _events) = handler();
        backend.set_capabilities(clip_platform::clipboard::BackendCapabilities {
            paste_simulation: true,
            hotkeys: false,
            ..Default::default()
        });

        let response = handler.handle(Command::GetDiagnostics).unwrap();

        assert_eq!(response["backend"], serde_json::json!("x11"));
        assert_eq!(response["capabilities"]["paste_simulation"], serde_json::json!(true));
        assert_eq!(response["capabilities"]["hotkeys"], serde_json::json!(false));
    }

    #[test]
    fn list_groups_returns_seeded_groups() {
        let (handler, store, _backend, _events) = handler();
        store.seed_group(Group::new("g1", "Work", None).unwrap());

        let response = handler.handle(Command::ListGroups).unwrap();

        let groups: Vec<Group> = serde_json::from_value(response).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "g1");
    }

    #[test]
    fn list_rules_returns_every_rule_regardless_of_enabled_state() {
        let (handler, store, _backend, _events) = handler();
        store.save_rule(&Rule::new("r1", "1Password", None, None, RuleAction::Exclude)).unwrap();
        store
            .save_rule(&Rule { enabled: false, ..Rule::new("r2", "Bitwarden", None, None, RuleAction::Exclude) })
            .unwrap();

        let response = handler.handle(Command::ListRules).unwrap();

        let rules: Vec<Rule> = serde_json::from_value(response).unwrap();
        assert_eq!(rules.len(), 2);
        assert!(rules.iter().any(|r| r.id == "r1" && r.enabled));
        assert!(rules.iter().any(|r| r.id == "r2" && !r.enabled));
    }
}
