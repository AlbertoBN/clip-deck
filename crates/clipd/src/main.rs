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

    let (backend, hotkeys, backend_name): (
        std::sync::Arc<dyn app::Backend>,
        std::sync::Arc<dyn clip_platform::hotkeys::HotkeyBackend>,
        &str,
    ) = match app::X11DaemonBackend::connect() {
        Ok(x11_backend) => (
            std::sync::Arc::new(x11_backend),
            std::sync::Arc::new(clip_platform::hotkeys::GlobalHotkeyBackend::new()?),
            "x11",
        ),
        Err(_) if app::is_wayland_session(wayland_display.as_deref()) => (
            std::sync::Arc::new(app::WaylandDaemonBackend::connect()?),
            std::sync::Arc::new(clip_platform::hotkeys::UnsupportedHotkeyBackend::new()),
            "wayland",
        ),
        Err(x11_error) => return Err(x11_error),
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
    Ok(())
}
