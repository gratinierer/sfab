//! Standing, worked out fresh by the daily world job, and how the routes read it:
//! `.stories/4-life-in-the-world/spec.md` "Vouches: trust" ("Standing, worked out fresh by the
//! daily world job", "Whose words count", "What's visible").
//!
//! Standing flows outward from the founders only. Each round reads the vouch files of the agents
//! that just gained standing, and an agent gains it with [`MIN_VOUCHES`] vouches from agents
//! that have it, and [`AGE_DAYS`] of age unless it was born before `age_rule_starts`. Nothing
//! carries over from an earlier run, so there's no grace period, and a voucher losing standing
//! cascades. Only vouch files of agents with standing are read, so fake vouches cost nothing to
//! process, and only residents count, so a takedown marker or an unknown ID gives and gets
//! nothing.
//!
//! What it writes, through `sfab_core::world::rebuild`: `world/standing/<id>.json` for each agent
//! with standing, `world/vouches/<id>.json` with each agent's counted vouches, and last
//! `world/standing.json` with the run's time, which says the job has run and gives `as_of`.

use std::collections::{BTreeMap, BTreeSet};

use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::{DAY, rfc3339};
use sfab_core::id::AgentId;
use sfab_core::problem::Problem;
use sfab_core::storage::{Bucket, Store, StoreError};
use sfab_core::world::{self, PARALLEL, Rebuilt};

use crate::{Given, PREFIX, given_key, to_json};

/// Where the job writes each agent with standing, `world/standing/<id>.json`.
pub const STANDING_PREFIX: &str = "world/standing/";
/// Where the job writes each agent's counted vouches, `world/vouches/<id>.json`.
pub const VOUCHED_BY_PREFIX: &str = "world/vouches/";
/// The job's last run, `{"as_of": ...}`: once it exists, a missing file above means nothing to
/// show, rather than not built yet.
const BUILT_KEY: &str = "world/standing.json";
/// Counted vouches an agent needs for standing.
pub const MIN_VOUCHES: usize = 3;
/// Days of age an agent needs for standing, unless born before `age_rule_starts`.
pub const AGE_DAYS: i64 = 30;

pub(crate) fn standing_key(id: &AgentId) -> String {
    format!("{STANDING_PREFIX}{id}.json")
}

pub(crate) fn vouched_by_key(id: &AgentId) -> String {
    format!("{VOUCHED_BY_PREFIX}{id}.json")
}

/// One counted vouch, as `vouched-by` shows it: the giver's ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Received {
    id: String,
    reason: String,
    given_at: String,
}

/// `world/vouches/<id>.json`, by giver ID.
#[derive(Debug, Serialize, Deserialize)]
struct Counted {
    items: Vec<Received>,
}

/// What a run did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyReport {
    /// Agents with standing, founders included.
    pub standing: usize,
    /// Agents with at least one counted vouch.
    pub vouched: usize,
    pub rebuilt: Rebuilt,
}

