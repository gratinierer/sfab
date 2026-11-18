//! The census and the resident list: `.stories/2-identity-api/spec.md` "World". The daily world
//! job writes a [`snapshot`] once a day, and the API serves it through [`Census`], so a reading
//! can be up to a day old. This crate owns the `world/census.json` and `world/residents/` keys;
//! the residents themselves come from `sfab_identity::residents`, which owns the claims.
//!
//! Each route reads one or two small files. At the daily limit, 100,000 calls a day, that's at
//! most 200,000 GETs, about $0.08 a day with the Lambda, well under `docs/api-design.md`'s $2.

use std::collections::BTreeSet;
use std::sync::Arc;

use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::json;

use sfab_core::files::read_file;
use sfab_core::id::AgentId;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Store, StoreError};
use sfab_core::world;

/// Resident IDs on a page, the most a list of bare IDs may hold (`docs/api-design.md` "Lists").
pub const PAGE_SIZE: usize = 1_000;
/// S3 calls in flight at once: one at a time would pass the job's timeout at about 41,000
/// residents (spec.md "Daily world job").
pub const PARALLEL: usize = 50;
/// The route's path, which `next` repeats with `?page=`.
pub const RESIDENTS_PATH: &str = "/v1/world/residents";
/// The most digits a page token may have: up to page 9,999,999, which at `PAGE_SIZE` is about 10
/// billion residents.
const MAX_PAGE_DIGITS: usize = 7;

const CENSUS_KEY: &str = "world/census.json";
const WORLD_PREFIX: &str = "world/";
const PAGES_PREFIX: &str = "world/residents/";

fn page_key(page: usize) -> String {
    format!("{PAGES_PREFIX}{page}.json")
}

/// `world/census.json`, which is also the census route's reply.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct CensusFile {
    residents: usize,
    as_of: String,
}

/// `world/residents/<n>.json`: one page, and the number of the next, if any.
#[derive(Debug, Serialize, Deserialize)]
struct PageFile {
    items: Vec<String>,
    next: Option<usize>,
}

/// What a snapshot did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub residents: usize,
    pub pages: usize,
    /// Files written because they changed, the census included.
    pub written: usize,
    /// Pages beyond the last, deleted.
    pub deleted: usize,
}

/// Writes the census and the resident pages for `residents`, in the order given, writing only
/// the files that changed, then deletes every page beyond the last, so a removed ID leaves the
/// public list within a day. An empty world still has one empty page. Safe to repeat: a second
/// run with the same input writes nothing.
pub async fn snapshot<S: Store>(
    store: &S,
    residents: &[AgentId],
    as_of: &str,
) -> Result<Report, StoreError> {
    // One listing of `world/` says which files exist, so the job never asks for a missing key,
    // which S3 answers with `403` when the job may list only `world/` (spec.md "Permissions").
    let world: BTreeSet<String> = store
        .list(Bucket::Identities, WORLD_PREFIX)
        .await?
        .into_iter()
        .collect();
    let census_known = world.contains(CENSUS_KEY);
    let existing: BTreeSet<String> = world
        .into_iter()
        .filter(|k| k.starts_with(PAGES_PREFIX))
        .collect();
    let chunks: Vec<&[AgentId]> = if residents.is_empty() {
        vec![&[]]
    } else {
        residents.chunks(PAGE_SIZE).collect()
    };
    let pages = chunks.len();

    let files = chunks.iter().enumerate().map(|(i, chunk)| {
        let page = i + 1;
        let file = PageFile {
            items: chunk.iter().map(|id| id.as_str().to_owned()).collect(),
            next: (page < pages).then_some(page + 1),
        };
        let key = page_key(page);
        let known = existing.contains(&key);
        (
            key,
            serde_json::to_vec(&file).expect("a page serializes"),
            known,
        )
    });
    // Pages first, so the census never points past what's written.
    let written: usize =
        stream::iter(files)
            .map(|(key, body, known)| async move {
                world::write_if_changed(store, &key, body, known).await
            })
            .buffer_unordered(PARALLEL)
            .try_fold(0, |n, changed| async move { Ok(n + usize::from(changed)) })
            .await?;

    let census = CensusFile {
        residents: residents.len(),
        as_of: as_of.to_owned(),
    };
    let census = serde_json::to_vec(&census).expect("the census serializes");
    let written = written
        + usize::from(world::write_if_changed(store, CENSUS_KEY, census, census_known).await?);

    let wanted: BTreeSet<String> = (1..=pages).map(page_key).collect();
    let surplus: Vec<String> = existing.difference(&wanted).cloned().collect();
    let deleted = surplus.len();
    stream::iter(surplus)
        .map(|key| async move { store.delete(Bucket::Identities, &key).await })
        .buffer_unordered(PARALLEL)
        .try_collect::<Vec<()>>()
        .await?;

    Ok(Report {
        residents: residents.len(),
        pages,
        written,
        deleted,
    })
}

/// The census and resident routes, which read what the last [`snapshot`] wrote.
pub struct Census<S> {
    store: Arc<S>,
}

impl<S: Store> Census<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// `GET /world/census`: how many residents there are, and `as_of`.
    pub async fn census(&self) -> Result<Reply, Problem> {
        let file: CensusFile = self
            .read(CENSUS_KEY)
            .await?
            .ok_or_else(world::not_built_yet)?;
        Ok(Reply::ok(json!(file)))
    }

    /// `GET /world/residents`: one page of resident IDs, with `?page=` from `next`. A page past
    /// the last gets `404`, and so does every page before the first run, with `not-built-yet`.
    pub async fn residents(&self, page: Option<&str>) -> Result<Reply, Problem> {
        let number = match page {
            None => 1,
            Some(token) => parse_page(token).ok_or_else(|| {
                Problem::bad_request("?page= must be a token from a list's `next`")
            })?,
        };
        let Some(file) = self.read::<PageFile>(&page_key(number)).await? else {
            return Err(match self.read::<CensusFile>(CENSUS_KEY).await? {
                None => world::not_built_yet(),
                Some(_) => Problem::not_found("No such page: it's past the last one"),
            });
        };
        let next = file.next.map(|n| format!("{RESIDENTS_PATH}?page={n}"));
        Ok(Reply::ok(json!({"items": file.items, "next": next})))
    }

    async fn read<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, Problem> {
        let found = read_file(&*self.store, key, &format!("reading {key}")).await?;
        Ok(found.map(|(file, _)| file))
    }
}

/// A page number: a whole number from 1, with no leading zero.
fn parse_page(token: &str) -> Option<usize> {
    let ok = !token.is_empty()
        && token.len() <= MAX_PAGE_DIGITS
        && token.bytes().all(|b| b.is_ascii_digit())
        && !token.starts_with('0');
    ok.then(|| token.parse().ok()).flatten()
}
