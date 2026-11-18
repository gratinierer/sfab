//! The takedown procedure: `.stories/2-identity-api/spec.md` "Storage". It calls each crate's
//! takedown function, so every key format stays in the crate that owns it
//! (`docs/adr/0004-rules-live-in-crates.md`).

mod args;

use std::future::Future;

pub use args::{Args, Command, USAGE, parse_args};

use sfab_core::id::AgentId;
use sfab_core::storage::{Bucket, Condition, Store, StoreError};
use sfab_core::takedown::{Action, Location, Phase, index_key, remove_id};
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
    /// Every key the takedown would touch that exists now. Empty for an ID with no claim, marker
    /// or data, such as a typo, which the program reports and skips.
    pub found: Vec<(Bucket, String)>,
}

/// The one list of an agent's locations: every crate's takedown function. Story 4 adds its
/// crates here.
pub fn locations(id: &AgentId) -> Vec<Location> {
    // The takedown index, which the crates' locations read in step 3, goes last of the
    // agent's `world/` files, just before `identities/<id>/`, so a rerun still finds it.
    let mut all = vec![Location {
        phase: Phase::Last,
        bucket: Bucket::Identities,
        key: index_key(id),
        action: Action::Delete,
    }];
    all.extend(sfab_identity::takedown(id));
    all.extend(sfab_messaging::takedown(id));
    all.extend(sfab_vouches::takedown(id));
    all.extend(sfab_homes::takedown(id));
    all.extend(sfab_square::takedown(id));
    all.extend(sfab_academy::takedown(id));
    all.extend(sfab_library::takedown(id));
    all.extend(sfab_chronicle::takedown(id));
    all
}

/// The locations that remove one writing, by its key: a Square notice, a Library edition or an
/// Academy course, each crate saying what its own writing's removal means, or `None` for any
/// other key (`.stories/4-life-in-the-world/spec.md` "Takedown").
pub fn writing(key: &str) -> Option<Vec<Location>> {
    sfab_square::writing(key)
        .or_else(|| sfab_library::writing(key))
        .or_else(|| sfab_academy::writing(key))
}

/// The keys removing one writing would still change. It writes nothing, which makes it the
/// dry run.
pub async fn plan_writing<S: Store>(
    store: &S,
    locations: &[Location],
) -> Result<Vec<(Bucket, String)>, StoreError> {
    let mut all = Vec::new();
    for l in locations {
        all.extend(found_for(store, None, l).await?);
    }
    Ok(all)
}

/// Removes one writing, in its locations' order. Its author stays, so there's no marker and no
/// wait; each step is safe to repeat, so a rerun after an interruption finishes it.
pub async fn remove_writing<S: Store>(store: &S, locations: &[Location]) -> Result<(), StoreError> {
    for l in locations {
        apply_for(store, None, l).await?;
    }
    Ok(())
}

/// The agent an action needs, which removing one writing has none of: a crate's writing names
/// only actions that need none.
fn agent(id: Option<&AgentId>) -> Result<&AgentId, StoreError> {
    id.ok_or_else(|| StoreError::Other("this action needs an agent".to_owned()))
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
    let mut all = Vec::new();
    for l in locations(id) {
        all.extend(found(store, id, &l).await?);
    }
    // A key can sit in two locations, such as `aboutme.json` under `identities/<id>/`.
    all.sort();
    all.dedup();
    Ok(all)
}

/// The keys of one location that its action would still change: every key under a prefix to
/// delete, a key to delete, mark or note that exists and isn't yet a note, the keys under a
/// prefix that name the agent, and the keys in the agent's folders still to delete or note.
pub async fn found<S: Store>(
    store: &S,
    id: &AgentId,
    l: &Location,
) -> Result<Vec<(Bucket, String)>, StoreError> {
    found_for(store, Some(id), l).await
}

