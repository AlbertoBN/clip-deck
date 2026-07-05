## Context

`clip-platform-x11-adapter` implemented plain-text-only capture and a plain-text-only paste path, using a
fake `X11Connection` for most tests and `x11rb` for the real implementation. This change extends both the
capture side (more target formats) and the paste side (representation selection) without changing the
`ClipboardBackend` trait shape itself.

## Goals / Non-Goals

**Goals:**
- Multi-representation capture (plain text + HTML + image) from a single X11 copy event.
- Thumbnails for captured images, generated synchronously at capture time (small/fast enough not to block
  the watch loop meaningfully; revisit if profiling says otherwise).
- `PasteMode::Auto` prefers richer representations without breaking `PasteMode::PlainText`'s existing
  guarantee.

**Non-Goals:**
- No OCR (explicit PRD non-goal).
- No change to `clip-ingest-pipeline` or `clip-store`'s persistence - both already handle multi-
  representation clips generically.
- No Wayland rich-content support - that lives entirely in `clip-platform-wayland-adapter`.

## Decisions

- **HTML capture**: read the `text/html` X11 selection target (UTF-8 bytes) as-is; no sanitization here -
  sanitization for *display* is `clip-ui-tauri-shell`'s `preview-pane-ui` concern (via `ammonia`), so the
  captured/stored HTML remains the faithful original for potential future use cases (e.g. re-paste as
  HTML).
- **Image capture**: read the `image/png` X11 selection target (X11 clipboards conventionally offer PNG
  even for other source formats); decode via the `image` crate to obtain width/height and to normalize to
  PNG for the on-disk blob, rather than storing whatever raw bytes X11 handed back.
- **Thumbnail sizing**: max 256px on the long edge, `image::imageops::thumbnail` (preserves aspect ratio
  by construction), stored as a separate file next to the full image's `blob_path` rather than embedded in
  the database.
- **Auto paste-mode selection**: a small ranking (`Html > Text`, extendable later) picks the richest
  representation off the clip's existing `ClipRepresentation` list; this is pure selection logic against
  already-persisted representations, not a new capture concern, so it lives in `clip-platform::paste`
  exactly where the Milestone-1 plain-text path already lives.
- **Decode-failure isolation**: each representation's decode attempt is wrapped so one failing target does
  not abort the whole `read_current` call - matches the "skip, don't fail" requirement.

## Test strategy

- `rich-clipboard-capture`: extend the existing fake `X11Connection` (from `clip-platform-x11-adapter`) to
  offer configurable HTML/image targets; tests for both-representations-returned, image-dimensions-
  reported, and undecodable-image-skipped-others-kept. Run with `cargo test -p clip-platform x11::`.
- `thumbnail-generation`: unit tests against in-memory generated test images (no real file I/O required
  beyond a temp dir) - bounded-size test, aspect-ratio-preserved test (assert ratio within a small
  tolerance), thumbnail-failure-does-not-block-persistence test (inject a decode failure only in the
  thumbnail step via a fakeable thumbnailer trait). Run with `cargo test -p clip-platform thumbnail::` (or
  a `x11::` submodule if colocated).
- `paste-simulation` (modified): tests already existing from `clip-platform-x11-adapter` continue to pass
  unmodified; new tests add the Auto-prefers-HTML scenario and the PlainText-strips-HTML-when-both-
  present scenario, both against the fake connection with a clip carrying two representations.

Red-green-refactor: write each new/modified test against the extended fake connection first, confirm
failure, implement the minimum decode/selection logic to pass, run `cargo test -p clip-platform` for the
full crate, then refactor with tests green.

## Risks / Trade-offs

- [Risk] Synchronous thumbnail generation in the capture path could add latency to the watch loop →
  Mitigation: bounded thumbnail size keeps the operation fast in practice; if a task's manual verification
  shows otherwise, moving thumbnailing to `clipd`'s background jobs is a follow-up change, not something to
  guess at now.
- [Risk] X11 clipboard image format assumptions (PNG) may not hold for every source application →
  Mitigation: decode failures are non-fatal per `rich-clipboard-capture`'s spec, so an unexpected format
  degrades to "no image representation" rather than crashing capture.
