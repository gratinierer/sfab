//! Messages and knocks, from `.stories/2-identity-api/spec.md` "Messages" and "Testing", against
//! the in-memory fake: issue 14's Done when.

use std::sync::Arc;

use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::{self, Request};
use sfab_core::storage::{Bucket, MemoryStore};
use sfab_identity::{Identity, List};
use sfab_messaging::Messaging;

/// Enough bits that a proof for another request almost never passes by chance.
const BITS: u32 = 12;
/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;
const DAY: i64 = 86_400;

struct World {
    store: Arc<MemoryStore>,
    identity: Arc<Identity<MemoryStore>>,
    messaging: Messaging<MemoryStore>,
}

fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    let identity = Arc::new(Identity::new(store.clone(), BITS));
    World {
        messaging: Messaging::new(store.clone(), identity.clone(), BITS),
        identity,
        store,
    }
}

struct Sent {
    method: &'static str,
    path: String,
    authorization: Option<String>,
    proof: Option<String>,
    body: Vec<u8>,
}

impl Sent {
    fn call(&self) -> Call<'_> {
        Call {
            method: self.method,
            path: &self.path,
            authorization: self.authorization.as_deref(),
            proof: self.proof.as_deref(),
            body: &self.body,
        }
    }
}

/// A write with a correct proof, solved at `at`.
fn write(method: &'static str, path: String, auth: Option<String>, body: &[u8], at: i64) -> Sent {
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

struct Agent {
    id: String,
    claim: String,
}

impl Agent {
    fn auth(&self) -> Option<String> {
        Some(format!("Bearer {}.{}", self.id, self.claim))
    }
}

async fn born(w: &World, name: &str) -> Agent {
    let body = format!(r#"{{"name":"{name}"}}"#);
    let sent = write(
        "POST",
        "/v1/identity/birth".into(),
        None,
        body.as_bytes(),
        NOW,
    );
    let reply = w.identity.birth(&sent.call(), NOW).await.unwrap();
    Agent {
        id: reply.body["id"].as_str().unwrap().to_owned(),
        claim: reply.body["claim"].as_str().unwrap().to_owned(),
    }
}

/// `owner` adds `other` to one of its lists.
async fn list(w: &World, owner: &Agent, list: List, other: &Agent) {
    let segment = if list == List::Friends {
        "friends"
    } else {
        "blocked"
    };
    let path = format!("/v1/identity/self/{segment}/{}", other.id);
    let sent = write("PUT", path, owner.auth(), b"", NOW);
    let added = w
        .identity
        .add_contact(list, &sent.call(), &other.id, NOW)
        .await;
    assert_eq!(added.unwrap().status, 201);
}

fn message_call(from: &Agent, to: &str, text: &str, at: i64) -> Sent {
    let body = serde_json::to_vec(&json!({ "text": text })).unwrap();
    write(
        "POST",
        format!("/v1/identity/{to}/messages"),
        from.auth(),
        &body,
        at,
    )
}

async fn deliver(w: &World, sent: &Sent, to: &str, now: i64) -> Result<Reply, Problem> {
    w.messaging.send(&sent.call(), to, now).await
}

async fn send(w: &World, from: &Agent, to: &Agent, text: &str, at: i64) -> Result<Reply, Problem> {
    deliver(w, &message_call(from, &to.id, text, at), &to.id, at).await
}

fn status(result: &Result<Reply, Problem>) -> u16 {
    match result {
        Ok(reply) => reply.status,
        Err(problem) => problem.status,
    }
}

/// A page of messages, or of knocks with `knock`, as the reader sees it.
async fn read(w: &World, a: &Agent, kind: Option<&str>, page: Option<&str>) -> Value {
    let reply = w.messaging.read(a.auth().as_deref(), kind, page, NOW).await;
    reply.unwrap().body
}

fn texts(page: &Value) -> Vec<String> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["text"].as_str().unwrap().to_owned())
        .collect()
}

fn inbox(w: &World) -> Vec<String> {
    w.store
        .keys(Bucket::Identities)
        .into_iter()
        .filter(|k| k.starts_with("messages/"))
        .collect()
}

#[tokio::test]
async fn a_friend_sends_a_message_and_the_recipient_reads_it() {
    let w = world();
    let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
    list(&w, &bob, List::Friends, &ann).await;

    let sent = send(&w, &ann, &bob, "hello, Bob", NOW).await.unwrap();
    assert_eq!(sent.status, 202);
    assert_eq!(sent.body["kind"], "message");

    let page = read(&w, &bob, None, None).await;
    let item = &page["items"][0];
    assert_eq!(item["sender"], ann.id);
    assert_eq!(item["text"], "hello, Bob");
    assert_eq!(item["sent_at"], "2026-11-05T12:00:00Z");
    assert!(item["message_id"].as_str().unwrap().starts_with("m-"));
    assert_eq!(page["next"], Value::Null);
    assert_eq!(
        read(&w, &bob, Some("knock"), None).await["items"],
        json!([])
    );
}