async fn found_for<S: Store>(
    store: &S,
    id: Option<&AgentId>,
    l: &Location,
) -> Result<Vec<(Bucket, String)>, StoreError> {
    let mut keys = Vec::new();
    match l.action {
        Action::DeletePrefix => keys = store.list(l.bucket, &l.key).await?,
        Action::Delete | Action::Marker(_) => {
            if store.get(l.bucket, &l.key).await?.is_some() {
                keys.push(l.key.clone());
            }
        }
        Action::Note(note_of) => {
            if let Some(current) = store.get(l.bucket, &l.key).await?
                && note_of(&current.body).is_some()
            {
                keys.push(l.key.clone());
            }
        }
        Action::RemoveId => {
            let id = agent(id)?;
            for key in store.list(l.bucket, &l.key).await? {
                if let Some(current) = store.get(l.bucket, &key).await?
                    && remove_id(&current.body, id).is_some()
                {
                    keys.push(key);
                }
            }
        }
        Action::Pointers(pointers_of) => keys = pointed(store, agent(id)?, l, pointers_of).await?,
        Action::Search(belongs) => keys = searched(store, agent(id)?, l, belongs).await?,
        Action::Folders(pointers_of) => {
            keys = foldered(store, agent(id)?, l, pointers_of).await?;
        }
        Action::NoteFolders(pointers_of, note_of) => {
            for key in foldered(store, agent(id)?, l, pointers_of).await? {
                if let Some(current) = store.get(l.bucket, &key).await?
                    && note_of(&current.body).is_some()
                {
                    keys.push(key);
                }
            }
        }
        Action::Indexed(keys_of, belongs) => {
            keys = listed(store, agent(id)?, l, keys_of, belongs).await?;
        }
    }
    Ok(keys.into_iter().map(|k| (l.bucket, k)).collect())
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
    let all: Vec<(&AgentId, Location)> = targets
        .iter()
        .flat_map(|t| locations(&t.born.id).into_iter().map(|l| (&t.born.id, l)))
        .collect();
    let in_phase = |phase: Phase| all.iter().filter(move |(_, l)| l.phase == phase);
    // (1) End private access and public reading at once.
    apply_each(store, in_phase(Phase::First)).await?;
    // (2) Once per batch.
    wait.await;
    // (3) Read the agent's own files for pointers elsewhere, while they exist, and delete those.
    apply_each(store, in_phase(Phase::Pointers)).await?;
    // (4) Every other location.
    apply_each(store, in_phase(Phase::Other)).await?;
    // (5) The agent's own files last, then the first deletes again, in case a write in flight
    // recreated one.
    apply_each(store, in_phase(Phase::Last)).await?;
    apply_each(
        store,
        in_phase(Phase::First).filter(|(_, l)| matches!(l.action, Action::Delete)),
    )
    .await?;
    Ok(())
}

async fn apply_each<'a, S: Store>(
    store: &S,
    locations: impl Iterator<Item = &'a (&'a AgentId, Location)>,
) -> Result<(), StoreError> {
    for (id, l) in locations {
        apply(store, id, l).await?;
    }
    Ok(())
}

/// Applies one location's action for the agent `id`. Each is safe to repeat: what's already
/// deleted, marked, noted or rewritten counts as done.
pub async fn apply<S: Store>(store: &S, id: &AgentId, l: &Location) -> Result<(), StoreError> {
    apply_for(store, Some(id), l).await
}

async fn apply_for<S: Store>(
    store: &S,
    id: Option<&AgentId>,
    l: &Location,
) -> Result<(), StoreError> {
    match l.action {
        Action::Delete => store.delete(l.bucket, &l.key).await,
        Action::DeletePrefix => {
            for key in store.list(l.bucket, &l.key).await? {
                store.delete(l.bucket, &key).await?;
            }
            Ok(())
        }
        Action::Marker(metadata_of) => mark(store, l, metadata_of).await,
        Action::Note(note_of) => rewrite(store, l.bucket, &l.key, note_of).await,
        Action::RemoveId => {
            let id = agent(id)?;
            for key in store.list(l.bucket, &l.key).await? {
                rewrite(store, l.bucket, &key, |body| remove_id(body, id)).await?;
            }
            Ok(())
        }
        Action::Pointers(pointers_of) => {
            for key in pointed(store, agent(id)?, l, pointers_of).await? {
                store.delete(l.bucket, &key).await?;
            }
            Ok(())
        }
        Action::Search(belongs) => {
            for key in searched(store, agent(id)?, l, belongs).await? {
                store.delete(l.bucket, &key).await?;
            }
            Ok(())
        }
        Action::Folders(pointers_of) => {
            for key in foldered(store, agent(id)?, l, pointers_of).await? {
                store.delete(l.bucket, &key).await?;
            }
            Ok(())
        }
        Action::NoteFolders(pointers_of, note_of) => {
            for key in foldered(store, agent(id)?, l, pointers_of).await? {
                rewrite(store, l.bucket, &key, note_of).await?;
            }
            Ok(())
        }
        Action::Indexed(keys_of, belongs) => {
            for key in listed(store, agent(id)?, l, keys_of, belongs).await? {
                store.delete(l.bucket, &key).await?;
            }
            Ok(())
        }
    }
}

