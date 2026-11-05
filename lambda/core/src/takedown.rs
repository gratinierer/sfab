//! What each crate's takedown function returns: its locations for an agent and the action on
//! each, so a key format exists only in its crate. `docs/adr/0004-rules-live-in-crates.md` and
//! `.stories/2-identity-api/spec.md` "Storage"; the program that runs them is issue 08's.

use crate::storage::{Bucket, Metadata};

/// When a location is handled, in the spec's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Step 1: end private access and public reading at once.
    First,
    /// Step 4: every other location.
    Other,
    /// Step 5: the agent's own files, which point elsewhere, so a rerun can still find them.
    Last,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    /// Delete one key. A key that's already gone counts as done.
    Delete,
    /// Delete every key under a prefix.
    DeletePrefix,
    /// Replace the object with a zero-byte marker, keeping the metadata this function takes
    /// from the current body. `None` means the object is already a marker, or gone.
    Marker(fn(&[u8]) -> Option<Metadata>),
}

#[derive(Debug, Clone)]
pub struct Location {
    pub phase: Phase,
    pub bucket: Bucket,
    /// A key, or for [`Action::DeletePrefix`] a prefix ending in `/`.
    pub key: String,
    pub action: Action,
}
