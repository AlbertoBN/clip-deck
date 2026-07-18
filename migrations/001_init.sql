-- Initial schema: clips, clip_representations, clips_fts, groups, app_rules,
-- settings, events. See the "Proposed SQLite migration file" section of
-- docs/ClipDeck-ubuntu-clipboard-manager-prd.md for the full DDL.

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
