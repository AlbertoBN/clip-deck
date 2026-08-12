//! Startup and lifecycle.

use std::sync::{Arc, Mutex};

use crate::commands::CommandHandler;
use crate::watch_loop::WatchLoop;
use clip_core::config::{AppSettings, SettingsKey};
use clip_core::models::{AppContext, Clip, ClipRepresentation, PasteMode, Rule};
use clip_core::search::SearchFilters;
use clip_platform::clipboard::{BackendCapabilities, ClipboardSnapshot, PlatformError};
use clip_store::retention::ClearScope;
use clip_store::StoreError;

/// Everything `ingest`/`commands`/`jobs` need from `clip-store`, behind a
/// trait so tests can inject an in-memory fake instead of a real SQLite file.
pub trait Store: Send + Sync {
    fn insert_clip(&self, clip: &Clip) -> Result<(), StoreError>;
    fn get_clip(&self, id: &str) -> Result<Option<Clip>, StoreError>;
    fn find_active_by_hash(&self, content_hash: &str, primary_mime: &str) -> Result<Option<Clip>, StoreError>;
    fn touch_last_used(&self, id: &str) -> Result<(), StoreError>;
    fn search(&self, query: &str, filters: &SearchFilters) -> Result<Vec<Clip>, StoreError>;
    fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), StoreError>;
    fn delete_clip(&self, id: &str) -> Result<(), StoreError>;
    fn clear_history(&self, scope: ClearScope) -> Result<Vec<String>, StoreError>;
    fn list_enabled_rules(&self) -> Result<Vec<Rule>, StoreError>;
    fn list_rules(&self) -> Result<Vec<Rule>, StoreError>;
    fn save_rule(&self, rule: &Rule) -> Result<(), StoreError>;
    fn delete_rule(&self, id: &str) -> Result<(), StoreError>;
    fn get_settings(&self) -> Result<AppSettings, StoreError>;
    fn update_settings(&self, settings: &AppSettings) -> Result<(), StoreError>;
    fn prune(&self, retention_days: Option<u32>) -> Result<usize, StoreError>;
}

/// Everything `commands`/`watch_loop` need from `clip-platform`, behind a
/// trait so tests can inject a fake instead of a real X11 connection. Narrower
/// than `clip_platform::clipboard::ClipboardBackend`: this daemon doesn't need
/// direct clipboard read/write, only capture-event delivery and paste.
pub trait Backend: Send + Sync {
    fn start(&self, on_capture: Box<dyn Fn(ClipboardSnapshot) + Send + Sync>) -> Result<(), PlatformError>;
    fn focused_app(&self) -> Option<AppContext>;
    fn simulate_paste(&self, representations: &[ClipRepresentation], mode: PasteMode) -> Result<(), PlatformError>;
    /// Places the clip's plain-text content on the clipboard only - no
    /// focused-window lookup, no key synthesis - so the user can paste it
    /// manually wherever they choose, per `paste-simulation`'s copy-only
    /// mode.
    fn copy_to_clipboard(&self, representations: &[ClipRepresentation]) -> Result<(), PlatformError>;
    fn capabilities(&self) -> BackendCapabilities;
}

/// Everything `commands`/`ingest` need from `clip-ipc`'s event side, behind a
/// trait so tests can inject a fake instead of a real Unix socket broadcast.
pub trait EventPublisher: Send + Sync {
    fn publish(&self, event: clip_ipc::protocol::Event);
}

impl EventPublisher for clip_ipc::server::EventPublisher {
    fn publish(&self, event: clip_ipc::protocol::Event) {
        clip_ipc::server::EventPublisher::publish(self, event);
    }
}

/// Real `clip-store`-backed `Store`. `rusqlite::Connection` isn't `Sync`, so
/// access is serialized behind a mutex; SQLite operations here are all fast,
/// local, single-writer operations, so this is not a contention concern.
pub struct SqliteStore {
    conn: Mutex<rusqlite::Connection>,
}

impl SqliteStore {
    pub fn new(conn: rusqlite::Connection) -> Self {
        Self { conn: Mutex::new(conn) }
    }
}

const SETTINGS_KEYS: [SettingsKey; 4] = [
    SettingsKey::HotkeyBinding,
    SettingsKey::RetentionWindowDays,
    SettingsKey::CapturePaused,
    SettingsKey::DefaultPasteMode,
];

impl Store for SqliteStore {
    fn insert_clip(&self, clip: &Clip) -> Result<(), StoreError> {
        clip_store::clips::insert(&self.conn.lock().unwrap(), clip)
    }

    fn get_clip(&self, id: &str) -> Result<Option<Clip>, StoreError> {
        clip_store::clips::get(&self.conn.lock().unwrap(), id)
    }

    fn find_active_by_hash(&self, content_hash: &str, primary_mime: &str) -> Result<Option<Clip>, StoreError> {
        clip_store::clips::get_by_hash(&self.conn.lock().unwrap(), content_hash, primary_mime)
    }

