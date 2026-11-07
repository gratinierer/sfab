//! The takedown program's tests, from issue 08's Done when, against `core`'s in-memory fake,
//! filled through each crate's own write functions.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use sfab_core::gate::Call;
use sfab_core::id::AgentId;
use sfab_core::proof::{self, Request};
use sfab_core::storage::{Bucket, Condition, MemoryStore, Metadata, Object, Store, StoreError};
use sfab_identity::{Identity, List};
use sfab_takedown::{Mode, execute, plan};

const BITS: u32 = 8;
/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;

type Answer = Result<serde_json::Value, (u16, &'static str)>;

struct World {
    store: Arc<MemoryStore>,
    identity: Identity<MemoryStore>,
}

fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    World {
        identity: Identity::new(store.clone(), BITS),
        store,
    }
}

struct Agent {
    id: AgentId,
    name: String,
    birth_proof: String,
    born_at: i64,
    claim: String,
    recovery: String,
}

fn bearer(id: &AgentId, secret: &str) -> String {
    format!("Bearer {id}.{secret}")
}

/// A correct proof for a write, solved at `now`.
fn solve(method: &str, path: &str, authorization: Option<&str>, body: &[u8], now: i64) -> String {
    let caller = sfab_core::credentials::caller_of(authorization).unwrap();
    let request = Request {
        method,
        path,
        caller: &caller,
        body,
    };
    proof::solve(&request, now, BITS)
}

/// Delivers a write with the given proof.
async fn deliver(
    w: &World,
    method: &'static str,
    path: &'static str,
    authorization: Option<String>,
    body: &[u8],
    proof: &str,
    now: i64,
) -> Answer {
    let call = Call {
        method,
        path,
        authorization: authorization.as_deref(),
        proof: Some(proof),
        body,
    };
    let reply = match path {
        "/v1/identity/birth" => w.identity.birth(&call, now).await,
        "/v1/identity/evolve" => w.identity.evolve(&call, now).await,
        _ => w.identity.reclaim(&call, now).await,
    };
    reply.map(|r| r.body).map_err(|p| (p.status, p.kind))
}

/// Sends a write with a fresh, correct proof.
async fn send(
    w: &World,
    method: &'static str,
    path: &'static str,
    authorization: Option<String>,
    body: &[u8],
    now: i64,
) -> Answer {
    let proof = solve(method, path, authorization.as_deref(), body, now);
    deliver(w, method, path, authorization, body, &proof, now).await
}

const BIRTH: &str = "/v1/identity/birth";