#[tokio::test]
async fn a_stranger_knocks_and_the_knock_is_read_with_kind_knock() {
    let w = world();
    let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);

    let sent = send(&w, &ann, &bob, "may I?", NOW).await.unwrap();
    assert_eq!((sent.status, &sent.body["kind"]), (202, &json!("knock")));

    let knocks = read(&w, &bob, Some("knock"), None).await;
    assert_eq!(knocks["items"][0]["message_id"], format!("k-{}", ann.id));
    assert_eq!(knocks["items"][0]["sender"], ann.id);
    assert_eq!(texts(&knocks), ["may I?"]);
    assert_eq!(read(&w, &bob, None, None).await["items"], json!([]));
}

#[tokio::test]
async fn a_blocked_friend_or_stranger_gets_403_with_a_proof_and_400_without_and_nothing_is_stored()
{
    for befriended in [true, false] {
        let w = world();
        let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
        if befriended {
            list(&w, &bob, List::Friends, &ann).await;
        }
        list(&w, &bob, List::Blocked, &ann).await;
        if befriended {
            // Blocking removed the friend, so befriend again: a block wins over a friendship.
            let path = format!("/v1/identity/self/friends/{}", ann.id);
            let sent = write("PUT", path, bob.auth(), b"", NOW);
            let call = sent.call();
            let again = w.identity.add_contact(List::Friends, &call, &ann.id, NOW);
            again.await.unwrap();
        }

        let refused = send(&w, &ann, &bob, "hi", NOW).await.unwrap_err();
        assert_eq!(
            (refused.status, refused.kind),
            (403, "blocked"),
            "{befriended}"
        );

        let mut unproved = message_call(&ann, &bob.id, "hi", NOW);
        unproved.proof = None;
        let refused = deliver(&w, &unproved, &bob.id, NOW).await.unwrap_err();
        assert_eq!(refused.status, 400);
        assert!(refused.extra.contains_key("required_bits"));
        assert_eq!(inbox(&w), Vec::<String>::new(), "{befriended}");
    }
}

#[tokio::test]
async fn a_friend_or_stranger_without_a_proof_gets_400() {
    for befriended in [true, false] {
        let w = world();
        let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
        if befriended {
            list(&w, &bob, List::Friends, &ann).await;
        }
        let mut unproved = message_call(&ann, &bob.id, "hi", NOW);
        unproved.proof = None;
        assert_eq!(status(&deliver(&w, &unproved, &bob.id, NOW).await), 400);
        assert_eq!(inbox(&w), Vec::<String>::new());
    }
}

#[tokio::test]
async fn a_retry_with_the_same_proof_returns_the_same_message_or_knock_with_200() {
    for befriended in [true, false] {
        let w = world();
        let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
        if befriended {
            list(&w, &bob, List::Friends, &ann).await;
        }
        let sent = message_call(&ann, &bob.id, "once", NOW);
        let first = deliver(&w, &sent, &bob.id, NOW).await.unwrap();
        let retry = deliver(&w, &sent, &bob.id, NOW + 30).await.unwrap();

        assert_eq!((first.status, retry.status), (202, 200), "{befriended}");
        assert_eq!(first.body, retry.body);
        assert_eq!(inbox(&w).len(), 1);
    }
}

#[tokio::test]
async fn a_new_message_with_a_new_proof_is_a_second_message_newest_first() {
    let w = world();
    let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
    list(&w, &bob, List::Friends, &ann).await;
    send(&w, &ann, &bob, "first", NOW - 60).await.unwrap();
    send(&w, &ann, &bob, "second", NOW).await.unwrap();

    assert_eq!(
        texts(&read(&w, &bob, None, None).await),
        ["second", "first"]
    );
}

#[tokio::test]
async fn a_second_knock_overwrites_the_first() {
    let w = world();
    let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
    send(&w, &ann, &bob, "first", NOW - 60).await.unwrap();

    let second = send(&w, &ann, &bob, "second", NOW).await.unwrap();

    assert_eq!(second.status, 202);
    let knocks = read(&w, &bob, Some("knock"), None).await;
    assert_eq!(texts(&knocks), ["second"]);
    assert_eq!(knocks["items"][0]["sent_at"], "2026-11-05T12:00:00Z");
}