    fn touch_last_used(&self, id: &str) -> Result<(), StoreError> {
        clip_store::clips::touch_last_used(&self.conn.lock().unwrap(), id)
    }

    fn search(&self, query: &str, filters: &SearchFilters) -> Result<Vec<Clip>, StoreError> {
        clip_store::fts::search(&self.conn.lock().unwrap(), query, filters)
    }

    fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), StoreError> {
        clip_store::clips::set_pinned(&self.conn.lock().unwrap(), id, pinned)
    }

    fn delete_clip(&self, id: &str) -> Result<(), StoreError> {
        clip_store::retention::delete_clip(&self.conn.lock().unwrap(), id)
    }

    fn clear_history(&self, scope: ClearScope) -> Result<Vec<String>, StoreError> {
        clip_store::retention::clear_with_ids(&self.conn.lock().unwrap(), scope)
    }

    fn list_enabled_rules(&self) -> Result<Vec<Rule>, StoreError> {
        clip_store::rules::list_enabled(&self.conn.lock().unwrap())
    }

    fn list_rules(&self) -> Result<Vec<Rule>, StoreError> {
        clip_store::rules::list_all(&self.conn.lock().unwrap())
    }

    fn save_rule(&self, rule: &Rule) -> Result<(), StoreError> {
        clip_store::rules::upsert(&self.conn.lock().unwrap(), rule)
    }

    fn delete_rule(&self, id: &str) -> Result<(), StoreError> {
        clip_store::rules::delete(&self.conn.lock().unwrap(), id)
    }

    fn get_settings(&self) -> Result<AppSettings, StoreError> {
        let entries = clip_store::settings::get_all(&self.conn.lock().unwrap())?;
        Ok(AppSettings::from_entries(&entries))
    }

    fn update_settings(&self, settings: &AppSettings) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        for key in SETTINGS_KEYS {
            clip_store::settings::set_value(&conn, key.as_str(), &settings.get_value(key))?;
        }
        Ok(())
    }

    fn prune(&self, retention_days: Option<u32>) -> Result<usize, StoreError> {
        clip_store::retention::prune(&self.conn.lock().unwrap(), retention_days)
    }
}

/// Real X11-backed `Backend`: a capture watch loop, a focus tracker, and a
/// paste simulator, each over their own `RealX11Connection` (X11 supports
/// many simultaneous client connections, so this is simpler than sharing
/// one).
pub struct X11DaemonBackend {
    capture: clip_platform::x11::X11Backend<clip_platform::x11::RealX11Connection>,
    focus: clip_platform::focus::FocusTracker<clip_platform::x11::RealX11Connection>,
    paste: clip_platform::paste::PasteSimulator<clip_platform::x11::RealX11Connection>,
}

impl X11DaemonBackend {
    /// `is_wayland_session` selects the paste strategy: GNOME/Mutter treats
    /// XTest key synthesis from an XWayland client as a security-sensitive
    /// Remote Desktop portal operation and pops up a consent dialog for it
    /// on every paste, so a Wayland session uses `PasteSimulator::
    /// clipboard_only` instead of the normal synthetic-keystroke path, even
    /// though X11 is still preferred here for its richer capture
    /// capabilities (HTML/image representations, working change detection).
    pub fn connect(is_wayland_session: bool) -> anyhow::Result<Self> {
        let capture_conn = clip_platform::x11::RealX11Connection::connect(None)
            .map_err(|e| anyhow::anyhow!("failed to open X11 connection for capture: {e}"))?;
        let focus_conn = clip_platform::x11::RealX11Connection::connect(None)
            .map_err(|e| anyhow::anyhow!("failed to open X11 connection for focus tracking: {e}"))?;
        let paste_conn = clip_platform::x11::RealX11Connection::connect(None)
            .map_err(|e| anyhow::anyhow!("failed to open X11 connection for paste: {e}"))?;
        let blob_dir = clip_core::config::AppPaths::resolve().data_dir.join("blobs");
        let paste = if is_wayland_session {
            clip_platform::paste::PasteSimulator::clipboard_only(paste_conn)
        } else {
            clip_platform::paste::PasteSimulator::new(paste_conn)
        };
        Ok(Self {
            capture: clip_platform::x11::X11Backend::new(capture_conn, blob_dir),
            focus: clip_platform::focus::FocusTracker::new(focus_conn),
            paste,
        })
    }
}

impl Backend for X11DaemonBackend {
    fn start(&self, on_capture: Box<dyn Fn(ClipboardSnapshot) + Send + Sync>) -> Result<(), PlatformError> {
        self.capture.start(on_capture)
    }

    fn focused_app(&self) -> Option<AppContext> {
        self.focus.focused_app()
    }

