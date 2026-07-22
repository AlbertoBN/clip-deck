## 1. Fake connection extension

- [x] 1.1 Extend the fake `X11Connection` (from `clip-platform-x11-adapter`) to support configuring
      HTML and image selection targets in addition to plain text.

## 2. Rich clipboard capture (`rich-clipboard-capture`)

- [x] 2.1 Write a failing test asserting `read_current` returns both plain-text and HTML representations
      when both are offered, per `specs/rich-clipboard-capture/spec.md`.
- [x] 2.2 Run `cargo test -p clip-platform x11::` and confirm failure.
- [x] 2.3 Implement HTML target reading in `crates/clip-platform/src/x11/mod.rs` - minimum code to pass.
- [x] 2.4 Write a failing test asserting an image target produces an image representation with the
      correct MIME type.
- [x] 2.5 Add the `image` crate to `crates/clip-platform/Cargo.toml` and implement image target reading
      and decoding.
- [x] 2.6 Write a failing test asserting a captured image representation reports its width and height.
- [x] 2.7 Implement dimension extraction from the decoded image.
- [x] 2.8 Write a failing test asserting an undecodable image target is skipped while a valid plain-text
      representation is still returned.
- [x] 2.9 Implement per-representation decode-failure isolation; run `cargo test -p clip-platform` and
      confirm all green.

## 3. Thumbnail generation (`thumbnail-generation`)

- [x] 3.1 Write a failing test asserting a large captured image produces a thumbnail within the configured
      maximum bound, per `specs/thumbnail-generation/spec.md`.
- [x] 3.2 Run the thumbnail test module and confirm failure.
- [x] 3.3 Implement bounded thumbnail generation via `image::imageops::thumbnail` - minimum code to pass.
- [x] 3.4 Write a failing test asserting a non-square image's thumbnail preserves its aspect ratio.
- [x] 3.5 Confirm/adjust the implementation to preserve aspect ratio (should already hold given
      `thumbnail`'s semantics; test locks in the guarantee).
- [x] 3.6 Write a failing test asserting a thumbnail-generation failure still results in the clip's full
      image representation being returned/persisted, with no thumbnail reference set.
- [x] 3.7 Implement failure isolation for the thumbnailing step; run `cargo test -p clip-platform` and
      confirm all green.

## 4. Paste-mode representation selection (`paste-simulation`, modified)

- [x] 4.1 Re-run the existing `paste-simulation` test suite from `clip-platform-x11-adapter` and confirm
      it still passes unmodified before making changes.
- [x] 4.2 Write a failing test asserting `PasteMode::Auto` selects the HTML representation when a clip has
      both HTML and plain-text representations, per this change's modified
      `specs/paste-simulation/spec.md`.
- [x] 4.3 Implement the richest-representation selection for `Auto` mode in
      `crates/clip-platform/src/paste.rs`.
- [x] 4.4 Write a failing test asserting `PasteMode::PlainText` still selects the plain-text representation
      when a clip has both HTML and plain-text representations.
- [x] 4.5 Confirm/adjust `PlainText` handling to ignore richer representations; run
      `cargo test -p clip-platform` and confirm all green.

## 5. Crate-level verification

- [x] 5.1 Run `cargo test -p clip-platform` and confirm every test from sections 2-4 passes alongside the
      unmodified `clip-platform-x11-adapter` test suite.
- [x] 5.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [x] 5.3 Run `cargo clippy -p clip-platform -- -D warnings` and fix any lints introduced by this change.
- [ ] 5.4 Manually verify on a real X11 session: copying rich text from a browser captures both plain-text
      and HTML, copying an image captures it with a thumbnail, and Auto-mode paste reproduces the richer
      representation in a target that accepts HTML. Record the result in the PR description.

## 6. Amendments: extensions discovered during implementation

- [x] 6.1 Add `read_selection_target` to the `X11Connection` trait (`crates/clip-platform/src/x11/mod.rs`),
      implement it on the fake and on `RealX11Connection` (`crates/clip-platform/src/x11/real.rs`), driven
      by the section 2 tests above rather than a separate red/green cycle of its own (it's the read-path
      seam those tests exercise).
- [x] 6.2 Add a `blob_dir: PathBuf` parameter to `X11Backend::new` (`crates/clip-platform/src/x11/mod.rs`)
      so captured images/thumbnails can be written to disk and `blob_path` set on their representations;
      update every existing call site (`x11/mod.rs` tests, `x11/real.rs`'s ignored integration test, and
      `clipd-daemon-core`'s `X11DaemonBackend::connect` in `crates/clipd/src/app.rs`, now resolving
      `AppPaths::resolve().data_dir.join("blobs")`).
- [x] 6.3 Write a failing regression test asserting `X11Backend::start` still emits a capture event for an
      image-only copy (no text representation), confirm it fails against the original
      first-representation-`text_value`-only hash computation, then fix `identity_hash` to fall back to
      `blob_path` when `text_value` is absent; run `cargo test -p clip-platform` and confirm green.
- [x] 6.4 Correct the thumbnail-sizing implementation: `image::imageops::thumbnail` resamples to the exact
      box given rather than preserving aspect ratio on its own (the non-square-aspect-ratio test in section
      3 caught this) - compute the target width/height from the source's aspect ratio, bounded to
      `MAX_THUMBNAIL_DIM` on the long edge, before calling it. Update `design.md`'s "Thumbnail sizing"
      decision to record the correction.
- [x] 6.5 Run `cargo test -p clipd` and `cargo clippy -p clipd --all-targets -- -D warnings` to confirm the
      `X11DaemonBackend::connect` call-site update (6.2) didn't regress the dependent crate.
