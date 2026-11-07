//! The daily world job's federation step, with a fake fetch and a fake email: a published token
//! moves its request to verified, and verified requests are emailed once a day.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::id::Caller;
use sfab_core::proof::{self, Request};
use sfab_core::storage::{Bucket, MemoryStore};
use sfab_federation::{Federation, Fetch, Here, Notify, daily, matches};

const BITS: u32 = 4;
/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;
/// The next run: 2026-11-06T00:05:00Z.
const RUN: i64 = 1_793_836_800 + 86_400 + 300;
const DAY: i64 = 86_400;

/// Serves a body per URL, and records every URL asked for.
#[derive(Default)]
struct FakeWeb {
    pages: HashMap<String, Vec<u8>>,
    asked: Mutex<Vec<String>>,
}

impl Fetch for FakeWeb {
    async fn fetch(&self, domain: &str, path: &str) -> Option<Vec<u8>> {
        let url = format!("https://{domain}{path}");
        self.asked.lock().unwrap().push(url.clone());
        self.pages.get(&url).cloned()
    }
}

#[derive(Default)]
struct FakeEmail {
    sent: Mutex<Vec<(String, String)>>,
}

impl Notify for FakeEmail {
    async fn notify(&self, subject: &str, message: &str) -> Result<(), String> {
        self.sent
            .lock()
            .unwrap()
            .push((subject.to_owned(), message.to_owned()));
        Ok(())
    }
}

async fn ask(store: &Arc<MemoryStore>, domain: &str) -> Value {
    let here = Here {
        domain: "sfab.example.org".into(),
        contact: "abuse@sfab.example.org".into(),
    };
    let f = Federation::new(store.clone(), BITS, here);
    let body = serde_json::to_vec(&json!({ "domain": domain })).unwrap();
    let path = "/v1/federation/requests";
    let request = Request {
        method: "POST",
        path,
        caller: &Caller::Anonymous,
        body: &body,
    };
    let proof = proof::solve(&request, NOW, BITS);
    let call = Call {
        method: "POST",
        path,
        authorization: None,
        proof: Some(&proof),
        body: &body,
    };
    f.request(&call, NOW).await.unwrap().body
}

fn publish(web: &mut FakeWeb, reply: &Value, body: &[u8]) {
    web.pages
        .insert(reply["url"].as_str().unwrap().to_owned(), body.to_vec());
}

#[tokio::test]
async fn a_published_token_moves_its_request_to_verified() {
    let store = Arc::new(MemoryStore::new());
    let mut web = FakeWeb::default();
    let shown = ask(&store, "shown.example.org").await;
    ask(&store, "hidden.example.org").await;
    publish(&mut web, &shown, shown["token"].as_str().unwrap().as_bytes());

    let report = daily(&*store, &web, &FakeEmail::default(), RUN).await.unwrap();

    assert_eq!((report.checked, report.verified), (2, 1));
    assert_eq!(
        store.keys(Bucket::Identities),
        [
            "federation/emailed.json",
            "federation/pending/hidden.example.org.json",
            "federation/verified/shown.example.org.json",
        ]
    );
    let verified = store
        .object(Bucket::Identities, "federation/verified/shown.example.org.json")
        .unwrap();
    let verified: Value = serde_json::from_slice(&verified.body).unwrap();
    assert_eq!(verified["token"], shown["token"]);
    assert_eq!(verified["verified_at"], "2026-11-06T00:05:00Z");
    // Each pending request's one fixed URL, and nothing else.
    let mut asked = web.asked.lock().unwrap().clone();
    asked.sort();
    assert_eq!(asked.len(), 2);
    assert!(asked[0].starts_with("https://hidden.example.org/.well-known/sfab-federation/"));
    assert_eq!(asked[1], shown["url"]);
}

#[tokio::test]
async fn verified_requests_are_emailed_once_a_day_listing_them() {
    let store = Arc::new(MemoryStore::new());
    let mut web = FakeWeb::default();
    for domain in ["a.example.org", "b.example.org"] {
        let reply = ask(&store, domain).await;
        publish(&mut web, &reply, reply["token"].as_str().unwrap().as_bytes());
    }
    let email = FakeEmail::default();

    daily(&*store, &web, &email, RUN).await.unwrap();
    let again = daily(&*store, &web, &email, RUN + 600).await.unwrap();

    assert!(!again.emailed);
    let sent = email.sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 1, "one email a day, however often the job runs");
    let (subject, message) = &sent[0];
    assert!(subject.contains('2'), "{subject}");
    assert!(message.contains("a.example.org") && message.contains("b.example.org"));
    assert!(message.contains("docs/federation.md"));

    // The next day, while the owner hasn't decided, they're listed again.
    let next_day = daily(&*store, &web, &email, RUN + DAY).await.unwrap();
    assert!(next_day.emailed);
    assert_eq!(email.sent.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn nothing_verified_sends_no_email() {
    let store = Arc::new(MemoryStore::new());
    ask(&store, "hidden.example.org").await;
    let email = FakeEmail::default();

    let report = daily(&*store, &FakeWeb::default(), &email, RUN).await.unwrap();

    assert!(!report.emailed);
    assert!(email.sent.lock().unwrap().is_empty());
}

#[tokio::test]
async fn running_twice_changes_nothing_more() {
    let store = Arc::new(MemoryStore::new());
    let mut web = FakeWeb::default();
    let reply = ask(&store, "shown.example.org").await;
    publish(&mut web, &reply, reply["token"].as_str().unwrap().as_bytes());
    let email = FakeEmail::default();
    daily(&*store, &web, &email, RUN).await.unwrap();
    let before: Vec<_> = store
        .keys(Bucket::Identities)
        .iter()
        .map(|k| store.object(Bucket::Identities, k).unwrap().body)
        .collect();

    let report = daily(&*store, &web, &email, RUN + 60).await.unwrap();

    assert_eq!((report.checked, report.verified), (0, 0));
    let after: Vec<_> = store
        .keys(Bucket::Identities)
        .iter()
        .map(|k| store.object(Bucket::Identities, k).unwrap().body)
        .collect();
    assert_eq!(before, after);
}

#[test]
fn a_body_matches_only_as_the_token_with_at_most_one_trailing_newline() {
    let token = "0123456789abcdef0123456789abcdef";
    assert!(matches(token.as_bytes(), token));
    assert!(matches(format!("{token}\n").as_bytes(), token));
    for body in [
        format!("{token}\n\n"),
        format!("{token}\r\n"),
        format!("{token} "),
        format!(" {token}"),
        format!("{token}x"),
        format!("<html>{token}</html>"),
        token[..31].to_owned(),
        "fedcba9876543210fedcba9876543210".to_owned(),
        String::new(),
    ] {
        assert!(!matches(body.as_bytes(), token), "{body:?}");
    }
}
