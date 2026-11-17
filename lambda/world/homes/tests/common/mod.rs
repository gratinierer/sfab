//! What the homes tests share: a world on the in-memory fake, agents born through `birth`, and
//! the home routes called with correct proofs.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::id::AgentId;
use sfab_core::proof::{self, Request};
use sfab_core::storage::{MemoryStore, Store};
use sfab_genesis::testing as genesis;
use sfab_homes::{Homes, Quarter, parse_quarters, quarters};
use sfab_identity::Identity;

pub const BITS: u32 = 8;
/// 2026-11-05T12:00:00Z.
pub const NOW: i64 = 1_793_836_800 + 43_200;
pub const DAY: i64 = 86_400;

pub struct World<S = MemoryStore> {
    pub store: Arc<S>,
    pub identity: Arc<Identity<S>>,
    pub homes: Homes<S>,
}

/// The first free home every time.
pub fn first(_: usize) -> usize {
    0
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Each home in turn, so moves on the fallback never meet.
pub fn in_turn(n: usize) -> usize {
    NEXT.fetch_add(1, Ordering::SeqCst) % n
}

/// One small quarter: two lanes, 40 homes.
pub fn small_map() -> &'static [Quarter] {
    let text = "quarters:\n  - name: dock\n    lanes: [pier-lane, net-row]\n";
    Box::leak(parse_quarters(text).unwrap().into_boxed_slice())
}

/// On the compiled-in map, always picking the first free home.
pub fn world() -> World {
    world_on(Arc::new(MemoryStore::new()), quarters(), first)
}

pub fn world_on<S: Store>(
    store: Arc<S>,
    map: &'static [Quarter],
    pick: sfab_homes::Pick,
) -> World<S> {
    let identity = Arc::new(Identity::new(store.clone(), BITS, genesis::genesis(&store)));
    World {
        homes: Homes::new(store.clone(), identity.clone(), BITS).with(map, pick),
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
pub fn write(method: &'static str, path: &str, auth: Option<String>, body: &[u8], at: i64) -> Sent {
    let caller = sfab_core::credentials::caller_of(auth.as_deref()).unwrap();
    let request = Request {
        method,
        path,
        caller: &caller,
        body,
    };
    Sent {
        method,
        proof: Some(proof::solve(&request, at, BITS)),
        path: path.to_owned(),
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

/// An agent born at [`NOW`].
pub async fn born<S: Store>(w: &World<S>, name: &str) -> Agent {
    let code = genesis::seed_exempt_code(&*w.store).await;
    let body = json!({ "name": name, "genesis_code": code }).to_string();
    let sent = write("POST", "/v1/identity/birth", None, body.as_bytes(), NOW);
    let reply = w.identity.birth(&sent.call(), NOW).await.unwrap();
    Agent {
        id: AgentId::parse(reply.body["id"].as_str().unwrap()).unwrap(),
        claim: reply.body["claim"].as_str().unwrap().to_owned(),
    }
}

pub type Answer = Result<(u16, Value), (u16, &'static str)>;

pub fn move_call(agent: &Agent, quarter: &str, at: i64) -> Sent {
    let body = serde_json::to_vec(&json!({ "quarter": quarter })).unwrap();
    write("PUT", "/v1/identity/self/home", agent.auth(), &body, at)
}

pub async fn send_move<S: Store>(w: &World<S>, sent: &Sent, at: i64) -> Answer {
    w.homes
        .move_home(&sent.call(), at)
        .await
        .map(|r| (r.status, r.body))
        .map_err(|p| (p.status, p.kind))
}

/// `agent` moves into `quarter` at `at`.
pub async fn move_to<S: Store>(w: &World<S>, agent: &Agent, quarter: &str, at: i64) -> Answer {
    send_move(w, &move_call(agent, quarter, at), at).await
}

pub async fn move_out<S: Store>(w: &World<S>, agent: &Agent, at: i64) -> Answer {
    w.homes
        .move_out(agent.auth().as_deref(), at)
        .await
        .map(|r| (r.status, r.body))
        .map_err(|p| (p.status, p.kind))
}

pub async fn own_home<S: Store>(w: &World<S>, agent: &Agent) -> Value {
    w.homes
        .own_home(agent.auth().as_deref(), NOW)
        .await
        .unwrap()
        .body
}

/// `agent` publishes `fields`.
pub async fn publish<S: Store>(w: &World<S>, agent: &Agent, fields: Value) {
    let body = serde_json::to_vec(&fields).unwrap();
    let sent = write(
        "PUT",
        "/v1/identity/self/publication",
        agent.auth(),
        &body,
        NOW,
    );
    w.identity.publish(&sent.call(), NOW).await.unwrap();
}

/// `aboutme`'s body, or its status.
pub async fn aboutme<S: Store>(w: &World<S>, agent: &Agent) -> Result<Value, u16> {
    w.identity
        .aboutme(agent.id())
        .await
        .map(|r| r.body)
        .map_err(|p| p.status)
}

/// The home route for an address `quarter/lane/home`.
pub async fn home_at<S: Store>(w: &World<S>, address: &str) -> Result<Value, u16> {
    let parts: Vec<&str> = address.split('/').collect();
    w.homes
        .home(parts[0], parts[1], parts[2])
        .await
        .map(|r| r.body)
        .map_err(|p| p.status)
}
