//! What the vouches tests share: a world on the in-memory fake, agents born through `birth`, and
//! the vouch routes called with correct proofs.

#![allow(dead_code)]

use std::sync::Arc;

use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::id::AgentId;
use sfab_core::proof::{self, Request};
use sfab_core::storage::MemoryStore;
use sfab_genesis::testing as genesis;
use sfab_identity::{Identity, List};
use sfab_vouches::Vouches;

pub const BITS: u32 = 8;
/// 2026-11-05T12:00:00Z.
pub const NOW: i64 = 1_793_836_800 + 43_200;
pub const DAY: i64 = 86_400;

pub struct World {
    pub store: Arc<MemoryStore>,
    pub identity: Arc<Identity<MemoryStore>>,
    pub vouches: Vouches<MemoryStore>,
}

pub fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    let identity = Arc::new(Identity::new(store.clone(), BITS, genesis::genesis(&store)));
    World {
        vouches: Vouches::new(store.clone(), identity.clone(), BITS),
        identity,
        store,
    }
}

pub struct Sent {
    pub method: &'static str,
    pub path: String,
    pub authorization: Option<String>,
    pub proof: Option<String>,
    pub body: Vec<u8>,
}

impl Sent {
    pub fn call(&self) -> Call<'_> {
        Call {
            method: self.method,
            path: &self.path,
            authorization: self.authorization.as_deref(),
            proof: self.proof.as_deref(),
            body: &self.body,
            viewer: None,
        }
    }
}

/// A write with a correct proof, solved at `at`.
pub fn write(
    method: &'static str,
    path: String,
    auth: Option<String>,
    body: &[u8],
    at: i64,
) -> Sent {
    let caller = sfab_core::credentials::caller_of(auth.as_deref()).unwrap();
    let request = Request {
        method,
        path: &path,
        caller: &caller,
        body,
    };
    Sent {
        method,
        proof: Some(proof::solve(&request, at, BITS)),
        path,
        authorization: auth,
        body: body.to_vec(),
    }
}

#[derive(Clone)]
pub struct Agent {
    pub id: AgentId,
    pub claim: String,
}

impl Agent {
    pub fn auth(&self) -> Option<String> {
        Some(format!("Bearer {}.{}", self.id, self.claim))
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }
}

/// An agent born at `at`.
pub async fn born_at(w: &World, name: &str, at: i64) -> Agent {
    let code = genesis::seed_exempt_code(&*w.store).await;
    let body = json!({ "name": name, "genesis_code": code }).to_string();
    let sent = write(
        "POST",
        "/v1/identity/birth".into(),
        None,
        body.as_bytes(),
        at,
    );
    let reply = w.identity.birth(&sent.call(), at).await.unwrap();
    Agent {
        id: AgentId::parse(reply.body["id"].as_str().unwrap()).unwrap(),
        claim: reply.body["claim"].as_str().unwrap().to_owned(),
    }
}

pub async fn born(w: &World, name: &str) -> Agent {
    born_at(w, name, NOW).await
}

pub fn vouch_call(from: &Agent, to: &str, reason: &str, at: i64) -> Sent {
    let body = serde_json::to_vec(&json!({ "reason": reason })).unwrap();
    write(
        "PUT",
        format!("/v1/identity/self/vouches/{to}"),
        from.auth(),
        &body,
        at,
    )
}

/// `from` vouches for `to` at `at`: the status and body, or the problem's status and type.
pub async fn vouch_at(
    w: &World,
    from: &Agent,
    to: &str,
    reason: &str,
    at: i64,
) -> Result<(u16, Value), (u16, &'static str)> {
    let sent = vouch_call(from, to, reason, at);
    w.vouches
        .give(&sent.call(), to, at)
        .await
        .map(|r| (r.status, r.body))
        .map_err(|p| (p.status, p.kind))
}

pub async fn vouch(w: &World, from: &Agent, to: &Agent) {
    let (status, _) = vouch_at(w, from, to.id(), "trusted", NOW).await.unwrap();
    assert_eq!(status, 201);
}

/// `owner` blocks `other`.
pub async fn block(w: &World, owner: &Agent, other: &Agent) {
    let path = format!("/v1/identity/self/blocked/{}", other.id);
    let sent = write("PUT", path, owner.auth(), b"", NOW);
    let added = w
        .identity
        .add_contact(List::Blocked, &sent.call(), other.id(), NOW)
        .await;
    assert_eq!(added.unwrap().status, 201);
}

/// The IDs on a page of vouches.
pub fn ids(page: &Value) -> Vec<String> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_owned())
        .collect()
}
