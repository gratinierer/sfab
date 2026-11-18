//! What each crate's takedown function returns: its locations for an agent and the action on
//! each, so a key format exists only in its crate. `docs/adr/0004-rules-live-in-crates.md` and
//! `.stories/2-identity-api/spec.md` "Storage"; the program that runs them is issue 08's.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::id::AgentId;
use crate::storage::{Bucket, Metadata, Store, StoreError};
use crate::world::{self, Rebuilt};

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
    /// [`Action::Folders`], but leaving a note instead of deleting: each object in each
    /// folder gets the note the second function makes from its body, the named key last, so a
    /// rerun still finds the folder. Such as an author's documents, named in
    /// `identities/<id>/library.json`, each edition showing "removed". `None` from the second
    /// function means the object is already a note.
    NoteFolders(fn(&[u8]) -> Vec<String>, fn(&[u8]) -> Option<Vec<u8>>),
    /// Read the agent's own file at the key, such as the takedown index, and delete each key
    /// the first function finds in its body that the second says is the agent's from the key
    /// alone, such as an enrolment, whose body names nobody. A key that isn't, because the
    /// file was out of date or wrong, stays.
    Indexed(fn(&[u8]) -> Vec<String>, fn(&str, &AgentId) -> bool),
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

/// The takedown index's key for an agent, `world/agents/<id>.json`, beside its public pages
/// under `world/agents/<id>/`: `.stories/4-life-in-the-world/spec.md` "Takedown". Only the daily
/// job writes it, and no route serves it, because it names private attempts too.
pub fn index_key(id: &AgentId) -> String {
    format!("{}{id}.json", world::AGENTS_PREFIX)
}

/// Whether a key under `world/agents/` is a takedown index, not a public page.
fn is_index(key: &str) -> bool {
    key.strip_prefix(world::AGENTS_PREFIX)
        .and_then(|rest| rest.strip_suffix(".json"))
        .is_some_and(|id| AgentId::parse(id).is_some())
}

/// What the daily job's steps found of each agent's, for the takedown index: per agent, per
/// named list, such as the Academy's folders, the keys. Each crate names its own lists, and
/// reads them back with [`indexed`] in its takedown function. It holds every agent with keys,
/// takedown markers too, so a takedown's second run, after the next daily run, finds what the
/// first missed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Index(BTreeMap<AgentId, BTreeMap<&'static str, BTreeSet<String>>>);

impl Index {
    pub fn add(&mut self, id: AgentId, list: &'static str, key: String) {
        self.0
            .entry(id)
            .or_default()
            .entry(list)
            .or_default()
            .insert(key);
    }

    pub fn merge(&mut self, other: Index) {
        for (id, lists) in other.0 {
            for (list, keys) in lists {
                for key in keys {
                    self.add(id.clone(), list, key);
                }
            }
        }
    }

    /// The agents it has keys for.
    pub fn agents(&self) -> usize {
        self.0.len()
    }
}

/// Makes `world/agents/` hold exactly one index for each agent in `index`, leaving the public
/// pages there alone, so an agent with nothing left loses its index.
pub async fn rebuild_index<S: Store>(store: &S, index: Index) -> Result<Rebuilt, StoreError> {
    let files = index
        .0
        .into_iter()
        .map(|(id, lists)| {
            // `{"id": <id>, "lists": {<list>: [<key>, ...]}}`.
            let lists: BTreeMap<&str, Vec<String>> = lists
                .into_iter()
                .map(|(list, keys)| (list, keys.into_iter().collect()))
                .collect();
            let file = json!({"id": id.as_str(), "lists": lists});
            let body = serde_json::to_vec(&file).expect("an index serializes");
            (index_key(&id), body)
        })
        .collect();
    world::rebuild_where(store, world::AGENTS_PREFIX, files, is_index).await
}

/// The keys an index body names in `list`, none when it doesn't parse.
pub fn indexed(body: &[u8], list: &str) -> Vec<String> {
    let file: Option<Value> = serde_json::from_slice(body).ok();
    let keys = file.as_ref().and_then(|f| f["lists"][list].as_array());
    keys.map(|keys| {
        keys.iter()
            .filter_map(|k| k.as_str().map(str::to_owned))
            .collect()
    })
    .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct Location {
    pub phase: Phase,
    pub bucket: Bucket,
    /// A key, or for [`Action::DeletePrefix`] and [`Action::RemoveId`] a prefix ending in `/`.
    pub key: String,
    pub action: Action,
}
