//! What the Academy tests share: a world on the in-memory fake, agents born through `birth`, and
//! the Academy's routes called with correct proofs.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::{Value, json};
use sfab_academy::Academy;
use sfab_core::gate::Call;
use sfab_core::id::AgentId;
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::{self, Request};
use sfab_core::storage::MemoryStore;
use sfab_genesis::testing as genesis;
use sfab_identity::Identity;

pub const BITS: u32 = 8;
/// 2026-11-05T12:00:00Z.
pub const NOW: i64 = 1_793_836_800 + 43_200;
pub const DAY: i64 = 86_400;
pub const TODAY: &str = "2026-11-05";

pub struct World {
    pub store: Arc<MemoryStore>,
    pub identity: Arc<Identity<MemoryStore>>,
    pub academy: Academy<MemoryStore>,
}

pub fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    let identity = Arc::new(Identity::new(store.clone(), BITS, genesis::genesis(&store)));
    World {
        academy: Academy::new(store.clone(), identity.clone(), BITS),
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

pub type Answer = Result<(u16, Value), (u16, &'static str)>;

pub fn answer(r: Result<Reply, Problem>) -> Answer {
    r.map(|r| (r.status, r.body))
        .map_err(|p| (p.status, p.kind))
}

/// A course of `checks` answer checks, `Task 1` answered `a1` and so on, then `works` handed-in
/// tasks, passing at `pass` percent.
pub fn course_body(title: &str, checks: usize, works: usize, pass: u8) -> Value {
    let mut tasks: Vec<Value> = (1..=checks)
        .map(|i| json!({"kind": "check", "text": format!("Task {i}"), "answer": format!("a{i}")}))
        .collect();
    tasks.extend((1..=works).map(|i| json!({"kind": "work", "text": format!("Work {i}")})));
    json!({"title": title, "preparation": "Read this first.", "pass": pass, "tasks": tasks})
}

pub fn publish_call(agent: &Agent, body: &Value, at: i64) -> Sent {
    let body = serde_json::to_vec(body).unwrap();
    write("POST", "/v1/academy/courses", agent.auth(), &body, at)
}

pub async fn send_publish(w: &World, sent: &Sent, at: i64) -> Answer {
    answer(w.academy.publish(&sent.call(), at).await)
}

/// `teacher` publishes `body` at `at`.
pub async fn publish(w: &World, teacher: &Agent, body: &Value, at: i64) -> Answer {
    send_publish(w, &publish_call(teacher, body, at), at).await
}

/// The ID of a course that must be published.
pub async fn published(w: &World, teacher: &Agent, body: &Value) -> String {
    let (status, course) = publish(w, teacher, body, NOW).await.unwrap();
    assert_eq!(status, 201);
    course["course"].as_str().unwrap().to_owned()
}

pub async fn edit(w: &World, teacher: &Agent, course: &str, body: &Value, at: i64) -> Answer {
    let path = format!("/v1/academy/courses/{course}");
    let sent = write(
        "PUT",
        &path,
        teacher.auth(),
        &serde_json::to_vec(body).unwrap(),
        at,
    );
    answer(w.academy.edit(&sent.call(), course, at).await)
}

pub async fn retire(w: &World, teacher: &Agent, course: &str, at: i64) -> Answer {
    let path = format!("/v1/academy/courses/{course}/retirement");
    let sent = write("PUT", &path, teacher.auth(), b"", at);
    answer(w.academy.retire(&sent.call(), course, at).await)
}

pub async fn enrol(w: &World, student: &Agent, course: &str, at: i64) -> Answer {
    let path = format!("/v1/academy/courses/{course}/enrolments/self");
    let sent = write("PUT", &path, student.auth(), b"", at);
    answer(w.academy.enrol(&sent.call(), course, at).await)
}

pub async fn leave(w: &World, student: &Agent, course: &str, at: i64) -> Answer {
    let auth = student.auth();
    answer(w.academy.leave(auth.as_deref(), course, at).await)
}

pub fn start_call(student: &Agent, course: &str, at: i64) -> Sent {
    let path = format!("/v1/academy/courses/{course}/attempts");
    write("POST", &path, student.auth(), b"", at)
}

pub async fn start(w: &World, student: &Agent, course: &str, at: i64) -> Answer {
    let sent = start_call(student, course, at);
    answer(w.academy.start(&sent.call(), course, at).await)
}

/// The ID of an attempt that must start.
pub async fn started(w: &World, student: &Agent, course: &str, at: i64) -> String {
    let (status, attempt) = start(w, student, course, at).await.unwrap();
    assert_eq!(status, 201);
    attempt["attempt"].as_str().unwrap().to_owned()
}

pub async fn read_attempt(w: &World, who: &Agent, course: &str, attempt: &str) -> Answer {
    let auth = who.auth();
    answer(
        w.academy
            .attempt(auth.as_deref(), course, attempt, NOW)
            .await,
    )
}

pub fn answer_call(
    student: &Agent,
    course: &str,
    attempt: &str,
    n: &str,
    text: &str,
    at: i64,
) -> Sent {
    let path = format!("/v1/academy/courses/{course}/attempts/{attempt}/answers/{n}");
    let body = serde_json::to_vec(&json!({ "answer": text })).unwrap();
    write("PUT", &path, student.auth(), &body, at)
}

pub async fn send_answer(
    w: &World,
    sent: &Sent,
    course: &str,
    attempt: &str,
    n: &str,
    at: i64,
) -> Answer {
    answer(w.academy.answer(&sent.call(), course, attempt, n, at).await)
}

pub async fn hand_in(
    w: &World,
    student: &Agent,
    course: &str,
    attempt: &str,
    n: &str,
    text: &str,
) -> Answer {
    let sent = answer_call(student, course, attempt, n, text, NOW);
    send_answer(w, &sent, course, attempt, n, NOW).await
}

pub async fn submit(w: &World, student: &Agent, course: &str, attempt: &str, at: i64) -> Answer {
    let path = format!("/v1/academy/courses/{course}/attempts/{attempt}/submission");
    let sent = write("PUT", &path, student.auth(), b"", at);
    answer(w.academy.submit(&sent.call(), course, attempt, at).await)
}

pub async fn queue(w: &World, who: &Agent, course: &str, page: Option<&str>) -> Answer {
    let auth = who.auth();
    answer(w.academy.queue(auth.as_deref(), course, page, NOW).await)
}

pub async fn marking(w: &World, who: &Agent, course: &str, attempt: &str) -> Answer {
    let auth = who.auth();
    answer(
        w.academy
            .marking(auth.as_deref(), course, attempt, NOW)
            .await,
    )
}

pub async fn mark(w: &World, who: &Agent, course: &str, attempt: &str, body: &Value) -> Answer {
    let path = format!("/v1/academy/courses/{course}/marking/{attempt}");
    let sent = write(
        "PUT",
        &path,
        who.auth(),
        &serde_json::to_vec(body).unwrap(),
        NOW,
    );
    answer(w.academy.mark(&sent.call(), course, attempt, NOW).await)
}

pub async fn certificate(w: &World, id: &str) -> Answer {
    answer(w.academy.certificate(id).await)
}

pub async fn revoke(w: &World, who: &Agent, id: &str) -> Answer {
    let path = format!("/v1/academy/certificates/{id}/revocation");
    let sent = write("PUT", &path, who.auth(), b"", NOW);
    answer(w.academy.revoke(&sent.call(), id, NOW).await)
}

/// Every answer check of `attempt` answered right, so a check-only course passes.
pub async fn answer_all(w: &World, student: &Agent, course: &str, attempt: &str) {
    let (_, read) = read_attempt(w, student, course, attempt).await.unwrap();
    for task in read["tasks"].as_array().unwrap() {
        let n = task["n"].to_string();
        let text = task["text"].as_str().unwrap();
        let reply = match text.strip_prefix("Task ") {
            Some(i) => format!("a{i}"),
            None => "My work.".to_owned(),
        };
        let (status, _) = hand_in(w, student, course, attempt, &n, &reply)
            .await
            .unwrap();
        assert_eq!(status, 200);
    }
}

/// `student` enrols, sits a check-only `course` and passes, so it holds
/// `exam-<course>-<student>`.
pub async fn graduate(w: &World, student: &Agent, course: &str) -> String {
    enrol(w, student, course, NOW).await.unwrap();
    let attempt = started(w, student, course, NOW).await;
    answer_all(w, student, course, &attempt).await;
    let (_, done) = submit(w, student, course, &attempt, NOW).await.unwrap();
    assert_eq!(done["status"], "passed", "{done}");
    format!("exam-{course}-{}", student.id())
}

/// The daily step with every resident, and `standing` the agents with standing.
pub async fn daily(w: &World, standing: &[&Agent], at: i64) -> sfab_academy::DailyReport {
    let residents = sfab_identity::residents(&*w.store).await.unwrap();
    let standing: BTreeSet<AgentId> = standing.iter().map(|a| a.id.clone()).collect();
    sfab_academy::daily(&*w.store, &residents, &standing, at)
        .await
        .unwrap()
}