    fn simulate_paste(&self, representations: &[ClipRepresentation], mode: PasteMode) -> Result<(), PlatformError> {
        self.paste.paste_to_focused_window(representations, mode)
    }

    fn copy_to_clipboard(&self, representations: &[ClipRepresentation]) -> Result<(), PlatformError> {
        self.paste.copy_to_clipboard(representations, PasteMode::PlainText);
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.capture.capabilities()
    }
}

/// Detects whether the process is running under a Wayland session, per
/// `daemon-lifecycle`'s session-based backend-selection spec. Takes the
/// `WAYLAND_DISPLAY` value as a parameter (rather than reading `std::env`
/// directly) so this stays a pure, directly-testable predicate; `main` reads
/// the real environment variable once and calls this.
pub(crate) fn is_wayland_session(wayland_display: Option<&str>) -> bool {
    wayland_display.is_some()
}

/// Real Wayland-backed `Backend`: a capture watch loop over
/// `wlr-data-control`, plus an always-unsupported focus tracker (Wayland's
/// security model withholds focused-window info from clients). `simulate_paste`
/// places content on the clipboard only - see `paste-simulation`'s Wayland
/// scenario - no synthetic key delivery is attempted, since Wayland has no
/// input-synthesis mechanism wired in this workspace.
pub struct WaylandDaemonBackend {
    capture: clip_platform::wayland::WaylandBackend<clip_platform::wayland::RealWaylandConnection>,
    focus: clip_platform::focus::UnsupportedFocusTracker,
}

impl WaylandDaemonBackend {
    pub fn connect() -> anyhow::Result<Self> {
        let conn = clip_platform::wayland::RealWaylandConnection::connect()
            .map_err(|e| anyhow::anyhow!("failed to open Wayland connection: {e}"))?;
        let capture = clip_platform::wayland::WaylandBackend::new(conn)
            .map_err(|e| anyhow::anyhow!("failed to construct Wayland backend: {e}"))?;
        Ok(Self { capture, focus: clip_platform::focus::UnsupportedFocusTracker::new() })
    }
}

impl Backend for WaylandDaemonBackend {
    fn start(&self, on_capture: Box<dyn Fn(ClipboardSnapshot) + Send + Sync>) -> Result<(), PlatformError> {
        self.capture.start(on_capture)
    }

    fn focused_app(&self) -> Option<AppContext> {
        self.focus.focused_app()
    }

    fn simulate_paste(&self, representations: &[ClipRepresentation], mode: PasteMode) -> Result<(), PlatformError> {
        let text = clip_platform::paste::resolve_paste_text(representations, mode).unwrap_or_default();
        self.capture.set_current(&text)
    }

    fn copy_to_clipboard(&self, representations: &[ClipRepresentation]) -> Result<(), PlatformError> {
        let text = clip_platform::paste::resolve_paste_text(representations, PasteMode::PlainText).unwrap_or_default();
        self.capture.set_current(&text)
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.capture.capabilities()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("clipd is already running (socket already bound at {0})")]
    AlreadyRunning(std::path::PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Platform(#[from] PlatformError),
}

/// Registers the persisted `hotkey_binding` (per `GetSettings`) with
/// `hotkeys`, wiring its callback to publish `Event::HotkeyPressed`. A
/// missing/unparseable binding or a registration failure (e.g. an
/// unsupported compositor) is logged and does not propagate - the caller
/// (`run`) must keep starting up regardless, per `hotkey-registration`'s
/// spec.
fn register_hotkey(store: &dyn Store, hotkeys: &Arc<dyn clip_platform::hotkeys::HotkeyBackend>, events: Arc<dyn EventPublisher>) {
    let settings = match store.get_settings() {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!(%error, "failed to load settings for hotkey registration; skipping");
            return;
        }
    };

    let binding = match clip_platform::hotkeys::parse_binding(&settings.hotkey_binding) {
        Ok(binding) => binding,
        Err(error) => {
            tracing::warn!(%error, binding = %settings.hotkey_binding, "persisted hotkey binding failed to parse; skipping registration");
            return;
        }
    };

    if let Err(error) = hotkeys.register(binding, Box::new(move || events.publish(clip_ipc::protocol::Event::HotkeyPressed))) {
        tracing::warn!(%error, "failed to register global hotkey; hotkey-triggered popup activation will not work");
    }
}

/// Starts the daemon: applies migrations, binds the IPC server (failing fast
/// if another instance is already running), starts the clipboard watch loop,
/// and schedules the retention job - then serves IPC commands until
/// `shutdown` resolves, draining any in-flight command before returning.
pub async fn run(
    db_path: String,
    socket_path: std::path::PathBuf,
    backend: Arc<dyn Backend>,
    backend_name: String,
    hotkeys: Arc<dyn clip_platform::hotkeys::HotkeyBackend>,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), AppError> {
    // Fail fast if another instance is already live on this socket, before
    // touching the (possibly stale) socket file at all.
    if socket_path.exists() && tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
        return Err(AppError::AlreadyRunning(socket_path));
    }

