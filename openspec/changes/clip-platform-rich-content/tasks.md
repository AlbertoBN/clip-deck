## 1. Fake connection extension

- [ ] 1.1 Extend the fake `X11Connection` (from `clip-platform-x11-adapter`) to support configuring
      HTML and image selection targets in addition to plain text.

## 2. Rich clipboard capture (`rich-clipboard-capture`)

- [ ] 2.1 Write a failing test asserting `read_current` returns both plain-text and HTML representations
      when both are offered, per `specs/rich-clipboard-capture/spec.md`.
- [ ] 2.2 Run `cargo test -p clip-platform x11::` and confirm failure.
- [ ] 2.3 Implement HTML target reading in `crates/clip-platform/src/x11/mod.rs` - minimum code to pass.
- [ ] 2.4 Write a failing test asserting an image target produces an image representation with the
      correct MIME type.
- [ ] 2.5 Add the `image` crate to `crates/clip-platform/Cargo.toml` and implement image target reading
      and decoding.
- [ ] 2.6 Write a failing test asserting a captured image representation reports its width and height.
- [ ] 2.7 Implement dimension extraction from the decoded image.
- [ ] 2.8 Write a failing test asserting an undecodable image target is skipped while a valid plain-text
      representation is still returned.
- [ ] 2.9 Implement per-representation decode-failure isolation; run `cargo test -p clip-platform` and
      confirm all green.

## 3. Thumbnail generation (`thumbnail-generation`)

- [ ] 3.1 Write a failing test asserting a large captured image produces a thumbnail within the configured
      maximum bound, per `specs/thumbnail-generation/spec.md`.
- [ ] 3.2 Run the thumbnail test module and confirm failure.
- [ ] 3.3 Implement bounded thumbnail generation via `image::imageops::thumbnail` - minimum code to pass.
- [ ] 3.4 Write a failing test asserting a non-square image's thumbnail preserves its aspect ratio.
- [ ] 3.5 Confirm/adjust the implementation to preserve aspect ratio (should already hold given
      `thumbnail`'s semantics; test locks in the guarantee).
- [ ] 3.6 Write a failing test asserting a thumbnail-generation failure still results in the clip's full
      image representation being returned/persisted, with no thumbnail reference set.
- [ ] 3.7 Implement failure isolation for the thumbnailing step; run `cargo test -p clip-platform` and
      confirm all green.

## 4. Paste-mode representation selection (`paste-simulation`, modified)

- [ ] 4.1 Re-run the existing `paste-simulation` test suite from `clip-platform-x11-adapter` and confirm
      it still passes unmodified before making changes.
- [ ] 4.2 Write a failing test asserting `PasteMode::Auto` selects the HTML representation when a clip has
      both HTML and plain-text representations, per this change's modified
      `specs/paste-simulation/spec.md`.
- [ ] 4.3 Implement the richest-representation selection for `Auto` mode in
      `crates/clip-platform/src/paste.rs`.
- [ ] 4.4 Write a failing test asserting `PasteMode::PlainText` still selects the plain-text representation
      when a clip has both HTML and plain-text representations.
- [ ] 4.5 Confirm/adjust `PlainText` handling to ignore richer representations; run
      `cargo test -p clip-platform` and confirm all green.

## 5. Crate-level verification

- [ ] 5.1 Run `cargo test -p clip-platform` and confirm every test from sections 2-4 passes alongside the
      unmodified `clip-platform-x11-adapter` test suite.
- [ ] 5.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [ ] 5.3 Run `cargo clippy -p clip-platform -- -D warnings` and fix any lints introduced by this change.
- [ ] 5.4 Manually verify on a real X11 session: copying rich text from a browser captures both plain-text
      and HTML, copying an image captures it with a thumbnail, and Auto-mode paste reproduces the richer
      representation in a target that accepts HTML. Record the result in the PR description.