#[tokio::test]
async fn a_proof_for_another_sender_or_a_wrong_proof_is_rejected() {
    let w = world();
    let (ann, bob, cat) = (
        born(&w, "Ann").await,
        born(&w, "Bob").await,
        born(&w, "Cat").await,
    );
    list(&w, &bob, List::Friends, &ann).await;

    // Cat's solution, sent with Ann's credentials.
    let mut borrowed = message_call(&cat, &bob.id, "hi", NOW);
    borrowed.authorization = ann.auth();
    // A solution for another recipient.
    let mut elsewhere = message_call(&ann, &cat.id, "hi", NOW);
    elsewhere.path = format!("/v1/identity/{}/messages", bob.id);
    // A solution for another text.
    let mut other_text = message_call(&ann, &bob.id, "hi", NOW);
    other_text.body = br#"{"text":"bye"}"#.to_vec();
    // A nonce nobody solved.
    let mut made_up = message_call(&ann, &bob.id, "hi", NOW);
    made_up.proof = Some(format!("{NOW}:nonce"));

    for (name, sent) in [
        ("another sender", borrowed),
        ("another recipient", elsewhere),
        ("another body", other_text),
        ("a made-up nonce", made_up),
    ] {
        let refused = deliver(&w, &sent, &bob.id, NOW).await.unwrap_err();
        assert_eq!(refused.status, 400, "{name}");
        assert!(refused.extra.contains_key("required_bits"), "{name}");
    }
    assert_eq!(inbox(&w), Vec::<String>::new());
}

#[tokio::test]
async fn a_message_to_an_unknown_agent_gets_404_and_a_malformed_id_400() {
    let w = world();
    let ann = born(&w, "Ann").await;

    let nobody = Agent {
        id: "aaaaaaaaaaaa".into(),
        claim: String::new(),
    };
    assert_eq!(status(&send(&w, &ann, &nobody, "anyone?", NOW).await), 404);
    let sent = message_call(&ann, "nope", "hi", NOW);
    assert_eq!(status(&deliver(&w, &sent, "nope", NOW).await), 400);
    assert_eq!(inbox(&w), Vec::<String>::new());
}

#[tokio::test]
async fn a_text_must_be_1_to_80_characters_of_a_body_with_only_text() {
    let w = world();
    let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
    let longest = "é".repeat(80);
    assert_eq!(status(&send(&w, &ann, &bob, &longest, NOW).await), 202);

    for body in [
        json!({ "text": "é".repeat(81) }),
        json!({ "text": "" }),
        json!({ "text": "hi", "extra": 1 }),
        json!({}),
    ] {
        let bytes = serde_json::to_vec(&body).unwrap();
        let path = format!("/v1/identity/{}/messages", bob.id);
        let sent = write("POST", path, ann.auth(), &bytes, NOW);
        assert_eq!(
            status(&deliver(&w, &sent, &bob.id, NOW).await),
            400,
            "{body}"
        );
    }
}

#[tokio::test]
async fn sending_and_reading_need_the_claim() {
    let w = world();
    let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
    let wrong = Agent {
        id: ann.id.clone(),
        claim: "A".repeat(43),
    };
    assert_eq!(status(&send(&w, &wrong, &bob, "hi", NOW).await), 401);
    let read = w
        .messaging
        .read(wrong.auth().as_deref(), None, None, NOW)
        .await;
    assert_eq!(status(&read), 401);
    let removed = w.messaging.remove(None, "k-aaaaaaaaaaaa", NOW).await;
    assert_eq!(status(&removed), 401);
}

#[tokio::test]
async fn messages_come_20_a_page_following_next() {
    let w = world();
    let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
    list(&w, &bob, List::Friends, &ann).await;
    for i in 0..25 {
        send(&w, &ann, &bob, &format!("m{i}"), NOW - 250 + 10 * i)
            .await
            .unwrap();
    }

    let first = read(&w, &bob, None, None).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    assert_eq!(texts(&first)[0], "m24");
    let next = first["next"].as_str().unwrap();
    let token = next
        .strip_prefix("/v1/identity/self/messages?page=")
        .unwrap();

    let second = read(&w, &bob, None, Some(token)).await;
    assert_eq!(texts(&second), ["m4", "m3", "m2", "m1", "m0"]);
    assert_eq!(second["next"], Value::Null);
}

#[tokio::test]
async fn knocks_come_20_a_page_with_kind_kept_in_next() {
    let w = world();
    let bob = born(&w, "Bob").await;
    for i in 0..21 {
        let stranger = born(&w, &format!("S{i}")).await;
        send(&w, &stranger, &bob, "knock", NOW).await.unwrap();
    }
    let first = read(&w, &bob, Some("knock"), None).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    let next = first["next"].as_str().unwrap();
    let token = next
        .strip_prefix("/v1/identity/self/messages?kind=knock&page=")
        .unwrap();
    let second = read(&w, &bob, Some("knock"), Some(token)).await;
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert_eq!(second["next"], Value::Null);
}

