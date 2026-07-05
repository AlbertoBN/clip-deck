## Context

`clip-core` is currently an empty stub: `crates/clip-core/src/{models,mime,hashing,search,config,errors}.rs`
each contain only a one-line module doc comment (see `clip-core-foundations` is the first real
implementation work in the workspace). Every other crate takes `clip-core` as a path dependency but does
not yet import any of its types, so this change has no consumers to keep working - it only needs to be
internally correct and well-tested before `clip-store-persistence` starts building on top of it.

## Goals / Non-Goals

**Goals:**
- Provide the domain types, MIME handling, hashing, search-query parsing, and settings model that every
  other crate needs, exactly matching the shapes implied by the PRD's SQLite schema and IPC contract so
  `clip-store` and `clip-ipc` don't have to invent conversions later.
- Get every public type to 100% behavior coverage via unit tests before any downstream crate depends on it.

**Non-Goals:**
- No SQLite/database code (that's `clip-store-persistence`).
- No IPC (de)serialization wiring (that's `clip-ipc-transport`) - `clip-core` only needs `serde::Serialize`/
  `Deserialize` derives, not the wire protocol itself.
- No actual FTS5 query execution or ranking math - `search-query-parsing` only produces the structured
  inputs that `clip-store` later turns into SQL.

## Decisions

- **Errors**: a single `clip_core::errors::CoreError` (via `thiserror`) covering MIME-parse failures,
  invalid-group-parent, and similar validation errors, rather than a bespoke error enum per module.
  Alternative considered: per-module error types - rejected for now since the error surface is small and a
  shared enum keeps downstream `match` arms simpler; can be split later if it grows unwieldy.
- **IDs**: `uuid::Uuid` (v4) for `Clip`/`Group`/`Rule` ids, matching the PRD's `TEXT PRIMARY KEY` columns
  (UUIDs stored as their string form).
- **Timestamps**: `time::OffsetDateTime`, matching the PRD's `TEXT NOT NULL` timestamp columns (stored as
  RFC 3339 strings), rather than `chrono`, per the workspace's crate-choice table.
- **MIME family classification**: a small closed `MimeFamily` enum (`Text`, `Html`, `Image`, `Other`)
  computed from the normalized MIME string rather than string-matching MIME types ad hoc at every call
  site (capture, preview, paste).
- **Settings storage shape**: `AppSettings` fields are individually (de)serializable via a `SettingsKey`
  enum + JSON value, matching the PRD's per-key `settings` table, instead of one big JSON blob for the
  whole settings object - this lets `clip-store` update a single setting without re-serializing all of
  them.

## Test strategy

Every requirement in `specs/*/spec.md` maps 1:1 to a `#[test]` in the corresponding module, written before
the implementation exists (see `tasks.md` - each task group starts with a failing-test task). Concretely,
per component:

- `models`: unit tests per scenario in `clip-domain-model/spec.md` (dedup key equality, PasteMode default
  and serde round trip, Group self-parent rejection, Rule matching predicate, Clip metadata round trip).
  Run with `cargo test -p clip-core models::`.
- `mime`: table-driven tests for normalization and family classification, plus one malformed-input test
  asserting an `Err`. Run with `cargo test -p clip-core mime::`.
- `hashing`: determinism test (hash twice, assert equal), difference test (different bytes, different
  hash), fixed-length assertion across varying input sizes, and a serde round-trip test. Run with
  `cargo test -p clip-core hashing::`.
- `search`: parser tests for term splitting, prefix-term flagging, empty/whitespace detection, and a
  `SearchFilters` construction test independent of query text. Run with `cargo test -p clip-core search::`.
- `config`: default-value assertions for every `AppSettings` field, a per-key serde round-trip test, a
  missing-key-falls-back-to-default test, and an `AppPaths` override test using a temp-dir env var. Run
  with `cargo test -p clip-core config::`.

Red-green-refactor loop for each task: write the test against the not-yet-existing API (crate fails to
compile / test fails), implement just enough to compile and pass, run `cargo test -p clip-core`, then
refactor only with the suite green. No task is marked done with a failing or missing test.

## Risks / Trade-offs

- [Risk] Guessing at exact PRD schema field types (e.g. timestamp representation) before `clip-store`
  exists to round-trip them → Mitigation: model types directly against the PRD's migration SQL
  (`docs/ClipDeck-ubuntu-clipboard-manager-prd.md`, "Proposed SQLite migration file") so `clip-store-
  persistence` doesn't need to change `clip-core` types when it lands.
- [Risk] Over-fitting `SearchFilters`/ranking inputs to a guessed SQL shape before FTS5 queries are written
  → Mitigation: keep the struct minimal (documented fields only) and treat any FTS5-specific query-string
  construction as `clip-store`'s concern, not `clip-core`'s.