/// The keys the agent's file at `l.key` names that exist and that `belongs` says are the
/// agent's from the key alone.
async fn listed<S: Store>(
    store: &S,
    id: &AgentId,
    l: &Location,
    keys_of: fn(&[u8]) -> Vec<String>,
    belongs: fn(&str, &AgentId) -> bool,
) -> Result<Vec<String>, StoreError> {
    let Some(file) = store.get(l.bucket, &l.key).await? else {
        return Ok(Vec::new());
    };
    let mut keys = Vec::new();
    for key in keys_of(&file.body) {
        if belongs(&key, id) && store.head(l.bucket, &key).await?.is_some() {
            keys.push(key);
        }
    }
    Ok(keys)
}

/// Every key in the folder of each key the agent's file at `l.key` points to that still names
/// the agent, each folder's named key last.
async fn foldered<S: Store>(
    store: &S,
    id: &AgentId,
    l: &Location,
    pointers_of: fn(&[u8]) -> Vec<String>,
) -> Result<Vec<String>, StoreError> {
    let mut keys = Vec::new();
    for anchor in pointed(store, id, l, pointers_of).await? {
        let folder = &anchor[..anchor.rfind('/').map_or(0, |at| at + 1)];
        if folder.is_empty() {
            continue;
        }
        let listed = store.list(l.bucket, folder).await?;
        keys.extend(listed.into_iter().filter(|k| *k != anchor));
        keys.push(anchor);
    }
    Ok(keys)
}

/// The keys under the prefix `l.key` that `belongs` says are the agent's, from one listing.
async fn searched<S: Store>(
    store: &S,
    id: &AgentId,
    l: &Location,
    belongs: fn(&str, &AgentId) -> bool,
) -> Result<Vec<String>, StoreError> {
    let listed = store.list(l.bucket, &l.key).await?;
    Ok(listed.into_iter().filter(|k| belongs(k, id)).collect())
}

/// The keys the agent's file at `l.key` points to that exist and still name the agent, so a
/// file that's out of date can't delete someone else's pointer.
async fn pointed<S: Store>(
    store: &S,
    id: &AgentId,
    l: &Location,
    pointers_of: fn(&[u8]) -> Vec<String>,
) -> Result<Vec<String>, StoreError> {
    let Some(file) = store.get(l.bucket, &l.key).await? else {
        return Ok(Vec::new());
    };
    let mut keys = Vec::new();
    for key in pointers_of(&file.body) {
        if let Some(current) = store.get(l.bucket, &key).await?
            && remove_id(&current.body, id).is_some()
        {
            keys.push(key);
        }
    }
    Ok(keys)
}

/// Replaces the object's body with what `change` makes of it, with `If-Match`, so a write
/// racing the takedown, such as the daily job's, can't be lost or undo it: on a lost race it
/// reads again. `None` from `change`, or no object, means nothing to do.
async fn rewrite<S: Store>(
    store: &S,
    bucket: Bucket,
    key: &str,
    change: impl Fn(&[u8]) -> Option<Vec<u8>>,
) -> Result<(), StoreError> {
    loop {
        let Some(current) = store.get(bucket, key).await? else {
            return Ok(());
        };
        let Some(body) = change(&current.body) else {
            return Ok(());
        };
        let write = store
            .put(
                bucket,
                key,
                body,
                current.metadata,
                Condition::IfMatch(current.etag),
            )
            .await;
        match write {
            Err(StoreError::ConditionFailed | StoreError::Missing) => continue,
            other => return other,
        }
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
        // A claim that doesn't parse still gets its marker, so the ID can't be born again; it
        // keeps no `born_at`, so only a takedown by ID finds it again.
        let metadata = metadata_of(&current.body).unwrap_or_else(|| {
            eprintln!(
                "warning: {} doesn't parse, so its marker keeps no born_at",
                l.key
            );
            Default::default()
        });
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
