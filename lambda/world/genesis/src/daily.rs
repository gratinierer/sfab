//! The daily world job's genesis step: `.stories/17-genesis-codes/spec.md` "Two sources" (the
//! world) and "Tracking".

use std::collections::{BTreeMap, BTreeSet};

use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};

use sfab_core::daily::{DAY, utc_date};
use sfab_core::storage::{Condition, Metadata, Store, StoreError};

use crate::{
    BATCHES_PREFIX, BUCKET, CODES_PREFIX, IPS_PREFIX, Issue, WORLD_LABEL_PREFIX, Window, code_key,
    issue_new, to_json, write_pool,
};

const SUMMARY_KEY: &str = "genesis/summary.json";
/// World days the summary keeps.
const HISTORY_DAYS: i64 = 30;
/// S3 calls in flight at once, as the census uses.
const PARALLEL: usize = 50;

/// The job's settings, from the CDK code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Daily {
    /// `genesis_codes_per_day`: world codes made each day.
    pub codes_per_day: usize,
    /// IP records older than this are deleted.
    pub ip_window: Window,
}

/// What one run did, for the job's log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DailyReport {
    /// World codes made for today: none on a second run the same day.
    pub issued: usize,
    /// Unused world codes from earlier days, deleted.
    pub expired: usize,
    /// IP records older than the window, deleted.
    pub ips_deleted: usize,
}

/// `genesis/summary.json`: the owner's private summary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    /// Per owner label; never a `world-<date>` label.
    pub labels: BTreeMap<String, Batch>,
    /// Per UTC date, `YYYY-MM-DD`, for the last 30 days before the run.
    pub world: BTreeMap<String, WorldDay>,
}

/// One owner batch's counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Batch {
    /// Codes ever issued under the label.
    pub uploaded: usize,
    /// Codes not yet used, expired ones included.
    pub left: usize,
}

/// One world day's counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDay {
    pub issued: usize,
    pub used: usize,
}

/// The summary as the owner reads it, with when the daily job wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub summary: Summary,
    /// The Unix time of the write, S3's `LastModified`, since the summary holds no date of its
    /// own.
    pub at: i64,
}

/// The owner's summary, or `None` before the daily job's first run. One LIST, for the write
/// time, and one GET.
pub async fn read_summary<S: Store>(store: &S) -> Result<Option<Written>, StoreError> {
    let Some((body, at)) = summary_object(store).await? else {
        return Ok(None);
    };
    serde_json::from_slice(&body)
        .map(|summary| Some(Written { summary, at }))
        .map_err(|e| StoreError::Other(format!("{SUMMARY_KEY} doesn't parse: {e}")))
}

/// The summary's bytes and write time, or `None` before the first run. It lists the key before
/// reading it, since a GET on a missing key gets `403` under the job's prefix-conditioned
/// `ListBucket` (`docs/aws-facts.md`).
async fn summary_object<S: Store>(store: &S) -> Result<Option<(Vec<u8>, i64)>, StoreError> {
    let listed = store.list_sized(BUCKET, SUMMARY_KEY).await?;
    let Some(at) = listed
        .iter()
        .find(|l| l.key == SUMMARY_KEY)
        .map(|l| l.last_modified)
    else {
        return Ok(None);
    };
    let object = store.get(BUCKET, SUMMARY_KEY).await?;
    Ok(object.map(|o| (o.body, at)))
}

