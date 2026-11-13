//! `POST /federation/requests`, against the in-memory fake: the proof, invalid domains, the cap
//! and a repeat returning the same token (`.stories/2-identity-api/spec.md` "Federation" and
//! "Testing").

use std::sync::Arc;

use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::id::Caller;
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::{self, Request};
use sfab_core::storage::{Bucket, MemoryStore};
use sfab_federation::{Federation, Here};

const BITS: u32 = 12;
/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;
const PATH: &str = "/v1/federation/requests";

fn here() -> Here {
    Here {
        domain: "sfab.example".into(),
        contact: "abuse@sfab.example".into(),
    }
}

fn federation() -> (Arc<MemoryStore>, Federation<MemoryStore>) {
    let store = Arc::new(MemoryStore::new());
    (store.clone(), Federation::new(store, BITS, here()))
}

fn body(domain: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({ "domain": domain })).unwrap()
}

fn solved(body: &[u8], now: i64) -> String {
    let request = Request {
        method: "POST",
        path: PATH,
        caller: &Caller::Anonymous,
        body,
    };
    proof::solve(&request, now, BITS)
}

async fn send(
    f: &Federation<MemoryStore>,
    body: &[u8],
    proof: Option<&str>,
) -> Result<Reply, Problem> {
    let call = Call {
        method: "POST",
        path: PATH,
        authorization: None,
        proof,
        body,
        viewer: None,
    };
    f.request(&call, NOW).await
}

async fn ask(f: &Federation<MemoryStore>, domain: &str) -> Result<Reply, Problem> {
    let b = body(domain);
    let p = solved(&b, NOW);
    send(f, &b, Some(&p)).await
}

