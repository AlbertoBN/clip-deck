## ADDED Requirements

### Requirement: Content hashing is deterministic and stable across runs
The system SHALL compute a deterministic blake3 content hash for a given byte payload such that hashing
the same bytes always yields the same hash value, independent of process, machine, or time, so it can be
used as the durable dedup key described in the PRD's `idx_clips_hash_mime` unique index.

#### Scenario: Hashing the same bytes twice yields the same hash
- **WHEN** `hash_content(b"hello world")` is called twice
- **THEN** both calls return the identical hash value

#### Scenario: Hashing different bytes yields different hashes
- **WHEN** `hash_content(b"hello")` and `hash_content(b"world")` are both called
- **THEN** the two hash values are different

### Requirement: Hash output is a stable, storable string form
The system SHALL expose the computed hash as a fixed-length, storable string form (e.g. hex-encoded) so
it can round-trip through SQLite's `content_hash TEXT` column and through serde without loss.

#### Scenario: Hash string round-trips through serde
- **WHEN** a hash produced by `hash_content` is serialized to JSON and deserialized back
- **THEN** the resulting value is equal to the original hash

#### Scenario: Hash string has a fixed length regardless of input size
- **WHEN** `hash_content` is called on a 3-byte input and separately on a 300,000-byte input
- **THEN** both resulting hash strings have the same length

### Requirement: Dedup key combines content hash with MIME type
The system SHALL provide a helper that combines a content hash with a normalized MIME type into the dedup
key used by `Clip::dedup_key()`, so hashing and MIME normalization stay in one place rather than being
reimplemented per caller.

#### Scenario: Same bytes with different MIME types produce different dedup keys
- **WHEN** the same byte payload is hashed and combined with `"text/plain"` in one call and
  `"text/html"` in another
- **THEN** the two resulting dedup keys differ
