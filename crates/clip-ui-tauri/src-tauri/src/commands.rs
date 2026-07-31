//! `#[tauri::command]` wrappers bridging the frontend to a managed `Client`.

use clip_core::config::AppSettings;
use clip_core::models::{Clip, PasteMode, Rule};
use clip_core::search::SearchFilters;
use clip_ipc::protocol::{ClearScope, Command, Response};

use crate::client::{Client, ClientHandle};

fn response_payload<T: serde::de::DeserializeOwned>(response: Response) -> Result<T, String> {
    match response {
        Response::Ok { payload, .. } => serde_json::from_value(payload).map_err(|e| e.to_string()),
        Response::Err { error, .. } => Err(error),
    }
}

fn response_ok(response: Response) -> Result<(), String> {
    match response {
        Response::Ok { .. } => Ok(()),
        Response::Err { error, .. } => Err(error),
    }
}

pub async fn search_clips_with(
    client: &dyn Client,
    query: String,
    filters: SearchFilters,
    limit: u32,
    offset: u32,
) -> Result<Vec<Clip>, String> {
    response_payload(client.call(Command::SearchClips { query, filters, limit, offset }).await)
}

pub async fn get_clip_with(client: &dyn Client, id: String) -> Result<Option<Clip>, String> {
    response_payload(client.call(Command::GetClip { id }).await)
}

pub async fn paste_clip_with(client: &dyn Client, id: String, mode: PasteMode) -> Result<(), String> {
    response_ok(client.call(Command::PasteClip { id, mode }).await)
}

pub async fn copy_clip_with(client: &dyn Client, id: String) -> Result<(), String> {
    response_ok(client.call(Command::CopyClip { id }).await)
}

pub async fn pin_clip_with(client: &dyn Client, id: String, pinned: bool) -> Result<(), String> {
    response_ok(client.call(Command::PinClip { id, pinned }).await)
}

pub async fn delete_clip_with(client: &dyn Client, id: String) -> Result<(), String> {
    response_ok(client.call(Command::DeleteClip { id }).await)
}

pub async fn clear_history_with(client: &dyn Client, scope: ClearScope) -> Result<(), String> {
    response_ok(client.call(Command::ClearHistory { scope }).await)
}

pub async fn list_rules_with(client: &dyn Client) -> Result<Vec<Rule>, String> {
    response_payload(client.call(Command::ListRules).await)
}

pub async fn save_rule_with(client: &dyn Client, rule: Rule) -> Result<(), String> {
    response_ok(client.call(Command::SaveRule { rule }).await)
}

pub async fn delete_rule_with(client: &dyn Client, id: String) -> Result<(), String> {
    response_ok(client.call(Command::DeleteRule { id }).await)
}

pub async fn get_settings_with(client: &dyn Client) -> Result<AppSettings, String> {
    response_payload(client.call(Command::GetSettings).await)
}

pub async fn update_settings_with(client: &dyn Client, settings: AppSettings) -> Result<(), String> {
    response_ok(client.call(Command::UpdateSettings { settings }).await)
}

pub async fn get_diagnostics_with(client: &dyn Client) -> Result<serde_json::Value, String> {
    response_payload(client.call(Command::GetDiagnostics).await)
}

pub async fn pause_capture_with(client: &dyn Client, paused: bool) -> Result<(), String> {
    response_ok(client.call(Command::PauseCapture { paused }).await)
}

#[tauri::command]
pub async fn search_clips(
    state: tauri::State<'_, ClientHandle>,
    query: String,
    filters: SearchFilters,
    limit: u32,
    offset: u32,
) -> Result<Vec<Clip>, String> {
    search_clips_with(state.0.as_ref(), query, filters, limit, offset).await
}

#[tauri::command]
pub async fn get_clip(state: tauri::State<'_, ClientHandle>, id: String) -> Result<Option<Clip>, String> {
    get_clip_with(state.0.as_ref(), id).await
}

#[tauri::command]
pub async fn paste_clip(state: tauri::State<'_, ClientHandle>, id: String, mode: PasteMode) -> Result<(), String> {
    paste_clip_with(state.0.as_ref(), id, mode).await
}

