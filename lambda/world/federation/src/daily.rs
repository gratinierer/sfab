//! The daily world job's federation step, run last (spec.md "Daily world job"): fetch each
//! pending request's token, move a match to `federation/verified/`, and email the owner once a
//! day if any request is verified. Unverified requests never reach the owner.
//!
//! Safe to repeat: a move writes `verified/` before deleting `pending/`, so a rerun finishes it,
//! and `federation/emailed.json` names the last day emailed, written before sending, so a day's
//! email is never sent twice. A failed send is a job error, and the next day sends again.

use std::future::Future;

use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};

use sfab_core::daily::{rfc3339, utc_day};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};

use crate::{MANUAL, PENDING_PREFIX, Stored, TOKEN_PATH, VERIFIED_PREFIX, verified_key};

/// Every pending request is fetched at once: there are at most about 100, and the job's
/// outbound check bounds each to 3 seconds, so slow servers can't use up the job's time.
const PARALLEL: usize = 100;
const EMAILED_KEY: &str = "federation/emailed.json";

/// The one outbound call: the body at `https://<domain><path>`, or `None` for anything that
/// isn't a plain answer within the limits, such as a redirect or a refused address. Only the daily
/// world job implements it.
pub trait Fetch: Sync {
    fn fetch(&self, domain: &str, path: &str) -> impl Future<Output = Option<Vec<u8>>> + Send;
}

/// The owner's email, through the alerts topic.
pub trait Notify: Sync {
    fn notify(
        &self,
        subject: &str,
        message: &str,
    ) -> impl Future<Output = Result<(), String>> + Send;
}

/// What the step did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyReport {
    /// Pending requests whose token was fetched.
    pub checked: usize,
    /// Of those, the ones that matched and moved to verified.
    pub verified: usize,
    /// Whether today's email went out on this run.
    pub emailed: bool,
}

#[derive(Serialize, Deserialize)]
struct Emailed {
    day: String,
}

/// The token as the whole body, with one trailing newline allowed (spec.md "Federation" step 3).
pub fn matches(body: &[u8], token: &str) -> bool {
    let body = body.strip_suffix(b"\n").unwrap_or(body);
    !token.is_empty() && body == token.as_bytes()
}

/// The step, at `now`.
pub async fn daily<S: Store, F: Fetch, N: Notify>(
    store: &S,
    fetch: &F,
    notify: &N,
    now: i64,
) -> Result<DailyReport, StoreError> {
    let pending = store.list(Bucket::Identities, PENDING_PREFIX).await?;
    let checked = pending.len();
    let verified: usize = stream::iter(pending)
        .map(|key| verify(store, fetch, key, now))
        .buffer_unordered(PARALLEL)
        .try_fold(0, |n, moved| async move { Ok(n + usize::from(moved)) })
        .await?;
    let emailed = email(store, notify, now).await?;
    Ok(DailyReport {
        checked,
        verified,
        emailed,
    })
}

/// Fetches one request's token, and moves the request on a match.
async fn verify<S: Store, F: Fetch>(
    store: &S,
    fetch: &F,
    key: String,
    now: i64,
) -> Result<bool, StoreError> {
    let Some(object) = store.get(Bucket::Identities, &key).await? else {
        return Ok(false);
    };
    let Ok(mut request) = serde_json::from_slice::<Stored>(&object.body) else {
        tracing::error!("{key} in storage doesn't parse");
        return Ok(false);
    };
    let path = format!("{TOKEN_PATH}{}", request.token);
    let Some(body) = fetch.fetch(&request.domain, &path).await else {
        return Ok(false);
    };
    if !matches(&body, &request.token) {
        return Ok(false);
    }
    request.verified_at = Some(rfc3339(now));
    let bytes = serde_json::to_vec(&request).expect("a request serializes");
    store
        .put(
            Bucket::Identities,
            &verified_key(&request.domain),
            bytes,
            Metadata::new(),
            Condition::Always,
        )
        .await?;
    store.delete(Bucket::Identities, &key).await?;
    Ok(true)
}

/// Today's one email, if any request is verified and none went out today.
async fn email<S: Store, N: Notify>(store: &S, notify: &N, now: i64) -> Result<bool, StoreError> {
    let verified = store.list(Bucket::Identities, VERIFIED_PREFIX).await?;
    if verified.is_empty() {
        return Ok(false);
    }
    let today = rfc3339(utc_day(now) * 86_400)[..10].to_owned();
    if let Some(object) = store.get(Bucket::Identities, EMAILED_KEY).await?
        && serde_json::from_slice::<Emailed>(&object.body).is_ok_and(|e| e.day == today)
    {
        return Ok(false);
    }
    let marker = serde_json::to_vec(&Emailed { day: today }).expect("a day serializes");
    store
        .put(
            Bucket::Identities,
            EMAILED_KEY,
            marker,
            Metadata::new(),
            Condition::Always,
        )
        .await?;

    let domains: Vec<&str> = verified
        .iter()
        .filter_map(|k| k.strip_prefix(VERIFIED_PREFIX)?.strip_suffix(".json"))
        .collect();
    let subject = format!("sfab: {} federation request(s) verified", domains.len());
    let message = format!(
        "These worlds proved they control their domain, and are waiting for your decision:\n\n\
         {}\n\n\
         Check each world's GET https://api.<domain>/v1/federation/world, or its own published \
         contact. To approve, add the domain to federation/worlds.yaml, commit and deploy. Either \
         way, then delete its request with\n\n    aws s3 rm \
         s3://<identities bucket>/federation/verified/<domain>.json\n\n\
         A request you leave expires after 30 days. See {MANUAL}\n",
        domains
            .iter()
            .map(|d| format!("- {d}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    notify
        .notify(&subject, &message)
        .await
        .map_err(StoreError::Other)?;
    Ok(true)
}
