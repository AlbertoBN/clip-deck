//! Commands, events, DTOs. See the PRD's "IPC contract" section before extending.

use clip_core::config::AppSettings;
use clip_core::models::{PasteMode, Rule};
use clip_core::search::SearchFilters;
use serde::{Deserialize, Serialize};

/// Bulk-clear scope for `ClearHistory`, mirroring `clip_store::retention::ClearScope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClearScope {
    All,
    ExcludingPinned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    SearchClips { query: String, filters: SearchFilters, limit: u32, offset: u32 },
    GetClip { id: String },
    PasteClip { id: String, mode: PasteMode },
    PinClip { id: String, pinned: bool },
    AssignGroup { id: String, group_id: Option<String> },
    DeleteClip { id: String },
    ClearHistory { scope: ClearScope },
    ListGroups,
    SaveRule { rule: Rule },
    DeleteRule { id: String },
    GetSettings,
    UpdateSettings { settings: AppSettings },
    GetDiagnostics,
    PauseCapture { paused: bool },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    ClipCaptured { clip_id: String },
    ClipUpdated { clip_id: String },
    ClipDeleted { clip_id: String },
    CapturePaused { paused: bool },
    DiagnosticsChanged,
    HotkeyPressed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub request_id: String,
    pub command: Command,
}

impl Request {
    pub fn new(request_id: impl Into<String>, command: Command) -> Self {
        Self { request_id: request_id.into(), command }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Ok { request_id: String, payload: serde_json::Value },
    Err { request_id: String, error: String },
}

impl Response {
    pub fn ok(request_id: impl Into<String>, payload: serde_json::Value) -> Self {
        Self::Ok { request_id: request_id.into(), payload }
    }

    pub fn err(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::Err { request_id: request_id.into(), error: error.into() }
    }

    pub fn request_id(&self) -> &str {
        match self {
            Response::Ok { request_id, .. } | Response::Err { request_id, .. } => request_id,
        }
    }
}

/// Wire-level framing for server -> client lines: disambiguates a command
/// response from a broadcast event on the same duplex connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ServerMessage {
    Response(Response),
    Event(Event),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clip_core::config::AppSettings;
    use clip_core::models::{PasteMode, Rule, RuleAction};
    use clip_core::search::SearchFilters;

    fn all_commands() -> Vec<Command> {
        vec![
            Command::SearchClips { query: "ssh".to_string(), filters: SearchFilters::default(), limit: 20, offset: 0 },
            Command::GetClip { id: "c1".to_string() },
            Command::PasteClip { id: "c1".to_string(), mode: PasteMode::Auto },
            Command::PinClip { id: "c1".to_string(), pinned: true },
            Command::AssignGroup { id: "c1".to_string(), group_id: Some("g1".to_string()) },
            Command::DeleteClip { id: "c1".to_string() },
            Command::ClearHistory { scope: ClearScope::ExcludingPinned },
            Command::ListGroups,
            Command::SaveRule { rule: Rule::new("r1", "1Password", None, None, RuleAction::Exclude) },
            Command::DeleteRule { id: "r1".to_string() },
            Command::GetSettings,
            Command::UpdateSettings { settings: AppSettings::default() },
            Command::GetDiagnostics,
            Command::PauseCapture { paused: true },
        ]
    }

    fn all_events() -> Vec<Event> {
        vec![
            Event::ClipCaptured { clip_id: "c1".to_string() },
            Event::ClipUpdated { clip_id: "c1".to_string() },
            Event::ClipDeleted { clip_id: "c1".to_string() },
            Event::CapturePaused { paused: true },
            Event::DiagnosticsChanged,
            Event::HotkeyPressed,
        ]
    }

    #[test]
    fn every_command_variant_round_trips_through_the_wire_format() {
        for command in all_commands() {
            let json = serde_json::to_string(&command).unwrap();
            let round_tripped: Command = serde_json::from_str(&json).unwrap();
            assert_eq!(round_tripped, command);
        }
    }

    #[test]
    fn every_event_variant_round_trips_through_the_wire_format() {
        for event in all_events() {
            let json = serde_json::to_string(&event).unwrap();
            let round_tripped: Event = serde_json::from_str(&json).unwrap();
            assert_eq!(round_tripped, event);
        }
    }

    #[test]
    fn two_concurrent_requests_get_distinguishable_responses() {
        let r1 = Request::new("r1", Command::ListGroups);
        let r2 = Request::new("r2", Command::GetSettings);
        assert_ne!(r1.request_id, r2.request_id);
        assert_eq!(r1.command, Command::ListGroups);
        assert_eq!(r2.command, Command::GetSettings);
    }

    #[test]
    fn a_successful_response_round_trips_its_payload() {
        let response = Response::ok("r1", serde_json::json!({"id": "c1"}));
        let json = serde_json::to_string(&response).unwrap();
        let round_tripped: Response = serde_json::from_str(&json).unwrap();
        match round_tripped {
            Response::Ok { request_id, payload } => {
                assert_eq!(request_id, "r1");
                assert_eq!(payload, serde_json::json!({"id": "c1"}));
            }
            Response::Err { .. } => panic!("expected Ok"),
        }
    }

    #[test]
    fn an_error_response_round_trips_its_error_message() {
        let response = Response::err("r1", "clip not found");
        let json = serde_json::to_string(&response).unwrap();
        let round_tripped: Response = serde_json::from_str(&json).unwrap();
        match round_tripped {
            Response::Err { request_id, error } => {
                assert_eq!(request_id, "r1");
                assert_eq!(error, "clip not found");
            }
            Response::Ok { .. } => panic!("expected Err"),
        }
    }
}
