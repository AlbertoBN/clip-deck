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

### Amendment (discovered during implementation)

Four things surfaced while implementing that this proposal didn't anticipate:

- **`X11Connection` gained a new trait method**, `read_selection_target(&self, mime: &str) -> Option<Vec<u8>>`,
  generalizing the existing text-only `read_selection` to any selection target (`"text/html"`,
  `"image/png"`). Implemented for both the fake (`crates/clip-platform/src/x11/mod.rs`) and
  `RealX11Connection` (`crates/clip-platform/src/x11/real.rs`, mirroring `read_selection`'s
  `ConvertSelection`/`SelectionNotify` flow, generalized to an arbitrary target atom).
- **`X11Backend::new` gained a required `blob_dir` parameter** (`crates/clip-platform/src/x11/mod.rs`):
  captured images and their thumbnails are written to disk there (content-addressed by `blake3` hash,
  matching the project's existing dedup-hashing convention), since `ClipRepresentation` has no in-memory
  raw-bytes carrier field for a later layer to write from - `clip-platform` is the layer that already
  decodes the image, so it's also the natural place to write the blob and set `blob_path` itself. This
  required a one-line downstream update to `clipd-daemon-core`'s `X11DaemonBackend::connect()`
  (`crates/clipd/src/app.rs`), which now resolves `AppPaths::resolve().data_dir.join("blobs")` and passes
  it through.
- **`X11Backend::start`'s dedup-hash computation was buggy for image-only copies**: it hashed only the
  first representation's `text_value`, which is `None` for an image-only snapshot, so `None != None` never
  triggered a capture event. Fixed by falling back to the (already content-addressed) `blob_path` when
  `text_value` is absent - see `design.md`'s amendment note.
- **`image::imageops::thumbnail` does not preserve aspect ratio on its own** - it resamples to the *exact*
  box given. `design.md`'s original claim that it does was wrong; the fix computes a target box from the
  source's aspect ratio (bounded to 256px on the long edge) before calling it. See `design.md`'s amendment
  note under "Decisions" for detail.
