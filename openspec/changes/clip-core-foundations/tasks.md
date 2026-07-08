## 1. Errors and shared scaffolding

- [x] 1.1 Write a failing test in `crates/clip-core/src/errors.rs` asserting `CoreError` variants for
      `InvalidMime`, `InvalidGroupParent`, and `InvalidRule` exist and implement `std::error::Error` +
      `Display` with a non-empty message.
- [x] 1.2 Run `cargo test -p clip-core errors::` and confirm it fails (module has no `CoreError` yet).
- [x] 1.3 Implement `CoreError` (via `thiserror::Error`) with those variants - minimum code to pass.
- [x] 1.4 Run `cargo test -p clip-core` and confirm all green.

## 2. MIME normalization (`mime-normalization`)

- [x] 2.1 Write failing tests for `normalize_mime` (lowercasing, parameter stripping) and `mime_family`
      (Text/Html/Image/Other classification) and malformed-input rejection, per
      `specs/mime-normalization/spec.md`.
- [x] 2.2 Run `cargo test -p clip-core mime::` and confirm the new tests fail (functions don't exist).
- [x] 2.3 Implement `normalize_mime`, `MimeFamily`, and `mime_family` in `crates/clip-core/src/mime.rs`
      using the `mime`/`mime_guess` crates - minimum code to make the tests pass.
- [x] 2.4 Run `cargo test -p clip-core` and confirm all green; refactor for clarity if needed while keeping
      tests green.

## 3. Content hashing (`content-hashing`)

- [x] 3.1 Write failing tests for `hash_content` determinism, difference on different input, fixed output
      length, and serde round trip, plus a `dedup_key(hash, mime)` differentiation test, per
      `specs/content-hashing/spec.md`.
- [x] 3.2 Run `cargo test -p clip-core hashing::` and confirm failure.
- [x] 3.3 Implement `hash_content` (blake3) and `dedup_key` in `crates/clip-core/src/hashing.rs`.
- [x] 3.4 Run `cargo test -p clip-core` and confirm all green.

## 4. Domain models (`clip-domain-model`)

- [x] 4.1 Write failing tests for `PasteMode` default + serde round trip, per
      `specs/clip-domain-model/spec.md`.
- [x] 4.2 Run `cargo test -p clip-core models::` and confirm failure (no `PasteMode` type yet).
- [x] 4.3 Implement `PasteMode` enum with serde support - minimum code to pass.
- [x] 4.4 Write failing tests for `AppContext` construction (app-only, app+window).
- [x] 4.5 Implement `AppContext` - minimum code to pass; run `cargo test -p clip-core` and confirm green.
- [x] 4.6 Write failing tests for `Group` construction, including the self-parent rejection scenario.
- [x] 4.7 Implement `Group` with the self-parent validation; run `cargo test -p clip-core` and confirm
      green.
- [x] 4.8 Write failing tests for `Rule` construction and its `matches(&AppContext, mime)` predicate
      (matching by app, disabled-rule-never-matches).
- [x] 4.9 Implement `Rule` and its predicate; run `cargo test -p clip-core` and confirm green.
- [x] 4.10 Write failing tests for `ClipRepresentation` (independent byte_size/mime_type per
      representation) and `Clip` (multi-representation preservation, default flags, dedup key equality/
      inequality, metadata serde round trip).
- [x] 4.11 Implement `ClipRepresentation` and `Clip` (using `hashing::dedup_key` internally) - minimum
      code to pass.
- [x] 4.12 Run `cargo test -p clip-core` and confirm all green; refactor if needed while keeping tests
      green.

## 5. Search query parsing (`search-query-parsing`)

- [x] 5.1 Write failing tests for `parse_query` term splitting, whitespace collapsing, prefix-term
      flagging on the final term, and explicit empty/whitespace-only detection, per
      `specs/search-query-parsing/spec.md`.
- [x] 5.2 Run `cargo test -p clip-core search::` and confirm failure.
- [x] 5.3 Implement `parse_query` and its result type - minimum code to pass.
- [x] 5.4 Write failing tests for `SearchFilters` standalone construction and default ranking inputs
      (pinned-boost enabled by default).
- [x] 5.5 Implement `SearchFilters` and ranking-input defaults; run `cargo test -p clip-core` and confirm
      green.

## 6. App config (`app-config`)

- [x] 6.1 Write failing tests for `AppSettings::default()` (capture not paused, no retention window) and
      per-key serde round trip / missing-key-falls-back-to-default, per `specs/app-config/spec.md`.
- [x] 6.2 Run `cargo test -p clip-core config::` and confirm failure.
- [x] 6.3 Implement `AppSettings`, its defaults, and its `SettingsKey`-keyed (de)serialization - minimum
      code to pass.
- [x] 6.4 Write a failing test for `AppPaths::resolve()` producing distinct config/data/cache paths, and a
      failing test for the test-only override environment variable.
- [x] 6.5 Implement `AppPaths` using the `directories` crate with the override hook.
- [x] 6.6 Run `cargo test -p clip-core` and confirm all green.

## 7. Crate-level verification

- [x] 7.1 Run `cargo test -p clip-core` and confirm every test from sections 1-6 passes.
- [x] 7.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles against the
      now-implemented `clip-core` types.
- [x] 7.3 Run `cargo clippy -p clip-core -- -D warnings` and fix any lints introduced by this change.
