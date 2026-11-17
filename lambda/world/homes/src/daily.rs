//! The daily world job's homes step: `.stories/4-life-in-the-world/spec.md` "Homes and quarters"
//! ("A home" and "Storage").
//!
//! One listing of `places/` gives every pointer. Each is read, with its resident's `home.json`,
//! and deleted unless `home.json` agrees, so a ghost, such as an old pointer a retried move
//! skipped or a taken-down agent's, lasts at most a day. A pointer written in the last
//! [`FRESH_SECONDS`] is left alone and counts as taken, because a move in flight writes its
//! pointer before `home.json`. Then, through `sfab_core::world::rebuild`, each quarter on the map
//! gets `world/quarters/<quarter>.json`, with its residents, free homes and lanes, and
//! `world/quarters/<quarter>/free.json`, up to [`MAX_FREE`] free addresses, in the map's lane
//! order, which puts gaps first, then the newest lane, then the next ones.
//!
//! The job may list `places/` but not `identities/`, and S3 answers a `GetObject` on a missing key
//! with `403` there (`docs/aws-facts.md`, a missing key under a prefix-conditioned `ListBucket`),
//! so each `home.json` is listed before it's read.

use std::collections::{BTreeMap, BTreeSet};

use futures_util::{StreamExt, TryStreamExt, stream};
use serde_json::json;

use sfab_core::daily::rfc3339;
use sfab_core::id::AgentId;
use sfab_core::storage::{Bucket, Store, StoreError};
use sfab_core::world::{self, PARALLEL, Rebuilt};
use sfab_identity::home_key;

use crate::{
    Address, Free, HOMES_PER_LANE, HomeFile, MAX_FREE, PREFIX, QUARTERS_PREFIX, Quarter, free_key,
    quarter_key, resident, to_json,
};

/// A pointer this new is left alone: longer than a move can take, which the API Lambda's
/// 10-second timeout bounds.
pub const FRESH_SECONDS: i64 = 15 * 60;

/// What a run did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyReport {
    /// Homes whose pointer and `home.json` agree, across the map.
    pub residents: usize,
    /// Pointers deleted because `home.json` disagreed.
    pub ghosts: usize,
    pub rebuilt: Rebuilt,
}

/// Cleans the pointers and writes each quarter's counts and free homes at `now`, for `map`. Safe
/// to repeat.
pub async fn daily<S: Store>(
    store: &S,
    map: &[Quarter],
    now: i64,
) -> Result<DailyReport, StoreError> {
    let listed = store.list_sized(Bucket::Identities, PREFIX).await?;
    let mut taken: BTreeSet<Address> = BTreeSet::new();
    let mut to_check = Vec::new();
    for entry in listed {
        let Some(address) = address_of_key(&entry.key, map) else {
            // A lane the owner took off the map: neither counted nor deleted.
            tracing::warn!("a pointer off the map is left alone");
            continue;
        };
        if entry.last_modified > now - FRESH_SECONDS {
            taken.insert(address);
        } else {
            to_check.push(address);
        }
    }
    let checks = to_check.into_iter().map(|address| async move {
        let agrees = agrees(store, &address, map).await?;
        Ok::<_, StoreError>((address, agrees))
    });
    let checked: Vec<(Address, bool)> = stream::iter(checks)
        .buffer_unordered(PARALLEL)
        .try_collect()
        .await?;
    let mut ghosts = 0;
    for (address, agrees) in checked {
        if agrees {
            taken.insert(address);
        } else {
            store
                .delete(Bucket::Identities, &address.pointer_key())
                .await?;
            ghosts += 1;
        }
    }

    let as_of = rfc3339(now);
    let mut files = BTreeMap::new();
    for quarter in map {
        let mut free = Vec::new();
        let mut lanes = Vec::new();
        let mut residents = 0;
        for lane in &quarter.lanes {
            let mut on_lane = 0;
            for home in 1..=HOMES_PER_LANE {
                let address = Address {
                    quarter: quarter.name.clone(),
                    lane: lane.clone(),
                    home,
                };
                if taken.contains(&address) {
                    on_lane += 1;
                } else if free.len() < MAX_FREE {
                    free.push(address.text());
                }
            }
            if on_lane > 0 {
                lanes.push(json!({"lane": lane, "residents": on_lane}));
            }
            residents += on_lane;
        }
        let capacity = quarter.lanes.len() * HOMES_PER_LANE as usize;
        let counts = json!({
            "residents": residents,
            "free": capacity - residents,
            "lanes": lanes,
            "as_of": as_of,
        });
        files.insert(quarter_key(&quarter.name), to_json(&counts));
        files.insert(free_key(&quarter.name), to_json(&Free { free }));
    }
    let rebuilt = world::rebuild(store, QUARTERS_PREFIX, files).await?;
    Ok(DailyReport {
        residents: taken.len(),
        ghosts,
        rebuilt,
    })
}

/// The address a pointer's key names, if it's on `map`.
fn address_of_key(key: &str, map: &[Quarter]) -> Option<Address> {
    let rest = key.strip_prefix(PREFIX)?.strip_suffix(".json")?;
    Address::parse(rest, map)
}

/// Whether the pointer at `address` names an agent whose `home.json` says it lives there.
async fn agrees<S: Store>(
    store: &S,
    address: &Address,
    map: &[Quarter],
) -> Result<bool, StoreError> {
    let Some(id) = resident(store, address).await? else {
        return Ok(false);
    };
    let Some(home) = home_of(store, &id).await? else {
        return Ok(false);
    };
    let lives_here = home.address.and_then(|a| Address::parse(&a, map));
    Ok(lives_here.as_ref() == Some(address))
}

/// `home.json`, listed first so a missing one is never asked for. One that doesn't parse is
/// logged and agrees with nothing.
async fn home_of<S: Store>(store: &S, id: &AgentId) -> Result<Option<HomeFile>, StoreError> {
    let key = home_key(id);
    let listed = store.list(Bucket::Identities, &key).await?;
    if !listed.contains(&key) {
        return Ok(None);
    }
    let Some(object) = store.get(Bucket::Identities, &key).await? else {
        return Ok(None);
    };
    Ok(serde_json::from_slice(&object.body)
        .inspect_err(|_| tracing::error!("a home in storage doesn't parse"))
        .ok())
}