#[tauri::command]
pub async fn copy_clip(state: tauri::State<'_, ClientHandle>, id: String) -> Result<(), String> {
    copy_clip_with(state.0.as_ref(), id).await
}

#[tauri::command]
pub async fn pin_clip(state: tauri::State<'_, ClientHandle>, id: String, pinned: bool) -> Result<(), String> {
    pin_clip_with(state.0.as_ref(), id, pinned).await
}

#[tauri::command]
pub async fn delete_clip(state: tauri::State<'_, ClientHandle>, id: String) -> Result<(), String> {
    delete_clip_with(state.0.as_ref(), id).await
}

#[tauri::command]
pub async fn clear_history(state: tauri::State<'_, ClientHandle>, scope: ClearScope) -> Result<(), String> {
    clear_history_with(state.0.as_ref(), scope).await
}

#[tauri::command]
pub async fn list_rules(state: tauri::State<'_, ClientHandle>) -> Result<Vec<Rule>, String> {
    list_rules_with(state.0.as_ref()).await
}

#[tauri::command]
pub async fn save_rule(state: tauri::State<'_, ClientHandle>, rule: Rule) -> Result<(), String> {
    save_rule_with(state.0.as_ref(), rule).await
}

#[tauri::command]
pub async fn delete_rule(state: tauri::State<'_, ClientHandle>, id: String) -> Result<(), String> {
    delete_rule_with(state.0.as_ref(), id).await
}

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, ClientHandle>) -> Result<AppSettings, String> {
    get_settings_with(state.0.as_ref()).await
}

#[tauri::command]
pub async fn update_settings(state: tauri::State<'_, ClientHandle>, settings: AppSettings) -> Result<(), String> {
    update_settings_with(state.0.as_ref(), settings).await
}

#[tauri::command]
pub async fn get_diagnostics(state: tauri::State<'_, ClientHandle>) -> Result<serde_json::Value, String> {
    get_diagnostics_with(state.0.as_ref()).await
}

#[tauri::command]
pub async fn pause_capture(state: tauri::State<'_, ClientHandle>, paused: bool) -> Result<(), String> {
    pause_capture_with(state.0.as_ref(), paused).await
}

#[cfg(test)]
mod tests {
    use crate::client::fakes::FakeClient;
    use clip_core::models::Clip;
    use clip_ipc::protocol::Response;

    #[tokio::test]
    async fn search_clips_returns_the_fakes_canned_clip_list() {
        let client = FakeClient::new();
        let clip = Clip::new("c1", "hash1", "text/plain", vec![]);
        client.push_response(Response::ok("test", serde_json::to_value(vec![clip.clone()]).unwrap()));

        let result = super::search_clips_with(&client, "".to_string(), Default::default(), 20, 0).await.unwrap();

        assert_eq!(result, vec![clip]);
    }

    #[tokio::test]
    async fn list_rules_returns_the_fakes_canned_rule_list() {
        use clip_core::models::{Rule, RuleAction};

        let client = FakeClient::new();
        let rule = Rule::new("r1", "1Password", None, None, RuleAction::Exclude);
        client.push_response(Response::ok("test", serde_json::to_value(vec![rule.clone()]).unwrap()));

        let result = super::list_rules_with(&client).await.unwrap();

        assert_eq!(result, vec![rule]);
    }

    #[tokio::test]
    async fn copy_clip_with_calls_copy_clip_and_returns_ok() {
        let client = FakeClient::new();
        client.push_response(Response::ok("test", serde_json::json!({"ok": true})));

        let result = super::copy_clip_with(&client, "c1".to_string()).await;

        assert_eq!(result, Ok(()));
        assert_eq!(client.calls(), vec![clip_ipc::protocol::Command::CopyClip { id: "c1".to_string() }]);
    }

    #[tokio::test]
    async fn a_daemon_error_response_is_surfaced_as_an_error_not_swallowed() {
        let client = FakeClient::new();
        client.push_response(Response::err("test", "clip not found"));

        let result = super::get_clip_with(&client, "missing".to_string()).await;

        assert_eq!(result, Err("clip not found".to_string()));
    }
}
