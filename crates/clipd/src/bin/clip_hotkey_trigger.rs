//! Standalone CLI: connects to a running `clipd` over its Unix socket, sends
//! `TriggerHotkey`, and exits. This is what GNOME's GSettings custom
//! keybinding (see `clip_platform::gsettings_hotkey`) actually runs on
//! keypress, rather than the daemon holding a live in-process key grab.
//! Exits quietly (no panic, no stack trace) if the daemon isn't running.

#[tokio::main]
async fn main() {
    let socket_path = clip_core::config::AppPaths::resolve().config_dir.join("clipd.sock");

    let Ok(client) = clip_ipc::client::IpcClient::connect(&socket_path).await else {
        return;
    };

    let _ = client.call(clip_ipc::protocol::Command::TriggerHotkey).await;
}