    // Migrate before the IPC socket becomes connectable, so no client can
    // ever observe a partially-migrated database.
    let conn = tokio::task::spawn_blocking(move || clip_store::db::open(&db_path))
        .await
        .expect("migration task panicked")?;
    let store: Arc<dyn Store> = Arc::new(SqliteStore::new(conn));

    // The command handler needs the server's event publisher, which only
    // exists once the server is bound; the server, in turn, needs a handler
    // at bind time. Break the cycle with a cell filled immediately after
    // binding, before the server ever accepts a connection.
    let handler_cell: Arc<std::sync::OnceLock<CommandHandler>> = Arc::new(std::sync::OnceLock::new());
    let handler_cell_for_closure = handler_cell.clone();
    let handler_fn: clip_ipc::server::HandlerFn = Arc::new(move |command| {
        let handler_cell = handler_cell_for_closure.clone();
        Box::pin(async move {
            handler_cell
                .get()
                .expect("command handler installed before the server accepts connections")
                .handle(command)
        })
    });

    let server = clip_ipc::server::Server::bind(&socket_path, handler_fn)?;
    let events: Arc<dyn EventPublisher> = Arc::new(server.event_publisher());
    let watch_loop = Arc::new(WatchLoop::new());
    let command_handler =
        CommandHandler::new(store.clone(), backend.clone(), events.clone(), watch_loop.clone(), backend_name);
    handler_cell.set(command_handler).ok();

    // `Backend::start()` blocks the calling thread for the daemon's entire
    // lifetime on a real connection (a blocking event-channel `recv()` loop -
    // see `x11::real`/`wayland::real`), so it must run on a dedicated
    // blocking thread rather than inline here - otherwise this task would
    // never reach `server.run_with_shutdown` below, and the IPC server would
    // never actually serve a single request.
    let capture_backend = backend.clone();
    let capture_store = store.clone();
    let capture_events = events.clone();
    let capture_watch_loop = watch_loop.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(error) = capture_watch_loop.start(capture_backend, capture_store, capture_events) {
            tracing::error!(%error, "clipboard watch loop exited with an error; capture will no longer be forwarded");
        }
    });
    tokio::spawn(crate::jobs::run_retention_job(store.clone(), std::time::Duration::from_secs(3600)));
    register_hotkey(store.as_ref(), &hotkeys, events.clone());

    server.run_with_shutdown(shutdown).await?;
    Ok(())
}

