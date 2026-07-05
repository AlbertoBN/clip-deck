# Ubuntu Clipboard Manager v1 PRD

## Overview

This document defines a concrete product requirements document (PRD) and implementation plan for a Linux-first clipboard manager for Ubuntu that matches the core workflow and power-user value of Ditto while using a more modern architecture and neutral desktop UI. Ditto’s core interaction model is automatic clipboard capture, tray and hotkey activation, searchable history, and pasting selected clips back into the previously focused window.[cite:1][cite:17]

The recommended v1 architecture is a background Rust daemon that owns clipboard capture, storage, search, retention, and paste orchestration, paired with a thin Tauri 2 desktop UI for popup search, tray integration, and settings.[cite:16][cite:17] This split is important on Linux because clipboard and global shortcut behavior can vary by environment, and the most failure-prone code should stay isolated from the UI process.[cite:17]

## Product goals

### Goals

- Capture normal copy operations automatically, without requiring a special “send to clipboard manager” action.
- Provide a fast keyboard-first popup for searching and selecting previous clips.
- Support the most useful Ditto-style behaviors in v1: tray access, global hotkey activation, searchable history, pinned clips, groups, preview, and pasting back to the prior window.[cite:1][cite:17]
- Keep the UI neutral, dense, and distraction-free.
- Store data locally first, with a path to optional sync later.

### Non-goals for v1

- Cross-device sync.
- OCR.
- Scripting and macro engines.
- Full file-content indexing.
- Browser extension integrations.
- Full parity with every Ditto advanced option.

## User and use cases

The primary user is a Linux power user who copies technical text, commands, URLs, code snippets, structured text, and images many times per day and wants clipboard history to behave as a system utility rather than as a document app. Ditto’s reference workflow assumes the user copies content normally, opens the app from the tray or hotkey, then presses Enter or double-clicks to paste the selected clip into the target window.[cite:17]

Core v1 use cases:
- Recover something copied 20 minutes ago.
- Search all recent copied text with low latency.
- Paste a prior clip as rich content or plain text.
- Pin important snippets such as SSH commands or ticket templates.
- Group reusable clips.
- Exclude secrets or sensitive apps from persistence.

## Functional requirements

### Core clipboard requirements

1. The app shall monitor clipboard changes automatically after standard copy actions.
2. The app shall persist captured clips locally in a database.
3. The app shall capture at least plain text, HTML, and image content in v1.
4. The app shall preserve multiple available representations for one clip when possible, such as plain text plus HTML.
5. The app shall deduplicate repeated copies of the same content using a deterministic hash.
6. The app shall support per-app and per-content-type exclusion rules.
7. The app shall allow pause and resume of capture from the tray menu.

### Search and retrieval

1. The app shall support full-text search over captured text using SQLite FTS5.[cite:16]
2. The app shall support prefix matching for incremental search and ranked results from the search index.[cite:16]
3. The app shall allow filtering by type, pinned state, and group.
4. The app shall return search results quickly enough to feel instant for a local desktop app.

### Paste and interaction

1. The app shall expose a global hotkey that opens the popup selector, similar to Ditto’s default activation flow.[cite:17]
2. The app shall allow Enter to paste the selected clip to the previously focused window, matching the core Ditto behavior.[cite:17]
3. The app shall allow plain-text paste mode.
4. The app shall allow opening a full preview of long text, HTML, or images, similar to Ditto’s preview workflow.[cite:17]
5. The app shall expose tray actions for show, hide, pause capture, clear history, settings, and quit. Tauri 2 supports tray creation, tray menus, and tray event listeners in Rust and JavaScript.[cite:17]

### Organization and lifecycle

1. The app shall support pinned clips.
2. The app shall support groups or folders.
3. The app shall support retention settings, including auto-delete windows.
4. The app shall support deletion of single clips and bulk clear actions.
5. The app shall record clip metadata such as creation time, last used time, source app when available, MIME types, and byte size.

## UX requirements

The UI should feel closer to a command palette than to a dashboard. Ditto itself centers on tray access, hotkeys, keyboard bindings, Enter-to-paste, and a list-driven popup.[cite:17]

### UX principles

- Neutral UI, low saturation, minimal decorative styling.
- Dense list rows optimized for keyboard use.
- Popup first, manager window second.
- Search is focused on open.
- Common actions must be reachable with keyboard only.
- Long text and HTML need a readable preview pane.

### Main surfaces

