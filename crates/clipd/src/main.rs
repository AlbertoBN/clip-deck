//! Startup and lifecycle.

mod app;
mod commands;
mod ingest;
mod jobs;
mod telemetry;
mod watch_loop;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_subscriber();

    let paths = clip_core::config::AppPaths::resolve();
    std::fs::create_dir_all(&paths.data_dir)?;
    std::fs::create_dir_all(&paths.config_dir)?;

    let db_path = paths.data_dir.join("clipdeck.sqlite3");
    let socket_path = paths.config_dir.join("clipd.sock");

    let backend: std::sync::Arc<dyn app::Backend> = std::sync::Arc::new(app::X11DaemonBackend::connect()?);
    let hotkeys: std::sync::Arc<dyn clip_platform::hotkeys::HotkeyBackend> =
        std::sync::Arc::new(clip_platform::hotkeys::GlobalHotkeyBackend::new()?);

    let shutdown = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
        tracing::info!("shutdown signal received, draining in-flight work");
    };

    app::run(db_path.to_string_lossy().to_string(), socket_path, backend, "x11".to_string(), hotkeys, shutdown).await?;
    Ok(())
}
