//! What each crate's takedown function returns: its locations for an agent and the action on
//! each, so a key format exists only in its crate. `docs/adr/0004-rules-live-in-crates.md` and
//! `.stories/2-identity-api/spec.md` "Storage"; the program that runs them is issue 08's.

use crate::id::AgentId;
use crate::storage::{Bucket, Metadata};

/// When a location is handled, in the spec's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Step 1: end private access and public reading at once.
    First,
    /// Step 3: read the agent's own files for what they point to elsewhere, such as a home's
    /// pointer on its lane, while they still exist.
    Pointers,
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
    /// Leave a note: replace the content with the note this function makes from the current
    /// body, such as a Library edition's "removed", so what points at it doesn't silently break.
    /// `None` means the object is already a note.
    Note(fn(&[u8]) -> Option<Vec<u8>>),
    /// Replace the agent's ID with [`REMOVED`] in every object under a prefix that names it,
    /// such as the Chronicle's daily entries, which name agents by ID only.
    RemoveId,
    /// Read the agent's own file at the key, and delete each key this function finds in its
    /// body, in the same bucket, that still names the agent, such as the `places/...` pointer in
    /// `home.json`, which frees the address. A key naming someone else stays, because the file
    /// was out of date.
    Pointers(fn(&[u8]) -> Vec<String>),
    /// Delete every key under a prefix that this function says belongs to the agent, from the
    /// key alone, such as the Square's notices and answers, whose keys name their authors.
    Search(fn(&str, &AgentId) -> bool),
    /// Read the agent's own file at the key, and for each key this function finds in its body,
    /// in the same bucket, that still names the agent, delete everything in that key's folder,
    /// the key itself last, so a rerun still finds the folder. Such as a teacher's courses,
    /// named in `identities/<id>/academy.json`, each with its enrolments and attempts beside
    /// `course.json`. A key naming someone else stays, with its folder.
    Folders(fn(&[u8]) -> Vec<String>),
}

/// What [`Action::RemoveId`] writes in place of the ID.
pub const REMOVED: &str = "[removed]";

/// `body` with every occurrence of `id` replaced by [`REMOVED`], or `None` when it names no ID.
/// An ID is base32 letters and digits only, so the result of a JSON body is still JSON.
pub fn remove_id(body: &[u8], id: &AgentId) -> Option<Vec<u8>> {
    let id = id.as_str().as_bytes();
    let mut out = Vec::with_capacity(body.len());
    let mut rest = body;
    let mut found = false;
    while let Some(at) = rest.windows(id.len()).position(|w| w == id) {
        found = true;
        out.extend_from_slice(&rest[..at]);
        out.extend_from_slice(REMOVED.as_bytes());
        rest = &rest[at + id.len()..];
    }
    out.extend_from_slice(rest);
    found.then_some(out)
}

#[derive(Debug, Clone)]
pub struct Location {
    pub phase: Phase,
    pub bucket: Bucket,
    /// A key, or for [`Action::DeletePrefix`] and [`Action::RemoveId`] a prefix ending in `/`.
    pub key: String,
    pub action: Action,
}