| Surface | Purpose | Notes |
|---|---|---|
| Popup picker | Fast search and paste | Opens from global hotkey; search field focused by default.[cite:17] |
| Main manager | Browse, filter, group, delete, inspect | Opens from tray menu or expanded popup action. |
| Preview dialog/pane | Full text, HTML, image preview | Ditto exposes dedicated preview behavior for full text or image/html inspection.[cite:17] |
| Tray menu | Utility entry point | Tauri 2 supports tray icon creation, menus, and event handling.[cite:17] |
| Settings | Rules, retention, hotkeys, diagnostics | Includes backend capability status for X11/Wayland. |

## Technical architecture

### High-level architecture

The recommended v1 architecture is:

```text
+------------------------+      IPC / events      +------------------------+
| clip-ui (Tauri 2)      | <--------------------> | clipd (Rust daemon)    |
| popup, manager, tray   |                        | watch, store, search   |
| neutral React/TS UI    |                        | paste, rules, previews |
+------------------------+                        +------------------------+
                                                             |
                                                             v
                                                    +------------------+
                                                    | SQLite + FTS5    |
                                                    | blobs/previews   |
                                                    +------------------+
```

The daemon owns the risky platform integration code so the app can keep collecting clips even if the UI is closed or restarted. This is especially important on Linux, where tray, shortcut, and clipboard behavior may vary by environment; Tauri 2’s tray support makes it a good fit for the UI shell while leaving OS-sensitive work to Rust.[cite:17]

### Runtime components

#### 1. `clipd` daemon

Responsibilities:
- Clipboard monitoring.
- MIME-aware capture and normalization.
- Exclusion and privacy filtering.
- Deduplication.
- Storage and search.
- Paste orchestration.
- Event publication to UI clients.
- Diagnostics and capability reporting.

#### 2. `clip-ui` desktop shell

Responsibilities:
- Popup picker UI.
- Main manager window.
- Preview views.
- Settings and diagnostics.
- Tray icon and tray menu.
- Local interaction with daemon over IPC.

#### 3. `SQLite` data store

Responsibilities:
- Durable clip metadata.
- FTS5 search index.[cite:16]
- Group and rule storage.
- Event log.
- Optional WAL-backed local reliability.

#### 4. `blob store`

Responsibilities:
- Large images and binary payloads on disk.
- Cached thumbnails and previews.

## Proposed Rust workspace

```text
clip-deck/
├── Cargo.toml
├── crates/
│   ├── clip-core/
│   ├── clip-store/
│   ├── clip-platform/
│   ├── clip-ipc/
│   ├── clipd/
│   └── clip-ui-tauri/
├── migrations/
│   ├── 001_init.sql
│   └── 002_seed_defaults.sql
├── docs/
│   ├── prd.md
│   ├── architecture.md
│   └── wayland-notes.md
└── assets/
```

## Module plan

### `clip-core`

Shared domain logic and models.

Suggested modules:
- `models`: `Clip`, `ClipRepresentation`, `Group`, `Rule`, `AppContext`, `PasteMode`.
- `mime`: canonical representation handling.
- `hashing`: stable content hashing.
- `search`: query parsing helpers and ranking inputs.
- `errors`: shared error types.
- `config`: settings model and defaults.

### `clip-store`

Persistence and search.

Suggested modules:
- `db`: connection pool, pragmas, migrations.
- `clips`: insert, update, delete, list, get.
- `fts`: FTS5 synchronization and search queries.[cite:16]
- `groups`: CRUD and hierarchy.
- `rules`: CRUD for exclusion and privacy rules.
- `events`: audit/event log.
- `retention`: auto-delete and pruning jobs.

### `clip-platform`

Linux integration boundary.

Suggested modules:
- `clipboard`: backend trait.
- `x11`: X11 clipboard and automation adapter.
- `wayland`: Wayland adapter and capability detection.
- `hotkeys`: global hotkey registration.
- `focus`: focused app/window discovery where available.
- `paste`: synthetic paste and plain-text paste.
- `tray_support`: optional tray helpers shared with UI.
- `diagnostics`: platform support report.

### `clip-ipc`

Transport between daemon and UI.

Suggested modules:
- `protocol`: commands, events, DTOs.
- `server`: daemon-side Unix socket or DBus server.
- `client`: UI-side client.
- `auth`: local-socket scope and single-user protections.

### `clipd`

Application daemon.

Suggested modules:
- `main`: startup and lifecycle.
- `app`: dependency wiring.
- `watch_loop`: clipboard event loop.
- `ingest`: normalization pipeline.
- `commands`: IPC command handlers.
- `jobs`: retention, thumbnailing, cleanup.
- `telemetry`: structured logs and debug output.

