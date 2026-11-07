//! The takedown program's tests, from issue 08's Done when, against `core`'s in-memory fake,
//! filled through each crate's own write functions.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sfab_core::gate::Call;
use sfab_core::id::AgentId;
use sfab_core::proof::{self, Request};
use sfab_core::storage::{Bucket, Condition, MemoryStore, Metadata, Store};
use sfab_identity::Identity;
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
    // aboutme.json (issue 13), contacts.json (issue 13) and messages (issue 14) have no write
    // functions yet; those tickets replace these direct writes with them.
    for key in [
        format!("identities/{id}/aboutme.json"),
        format!("identities/{id}/contacts.json"),
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
    let reply = deliver(w, "POST", BIRTH, None, body.as_bytes(), proof, agent.born_at);
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
    execute(&*w.store, &targets, counting(&waits)).await.unwrap();

    assert_eq!(waits.load(Ordering::SeqCst), 1);
    assert_taken_down(&w, &agent, "2026-11-05T12:00:00Z").await;
    assert_eq!(keys_of(&w.store, &bystander.id), bystander_keys);
}
