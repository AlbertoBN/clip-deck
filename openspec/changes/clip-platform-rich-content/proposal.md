## Why

Milestone 1 (and `clip-platform-x11-adapter`) intentionally captured plain text only. Per the PRD's
Milestone 2, ClipDeck needs HTML and image capture with thumbnailing to be a useful Ditto-equivalent, and
paste needs to choose between rich and plain-text representations rather than always pasting plain text.

## What Changes

- Extend X11 capture to read HTML and image representations from the clipboard alongside plain text,
  producing a multi-representation `ClipboardSnapshot` per copy event.
- Implement image thumbnail generation (via the `image` crate) for captured images and for preview use.
- Extend paste simulation so `PasteMode::Auto` selects the richest available representation (HTML over
  plain text) instead of always defaulting to plain text, while `PasteMode::PlainText` continues to strip
  down to plain text regardless of what else is available.

## Capabilities

### New Capabilities
- `rich-clipboard-capture`: Reading HTML and image representations from the X11 clipboard in addition to
  plain text, producing all available representations for one copy event.
- `thumbnail-generation`: Generating and caching a thumbnail image for a captured image representation,
  for use by preview and list-row rendering.

### Modified Capabilities
- `paste-simulation`: `PasteMode::Auto` now selects the richest representation (HTML over plain text) when
  more than one is available, instead of the Milestone-1 behavior of only ever having plain text to choose
  from. `PasteMode::PlainText` behavior is unchanged but now has real richer representations to strip down
  from.

## Impact

- Affected code: `crates/clip-platform/src/x11/mod.rs` (extended read path), `crates/clip-platform/src/
  paste.rs` (representation selection), `crates/clip-platform/Cargo.toml` (adds the `image` crate,
  already listed in the workspace's shared dependencies).
- Depends on: `clip-platform-x11-adapter` (extends its capture and paste-simulation code), `clip-core-
  foundations` (`MimeFamily`, `ClipRepresentation`).
- Downstream: `clipd-daemon-core`'s ingest pipeline already persists whatever representations a snapshot
  contains (per `clip-ingest-pipeline`'s "one snapshot, multiple representations" requirement), so no
  change is needed there; `clip-ui-tauri-shell`'s preview pane (already spec'd to render HTML/image) simply
  starts receiving real HTML/image data instead of only plain text.