### `clip-ui-tauri`

Desktop app shell.

Suggested modules:
- `src-tauri`: Tauri host, commands, tray bootstrap.
- `src/app`: React app.
- `src/views/popup`: search-first popup.
- `src/views/manager`: full manager window.
- `src/views/settings`: settings and diagnostics.
- `src/components`: list, preview, filter bar, badges, keyboard hints.
- `src/state`: IPC-backed state management.

## Recommended crate choices

The stack below is intentionally conservative and optimized for Linux desktop reliability.

| Area | Crate choice | Why |
|---|---|---|
| Async runtime | `tokio` | Mature async runtime for daemon, IPC, jobs, and UI host integration. |
| Serialization | `serde`, `serde_json` | Standard Rust data model serialization. |
| Error handling | `thiserror`, `anyhow` | Clear domain errors plus ergonomic app errors. |
| Logging | `tracing`, `tracing-subscriber` | Structured logs and filtering for platform troubleshooting. |
| SQLite | `sqlx` or `rusqlite` | `rusqlite` is simpler for SQLite-first apps; `sqlx` is stronger if async DB access is preferred. |
| Migrations | `sqlx::migrate!` or `refinery` | Simple reproducible schema migrations. |
| UUIDs | `uuid` | Stable clip IDs. |
| Time | `time` or `chrono` | Timestamps and retention logic. |
| Hashing | `blake3` | Fast, deterministic content hashing for dedupe. |
| IPC | `tokio::net` Unix sockets or `zbus` | Unix sockets are simpler; DBus is more desktop-native. |
| Tray/UI shell | `tauri` | Modern, lightweight desktop shell with tray support.[cite:17] |
| Frontend | React + TypeScript | Fast UI iteration and good keyboard UX ergonomics. |
| Hotkeys | evaluate `global-hotkey` or Tauri plugin path | Depends on Linux environment support and Wayland behavior. |
| Images | `image` | Thumbnails and preview metadata. |
| Config paths | `directories` | Standard app config/data/cache locations. |
| MIME parsing | `mime`, `mime_guess` | Representation typing and validation. |

### Preferred persistence choice

For this project, `rusqlite` is the better default because the app is SQLite-native, local-only in v1, and likely to benefit from explicit transactional control more than from async query fan-out. SQLite FTS5 is built around normal SQL tables plus virtual tables and specialized `MATCH` queries, which maps well to a direct SQL approach.[cite:16]

## Proposed data model

The data model should preserve one canonical clip plus multiple representations, because the app may capture text, HTML, and image data from the same clipboard event. Ditto’s documented behavior and preview model imply that retaining multiple meaningful forms of the same clip is important for a useful clipboard history.[cite:17]

### Entities

- `clips`: canonical item metadata.
- `clip_representations`: one row per MIME representation.
- `clips_fts`: FTS5 index for text search.[cite:16]
- `groups`: logical organization.
- `app_rules`: exclusions and privacy rules.
- `settings`: key-value configuration.
- `events`: audit and usage trail.

