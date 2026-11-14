//! Shared core for the desktop and phone apps: store, clocks, sync, crypto, export, stats, templates.

pub mod crypto;
pub mod error;
pub mod export;
pub mod hlc;
pub mod pairing;
pub mod stats;
pub mod store;
pub mod sync;
pub mod templates;

pub use error::{Error, Result};
pub use store::{Change, Store};

/// Kinds that are only ever added, so they never conflict.
pub const APPEND_ONLY_KINDS: &[&str] = &[
    "attendance",
    "note",
    "injury",
    "match_event",
    "assessment",
];

pub fn is_append_only(kind: &str) -> bool {
    APPEND_ONLY_KINDS.contains(&kind)
}

/// Fields that only record the last touch: the newest wins and no conflict is raised.
pub const BOOKKEEPING_FIELDS: &[&str] = &["activity_at"];

pub fn is_bookkeeping(field: &str) -> bool {
    BOOKKEEPING_FIELDS.contains(&field)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