/// An agent born at `now`, with every kind of data this story stores.
async fn full_agent(w: &World, name: &str, now: i64) -> Agent {
    let body = format!(r#"{{"name":"{name}"}}"#);
    let birth_proof = solve("POST", BIRTH, None, body.as_bytes(), now);
    let reply = deliver(w, "POST", BIRTH, None, body.as_bytes(), &birth_proof, now)
        .await
        .expect("birth");
    let id = AgentId::parse(reply["id"].as_str().unwrap()).unwrap();
    let claim = reply["claim"].as_str().unwrap().to_owned();
    let recovery = reply["recovery"].as_str().unwrap().to_owned();
    send(
        w,
        "PUT",
        "/v1/identity/evolve",
        Some(bearer(&id, &claim)),
        br#"{"whoiam":"x"}"#,
        now,
    )
    .await
    .expect("evolve");
    // A public profile, a friend and a block: aboutme.json and contacts.json.
    let auth = bearer(&id, &claim);
    let settings = br#"{"name":true,"whoiam":true,"friends":true}"#;
    let publication = "/v1/identity/self/publication";
    let proof = solve("PUT", publication, Some(&auth), settings, now);
    let call = Call {
        method: "PUT",
        path: publication,
        authorization: Some(&auth),
        proof: Some(&proof),
        body: settings,
    };
    w.identity.publish(&call, now).await.expect("publication");
    for (list, other) in [
        (List::Friends, "friendfriend"),
        (List::Blocked, "blockedblock"),
    ] {
        let segment = if list == List::Friends {
            "friends"
        } else {
            "blocked"
        };
        let path = format!("/v1/identity/self/{segment}/{other}");
        let proof = solve("PUT", &path, Some(&auth), b"", now);
        let call = Call {
            method: "PUT",
            path: &path,
            authorization: Some(&auth),
            proof: Some(&proof),
            body: b"",
        };
        let added = w.identity.add_contact(list, &call, other, now).await;
        assert_eq!(added.expect("a contact").status, 201);
    }
    let about = w.identity.aboutme(id.as_str()).await.expect("aboutme");
    assert_eq!(about.body["friends"][0], "friendfriend");
    for file in ["aboutme.json", "contacts.json"] {
        let key = format!("identities/{id}/{file}");
        assert!(w.store.object(Bucket::Identities, &key).is_some(), "{file}");
    }
    // Messages (issue 14) have no write function yet; that ticket replaces these direct writes.
    for key in [
        format!("messages/{id}/mail/0001.json"),
        format!("messages/{id}/knock/0001.json"),
    ] {
        w.store
            .put(
                Bucket::Identities,
                &key,
                b"{}".to_vec(),
                Metadata::new(),
                Condition::Always,
            )
            .await
            .unwrap();
    }
    Agent {
        id,
        name: name.to_owned(),
        birth_proof,
        born_at: now,
        claim,
        recovery,
    }
}

/// Replays an agent's own `birth`, which would make the same ID.
async fn rebirth(w: &World, agent: &Agent) -> Result<(), (u16, &'static str)> {
    let body = format!(r#"{{"name":"{}"}}"#, agent.name);
    let proof = &agent.birth_proof;
    let reply = deliver(
        w,
        "POST",
        BIRTH,
        None,
        body.as_bytes(),
        proof,
        agent.born_at,
    );
    reply.await.map(|_| ())
}

/// Every key of `id` in both buckets.
fn keys_of(store: &MemoryStore, id: &AgentId) -> Vec<String> {
    [Bucket::Claims, Bucket::Identities]
        .into_iter()
        .flat_map(|b| store.keys(b))
        .filter(|k| k.contains(id.as_str()))
        .collect()
}

/// A wait that counts how often it ran.
async fn counting(waits: &AtomicUsize) {
    waits.fetch_add(1, Ordering::SeqCst);
}

/// The agent is gone: only a marker that keeps `born_at` is left, the ID can't be born again,
/// and its old secrets are dead.
async fn assert_taken_down(w: &World, agent: &Agent, born_at: &str) {
    let claim_key = format!("claims/{}/claim.json", agent.id);
    assert_eq!(keys_of(&w.store, &agent.id), vec![claim_key.clone()]);
    let marker = w.store.object(Bucket::Claims, &claim_key).unwrap();
    assert!(marker.body.is_empty());
    assert_eq!(marker.metadata["born_at"], born_at);
    let about = w.identity.aboutme(agent.id.as_str()).await;
    assert_eq!(about.unwrap_err().status, 404);

    assert_eq!(rebirth(w, agent).await, Err((409, "already-exists")));
    let evolve = send(
        w,
        "PUT",
        "/v1/identity/evolve",
        Some(bearer(&agent.id, &agent.claim)),
        br#"{"whoiam":"y"}"#,
        NOW,
    )
    .await;
    assert_eq!(evolve.unwrap_err().0, 401);
    let reclaim = send(
        w,
        "POST",
        "/v1/identity/reclaim",
        Some(bearer(&agent.id, &agent.recovery)),
        b"",
        NOW,
    )
    .await;
    assert_eq!(reclaim.unwrap_err().0, 401);
}

#[tokio::test]
async fn one_agent_with_every_kind_of_data_leaves_only_a_marker() {
    let w = world();
    let agent = full_agent(&w, "Wren", NOW).await;
    let bystander = full_agent(&w, "Moss", NOW).await;
    let bystander_keys = keys_of(&w.store, &bystander.id);

    let targets = plan(&*w.store, &Mode::One(agent.id.clone())).await.unwrap();
    let waits = AtomicUsize::new(0);
    execute(&*w.store, &targets, counting(&waits))
        .await
        .unwrap();

    assert_eq!(waits.load(Ordering::SeqCst), 1);
    assert_taken_down(&w, &agent, "2026-11-05T12:00:00Z").await;
    assert_eq!(keys_of(&w.store, &bystander.id), bystander_keys);
}

/// Every key in both buckets with its body.
fn snapshot(store: &MemoryStore) -> Vec<(String, Vec<u8>)> {
    [Bucket::Claims, Bucket::Identities]
        .into_iter()
        .flat_map(|b| {
            store
                .keys(b)
                .into_iter()
                .map(move |k| (k.clone(), store.object(b, &k).unwrap().body))
        })
        .collect()
}

const HOUR: i64 = 3_600;

fn window(from: &str, to: &str) -> Mode {
    Mode::Window {
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

/// Four agents born an hour apart, from 11:00; the window 12:00 to 14:00 catches the middle two.
async fn four_agents(w: &World) -> [Agent; 4] {
    [
        full_agent(w, "Early", NOW - HOUR).await,
        full_agent(w, "Flood1", NOW).await,
        full_agent(w, "Flood2", NOW + HOUR).await,
        full_agent(w, "Late", NOW + 2 * HOUR).await,
    ]
}

fn flood() -> Mode {
    window("2026-11-05T12:00:00Z", "2026-11-05T14:00:00Z")
}

fn sorted(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys
}

fn sorted_ids<'a>(ids: impl IntoIterator<Item = &'a AgentId>) -> Vec<AgentId> {
    let mut ids: Vec<_> = ids.into_iter().cloned().collect();
    ids.sort();
    ids
}

#[tokio::test]
async fn a_dry_run_shows_the_window_and_writes_nothing() {
    let w = world();
    let [_, first, second, _] = four_agents(&w).await;
    let stored = snapshot(&w.store);

    let targets = plan(&*w.store, &flood()).await.unwrap();

    assert_eq!(snapshot(&w.store), stored);
    assert_eq!(
        sorted_ids(targets.iter().map(|t| &t.id)),
        sorted_ids([&first.id, &second.id])
    );
    let t = targets.iter().find(|t| t.id == first.id).unwrap();
    assert_eq!(t.born_at.as_deref(), Some("2026-11-05T12:00:00Z"));
    assert!(!t.taken_down);
    let found: Vec<_> = t.found.iter().map(|(_, k)| k.clone()).collect();
    assert_eq!(sorted(found), sorted(keys_of(&w.store, &first.id)));
}

/// The fake, with deletes under `vitals/` failing while `broken` is set: a run cut off after
/// step 1 wrote the markers.
struct Flaky {
    inner: Arc<MemoryStore>,
    broken: AtomicBool,
}

impl Store for Flaky {
    async fn get(&self, bucket: Bucket, key: &str) -> Result<Option<Object>, StoreError> {
        self.inner.get(bucket, key).await
    }

    async fn put(
        &self,
        bucket: Bucket,
        key: &str,
        body: Vec<u8>,
        metadata: Metadata,
        condition: Condition,
    ) -> Result<(), StoreError> {
        self.inner.put(bucket, key, body, metadata, condition).await
    }

    async fn delete(&self, bucket: Bucket, key: &str) -> Result<(), StoreError> {
        if self.broken.load(Ordering::SeqCst) && key.starts_with("vitals/") {
            return Err(StoreError::Other("cut off".to_owned()));
        }
        self.inner.delete(bucket, key).await
    }

    async fn list(&self, bucket: Bucket, prefix: &str) -> Result<Vec<String>, StoreError> {
        self.inner.list(bucket, prefix).await
    }
}

#[tokio::test]
async fn a_window_run_cut_off_after_the_markers_finishes_on_a_rerun() {
    let w = world();
    let [before, first, second, after] = four_agents(&w).await;
    let untouched = [keys_of(&w.store, &before.id), keys_of(&w.store, &after.id)];
    let flaky = Flaky {
        inner: w.store.clone(),
        broken: AtomicBool::new(true),
    };

    let targets = plan(&flaky, &flood()).await.unwrap();
    let waits = AtomicUsize::new(0);
    assert!(execute(&flaky, &targets, counting(&waits)).await.is_err());
    let claim = w
        .store
        .object(Bucket::Claims, &format!("claims/{}/claim.json", first.id))
        .unwrap();
    assert!(claim.body.is_empty(), "the markers were written");

    // The rerun lists again, finding the markers by their born_at.
    flaky.broken.store(false, Ordering::SeqCst);
    let targets = plan(&flaky, &flood()).await.unwrap();
    assert_eq!(
        sorted_ids(targets.iter().map(|t| &t.id)),
        sorted_ids([&first.id, &second.id])
    );
    assert!(targets.iter().all(|t| t.taken_down));
    execute(&flaky, &targets, counting(&waits)).await.unwrap();

    assert_taken_down(&w, &first, "2026-11-05T12:00:00Z").await;
    assert_taken_down(&w, &second, "2026-11-05T13:00:00Z").await;
    assert_eq!(
        [keys_of(&w.store, &before.id), keys_of(&w.store, &after.id)],
        untouched
    );
}

#[tokio::test]
async fn writes_in_flight_during_the_wait_are_removed_too() {
    let w = world();
    let agent = full_agent(&w, "Wren", NOW).await;
    let targets = plan(&*w.store, &Mode::One(agent.id.clone())).await.unwrap();

    // Writes that passed the claim check just before step 1 land during the wait.
    let in_flight = async {
        for key in ["aboutme.json", "identity.json"] {
            let key = format!("identities/{}/{key}", agent.id);
            let body = b"{}".to_vec();
            let (b, m, c) = (Bucket::Identities, Metadata::new(), Condition::Always);
            w.store.put(b, &key, body, m, c).await.unwrap();
        }
    };
    execute(&*w.store, &targets, in_flight).await.unwrap();

    assert_taken_down(&w, &agent, "2026-11-05T12:00:00Z").await;
}

#[tokio::test]
async fn a_rerun_of_a_finished_takedown_succeeds_and_changes_nothing() {
    let w = world();
    let agent = full_agent(&w, "Wren", NOW).await;
    let mode = Mode::One(agent.id.clone());
    let waits = AtomicUsize::new(0);
    let targets = plan(&*w.store, &mode).await.unwrap();
    execute(&*w.store, &targets, counting(&waits))
        .await
        .unwrap();
    let stored = snapshot(&w.store);

    let targets = plan(&*w.store, &mode).await.unwrap();
    assert!(targets[0].taken_down);
    execute(&*w.store, &targets, counting(&waits))
        .await
        .unwrap();

    assert_eq!(snapshot(&w.store), stored);
    assert_taken_down(&w, &agent, "2026-11-05T12:00:00Z").await;
}

#[tokio::test]
async fn a_window_takes_down_everyone_born_in_it_with_one_wait() {
    let w = world();
    let [before, first, second, after] = four_agents(&w).await;
    let untouched = [keys_of(&w.store, &before.id), keys_of(&w.store, &after.id)];

    let targets = plan(&*w.store, &flood()).await.unwrap();
    let waits = AtomicUsize::new(0);
    execute(&*w.store, &targets, counting(&waits))
        .await
        .unwrap();

    assert_eq!(waits.load(Ordering::SeqCst), 1);
    assert_taken_down(&w, &first, "2026-11-05T12:00:00Z").await;
    assert_taken_down(&w, &second, "2026-11-05T13:00:00Z").await;
    assert_eq!(
        [keys_of(&w.store, &before.id), keys_of(&w.store, &after.id)],
        untouched
    );
}