### Proposed SQLite migration file

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_group_id TEXT NULL REFERENCES groups(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS clips (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_used_at TEXT NULL,
    source_app TEXT NULL,
    source_window TEXT NULL,
    primary_mime TEXT NOT NULL,
    display_text TEXT NULL,
    content_hash TEXT NOT NULL,
    byte_size INTEGER NOT NULL DEFAULT 0,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    group_id TEXT NULL REFERENCES groups(id) ON DELETE SET NULL,
    paste_mode_default TEXT NOT NULL DEFAULT 'auto',
    metadata_json TEXT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_clips_hash_mime
ON clips(content_hash, primary_mime, is_deleted);

CREATE INDEX IF NOT EXISTS idx_clips_created_at
ON clips(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_clips_last_used_at
ON clips(last_used_at DESC);

CREATE INDEX IF NOT EXISTS idx_clips_group_id
ON clips(group_id);

CREATE INDEX IF NOT EXISTS idx_clips_pinned
ON clips(is_pinned, created_at DESC);

CREATE TABLE IF NOT EXISTS clip_representations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    clip_id TEXT NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
    mime_type TEXT NOT NULL,
    text_value TEXT NULL,
    blob_path TEXT NULL,
    preview_text TEXT NULL,
    width INTEGER NULL,
    height INTEGER NULL,
    byte_size INTEGER NOT NULL DEFAULT 0,
    ordinal INTEGER NOT NULL DEFAULT 0,
    is_preview INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_clip_representations_clip_id
ON clip_representations(clip_id, ordinal);

CREATE INDEX IF NOT EXISTS idx_clip_representations_mime
ON clip_representations(mime_type);

CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
    clip_id UNINDEXED,
    display_text,
    extracted_text,
    source_app UNINDEXED,
    tags,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TABLE IF NOT EXISTS app_rules (
    id TEXT PRIMARY KEY,
    app_match TEXT NOT NULL,
    window_match TEXT NULL,
    mime_match TEXT NULL,
    action TEXT NOT NULL,
    notes TEXT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_app_rules_enabled
ON app_rules(enabled, app_match);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    clip_id TEXT NULL REFERENCES clips(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_clip_id
ON events(clip_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_events_type
ON events(event_type, created_at DESC);

CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips
BEGIN
    INSERT INTO clips_fts (clip_id, display_text, extracted_text, source_app, tags)
    VALUES (new.id, COALESCE(new.display_text, ''), COALESCE(new.display_text, ''), COALESCE(new.source_app, ''), '');
END;

CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips
BEGIN
    DELETE FROM clips_fts WHERE clip_id = old.id;
    INSERT INTO clips_fts (clip_id, display_text, extracted_text, source_app, tags)
    VALUES (new.id, COALESCE(new.display_text, ''), COALESCE(new.display_text, ''), COALESCE(new.source_app, ''), '');
END;

CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips
BEGIN
    DELETE FROM clips_fts WHERE clip_id = old.id;
END;
```

### Schema notes

- Large binary payloads should live on disk, referenced by `blob_path`, so the main SQLite file stays lean.
- `display_text` is the primary search and list field.
- `metadata_json` can store environment-specific payloads without forcing schema churn.
- FTS synchronization can also be done in application code rather than triggers if ingest throughput or observability becomes a concern.[cite:16]

## IPC contract

A Unix domain socket is the simplest v1 transport. If tighter desktop integration becomes important later, DBus can be added behind the same protocol abstraction.

### Commands

- `SearchClips { query, filters, limit, offset }`
- `GetClip { id }`
- `PasteClip { id, mode }`
- `PinClip { id, pinned }`
- `AssignGroup { id, group_id }`
- `DeleteClip { id }`
- `ClearHistory { scope }`
- `ListGroups`
- `SaveRule`
- `DeleteRule`
- `GetSettings`
- `UpdateSettings`
- `GetDiagnostics`
- `PauseCapture { paused }`

### Events

- `ClipCaptured`
- `ClipUpdated`
- `ClipDeleted`
- `CapturePaused`
- `DiagnosticsChanged`
- `HotkeyPressed`

## Search design

SQLite FTS5 supports full-text search through virtual tables, prefix queries, phrase search, boolean operators, and NEAR queries, which is more than enough for a fast local clipboard manager search experience.[cite:16]

Suggested query strategy:
- Incremental UI search uses prefix matching for the last term.
- Ranking uses BM25 or a hybrid score that also boosts recency and pinned status.
- Empty query falls back to recent clips sorted by `created_at DESC` and then pinned first.
- Search filters narrow by MIME family, group, source app, and favorite state.

## Clipboard and platform strategy

### Adapter trait

```rust
pub trait ClipboardBackend {
    fn start(&mut self, tx: EventSender) -> anyhow::Result<()>;
    fn read_current(&self) -> anyhow::Result<ClipboardSnapshot>;
    fn set_current(&self, clip: &ClipPayload) -> anyhow::Result<()>;
    fn focused_app(&self) -> anyhow::Result<Option<AppContext>>;
    fn simulate_paste(&self, mode: PasteMode) -> anyhow::Result<()>;
    fn capabilities(&self) -> BackendCapabilities;
}
```

### X11 strategy

- Prioritize as the first fully supported environment.
- Implement full capture, previous-window targeting, and paste automation.
- Use X11 support as the baseline for end-to-end acceptance tests.

### Wayland strategy

- Build as a separate adapter with explicit capability reporting.
- Support automatic capture where compositor behavior allows it.
- Degrade gracefully when global hotkeys or automation are partially constrained.
- Surface status in Settings so users understand what is supported on their system.

This separation is necessary because Linux desktop behavior is not uniform, and tray and event support do not imply that automation behavior is equally available everywhere.[cite:17]

## Security and privacy requirements

- Exclusion rules by source app and MIME type.
- Optional ephemeral mode that stores only in memory until disabled.
- Auto-delete policies.
- “Never persist” mode for known sensitive apps.
- Clear history command from tray.
- No cloud sync in v1.
- Store only local paths for blobs; never expose arbitrary external network upload behavior.

## Non-functional requirements

| Area | Requirement |
|---|---|
| Startup | Daemon starts automatically and is ready soon after login. |
| Reliability | UI restarts must not interrupt clipboard capture. |
| Search latency | Typical search should feel instant for local use. |
| Storage | Database should remain healthy under tens of thousands of clips. |
| UX | Popup must be keyboard-first and low-friction. |
| Diagnostics | App must report environment support state clearly. |
| Accessibility | Basic keyboard accessibility throughout. |

## Milestones and implementation plan

### Milestone 0: architecture spike

Deliverables:
- Workspace skeleton.
- Daemon/UI split proof.
- Migration runner.
- Basic IPC skeleton.
- Diagnostic report for current environment.

### Milestone 1: end-to-end text capture on X11

Deliverables:
- Clipboard watch loop.
- Text capture and dedupe.
- SQLite persistence.
- Recent clips list.
- Popup with search box.
- Enter-to-paste text flow.

Acceptance:
- Copying plain text stores it automatically.
- Popup opens from hotkey and pastes selected text back to the previous app.
- Search over recent text clips works correctly.

### Milestone 2: rich content and previews

Deliverables:
- HTML capture.
- Image capture and thumbnailing.
- Full preview pane/dialog.
- Plain-text paste mode.

Acceptance:
- HTML and images are captured where available.
- Preview pane renders long content clearly.
- User can choose normal paste or plain-text paste.

### Milestone 3: organization and lifecycle

Deliverables:
- Pinned clips.
- Groups.
- Deletion and bulk clear.
- Retention jobs.
- App exclusion rules.

Acceptance:
- User can pin, group, delete, and auto-prune clips.
- Exclusion rules prevent persistence for matched apps.

### Milestone 4: Wayland adapter and diagnostics

Deliverables:
- Wayland backend implementation.
- Capability checks.
- Environment-specific diagnostics UI.
- Fallback behavior and clear messaging.

Acceptance:
- App works on supported Wayland setups.
- Unsupported capabilities are surfaced clearly instead of failing silently.

## Delivery plan by module

| Order | Module | Primary output |
|---|---|---|
| 1 | `clip-core` | Domain types and config |
| 2 | `clip-store` | DB layer, migrations, FTS queries |
| 3 | `clip-ipc` | Commands/events transport |
| 4 | `clip-platform::x11` | Text capture and paste loop |
| 5 | `clipd` | Watch loop and service composition |
| 6 | `clip-ui-tauri` | Popup, tray, manager shell |
| 7 | `clip-platform::rich` | HTML/image support |
| 8 | `clip-platform::wayland` | Wayland adapter and diagnostics |

## Testing strategy

### Unit tests

- Hashing and dedupe logic.
- MIME normalization.
- Search query construction.
- Rule evaluation.
- Retention policy.

### Integration tests

- Migration application.
- Insert/search/delete flows.
- FTS synchronization correctness.
- IPC request/response contract.

### Manual acceptance tests

- Copy/paste loop in terminal, browser, editor, and chat app.
- X11 and Wayland runs on Ubuntu.
- Hotkey activation under different session types.
- Pause capture, exclusion rules, and clear history.

## Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Wayland restrictions vary by compositor | High | Separate adapter, diagnostics, graceful degradation. |
| Hotkey support differs by environment | High | Make backend capabilities visible; support tray and manual popup as fallback.[cite:17] |
| Rich clipboard formats are inconsistent | Medium | Treat representations independently and persist best-effort forms. |
| Large binary payload growth | Medium | Keep blobs on disk and prune aggressively. |
| UI shell crash interrupts workflow | Medium | Keep daemon headless and resilient. |

## Recommended first sprint backlog

1. Create workspace and crates.
2. Add `rusqlite`, `tokio`, `serde`, `tracing`, `blake3`, `uuid`, `directories`.
3. Implement migration runner and apply `001_init.sql`.
4. Implement `Clip` and `ClipRepresentation` models.
5. Implement text capture path only.
6. Build popup UI with keyboard list navigation.
7. Implement `PasteClip` for text.
8. Add tray icon and menu.
9. Add recent-history and search queries.
10. Add diagnostics screen with backend capability report.

## Final recommendation

For v1, the highest-probability path is a Rust-first clipboard daemon with a Tauri 2 shell, SQLite plus FTS5 for search, disk-backed blobs for rich content, and a strict adapter boundary between X11 and Wayland behavior. That combination aligns with Ditto’s tray-plus-hotkey-plus-search workflow while keeping Linux integration concerns isolated and debuggable.[cite:16][cite:17]
