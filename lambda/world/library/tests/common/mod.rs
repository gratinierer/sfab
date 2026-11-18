//! What the Library tests share: a world on the in-memory fake, agents born through `birth`,
//! standing given as founders of the vouches step, and the Library's routes called with correct
//! proofs.

#![allow(dead_code)]

use std::sync::Arc;

use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::id::AgentId;
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::{self, Request};
use sfab_core::storage::MemoryStore;
use sfab_genesis::testing as genesis;
use sfab_identity::Identity;
use sfab_library::Library;

pub const BITS: u32 = 8;
/// 2026-11-05T12:00:00Z.
pub const NOW: i64 = 1_793_836_800 + 43_200;
pub const DAY: i64 = 86_400;

pub struct World {
    pub store: Arc<MemoryStore>,
    pub identity: Arc<Identity<MemoryStore>>,
    pub library: Library<MemoryStore>,
}

pub fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    let identity = Arc::new(Identity::new(store.clone(), BITS, genesis::genesis(&store)));
    World {
        library: Library::new(store.clone(), identity.clone(), BITS),
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

/// `authors` have standing, as founders of the vouches step's run at `NOW`.
pub async fn give_standing(w: &World, authors: &[&Agent]) {
    let founders: Vec<AgentId> = authors.iter().map(|a| a.id.clone()).collect();
    let residents = sfab_identity::residents(&*w.store).await.unwrap();
    sfab_vouches::daily(
        &*w.store,
        &founders,
        &residents,
        "2099-12-31T00:00:00Z",
        NOW,
    )
    .await
    .unwrap();
}

/// An author born at [`NOW`] with standing; `others` keep theirs.
pub async fn author(w: &World, name: &str, others: &[&Agent]) -> Agent {
    let a = born(w, name).await;
    let mut all: Vec<&Agent> = others.to_vec();
    all.push(&a);
    give_standing(w, &all).await;
    a
}

pub type Answer = Result<(u16, Value), (u16, &'static str)>;

pub fn answer(r: Result<Reply, Problem>) -> Answer {
    r.map(|r| (r.status, r.body))
        .map_err(|p| (p.status, p.kind))
}

/// A document's ID, 12 base32 characters, from a short name.
pub fn doc(name: &str) -> String {
    format!("{name:a<12}")
}

pub fn document_body(shelf: &str, title: &str, citations: &[&str]) -> Value {
    json!({"shelf": shelf, "title": title, "content": format!("The words of {title}."), "citations": citations})
}

pub fn edition_body(title: &str, citations: &[&str]) -> Value {
    json!({"title": title, "content": format!("The words of {title}."), "citations": citations})
}

pub fn publish_call(agent: &Agent, document: &str, body: &Value, at: i64) -> Sent {
    let path = format!("/v1/library/documents/{document}");
    write(
        "PUT",
        &path,
        agent.auth(),
        &serde_json::to_vec(body).unwrap(),
        at,
    )
}

pub async fn send_publish(w: &World, sent: &Sent, document: &str, at: i64) -> Answer {
    answer(w.library.publish(&sent.call(), document, at).await)
}

pub async fn publish(w: &World, author: &Agent, document: &str, body: &Value) -> Answer {
    send_publish(w, &publish_call(author, document, body, NOW), document, NOW).await
}

/// `author` publishes `document` on the first shelf, citing `citations`.
pub async fn published(w: &World, author: &Agent, document: &str, citations: &[&str]) {
    let body = document_body("craft", document, citations);
    let (status, _) = publish(w, author, document, &body).await.unwrap();
    assert_eq!(status, 201);
}

pub fn edition_call(agent: &Agent, document: &str, n: &str, body: &Value, at: i64) -> Sent {
    let path = format!("/v1/library/documents/{document}/editions/{n}");
    write(
        "PUT",
        &path,
        agent.auth(),
        &serde_json::to_vec(body).unwrap(),
        at,
    )
}

pub async fn send_edition(w: &World, sent: &Sent, document: &str, n: &str, at: i64) -> Answer {
    answer(
        w.library
            .publish_edition(&sent.call(), document, n, at)
            .await,
    )
}

pub async fn edition(w: &World, author: &Agent, document: &str, n: &str, body: &Value) -> Answer {
    send_edition(
        w,
        &edition_call(author, document, n, body, NOW),
        document,
        n,
        NOW,
    )
    .await
}

pub async fn withdraw(w: &World, author: &Agent, document: &str, n: &str) -> Answer {
    let path = format!("/v1/library/documents/{document}/editions/{n}/withdrawal");
    let sent = write("PUT", &path, author.auth(), b"", NOW);
    answer(w.library.withdraw(&sent.call(), document, n, NOW).await)
}

pub async fn read_document(w: &World, document: &str) -> Answer {
    answer(w.library.document(document).await)
}

pub async fn read_edition(w: &World, document: &str, n: &str) -> Answer {
    answer(w.library.edition(document, n).await)
}

/// The Library's daily step with every resident.
pub async fn daily(w: &World, at: i64) -> sfab_library::DailyReport {
    let residents = sfab_identity::residents(&*w.store).await.unwrap();
    sfab_library::daily(&*w.store, &residents, at)
        .await
        .unwrap()
}
