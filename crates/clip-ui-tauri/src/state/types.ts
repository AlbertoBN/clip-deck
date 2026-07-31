export type PasteMode = 'auto' | 'rich' | 'plain_text'

export interface ClipRepresentation {
  mime_type: string
  text_value: string | null
  blob_path: string | null
  preview_text: string | null
  width: number | null
  height: number | null
  byte_size: number
  ordinal: number
  is_preview: boolean
}

export interface Clip {
  id: string
  created_at: string
  updated_at: string
  last_used_at: string | null
  source_app: string | null
  source_window: string | null
  primary_mime: string
  display_text: string | null
  content_hash: string
  byte_size: number
  is_favorite: boolean
  is_pinned: boolean
  is_deleted: boolean
  paste_mode_default: PasteMode
  metadata: unknown
  representations: ClipRepresentation[]
}

export type RuleAction = 'exclude' | 'ephemeral'

export interface Rule {
  id: string
  app_match: string
  window_match: string | null
  mime_match: string | null
  action: RuleAction
  enabled: boolean
}

export interface AppSettings {
  hotkey_binding: string
  retention_window_days: number | null
  capture_paused: boolean
  default_paste_mode: PasteMode
}

export interface BackendCapabilities {
  capture: boolean
  paste_simulation: boolean
  hotkeys: boolean
  focus_detection: boolean
}

export interface DiagnosticsReport {
  backend: string
  capabilities: BackendCapabilities
}

export type ClearScope = 'all' | 'excluding_pinned'

export interface SearchFilters {
  mime_family?: 'text' | 'html' | 'image' | 'other' | null
  pinned_only?: boolean
  favorite_only?: boolean
  source_app?: string | null
}

export type DaemonEvent =
  | { type: 'ClipCaptured'; clip_id: string }
  | { type: 'ClipUpdated'; clip_id: string }
  | { type: 'ClipDeleted'; clip_id: string }
  | { type: 'CapturePaused'; paused: boolean }
  | { type: 'DiagnosticsChanged' }
  | { type: 'HotkeyPressed' }

export const DAEMON_EVENT_CHANNEL = 'daemon-event'
