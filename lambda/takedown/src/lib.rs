//! The takedown procedure: `.stories/2-identity-api/spec.md` "Storage". It calls each crate's
//! takedown function, so every key format stays in the crate that owns it
//! (`docs/adr/0004-rules-live-in-crates.md`).

mod args;

use std::future::Future;

pub use args::{Args, USAGE, parse_args};

use sfab_core::id::AgentId;
use sfab_core::storage::{Bucket, Condition, Store, StoreError};
use sfab_core::takedown::{Action, Location, Phase};
use sfab_identity::Born;

/// Which agents to take down.
#[derive(Debug, Clone)]
pub enum Mode {
    One(AgentId),
    /// Every agent whose `born_at` is at or after `from` and before `to`, as RFC 3339 in UTC to the
    /// second, such as `2026-11-05T12:00:00Z`.
    Window {
        from: String,
        to: String,
    },
}

/// One agent the takedown would handle, and what it found of the agent's data.
#[derive(Debug, Clone)]
pub struct Target {
    /// Its ID and `born_at`; `born_at` is `None` when there's no claim, or it doesn't parse.
    pub born: Born,
    /// Every key the takedown would touch that exists now.
    pub found: Vec<(Bucket, String)>,
}

/// The one list of an agent's locations: every crate's takedown function. Story 4 adds its
/// crates here.
pub fn locations(id: &AgentId) -> Vec<Location> {
    let mut all = sfab_identity::takedown(id);
    all.extend(sfab_messaging::takedown(id));
    all
}

/// What a takedown would handle. It writes nothing, which makes it the dry run.
pub async fn plan<S: Store>(store: &S, mode: &Mode) -> Result<Vec<Target>, StoreError> {
    let born = match mode {
        Mode::One(id) => {
            vec![
                sfab_identity::born(store, id)
                    .await?
                    .unwrap_or_else(|| Born::unclaimed(id.clone())),
            ]
        }
        Mode::Window { from, to } => sfab_identity::born_between(store, from, to).await?,
    };
    let mut targets = Vec::with_capacity(born.len());
    for born in born {
        let found = existing_keys(store, &born.id).await?;
        targets.push(Target { born, found });
    }
    Ok(targets)
}

/// The keys of an agent's locations that exist now.
async fn existing_keys<S: Store>(
    store: &S,
    id: &AgentId,
) -> Result<Vec<(Bucket, String)>, StoreError> {
    let mut found = Vec::new();
    for l in locations(id) {
        match l.action {
            Action::DeletePrefix => {
                for key in store.list(l.bucket, &l.key).await? {
                    found.push((l.bucket, key));
                }
            }
            Action::Delete | Action::Marker(_) => {
                if store.get(l.bucket, &l.key).await?.is_some() {
                    found.push((l.bucket, l.key));
                }
            }
        }
    }
    // A key can sit in two locations, such as `aboutme.json` under `identities/<id>/`.
    found.sort();
    found.dedup();
    Ok(found)
}

/// Takes the targets down in the spec's order, calling `wait` once for the whole batch, which
/// must last at least the API Lambda's timeout so any write in flight finishes. Deletes that
/// already happened, and claims that are already markers, count as done, so a rerun after an
/// interruption finishes the job.
pub async fn execute<S: Store, W: Future<Output = ()>>(
    store: &S,
    targets: &[Target],
    wait: W,
) -> Result<(), StoreError> {
    let all: Vec<Vec<Location>> = targets.iter().map(|t| locations(&t.born.id)).collect();
    let in_phase = |phase: Phase| all.iter().flatten().filter(move |l| l.phase == phase);
    // (1) End private access and public reading at once.
    apply_each(store, in_phase(Phase::First)).await?;
    // (2) Once per batch.
    wait.await;
    // (3) Reading the agent's files for pointers elsewhere arrives with story 4, whose pointers
    // they are. (4) Every other location.
    apply_each(store, in_phase(Phase::Other)).await?;
    // (5) The agent's own files last, then the first deletes again, in case a write in flight
    // recreated one.
    apply_each(store, in_phase(Phase::Last)).await?;
    apply_each(
        store,
        in_phase(Phase::First).filter(|l| matches!(l.action, Action::Delete)),
    )
    .await?;
    Ok(())
}

async fn apply_each<'a, S: Store>(
    store: &S,
    locations: impl Iterator<Item = &'a Location>,
) -> Result<(), StoreError> {
    for l in locations {
        apply(store, l).await?;
    }
    Ok(())
}

async fn apply<S: Store>(store: &S, l: &Location) -> Result<(), StoreError> {
    match l.action {
        Action::Delete => store.delete(l.bucket, &l.key).await,
        Action::DeletePrefix => {
            for key in store.list(l.bucket, &l.key).await? {
                store.delete(l.bucket, &key).await?;
            }
            Ok(())
        }
        Action::Marker(metadata_of) => mark(store, l, metadata_of).await,
    }
}

/// Replaces the object with a zero-byte marker, with `If-Match`, so a `reclaim` racing the
/// takedown can't leave a live claim behind: on a lost race it reads again.
async fn mark<S: Store>(
    store: &S,
    l: &Location,
    metadata_of: fn(&[u8]) -> Option<sfab_core::storage::Metadata>,
) -> Result<(), StoreError> {
    loop {
        let Some(current) = store.get(l.bucket, &l.key).await? else {
            // No claim: nothing to mark.
            return Ok(());
        };
        if current.body.is_empty() {
            return Ok(());
        }
        let metadata = metadata_of(&current.body)
            .ok_or_else(|| StoreError::Other(format!("{} doesn't parse", l.key)))?;
        let write = store
            .put(
                l.bucket,
                &l.key,
                Vec::new(),
                metadata,
                Condition::IfMatch(current.etag),
            )
            .await;
        match write {
            Err(StoreError::ConditionFailed | StoreError::Missing) => continue,
            other => return other,
        }
    }
}
