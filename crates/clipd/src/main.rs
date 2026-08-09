//! Startup and lifecycle.

mod app;
mod commands;
mod ingest;
mod jobs;
mod telemetry;
mod watch_loop;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Loads a `.env` file (searched from the current directory upward), if
    // one exists, into the process environment - lets `CLIPDECK_CONFIG_DIR`/
    // `CLIPDECK_DATA_DIR`/`CLIPDECK_CACHE_DIR`/etc. be pinned in a local,
    // gitignored file instead of exported by hand every run. Silently does
    // nothing when no `.env` file is found; never overrides a var already
    // set in the real environment.
    dotenvy::dotenv().ok();

    telemetry::init_subscriber();

    let paths = clip_core::config::AppPaths::resolve();
    std::fs::create_dir_all(&paths.data_dir)?;
    std::fs::create_dir_all(&paths.config_dir)?;

    let db_path = paths.data_dir.join("clipdeck.sqlite3");
    let socket_path = paths.config_dir.join("clipd.sock");

    // Prefer X11 whenever a display is actually reachable - this covers both
    // native X11 sessions and Wayland sessions with XWayland available (e.g.
    // GNOME/Mutter, which doesn't support wlr-data-control at all), so those
    // sessions keep working exactly as before this change. Only fall back to
    // the native Wayland backend when no X11 connection can be made at all.
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let is_wayland_session = app::is_wayland_session(wayland_display.as_deref());

    let (backend, backend_name): (std::sync::Arc<dyn app::Backend>, &str) = match app::X11DaemonBackend::connect(is_wayland_session) {
        Ok(x11_backend) => (std::sync::Arc::new(x11_backend), "x11"),
        Err(_) if is_wayland_session => (std::sync::Arc::new(app::WaylandDaemonBackend::connect()?), "wayland"),
        Err(x11_error) => return Err(x11_error),
    };

    // Hotkey backend selection depends on session type, not X11/XWayland
    // reachability: XWayland transparently supports clipboard-selection
    // ownership (why the clipboard backend above can prefer X11), but
    // GNOME/Mutter's Wayland compositor never forwards global key events to
    // XWayland clients' `XGrabKey` grabs, so `GlobalHotkeyBackend` would
    // silently never fire on a Wayland session even when XWayland is
    // reachable. Use the GNOME GSettings custom-keybinding backend for any
    // Wayland session instead.
    let hotkeys: std::sync::Arc<dyn clip_platform::hotkeys::HotkeyBackend> = if is_wayland_session {
        let trigger_command = std::env::current_exe()?
            .parent()
            .expect("daemon executable has a parent directory")
            .join("clip-hotkey-trigger");
        std::sync::Arc::new(clip_platform::gsettings_hotkey::GSettingsHotkeyBackend::new(
            clip_platform::gsettings_hotkey::RealGSettingsRunner::new(),
            trigger_command.to_string_lossy().to_string(),
        ))
    } else {
        std::sync::Arc::new(clip_platform::hotkeys::GlobalHotkeyBackend::new()?)
    };

    let shutdown = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
        tracing::info!("shutdown signal received, draining in-flight work");
    };

    app::run(db_path.to_string_lossy().to_string(), socket_path, backend, backend_name.to_string(), hotkeys, shutdown)
        .await?;

    // `run` already drains in-flight IPC work and returns cleanly once
    // `shutdown` resolves - but the clipboard capture loop runs on a
    // `spawn_blocking` thread that blocks forever in a synchronous read
    // waiting for the next clipboard change (see `x11::real::poll_selection_
    // change`/`wayland::real`'s equivalent), with no way to be woken up
    // early. Tokio's runtime, dropped when `main` returns normally, waits
    // for that thread to finish before the process can exit - which it
    // never does on its own, turning a plain SIGTERM into a hang that only
    // `kill -9` clears. Exit immediately instead: everything that matters
    // (in-flight IPC responses, SQLite writes) is already durable by the
    // time `run` returns, so there is nothing left to lose by not waiting
    // for that thread.
    std::process::exit(0);
}
