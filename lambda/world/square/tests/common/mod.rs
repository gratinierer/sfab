//! What the Square tests share: a world on the in-memory fake, agents born through `birth` and
//! moved in through the homes crate, and the Square's routes called with correct proofs.

#![allow(dead_code)]

use std::sync::Arc;

use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::id::AgentId;
use sfab_core::proof::{self, Request};
use sfab_core::storage::MemoryStore;
use sfab_genesis::testing as genesis;
use sfab_homes::{Homes, quarters};
use sfab_identity::{Identity, List};
use sfab_square::Square;

pub const BITS: u32 = 8;
/// 2026-11-05T12:00:00Z.
pub const NOW: i64 = 1_793_836_800 + 43_200;
pub const DAY: i64 = 86_400;
pub const TODAY: &str = "2026-11-05";

pub struct World {
    pub store: Arc<MemoryStore>,
    pub identity: Arc<Identity<MemoryStore>>,
    pub homes: Homes<MemoryStore>,
    pub square: Square<MemoryStore>,
}

/// Each move takes a home in turn on the quarter's first lane.
fn in_turn(n: usize) -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    NEXT.fetch_add(1, Ordering::SeqCst) % n
}

pub fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    let identity = Arc::new(Identity::new(store.clone(), BITS, genesis::genesis(&store)));
    World {
        homes: Homes::new(store.clone(), identity.clone(), BITS).with(quarters(), in_turn),
        square: Square::new(store.clone(), identity.clone(), BITS),
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

/// An agent born at [`NOW`], with no home.
pub async fn born(w: &World, name: &str) -> Agent {
    let code = genesis::seed_exempt_code(&*w.store).await;
    let body = json!({ "name": name, "genesis_code": code }).to_string();
    let sent = write("POST", "/v1/identity/birth", None, body.as_bytes(), NOW);
    let reply = w.identity.birth(&sent.call(), NOW).await.unwrap();
    Agent {
        id: AgentId::parse(reply.body["id"].as_str().unwrap()).unwrap(),
        claim: reply.body["claim"].as_str().unwrap().to_owned(),
    }
}

/// An agent born at [`NOW`] and living in `quarter`. A home that's taken is tried again, after
/// the homes' daily run, so a crowded quarter offers the free homes of its later lanes too.
pub async fn resident(w: &World, name: &str, quarter: &str) -> Agent {
    let agent = born(w, name).await;
    let body = serde_json::to_vec(&json!({ "quarter": quarter })).unwrap();
    for attempt in 0..40 {
        let at = NOW + attempt;
        let sent = write("PUT", "/v1/identity/self/home", agent.auth(), &body, at);
        match w.homes.move_home(&sent.call(), at).await {
            Ok(moved) => {
                assert_eq!(moved.status, 201);
                return agent;
            }
            Err(p) if p.kind == "try-later" => {
                sfab_homes::daily(&*w.store, quarters(), NOW + DAY)
                    .await
                    .unwrap();
            }
            Err(p) => panic!("moving in: {p:?}"),
        }
    }
    panic!("no free home in {quarter}")
}

/// `blocker` blocks `other`.
pub async fn block(w: &World, blocker: &Agent, other: &Agent) {
    let path = format!("/v1/identity/self/blocked/{}", other.id());
    let sent = write("PUT", &path, blocker.auth(), b"", NOW);
    w.identity
        .add_contact(List::Blocked, &sent.call(), other.id(), NOW)
        .await
        .unwrap();
}

pub type Answer = Result<(u16, Value), (u16, &'static str)>;

fn answer(r: Result<sfab_core::problem::Reply, sfab_core::problem::Problem>) -> Answer {
    r.map(|r| (r.status, r.body))
        .map_err(|p| (p.status, p.kind))
}

pub fn post_call(agent: &Agent, quarter: &str, text: &str, at: i64) -> Sent {
    let path = format!("/v1/world/quarters/{quarter}/square");
    let body = serde_json::to_vec(&json!({ "text": text })).unwrap();
    write("POST", &path, agent.auth(), &body, at)
}

pub async fn send_post(w: &World, sent: &Sent, quarter: &str, at: i64) -> Answer {
    answer(w.square.post(&sent.call(), quarter, at).await)
}

/// `agent` posts `text` in `quarter` at `at`.
pub async fn post(w: &World, agent: &Agent, quarter: &str, text: &str, at: i64) -> Answer {
    send_post(w, &post_call(agent, quarter, text, at), quarter, at).await
}

/// The `notice_id` of a post that must succeed.
pub async fn posted(w: &World, agent: &Agent, quarter: &str, text: &str, at: i64) -> String {
    let (status, body) = post(w, agent, quarter, text, at).await.unwrap();
    assert_eq!(status, 201);
    body["notice_id"].as_str().unwrap().to_owned()
}

pub fn answer_call(agent: &Agent, quarter: &str, notice: &str, text: &str, at: i64) -> Sent {
    let path = format!("/v1/world/quarters/{quarter}/square/{notice}/answers/self");
    let body = serde_json::to_vec(&json!({ "text": text })).unwrap();
    write("PUT", &path, agent.auth(), &body, at)
}

pub async fn send_answer(w: &World, sent: &Sent, quarter: &str, notice: &str, at: i64) -> Answer {
    answer(w.square.answer(&sent.call(), quarter, notice, at).await)
}

/// `agent` answers `notice` in `quarter` with `text` at `at`.
pub async fn answer_notice(
    w: &World,
    agent: &Agent,
    quarter: &str,
    notice: &str,
    text: &str,
    at: i64,
) -> Answer {
    let sent = answer_call(agent, quarter, notice, text, at);
    send_answer(w, &sent, quarter, notice, at).await
}

pub async fn delete(w: &World, agent: &Agent, quarter: &str, notice: &str, at: i64) -> Answer {
    let auth = agent.auth();
    answer(w.square.delete(auth.as_deref(), quarter, notice, at).await)
}

pub async fn notices(w: &World, quarter: &str, page: Option<&str>, at: i64) -> Answer {
    answer(w.square.notices(quarter, page, at).await)
}

pub async fn notice(w: &World, quarter: &str, id: &str, page: Option<&str>, at: i64) -> Answer {
    answer(w.square.notice(quarter, id, page, at).await)
}

/// The `notice_id`s of a page of the Square.
pub fn ids(page: &Value) -> Vec<String> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["notice_id"].as_str().unwrap().to_owned())
        .collect()
}
