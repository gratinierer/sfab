//! What the daily world job builds under `world/`, and how routes read it:
//! `.stories/4-life-in-the-world/spec.md` "Foundations" ("What the daily job writes", "Before the
//! first daily run" and "Unknown IDs").
//!
//! Each run works out the full set of files a rebuilt prefix should hold and hands it to
//! [`rebuild`], which writes only those that changed and deletes the rest under that prefix, such
//! as pages past the end of a list that shrank. The Chronicle's daily entries and
//! `milestones.json` are never deleted by it: only a takedown's "[removed]" rewrite touches them.

use std::collections::{BTreeMap, BTreeSet};

use futures_util::{StreamExt, TryStreamExt, stream};
use serde_json::Value;

use crate::problem::Problem;
use crate::storage::{Bucket, Condition, Metadata, Store, StoreError};

/// Everything the daily job builds lives under here.
pub const PREFIX: &str = "world/";
/// The Chronicle's folder: its direct children, the daily entries `<date>.json` and
/// `milestones.json`, are permanent; only its subfolders, such as `days/`, are rebuilt.
pub const CHRONICLE: &str = "world/chronicle/";
/// S3 calls in flight at once, as story 2 "Daily world job" sets for the job.
pub const PARALLEL: usize = 50;

/// What a [`rebuild`] did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rebuilt {
    /// Files written because they were new or changed.
    pub written: usize,
    /// Files already holding what this run built.
    pub unchanged: usize,
    /// Files under the prefix that this run's set doesn't hold.
    pub deleted: usize,
}

/// Makes `prefix`, which must be under `world/` and end in `/`, hold exactly `files`: writes each
/// one that's new or changed, and deletes every other key under `prefix`, except the
/// Chronicle's entries and milestones. Safe to repeat: a second run with the same set writes
/// nothing. One listing says which files exist, so a new file costs no read, and the job, which
/// may list only `world/`, never asks for a key it didn't see.
pub async fn rebuild<S: Store>(
    store: &S,
    prefix: &str,
    files: BTreeMap<String, Vec<u8>>,
) -> Result<Rebuilt, StoreError> {
    if !prefix.starts_with(PREFIX) || !prefix.ends_with('/') {
        return Err(StoreError::Other(format!(
            "a rebuild's prefix must be a folder under {PREFIX}: {prefix}"
        )));
    }
    if let Some(stray) = files.keys().find(|k| !k.starts_with(prefix)) {
        return Err(StoreError::Other(format!(
            "a rebuild of {prefix} can't write {stray}"
        )));
    }
    let existing: BTreeSet<String> = store
        .list(Bucket::Identities, prefix)
        .await?
        .into_iter()
        .collect();
    let surplus: Vec<String> = existing
        .iter()
        .filter(|k| !files.contains_key(*k) && !is_permanent(k))
        .cloned()
        .collect();

    let writes = files.into_iter().map(|(key, body)| {
        let known = existing.contains(&key);
        async move { write_if_changed(store, &key, body, known).await }
    });
    let changed: Vec<bool> = stream::iter(writes)
        .buffer_unordered(PARALLEL)
        .try_collect()
        .await?;
    let deleted = surplus.len();
    stream::iter(surplus)
        .map(|key| async move { store.delete(Bucket::Identities, &key).await })
        .buffer_unordered(PARALLEL)
        .try_collect::<Vec<()>>()
        .await?;

    let written = changed.iter().filter(|c| **c).count();
    Ok(Rebuilt {
        written,
        unchanged: changed.len() - written,
        deleted,
    })
}

/// A daily entry or `milestones.json`: a direct child of [`CHRONICLE`].
fn is_permanent(key: &str) -> bool {
    key.strip_prefix(CHRONICLE)
        .is_some_and(|rest| !rest.contains('/'))
}

/// Writes `body` unless the object already holds it. `known` says a listing saw the key, so a
/// new file costs no read.
async fn write_if_changed<S: Store>(
    store: &S,
    key: &str,
    body: Vec<u8>,
    known: bool,
) -> Result<bool, StoreError> {
    if known
        && let Some(current) = store.get(Bucket::Identities, key).await?
        && current.body == body
    {
        return Ok(false);
    }
    store
        .put(
            Bucket::Identities,
            key,
            body,
            Metadata::new(),
            Condition::Always,
        )
        .await?;
    Ok(true)
}

/// The `404` a route built by the daily job answers until the job has written its file once.
pub fn not_built_yet() -> Problem {
    Problem::not_built_yet(
        "The daily world job hasn't built this yet; try again after its next run, just after 00:00 UTC",
    )
}

/// A file the daily job built, or [`not_built_yet`] until it exists. Storage errors and a file
/// that doesn't parse are a logged `500`.
pub async fn read_built<S: Store>(store: &S, key: &str) -> Result<Value, Problem> {
    read(store, key).await?.ok_or_else(not_built_yet)
}

/// A file the daily job builds per agent, such as `world/academy/masters/<id>.json`. Once the
/// job has run, which `built` shows by existing, a missing file is `nothing`, the answer for an
/// agent with nothing to show, so an unknown or taken-down ID reads exactly like one and no
/// route reveals which IDs exist. Before the first run, every ID alike gets [`not_built_yet`].
pub async fn read_for_agent<S: Store>(
    store: &S,
    key: &str,
    built: &str,
    nothing: Value,
) -> Result<Value, Problem> {
    if let Some(found) = read(store, key).await? {
        return Ok(found);
    }
    match store.head(Bucket::Identities, built).await {
        Ok(Some(_)) => Ok(nothing),
        Ok(None) => Err(not_built_yet()),
        Err(e) => Err(Problem::storage(built, &e)),
    }
}

async fn read<S: Store>(store: &S, key: &str) -> Result<Option<Value>, Problem> {
    let object = store
        .get(Bucket::Identities, key)
        .await
        .map_err(|e| Problem::storage(key, &e))?;
    let Some(object) = object else {
        return Ok(None);
    };
    serde_json::from_slice(&object.body).map(Some).map_err(|_| {
        tracing::error!("{key} in storage doesn't parse");
        Problem::internal()
    })
}