/// Works out standing at `now` from `founders` and the vouch files, counting only `residents`,
/// the agents with a claim that isn't a takedown marker. `age_rule_starts` is RFC 3339 to the
/// second, as `born_at` is, so they compare as text. Safe to repeat: a second run with nothing
/// changed writes only its time.
pub async fn daily<S: Store>(
    store: &S,
    founders: &[AgentId],
    residents: &[AgentId],
    age_rule_starts: &str,
    now: i64,
) -> Result<DailyReport, StoreError> {
    let residents: BTreeSet<&AgentId> = residents.iter().collect();
    // One listing says which agents gave vouches, so no missing key is ever asked for.
    let with_files: BTreeSet<String> = store
        .list(Bucket::Identities, PREFIX)
        .await?
        .into_iter()
        .collect();
    let births = sfab_identity::birth_dates(store).await?;
    let oldest = rfc3339(now - AGE_DAYS * DAY);

    let mut standing: BTreeSet<AgentId> = founders
        .iter()
        .filter(|f| residents.contains(f))
        .cloned()
        .collect();
    let mut received: BTreeMap<AgentId, BTreeMap<String, Received>> = BTreeMap::new();
    let mut too_young: BTreeSet<AgentId> = BTreeSet::new();
    let mut frontier: Vec<AgentId> = standing.iter().cloned().collect();
    while !frontier.is_empty() {
        let reads = frontier
            .iter()
            .filter(|g| with_files.contains(&given_key(g)))
            .map(
                |giver| async move { Ok::<_, StoreError>((giver, given_by(store, giver).await?)) },
            );
        let files: Vec<(&AgentId, Given)> = stream::iter(reads)
            .buffer_unordered(PARALLEL)
            .try_collect()
            .await?;
        let mut candidates = BTreeSet::new();
        for (giver, given) in files {
            for vouch in given.vouches {
                let Some(to) = AgentId::parse(&vouch.id) else {
                    continue;
                };
                if &to == giver || !residents.contains(&to) {
                    continue;
                }
                let counted = received.entry(to.clone()).or_default();
                counted.insert(
                    giver.to_string(),
                    Received {
                        id: giver.to_string(),
                        reason: vouch.reason,
                        given_at: vouch.given_at,
                    },
                );
                if counted.len() >= MIN_VOUCHES && !standing.contains(&to) {
                    candidates.insert(to);
                }
            }
        }
        frontier = Vec::new();
        for candidate in candidates {
            if too_young.contains(&candidate) {
                continue;
            }
            let born_at = births.born_at(store, &candidate).await?;
            let old_enough = born_at
                .as_deref()
                .is_some_and(|b| b < age_rule_starts || b <= oldest.as_str());
            if old_enough {
                standing.insert(candidate.clone());
                frontier.push(candidate);
            } else {
                too_young.insert(candidate);
            }
        }
    }

    let standing_files = standing
        .iter()
        .map(|id| (standing_key(id), to_json(&json!({"standing": true}))))
        .collect();
    let vouched_files = received
        .iter()
        .map(|(id, counted)| {
            let items = counted.values().cloned().collect();
            (vouched_by_key(id), to_json(&Counted { items }))
        })
        .collect();
    let a = world::rebuild(store, STANDING_PREFIX, standing_files).await?;
    let b = world::rebuild(store, VOUCHED_BY_PREFIX, vouched_files).await?;
    world::write_built(store, BUILT_KEY, &rfc3339(now)).await?;
    Ok(DailyReport {
        standing: standing.len(),
        vouched: received.len(),
        rebuilt: Rebuilt {
            written: a.written + b.written + 1,
            unchanged: a.unchanged + b.unchanged,
            deleted: a.deleted + b.deleted,
        },
    })
}

/// A giver's vouches. One that doesn't parse is logged and counts for nothing, so one bad file
/// can't stop the whole world's standing.
async fn given_by<S: Store>(store: &S, giver: &AgentId) -> Result<Given, StoreError> {
    let key = given_key(giver);
    let Some(object) = store.get(Bucket::Identities, &key).await? else {
        return Ok(Given::default());
    };
    Ok(serde_json::from_slice(&object.body).unwrap_or_else(|_| {
        tracing::error!("{key} doesn't parse, so its vouches don't count today");
        Given::default()
    }))
}

/// The job's last run, or `None` before the first.
async fn built<S: Store>(store: &S) -> Result<Option<String>, Problem> {
    world::built_at(store, BUILT_KEY).await
}

/// Every agent with standing, from one listing of the files the job last wrote, for another
/// institution's daily step that runs after this one, such as the Academy's counting of
/// certificates. Empty before the first run.
pub async fn with_standing<S: Store>(store: &S) -> Result<BTreeSet<AgentId>, StoreError> {
    Ok(store
        .list(Bucket::Identities, STANDING_PREFIX)
        .await?
        .iter()
        .filter_map(|k| k.strip_prefix(STANDING_PREFIX)?.strip_suffix(".json"))
        .filter_map(AgentId::parse)
        .collect())
}

/// Whether `id` has standing, and `as_of` the job's last run: `(false, None)` before the first.
pub(crate) async fn read<S: Store>(
    store: &S,
    id: &AgentId,
) -> Result<(bool, Option<String>), Problem> {
    let Some(as_of) = built(store).await? else {
        return Ok((false, None));
    };
    let found = store
        .head(Bucket::Identities, &standing_key(id))
        .await
        .map_err(|e| Problem::storage("reading standing", &e))?;
    Ok((found.is_some(), Some(as_of)))
}

/// `id`'s counted vouches, by giver ID: none for an agent without a file once the job has run,
/// and `not-built-yet` before.
pub(crate) async fn counted<S: Store>(store: &S, id: &AgentId) -> Result<Vec<Value>, Problem> {
    let nothing = json!({"items": []});
    let file = world::read_for_agent(store, &vouched_by_key(id), BUILT_KEY, nothing).await?;
    Ok(file["items"].as_array().cloned().unwrap_or_default())
}