/// One run at `now`: counts every batch into the summary, then deletes every unused world code
/// from an earlier day and makes today's, then deletes the IP records older than the window.
/// Counting comes first, so it sees yesterday's codes before they go. Safe to repeat: a second
/// run on one day makes no new codes and refills the pool from today's unused ones.
pub async fn daily<S: Store>(
    store: &S,
    now: i64,
    settings: &Daily,
) -> Result<DailyReport, StoreError> {
    let today = utc_date(now);
    let mut summary = previous_summary(store).await?;
    let unused: BTreeSet<String> = store
        .list(BUCKET, CODES_PREFIX)
        .await?
        .into_iter()
        .filter_map(|k| k.strip_prefix(CODES_PREFIX).map(str::to_owned))
        .collect();
    let batches = batches(store).await?;

    summary.labels.clear();
    let oldest = utc_date(now - HISTORY_DAYS * DAY);
    summary.world.retain(|date, _| *date >= oldest);
    for (label, codes) in &batches {
        let left = still_unused(codes, &unused).count();
        match label.strip_prefix(WORLD_LABEL_PREFIX) {
            None => {
                let batch = Batch {
                    uploaded: codes.len(),
                    left,
                };
                summary.labels.insert(label.clone(), batch);
            }
            Some(date) if date < today.as_str() && date >= oldest.as_str() => {
                summary.world.entry(date.to_owned()).or_insert(WorldDay {
                    issued: codes.len(),
                    used: codes.len() - left,
                });
            }
            Some(_) => {}
        }
    }
    store
        .put(
            BUCKET,
            SUMMARY_KEY,
            to_json(&summary),
            Metadata::new(),
            Condition::Always,
        )
        .await?;

    let mut stale = Vec::new();
    let mut todays = Vec::new();
    for (label, codes) in &batches {
        let Some(date) = label.strip_prefix(WORLD_LABEL_PREFIX) else {
            continue;
        };
        let still = still_unused(codes, &unused).cloned();
        if date < today.as_str() {
            stale.extend(still);
        } else if date == today {
            todays.extend(still);
        }
    }
    let expired = stale.len();
    stream::iter(stale)
        .map(|code| async move { store.delete(BUCKET, &code_key(&code)).await })
        .buffer_unordered(PARALLEL)
        .try_collect::<Vec<()>>()
        .await?;

    let world_label = format!("{WORLD_LABEL_PREFIX}{today}");
    let issued = if batches.contains_key(&world_label) {
        0
    } else {
        let how = Issue::world(now);
        todays = stream::iter(0..settings.codes_per_day)
            .map(|_| issue_new(store, &how))
            .buffer_unordered(PARALLEL)
            .try_collect()
            .await?;
        todays.len()
    };
    write_pool(store, now, &todays).await?;

    let ips_deleted = forget_old_ips(store, now, settings.ip_window).await?;
    Ok(DailyReport {
        issued,
        expired,
        ips_deleted,
    })
}

/// The codes of a batch that are still unused.
fn still_unused<'a>(
    codes: &'a [String],
    unused: &'a BTreeSet<String>,
) -> impl Iterator<Item = &'a String> {
    codes.iter().filter(|c| unused.contains(*c))
}

/// Deletes every IP record last written more than `window` before `now`, so a record
/// usually lives about a day at most; lifecycle is only the backstop.
async fn forget_old_ips<S: Store>(
    store: &S,
    now: i64,
    window: Window,
) -> Result<usize, StoreError> {
    let cutoff = now - window.seconds();
    let old: Vec<String> = store
        .list_sized(BUCKET, IPS_PREFIX)
        .await?
        .into_iter()
        .filter(|l| l.last_modified < cutoff)
        .map(|l| l.key)
        .collect();
    let deleted = old.len();
    stream::iter(old)
        .map(|key| async move { store.delete(BUCKET, &key).await })
        .buffer_unordered(PARALLEL)
        .try_collect::<Vec<()>>()
        .await?;
    Ok(deleted)
}

/// The last run's summary, or an empty one.
async fn previous_summary<S: Store>(store: &S) -> Result<Summary, StoreError> {
    let Some((body, _)) = summary_object(store).await? else {
        return Ok(Summary::default());
    };
    // Only this crate writes it, so this never happens; if it does, the world's history starts
    // again rather than the pool stopping.
    Ok(serde_json::from_slice(&body).unwrap_or_else(|_| {
        tracing::error!("{SUMMARY_KEY} doesn't parse, starting a new one");
        Summary::default()
    }))
}

/// Every batch's codes, by label, from one listing of `genesis/batches/`.
async fn batches<S: Store>(store: &S) -> Result<BTreeMap<String, Vec<String>>, StoreError> {
    let mut batches: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for key in store.list(BUCKET, BATCHES_PREFIX).await? {
        let Some((label, code)) = key
            .strip_prefix(BATCHES_PREFIX)
            .and_then(|rest| rest.split_once('/'))
        else {
            continue;
        };
        batches
            .entry(label.to_owned())
            .or_default()
            .push(code.to_owned());
    }
    Ok(batches)
}