#[tokio::test]
async fn a_request_with_a_proof_is_stored_pending_and_returns_a_token_to_publish() {
    let (store, f) = federation();

    let reply = ask(&f, "other.example.org").await.unwrap();

    assert_eq!(reply.status, 202);
    let token = reply.body["token"].as_str().unwrap();
    assert_eq!(token.len(), 32, "128 bits in hex");
    assert!(
        token
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    assert_eq!(
        reply.body["url"],
        format!("https://other.example.org/.well-known/sfab-federation/{token}")
    );
    assert_eq!(reply.body["expires_at"], "2026-11-08T12:00:00Z");
    let instructions = reply.body["instructions"].as_str().unwrap();
    assert!(instructions.contains("website/.well-known/sfab-federation/"));
    assert!(
        reply.body["manual"]
            .as_str()
            .unwrap()
            .ends_with("docs/federation.md")
    );
    assert_eq!(
        store.keys(Bucket::Identities),
        ["federation/pending/other.example.org.json"]
    );
}

#[tokio::test]
async fn a_request_without_a_valid_proof_gets_400_and_stores_nothing() {
    let (store, f) = federation();
    let b = body("other.example.org");
    let for_another_body = solved(&body("else.example.org"), NOW);

    for proof in [None, Some(for_another_body.as_str())] {
        let err = send(&f, &b, proof).await.unwrap_err();
        assert_eq!(
            (err.status, &err.extra["required_bits"]),
            (400, &json!(BITS))
        );
    }
    assert!(store.keys(Bucket::Identities).is_empty());
}

#[tokio::test]
async fn an_invalid_domain_gets_400_and_stores_nothing() {
    let (store, f) = federation();
    let long_label = format!("{}.org", "a".repeat(64));
    let too_long = format!("{}org", "abcdefghi.".repeat(26));
    for domain in [
        "",
        "localhost",
        "org",
        "Other.Example.org",
        "other.example.org.",
        ".example.org",
        "other..example.org",
        "-other.example.org",
        "other-.example.org",
        "other_world.example.org",
        "192.168.0.1",
        "10.0.0.1",
        "[::1]",
        "other.example.org:8443",
        "other.example.org/path",
        "user@other.example.org",
        "other.localhost",
        "other.local",
        "other.internal",
        "other.invalid",
        "other.test",
        "1.in-addr.arpa",
        long_label.as_str(),
        too_long.as_str(),
    ] {
        let err = ask(&f, domain).await.unwrap_err();
        assert_eq!(err.status, 400, "{domain:?}");
        assert!(!err.extra.contains_key("required_bits"), "{domain:?}");
    }
    assert!(store.keys(Bucket::Identities).is_empty());
}

#[tokio::test]
async fn a_body_that_is_not_one_domain_gets_400() {
    let (_, f) = federation();
    for b in [
        b"".to_vec(),
        b"{".to_vec(),
        br#"{"domain":"other.example.org","extra":1}"#.to_vec(),
        br#"{"domain":7}"#.to_vec(),
    ] {
        let p = solved(&b, NOW);
        assert_eq!(send(&f, &b, Some(&p)).await.unwrap_err().status, 400);
    }
}

#[tokio::test]
async fn a_valid_hostname_is_accepted() {
    let (_, f) = federation();
    let longest_label = format!("{}.org", "a".repeat(63));
    for domain in [
        "a.co",
        "xn--bcher-kva.de",
        "w0rld-2.sub.example.org",
        longest_label.as_str(),
    ] {
        assert_eq!(ask(&f, domain).await.unwrap().status, 202, "{domain}");
    }
}

#[tokio::test]
async fn a_repeat_returns_the_same_token_with_200() {
    let (store, f) = federation();
    let first = ask(&f, "other.example.org").await.unwrap();

    // The same proof again, and a new solution for the same domain.
    let b = body("other.example.org");
    let again = send(&f, &b, Some(&solved(&b, NOW))).await.unwrap();
    let later = {
        let p = solved(&b, NOW + 30);
        send(&f, &b, Some(&p)).await.unwrap()
    };

    for repeat in [again, later] {
        assert_eq!(repeat.status, 200);
        assert_eq!(repeat.body, first.body);
    }
    assert_eq!(store.keys(Bucket::Identities).len(), 1);
}

#[tokio::test]
async fn at_100_pending_a_new_domain_gets_503_until_the_next_daily_run() {
    let (store, f) = federation();
    for i in 0..100 {
        assert_eq!(
            ask(&f, &format!("w{i}.example.org")).await.unwrap().status,
            202
        );
    }

    let err = ask(&f, "late.example.org").await.unwrap_err();

    assert_eq!(err.status, 503);
    // The daily world job runs at 00:05 UTC.
    assert_eq!(err.extra["retry_after"], json!("2026-11-06T00:05:00Z"));
    assert_eq!(store.keys(Bucket::Identities).len(), 100);
    // A repeat for a waiting domain still gets its token: the retry check comes before the cap.
    let repeat = ask(&f, "w7.example.org").await.unwrap();
    assert_eq!(repeat.status, 200);
    assert!(repeat.body["token"].is_string());
}

#[tokio::test]
async fn the_stored_request_holds_the_token_and_when_it_was_made() {
    let (store, f) = federation();
    let reply = ask(&f, "other.example.org").await.unwrap();

    let object = store
        .object(
            Bucket::Identities,
            "federation/pending/other.example.org.json",
        )
        .unwrap();
    let stored: Value = serde_json::from_slice(&object.body).unwrap();

    assert_eq!(stored["domain"], "other.example.org");
    assert_eq!(stored["token"], reply.body["token"]);
    assert_eq!(stored["requested_at"], "2026-11-05T12:00:00Z");
}

#[tokio::test]
async fn a_request_past_its_expiry_that_s3_hasnt_removed_yet_is_made_anew() {
    // S3 removes expired objects some time after their expiry (docs/aws-facts.md).
    let (_, f) = federation();
    let first = ask(&f, "other.example.org").await.unwrap();
    let later = NOW + 3 * 86_400;
    let b = body("other.example.org");
    let proof = solved(&b, later);
    let call = Call {
        method: "POST",
        path: PATH,
        authorization: None,
        proof: Some(&proof),
        body: &b,
        viewer: None,
    };

    let again = f.request(&call, later).await.unwrap();

    assert_eq!(again.status, 202);
    assert_ne!(again.body["token"], first.body["token"]);
    assert_eq!(again.body["expires_at"], "2026-11-11T12:00:00Z");
}
