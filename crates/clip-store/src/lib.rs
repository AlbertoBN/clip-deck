pub mod clips;
pub mod db;
pub mod events;
pub mod fts;
pub mod groups;
pub mod retention;
pub mod rules;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("a non-deleted clip with this content hash and MIME type already exists")]
    DedupConflict,
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}