#[tokio::test]
async fn a_made_up_page_token_or_kind_gets_400() {
    let w = world();
    let bob = born(&w, "Bob").await;
    for (kind, page) in [
        (None, Some("x")),
        (None, Some("k-aaaaaaaaaaaa")),
        (Some("knock"), Some("m-8206163199-aaaaaaaaaaaa-aaaaaaaa")),
        (Some("knock"), Some("k-../../x")),
        (Some("all"), None),
    ] {
        let reply = w
            .messaging
            .read(bob.auth().as_deref(), kind, page, NOW)
            .await;
        assert_eq!(status(&reply), 400, "{kind:?} {page:?}");
    }
    let auth = bob.auth();
    let message = w
        .messaging
        .read(auth.as_deref(), Some("message"), None, NOW);
    assert_eq!(status(&message.await), 200);
}

#[tokio::test]
async fn messages_older_than_30_days_are_not_shown_before_s3_expires_them() {
    let w = world();
    let (ann, bob, cat) = (
        born(&w, "Ann").await,
        born(&w, "Bob").await,
        born(&w, "Cat").await,
    );
    list(&w, &bob, List::Friends, &ann).await;
    send(&w, &ann, &bob, "old", NOW).await.unwrap();
    send(&w, &cat, &bob, "old knock", NOW).await.unwrap();
    let later = NOW + 30 * DAY + 1;
    send(&w, &ann, &bob, "new", later).await.unwrap();

    let reply = w
        .messaging
        .read(bob.auth().as_deref(), None, None, later)
        .await;
    assert_eq!(texts(&reply.unwrap().body), ["new"]);
    let auth = bob.auth();
    let knocks = w
        .messaging
        .read(auth.as_deref(), Some("knock"), None, later);
    assert_eq!(knocks.await.unwrap().body["items"], json!([]));
}

#[tokio::test]
async fn delete_removes_a_message_or_knock_and_an_unknown_id_still_gets_200() {
    let w = world();
    let (ann, bob, cat) = (
        born(&w, "Ann").await,
        born(&w, "Bob").await,
        born(&w, "Cat").await,
    );
    list(&w, &bob, List::Friends, &ann).await;
    send(&w, &ann, &bob, "message", NOW).await.unwrap();
    send(&w, &cat, &bob, "knock", NOW).await.unwrap();
    let message_id = read(&w, &bob, None, None).await["items"][0]["message_id"]
        .as_str()
        .unwrap()
        .to_owned();

    for id in [message_id, format!("k-{}", cat.id)] {
        let id = id.as_str();
        let removed = w.messaging.remove(bob.auth().as_deref(), id, NOW).await;
        assert_eq!(removed.unwrap().body, json!({ "message_id": id }));
        // A retry is harmless.
        let again = w.messaging.remove(bob.auth().as_deref(), id, NOW).await;
        assert_eq!(status(&again), 200);
    }
    assert_eq!(inbox(&w), Vec::<String>::new());
}

#[tokio::test]
async fn a_crafted_message_id_is_refused_and_another_inbox_is_untouched() {
    let w = world();
    let (ann, bob) = (born(&w, "Ann").await, born(&w, "Bob").await);
    send(&w, &ann, &bob, "knock", NOW).await.unwrap();
    let stored = inbox(&w);

    let crafted = [
        String::new(),
        "k-".into(),
        format!("k-{}/../../{}/knock/{}", ann.id, bob.id, ann.id),
        format!("x-{}", ann.id),
        format!("k-{}.json", ann.id),
        "m-1-aaaaaaaaaaaa-aaaaaaaa".into(),
        "m-8206163199-aaaaaaaaaaaa-aaaaaaa1".into(),
        "m-8206163199-AAAAAAAAAAAA-aaaaaaaa".into(),
    ];
    for crafted in &crafted {
        let removed = w
            .messaging
            .remove(ann.auth().as_deref(), crafted, NOW)
            .await;
        assert_eq!(status(&removed), 400, "{crafted:?}");
    }
    // A well-formed ID only ever reaches the caller's own inbox.
    let (auth, own_id) = (ann.auth(), format!("k-{}", ann.id));
    let own = w.messaging.remove(auth.as_deref(), &own_id, NOW);
    assert_eq!(status(&own.await), 200);
    assert_eq!(inbox(&w), stored);
}