/// In-memory fakes of `Store`, `Backend`, and `EventPublisher`, for the unit
/// tests in `ingest`, `watch_loop`, `commands`, and `jobs`.
#[cfg(test)]
pub(crate) mod fakes {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Default)]
    pub(crate) struct FakeStore {
        clips: Mutex<HashMap<String, Clip>>,
        rules: Mutex<HashMap<String, Rule>>,
        settings: Mutex<AppSettings>,
        prune_calls: Mutex<usize>,
        fail_next_prune: AtomicBool,
    }

    impl FakeStore {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn prune_call_count(&self) -> usize {
            *self.prune_calls.lock().unwrap()
        }

        pub(crate) fn fail_next_prune(&self) {
            self.fail_next_prune.store(true, Ordering::SeqCst);
        }
    }

    impl Store for FakeStore {
        fn insert_clip(&self, clip: &Clip) -> Result<(), StoreError> {
            let mut clips = self.clips.lock().unwrap();
            let dedup_key = clip.dedup_key();
            if clips.values().any(|c| !c.is_deleted && c.dedup_key() == dedup_key) {
                return Err(StoreError::DedupConflict);
            }
            clips.insert(clip.id.clone(), clip.clone());
            Ok(())
        }

        fn get_clip(&self, id: &str) -> Result<Option<Clip>, StoreError> {
            Ok(self.clips.lock().unwrap().get(id).cloned())
        }

        fn find_active_by_hash(&self, content_hash: &str, primary_mime: &str) -> Result<Option<Clip>, StoreError> {
            Ok(self
                .clips
                .lock()
                .unwrap()
                .values()
                .find(|c| !c.is_deleted && c.content_hash == content_hash && c.primary_mime == primary_mime)
                .cloned())
        }

        fn touch_last_used(&self, id: &str) -> Result<(), StoreError> {
            if let Some(clip) = self.clips.lock().unwrap().get_mut(id) {
                clip.last_used_at = Some(time::OffsetDateTime::now_utc());
            }
            Ok(())
        }

        fn search(&self, query: &str, filters: &SearchFilters) -> Result<Vec<Clip>, StoreError> {
            let clips = self.clips.lock().unwrap();
            let mut results: Vec<Clip> = clips
                .values()
                .filter(|c| !c.is_deleted)
                .filter(|c| query.is_empty() || c.display_text.as_deref().unwrap_or("").contains(query))
                .filter(|c| !filters.pinned_only || c.is_pinned)
                .filter(|c| !filters.favorite_only || c.is_favorite)
                .filter(|c| filters.source_app.is_none() || c.source_app == filters.source_app)
                .cloned()
                .collect();
            results.sort_by_key(|c| std::cmp::Reverse(c.created_at));
            Ok(results)
        }

        fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), StoreError> {
            if let Some(clip) = self.clips.lock().unwrap().get_mut(id) {
                clip.is_pinned = pinned;
            }
            Ok(())
        }

        fn delete_clip(&self, id: &str) -> Result<(), StoreError> {
            if let Some(clip) = self.clips.lock().unwrap().get_mut(id) {
                clip.is_deleted = true;
            }
            Ok(())
        }

        fn clear_history(&self, scope: ClearScope) -> Result<Vec<String>, StoreError> {
            let mut clips = self.clips.lock().unwrap();
            let ids: Vec<String> = clips
                .values()
                .filter(|c| !c.is_deleted)
                .filter(|c| scope == ClearScope::All || !c.is_pinned)
                .map(|c| c.id.clone())
                .collect();
            for id in &ids {
                clips.get_mut(id).unwrap().is_deleted = true;
            }
            Ok(ids)
        }

        fn list_enabled_rules(&self) -> Result<Vec<Rule>, StoreError> {
            Ok(self.rules.lock().unwrap().values().filter(|r| r.enabled).cloned().collect())
        }

        fn list_rules(&self) -> Result<Vec<Rule>, StoreError> {
            Ok(self.rules.lock().unwrap().values().cloned().collect())
        }

        fn save_rule(&self, rule: &Rule) -> Result<(), StoreError> {
            self.rules.lock().unwrap().insert(rule.id.clone(), rule.clone());
            Ok(())
        }

        fn delete_rule(&self, id: &str) -> Result<(), StoreError> {
            self.rules.lock().unwrap().remove(id);
            Ok(())
        }

        fn get_settings(&self) -> Result<AppSettings, StoreError> {
            Ok(self.settings.lock().unwrap().clone())
        }

        fn update_settings(&self, settings: &AppSettings) -> Result<(), StoreError> {
            *self.settings.lock().unwrap() = settings.clone();
            Ok(())
        }

        fn prune(&self, retention_days: Option<u32>) -> Result<usize, StoreError> {
            *self.prune_calls.lock().unwrap() += 1;
            if self.fail_next_prune.swap(false, Ordering::SeqCst) {
                return Err(StoreError::NotFound);
            }
            let Some(days) = retention_days else {
                return Ok(0);
            };
            let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(days as i64);
            let mut clips = self.clips.lock().unwrap();
            let to_remove: Vec<String> =
                clips.values().filter(|c| !c.is_pinned && c.created_at < cutoff).map(|c| c.id.clone()).collect();
            for id in &to_remove {
                clips.remove(id);
            }
            Ok(to_remove.len())
        }
    }

    type CaptureCallback = Box<dyn Fn(ClipboardSnapshot) + Send + Sync>;

    #[derive(Default)]
    pub(crate) struct FakeBackend {
        capture_callback: Mutex<Option<CaptureCallback>>,
        focused_app: Mutex<Option<AppContext>>,
        fail_paste: AtomicBool,
        paste_delay: Mutex<std::time::Duration>,
        capabilities: Mutex<BackendCapabilities>,
        blocks_on_start: AtomicBool,
        copied_text: Mutex<Option<String>>,
    }

    impl FakeBackend {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn set_fail_paste(&self, fail: bool) {
            self.fail_paste.store(fail, Ordering::SeqCst);
        }

        /// Test helper: makes `simulate_paste` block the calling thread for
        /// `delay` before returning, to create a window in which a shutdown
        /// signal can fire while a command is genuinely still in flight.
        pub(crate) fn set_paste_delay(&self, delay: std::time::Duration) {
            *self.paste_delay.lock().unwrap() = delay;
        }

        pub(crate) fn set_capabilities(&self, capabilities: BackendCapabilities) {
            *self.capabilities.lock().unwrap() = capabilities;
        }

        /// Test helper: makes `start` block the calling thread for a bounded
        /// but generous interval after registering the callback, mirroring
        /// the real X11/Wayland connections' `start()` (a blocking
        /// `Receiver::recv()` loop that only returns once the daemon shuts
        /// down) - so a regression test can confirm callers don't assume
        /// `start()` returns promptly. Bounded (rather than truly forever)
        /// so a misbehaving caller still lets the test process's runtime
        /// shut down cleanly instead of hanging the test suite itself.
        pub(crate) fn set_blocks_on_start(&self, blocks: bool) {
            self.blocks_on_start.store(blocks, Ordering::SeqCst);
        }

        /// Test helper: the plain-text content passed to the most recent
        /// `copy_to_clipboard` call, if any.
        pub(crate) fn copied_text(&self) -> Option<String> {
            self.copied_text.lock().unwrap().clone()
        }

        /// Test helper: simulates the backend emitting a capture event.
        pub(crate) fn emit_capture(&self, snapshot: ClipboardSnapshot) {
            if let Some(callback) = self.capture_callback.lock().unwrap().as_ref() {
                callback(snapshot);
            }
        }
    }

    impl Backend for FakeBackend {
        fn start(&self, on_capture: Box<dyn Fn(ClipboardSnapshot) + Send + Sync>) -> Result<(), PlatformError> {
            *self.capture_callback.lock().unwrap() = Some(on_capture);
            if self.blocks_on_start.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Ok(())
        }

        fn focused_app(&self) -> Option<AppContext> {
            self.focused_app.lock().unwrap().clone()
        }

        fn simulate_paste(&self, _representations: &[ClipRepresentation], _mode: PasteMode) -> Result<(), PlatformError> {
            let delay = *self.paste_delay.lock().unwrap();
            if !delay.is_zero() {
                std::thread::sleep(delay);
            }
            if self.fail_paste.load(Ordering::SeqCst) {
                Err(PlatformError::NoFocusedWindow)
            } else {
                Ok(())
            }
        }

        fn copy_to_clipboard(&self, representations: &[ClipRepresentation]) -> Result<(), PlatformError> {
            let text = clip_platform::paste::resolve_paste_text(representations, PasteMode::PlainText).unwrap_or_default();
            *self.copied_text.lock().unwrap() = Some(text);
            Ok(())
        }

        fn capabilities(&self) -> BackendCapabilities {
            *self.capabilities.lock().unwrap()
        }
    }

    #[derive(Default)]
    pub(crate) struct FakeEventPublisher {
        events: Mutex<Vec<clip_ipc::protocol::Event>>,
    }

    impl FakeEventPublisher {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn events(&self) -> Vec<clip_ipc::protocol::Event> {
            self.events.lock().unwrap().clone()
        }
    }

    impl EventPublisher for FakeEventPublisher {
        fn publish(&self, event: clip_ipc::protocol::Event) {
            self.events.lock().unwrap().push(event);
        }
    }

    type HotkeyCallback = Box<dyn Fn() + Send + Sync>;

    /// `clipd`'s own fake `HotkeyBackend` (clip-platform's is
    /// `#[cfg(test)] pub(crate)`-scoped to that crate, not visible here) -
    /// records the registered callback so tests can trigger it directly, and
    /// can be configured to fail registration like an unsupported-compositor
    /// backend would.
    #[derive(Default)]
    pub(crate) struct FakeHotkeyBackend {
        registered: Mutex<Option<HotkeyCallback>>,
        fail_registration: AtomicBool,
    }

    impl FakeHotkeyBackend {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn fail_registration(&self) {
            self.fail_registration.store(true, Ordering::SeqCst);
        }

        /// Test helper: simulates the registered hotkey firing.
        pub(crate) fn trigger(&self) {
            if let Some(callback) = self.registered.lock().unwrap().as_ref() {
                callback();
            }
        }
    }

    impl clip_platform::hotkeys::HotkeyBackend for FakeHotkeyBackend {
        fn register(
            &self,
            _binding: clip_platform::hotkeys::HotkeyBinding,
            callback: Box<dyn Fn() + Send + Sync>,
        ) -> Result<(), clip_platform::hotkeys::HotkeyError> {
            if self.fail_registration.load(Ordering::SeqCst) {
                return Err(clip_platform::hotkeys::HotkeyError::Unsupported);
            }
            *self.registered.lock().unwrap() = Some(callback);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fakes::{FakeBackend, FakeEventPublisher, FakeHotkeyBackend, FakeStore};
    use super::*;
    use clip_core::models::Clip;

    fn fake_hotkeys() -> Arc<dyn clip_platform::hotkeys::HotkeyBackend> {
        Arc::new(FakeHotkeyBackend::new())
    }

    #[test]
    fn fake_store_backend_and_event_publisher_satisfy_their_traits() {
        let store: Box<dyn Store> = Box::new(FakeStore::new());
        let backend: Box<dyn Backend> = Box::new(FakeBackend::new());
        let events: Box<dyn EventPublisher> = Box::new(FakeEventPublisher::new());

        store.insert_clip(&Clip::new("c1", "hash1", "text/plain", vec![])).unwrap();
        assert!(store.get_clip("c1").unwrap().is_some());

        backend.start(Box::new(|_| {})).unwrap();
        assert!(!backend.capabilities().capture);

        events.publish(clip_ipc::protocol::Event::HotkeyPressed);
    }

    #[test]
    fn hotkey_registration_publishes_hotkey_pressed_when_triggered() {
        let store = FakeStore::new();
        let hotkeys = Arc::new(FakeHotkeyBackend::new());
        let events = Arc::new(FakeEventPublisher::new());

        register_hotkey(&store, &(hotkeys.clone() as Arc<dyn clip_platform::hotkeys::HotkeyBackend>), events.clone());
        hotkeys.trigger();

        assert_eq!(events.events(), vec![clip_ipc::protocol::Event::HotkeyPressed]);
    }

    #[test]
    fn an_invalid_persisted_binding_is_skipped_without_publishing_or_panicking() {
        let store = FakeStore::new();
        store
            .update_settings(&AppSettings { hotkey_binding: "NotAKey+++".to_string(), ..AppSettings::default() })
            .unwrap();
        let hotkeys = Arc::new(FakeHotkeyBackend::new());
        let events = Arc::new(FakeEventPublisher::new());

        register_hotkey(&store, &(hotkeys.clone() as Arc<dyn clip_platform::hotkeys::HotkeyBackend>), events.clone());

        assert!(events.events().is_empty());
    }

    #[test]
    fn a_hotkey_registration_failure_is_swallowed_without_publishing_or_panicking() {
        let store = FakeStore::new();
        let hotkeys = Arc::new(FakeHotkeyBackend::new());
        hotkeys.fail_registration();
        let events = Arc::new(FakeEventPublisher::new());

        register_hotkey(&store, &(hotkeys.clone() as Arc<dyn clip_platform::hotkeys::HotkeyBackend>), events.clone());

        assert!(events.events().is_empty());
    }

    #[test]
    fn a_wayland_display_value_selects_the_wayland_session() {
        assert!(is_wayland_session(Some("wayland-0")));
    }

    #[test]
    fn no_wayland_display_selects_the_x11_session() {
        assert!(!is_wayland_session(None));
    }

    fn temp_db_and_socket() -> (tempfile::TempDir, String, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("clipdeck.sqlite3").to_str().unwrap().to_string();
        let socket_path = dir.path().join("clipd.sock");
        (dir, db_path, socket_path)
    }

    async fn wait_until_connectable(socket_path: &std::path::Path) {
        for _ in 0..200 {
            if tokio::net::UnixStream::connect(socket_path).await.is_ok() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("socket at {} never became connectable", socket_path.display());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ipc_socket_is_not_connectable_until_migrations_complete() {
        let (_dir, db_path, socket_path) = temp_db_and_socket();
        let backend: Arc<dyn Backend> = Arc::new(FakeBackend::new());

        assert!(tokio::net::UnixStream::connect(&socket_path).await.is_err(), "socket shouldn't exist yet");

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let run_handle = tokio::spawn(run(db_path, socket_path.clone(), backend, "fake".to_string(), fake_hotkeys(), async {
            let _ = shutdown_rx.await;
        }));

        wait_until_connectable(&socket_path).await;

        let conn = clip_store::db::open(socket_path.with_file_name("clipdeck.sqlite3").to_str().unwrap()).unwrap();
        assert!(clip_store::clips::list(&conn).unwrap().is_empty());

        let _ = shutdown_tx.send(());
        run_handle.await.unwrap().unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_second_instance_fails_fast_when_the_socket_is_already_bound() {
        let (_dir, db_path, socket_path) = temp_db_and_socket();
        let backend: Arc<dyn Backend> = Arc::new(FakeBackend::new());

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let first_handle = tokio::spawn(run(db_path.clone(), socket_path.clone(), backend, "fake".to_string(), fake_hotkeys(), async {
            let _ = shutdown_rx.await;
        }));

        wait_until_connectable(&socket_path).await;

        let second_backend: Arc<dyn Backend> = Arc::new(FakeBackend::new());
        let result = run(db_path, socket_path, second_backend, "fake".to_string(), fake_hotkeys(), std::future::pending()).await;

        assert!(matches!(result, Err(AppError::AlreadyRunning(_))), "expected AlreadyRunning, got {result:?}");

        let _ = shutdown_tx.send(());
        first_handle.await.unwrap().unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_signal_drains_an_in_flight_command_before_returning() {
        let (_dir, db_path, socket_path) = temp_db_and_socket();
        {
            let conn = clip_store::db::open(&db_path).unwrap();
            clip_store::clips::insert(&conn, &Clip::new("c1", "hash1", "text/plain", vec![])).unwrap();
        }

        let backend = Arc::new(FakeBackend::new());
        backend.set_paste_delay(std::time::Duration::from_millis(150));
        let backend_dyn: Arc<dyn Backend> = backend.clone();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let run_handle = tokio::spawn(run(db_path, socket_path.clone(), backend_dyn, "fake".to_string(), fake_hotkeys(), async {
            let _ = shutdown_rx.await;
        }));

        wait_until_connectable(&socket_path).await;
        let mut client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();

        use tokio::io::AsyncWriteExt;
        let request = clip_ipc::protocol::Request::new(
            "r1",
            clip_ipc::protocol::Command::PasteClip { id: "c1".to_string(), mode: clip_core::models::PasteMode::Auto },
        );
        let line = serde_json::to_string(&request).unwrap() + "\n";
        client.write_all(line.as_bytes()).await.unwrap();

        // Give the server a moment to dispatch and start the (slow) handler,
        // then fire shutdown while it's still mid-flight.
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        let _ = shutdown_tx.send(());

        let (read, _write) = client.into_split();
        let mut reader = tokio::io::BufReader::new(read);
        use tokio::io::AsyncBufReadExt;
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.unwrap();
        let message: clip_ipc::protocol::ServerMessage = serde_json::from_str(&response_line).unwrap();
        match message {
            clip_ipc::protocol::ServerMessage::Response(clip_ipc::protocol::Response::Ok { request_id, .. }) => {
                assert_eq!(request_id, "r1");
            }
            other => panic!("expected an Ok response for the in-flight paste, got {other:?}"),
        }

        run_handle.await.unwrap().unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_ipc_server_answers_commands_even_when_backend_start_never_returns() {
        // Regression test: the real X11/Wayland connections' `start()` blocks
        // the calling thread for the daemon's entire lifetime (a blocking
        // `Receiver::recv()` loop - see `x11::real`/`wayland::real`). `run()`
        // must not let that starve the IPC server of ever being reached.
        let (_dir, db_path, socket_path) = temp_db_and_socket();
        let backend = Arc::new(FakeBackend::new());
        backend.set_blocks_on_start(true);
        let backend_dyn: Arc<dyn Backend> = backend.clone();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let run_handle = tokio::spawn(run(db_path, socket_path.clone(), backend_dyn, "fake".to_string(), fake_hotkeys(), async {
            let _ = shutdown_rx.await;
        }));

        wait_until_connectable(&socket_path).await;
        let mut client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();

        use tokio::io::AsyncWriteExt;
        let request = clip_ipc::protocol::Request::new("r1", clip_ipc::protocol::Command::GetSettings);
        let line = serde_json::to_string(&request).unwrap() + "\n";
        client.write_all(line.as_bytes()).await.unwrap();

        let (read, _write) = client.into_split();
        let mut reader = tokio::io::BufReader::new(read);
        use tokio::io::AsyncBufReadExt;
        let mut response_line = String::new();
        // Well under the fake's 300ms simulated block: this must resolve
        // promptly, proving `run()` doesn't wait for `Backend::start()` to
        // return before it starts serving IPC commands.
        let read_result =
            tokio::time::timeout(std::time::Duration::from_millis(100), reader.read_line(&mut response_line)).await;

        let _ = shutdown_tx.send(());
        run_handle.await.unwrap().unwrap();

        assert!(
            read_result.is_ok(),
            "expected a prompt response even though Backend::start() hasn't returned yet; the server never got to start serving"
        );
        let message: clip_ipc::protocol::ServerMessage = serde_json::from_str(response_line.trim_end()).unwrap();
        assert!(
            matches!(message, clip_ipc::protocol::ServerMessage::Response(clip_ipc::protocol::Response::Ok { .. })),
            "expected an Ok response, got {message:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn capture_continues_with_no_clients_connected_and_after_a_client_disconnects() {
        let (_dir, db_path, socket_path) = temp_db_and_socket();
        let backend = Arc::new(FakeBackend::new());
        let backend_dyn: Arc<dyn Backend> = backend.clone();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let run_handle = tokio::spawn(run(db_path.clone(), socket_path.clone(), backend_dyn, "fake".to_string(), fake_hotkeys(), async {
            let _ = shutdown_rx.await;
        }));

        wait_until_connectable(&socket_path).await;
        // The socket becomes connectable as soon as the listener is bound,
        // slightly before `watch_loop.start()` registers the capture
        // callback on this (multi-threaded) runtime; give it a moment to
        // finish that synchronous setup.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // No client connected at all: capture should still be ingested.
        backend.emit_capture(ClipboardSnapshot {
            representations: vec![ClipRepresentation::new("text/plain", 0).with_text_value("no clients")],
        });

        // A client connects, then disconnects.
        {
            let _client = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        backend.emit_capture(ClipboardSnapshot {
            representations: vec![ClipRepresentation::new("text/plain", 0).with_text_value("after disconnect")],
        });

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let _ = shutdown_tx.send(());
        run_handle.await.unwrap().unwrap();

        let conn = clip_store::db::open(&db_path).unwrap();
        let clips = clip_store::clips::list(&conn).unwrap();
        let texts: Vec<_> = clips.iter().filter_map(|c| c.display_text.clone()).collect();
        assert!(texts.contains(&"no clients".to_string()));
        assert!(texts.contains(&"after disconnect".to_string()));
    }
}
