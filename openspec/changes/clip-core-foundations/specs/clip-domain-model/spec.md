## ADDED Requirements

### Requirement: Clip aggregates multiple representations
A `Clip` SHALL reference zero-or-more ordered `ClipRepresentation` values (e.g. plain text and HTML from
one copy event) rather than collapsing content into a single format field. Each `ClipRepresentation`
SHALL carry its own MIME type, optional text value, optional on-disk blob path, optional preview text,
optional width/height (for images), byte size, and an ordinal used for display order.

#### Scenario: Clip with two representations preserves both
- **WHEN** a `Clip` is constructed from a plain-text and an HTML representation of the same copy event
- **THEN** both representations are retrievable from the clip, in the order supplied, without either one
  overwriting the other

#### Scenario: Representation carries its own byte size and MIME type independent of the clip
- **WHEN** a representation with `mime_type = "text/html"` and a representation with
  `mime_type = "text/plain"` are added to the same clip
- **THEN** each representation reports its own `byte_size` and `mime_type` independently of the clip's
  `primary_mime`

### Requirement: Clip exposes a deterministic dedup key
A `Clip` SHALL expose a dedup key derived from `(content_hash, primary_mime)`, matching the unique index
`(content_hash, primary_mime, is_deleted)` from the storage schema, so `clip-store` can enforce
deduplication without recomputing hashing logic itself.

#### Scenario: Two clips with identical hash and MIME produce the same dedup key
- **WHEN** two `Clip` values are constructed with the same `content_hash` and the same `primary_mime`
- **THEN** `Clip::dedup_key()` returns equal values for both

#### Scenario: Same hash but different MIME produce different dedup keys
- **WHEN** two `Clip` values share the same `content_hash` but have different `primary_mime`
- **THEN** `Clip::dedup_key()` returns different values for each

### Requirement: Clip lifecycle flags and metadata are modeled explicitly
A `Clip` SHALL model `created_at`, `updated_at`, `last_used_at` (optional), `source_app` (optional),
`source_window` (optional), `is_favorite`, `is_pinned`, `is_deleted`, `group_id` (optional),
`paste_mode_default` (a `PasteMode`), and an optional structured `metadata` payload, matching the PRD's
proposed `clips` table so `clip-store` can persist a `Clip` without inventing additional fields.

#### Scenario: New clip defaults to not pinned, not favorite, not deleted
- **WHEN** a `Clip` is constructed via its standard constructor with only required fields supplied
- **THEN** `is_pinned`, `is_favorite`, and `is_deleted` are all `false`

#### Scenario: Metadata payload round-trips through serde
- **WHEN** a `Clip` with a non-empty structured `metadata` value is serialized to JSON and deserialized
  back
- **THEN** the resulting `Clip`'s metadata is equal to the original

### Requirement: PasteMode models supported paste behaviors
`PasteMode` SHALL be an enum with at least `Auto`, `Rich`, and `PlainText` variants, defaulting to `Auto`,
and SHALL serialize to/from the lowercase string tokens used by the `clips.paste_mode_default` column
(e.g. `"auto"`).

#### Scenario: Default paste mode is Auto
- **WHEN** a `Clip` is constructed without an explicit paste mode
- **THEN** its `paste_mode_default` is `PasteMode::Auto`

#### Scenario: PlainText paste mode round-trips through serde as "plain_text"
- **WHEN** `PasteMode::PlainText` is serialized
- **THEN** the resulting value deserializes back to `PasteMode::PlainText`

### Requirement: Group models optional nesting without self-reference
A `Group` SHALL support an optional `parent_group_id` for nested groups/folders, and construction or
mutation that would set a group's `parent_group_id` equal to its own `id` SHALL be rejected.

#### Scenario: Group can reference a different group as parent
- **WHEN** a `Group` with id `"child"` is constructed with `parent_group_id = Some("parent")`
- **THEN** construction succeeds and `parent_group_id` is `Some("parent")`

#### Scenario: Group cannot be its own parent
- **WHEN** a `Group` with id `"g1"` is constructed with `parent_group_id = Some("g1")`
- **THEN** construction returns an error identifying the self-reference

### Requirement: Rule models app/window/MIME exclusion matching
A `Rule` SHALL carry an `app_match`, optional `window_match`, optional `mime_match`, an `action` (at least
`Exclude` and `Ephemeral`/never-persist), and an `enabled` flag, and SHALL expose a predicate that
evaluates whether a given `AppContext` and MIME type match the rule.

#### Scenario: Rule matches by app name alone
- **WHEN** a `Rule` has `app_match = "1Password"` and no `window_match`/`mime_match`
- **THEN** `Rule::matches(&AppContext { app: "1Password", .. }, any_mime)` returns `true`

#### Scenario: Disabled rule never matches
- **WHEN** a `Rule` matching an app is constructed with `enabled = false`
- **THEN** `Rule::matches(..)` returns `false` regardless of the input context

### Requirement: AppContext models the source of a captured clip
An `AppContext` SHALL capture the source application identifier and, where available, the window title
or identifier, so capture, rules, and paste-back targeting can all reference the same struct.

#### Scenario: AppContext with only app name is valid
- **WHEN** an `AppContext` is constructed with only `app` set and no window info
- **THEN** construction succeeds and the window field reports as absent
