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
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::problem::{Problem, Reply};
use crate::storage::{Bucket, Condition, Metadata, Store, StoreError};

/// Everything the daily job builds lives under here.
pub const PREFIX: &str = "world/";
/// The Chronicle's folder: its direct children, the daily entries `<date>.json` and
/// `milestones.json`, are permanent; only its subfolders, such as `days/`, are rebuilt.
pub const CHRONICLE: &str = "world/chronicle/";
/// The per-agent files: public pages under `world/agents/<id>/`, which the Academy builds, and
/// beside them the takedown index `world/agents/<id>.json` of [`crate::takedown::index_key`].
pub const AGENTS_PREFIX: &str = "world/agents/";
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
    rebuild_where(store, prefix, files, |_| true).await
}

/// [`rebuild`] for the part of `prefix` that one writer owns, such as the Academy's per-agent
/// pages under `world/agents/`, beside the takedown index there: only a surplus key that
/// `owned` accepts is deleted.
pub async fn rebuild_where<S: Store>(
    store: &S,
    prefix: &str,
    files: BTreeMap<String, Vec<u8>>,
    owned: impl Fn(&str) -> bool,
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
        .filter(|k| !files.contains_key(*k) && !is_permanent(k) && owned(k))
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

/// Writes `body` unless the object already holds it, and says whether it wrote. `known` says a
/// listing saw the key, so a new file costs no read. For the daily job's own keys, which it alone
/// writes.
pub async fn write_if_changed<S: Store>(
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

/// Writes a daily step's run marker, `{"as_of": <as_of>}` at `key`, last in its run: its
/// routes take the key's existence as "the job has run" ([`read_for_agent`], [`read_page`]).
pub async fn write_built<S: Store>(store: &S, key: &str, as_of: &str) -> Result<(), StoreError> {
    let body = serde_json::to_vec(&serde_json::json!({"as_of": as_of})).expect("JSON serializes");
    store
        .put(
            Bucket::Identities,
            key,
            body,
            Metadata::new(),
            Condition::Always,
        )
        .await
}

/// The `as_of` of the run marker [`write_built`] wrote at `key`, or `None` before the step's
/// first run. A storage error or a marker that doesn't parse is a logged `500`.
pub async fn built_at<S: Store>(store: &S, key: &str) -> Result<Option<String>, Problem> {
    let Some(found) = read(store, key).await? else {
        return Ok(None);
    };
    match found["as_of"].as_str() {
        Some(as_of) => Ok(Some(as_of.to_owned())),
        None => {
            tracing::error!("{key} in storage has no as_of");
            Err(Problem::internal())
        }
    }
}

/// A JSON file the daily job listed, or `None` when it's gone since or doesn't parse, which is
/// logged and skipped so one bad file can't stop the run.
pub async fn read_json<S: Store, T: DeserializeOwned>(
    store: &S,
    key: &str,
) -> Result<Option<T>, StoreError> {
    let Some(object) = store.get(Bucket::Identities, key).await? else {
        return Ok(None);
    };
    match serde_json::from_slice(&object.body) {
        Ok(file) => Ok(Some(file)),
        Err(_) => {
            tracing::error!("{key} doesn't parse; skipped");
            Ok(None)
        }
    }
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
    read_if_built(store, key).await?.ok_or_else(not_built_yet)
}

/// A file the daily job built, or `None` until it exists, for a route that answers without it,
/// such as with zeros. Storage errors and a file that doesn't parse are a logged `500`.
pub async fn read_if_built<S: Store>(store: &S, key: &str) -> Result<Option<Value>, Problem> {
    read(store, key).await
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

/// The most digits a page number may have, as story 2's resident pages allow.
const MAX_PAGE_DIGITS: usize = 7;

/// A list the daily job builds as ready-made page files, `<prefix><n>.json` from 1, `size`
/// items each, each holding `{"items": [...], "next": <n + 1 or null>}`, for [`rebuild`]. An
/// empty list still has its first page.
pub fn page_files(prefix: &str, items: Vec<Value>, size: usize) -> BTreeMap<String, Vec<u8>> {
    let chunks: Vec<&[Value]> = if items.is_empty() {
        vec![&[]]
    } else {
        items.chunks(size).collect()
    };
    let pages = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let page = i + 1;
            let next = (page < pages).then_some(page + 1);
            let body = serde_json::json!({"items": chunk, "next": next});
            let body = serde_json::to_vec(&body).expect("JSON values serialize");
            (format!("{prefix}{page}.json"), body)
        })
        .collect()
}

/// A page of a list [`page_files`] built, as `{"items": [...], "next": "<path>?page=<n>"}`, with
/// `?page=` the page number. Before the job's first run, which `built` shows by existing, every
/// page is [`not_built_yet`]. After it, a list with no files reads as one empty page, so an
/// unknown course or agent reads like one with nothing to show, and a page past the end is `404`.
pub async fn read_page<S: Store>(
    store: &S,
    prefix: &str,
    token: Option<&str>,
    path: &str,
    built: &str,
) -> Result<Reply, Problem> {
    let number = match token {
        None => 1,
        Some(t) => parse_page(t)
            .ok_or_else(|| Problem::bad_request("?page= must be a token from a list's next"))?,
    };
    let key = format!("{prefix}{number}.json");
    if let Some(found) = read(store, &key).await? {
        let next = found["next"].as_u64().map(|n| format!("{path}?page={n}"));
        let items = found["items"].clone();
        return Ok(Reply::ok(serde_json::json!({"items": items, "next": next})));
    }
    match store.head(Bucket::Identities, built).await {
        Ok(None) => Err(not_built_yet()),
        Ok(Some(_)) if number == 1 => Ok(Reply::ok(serde_json::json!({"items": [], "next": null}))),
        Ok(Some(_)) => Err(Problem::not_found("This page is past the end of the list")),
        Err(e) => Err(Problem::storage(built, &e)),
    }
}

/// A whole number from 1, with no leading zero, as `next` writes it.
fn parse_page(token: &str) -> Option<usize> {
    let ok = !token.is_empty()
        && token.len() <= MAX_PAGE_DIGITS
        && token.bytes().all(|b| b.is_ascii_digit())
        && !token.starts_with('0');
    ok.then(|| token.parse().ok()).flatten()
}
