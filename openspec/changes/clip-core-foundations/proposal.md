## Why

Every other crate in the workspace (`clip-store`, `clip-ipc`, `clip-platform`, `clipd`, `clip-ui-tauri`)
depends on shared domain types, MIME handling, content hashing, search-query parsing, and config
defaults living in `clip-core`. Per the PRD's build order, `clip-core` must be implemented first since
nothing downstream can be built (or even compile against real types) until it exists.

## What Changes

- Implement the `Clip`, `ClipRepresentation`, `Group`, `Rule`, `AppContext`, and `PasteMode` domain
  models (serializable, matching the PRD's proposed data model) in `clip-core::models`.
- Implement canonical MIME-type normalization and representation typing in `clip-core::mime`.
- Implement deterministic content hashing (blake3) for dedup in `clip-core::hashing`.
- Implement search query parsing/ranking-input helpers in `clip-core::search`.
- Implement the settings model and defaults (config paths via `directories`) in `clip-core::config`.
- Implement shared error types in `clip-core::errors`.

## Capabilities

### New Capabilities
- `clip-domain-model`: Shared domain types (Clip, ClipRepresentation, Group, Rule, AppContext, PasteMode)
  with serde support and the invariants the rest of the workspace relies on (multi-representation clips,
  dedup key shape).
- `mime-normalization`: Canonical MIME type parsing/normalization and MIME-family classification used to
  tag clip representations consistently.
- `content-hashing`: Deterministic blake3-based content hashing combined with MIME type for dedup.
- `search-query-parsing`: Parsing a raw search string into structured query + filter inputs consumable by
  `clip-store`'s FTS5 layer, plus ranking-boost inputs (recency, pinned).
- `app-config`: Settings model, defaults, and resolution of on-disk config/data/cache paths.

### Modified Capabilities
(none - this is the first change in the workspace)

## Impact

- Affected code: `crates/clip-core/src/{models,mime,hashing,search,config,errors}.rs`,
  `crates/clip-core/Cargo.toml` (dependency versions only, no new deps expected beyond what's scaffolded:
  serde, serde_json, thiserror, uuid, blake3, mime, mime_guess, time).
- No downstream crate changes yet - `clip-store`, `clip-ipc`, `clip-platform`, `clipd`, and
  `clip-ui-tauri` currently depend on `clip-core` as a path dependency but do not import any of its types.
- No schema/DB impact (that's `clip-store-persistence`).
