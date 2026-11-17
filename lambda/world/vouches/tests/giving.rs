//! Giving, replacing and withdrawing vouches: `.stories/4-life-in-the-world/spec.md` "Vouches:
//! trust" ("A vouch" and the route table), against the in-memory fake.

mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn a_vouch_is_given_listed_and_a_retry_with_the_same_proof_returns_it() {
    let w = world();
    let (a, b) = (born(&w, "Wren").await, born(&w, "Moss").await);

    let sent = vouch_call(&a, b.id(), "kind and careful", NOW);
    let first = w.vouches.give(&sent.call(), b.id(), NOW).await.unwrap();
    assert_eq!(first.status, 201);
    assert_eq!(
        first.body,
        json!({"id": b.id(), "reason": "kind and careful", "given_at": "2026-11-05T12:00:00Z"})
    );
    let again = w.vouches.give(&sent.call(), b.id(), NOW + 5).await.unwrap();
    assert_eq!((again.status, &again.body), (200, &first.body));

    let page = w
        .vouches
        .given(a.auth().as_deref(), None, NOW)
        .await
        .unwrap();
    assert_eq!(page.body, json!({"items": [first.body], "next": null}));
}

#[tokio::test]
async fn a_new_reason_replaces_the_vouch_and_keeps_one_per_pair() {
    let w = world();
    let (a, b) = (born(&w, "Wren").await, born(&w, "Moss").await);
    vouch(&w, &a, &b).await;

    let (status, body) = vouch_at(&w, &a, b.id(), "even better", NOW + 60)
        .await
        .unwrap();
    assert_eq!((status, &body["reason"]), (200, &json!("even better")));
    let page = w
        .vouches
        .given(a.auth().as_deref(), None, NOW)
        .await
        .unwrap();
    assert_eq!(ids(&page.body), vec![b.id()]);
    assert_eq!(page.body["items"][0]["reason"], "even better");
}

#[tokio::test]
async fn an_eleventh_vouch_gets_409_list_full_and_withdrawing_frees_a_place() {
    let w = world();
    let a = born(&w, "Wren").await;
    let others: Vec<String> = (b'a'..=b'k')
        .map(|c| format!("othervouchx{}", c as char))
        .collect();
    for other in &others[..10] {
        let (status, _) = vouch_at(&w, &a, other, "x", NOW).await.unwrap();
        assert_eq!(status, 201, "{other}");
    }
    assert_eq!(
        vouch_at(&w, &a, &others[10], "x", NOW).await,
        Err((409, "list-full"))
    );
    // Replacing one already given isn't a new place.
    assert_eq!(vouch_at(&w, &a, &others[0], "y", NOW).await.unwrap().0, 200);

    let withdrawn = w
        .vouches
        .withdraw(a.auth().as_deref(), &others[0], NOW)
        .await
        .unwrap();
    assert_eq!(
        (withdrawn.status, withdrawn.body),
        (200, json!({"id": others[0]}))
    );
    assert_eq!(
        vouch_at(&w, &a, &others[10], "x", NOW).await.unwrap().0,
        201
    );
}

#[tokio::test]
async fn withdrawing_a_vouch_never_given_is_200_so_a_retry_is_safe() {
    let w = world();
    let (a, b) = (born(&w, "Wren").await, born(&w, "Moss").await);
    vouch(&w, &a, &b).await;
    for _ in 0..2 {
        let reply = w.vouches.withdraw(a.auth().as_deref(), b.id(), NOW).await;
        assert_eq!(reply.unwrap().status, 200);
    }
    let page = w
        .vouches
        .given(a.auth().as_deref(), None, NOW)
        .await
        .unwrap();
    assert_eq!(page.body, json!({"items": [], "next": null}));
}

#[tokio::test]
async fn a_vouch_for_an_agent_that_blocked_the_giver_gets_403_blocked() {
    let w = world();
    let (a, b) = (born(&w, "Wren").await, born(&w, "Moss").await);
    block(&w, &b, &a).await;
    assert_eq!(
        vouch_at(&w, &a, b.id(), "x", NOW).await,
        Err((403, "blocked"))
    );
    let page = w
        .vouches
        .given(a.auth().as_deref(), None, NOW)
        .await
        .unwrap();
    assert_eq!(page.body["items"], json!([]));
}

#[tokio::test]
async fn a_vouch_for_an_unknown_id_is_stored_so_ids_cant_be_probed() {
    let w = world();
    let a = born(&w, "Wren").await;
    assert_eq!(
        vouch_at(&w, &a, "unknownagent", "x", NOW).await.unwrap().0,
        201
    );
}

#[tokio::test]
async fn bad_vouches_get_400_and_bad_credentials_401() {
    let w = world();
    let (a, b) = (born(&w, "Wren").await, born(&w, "Moss").await);
    // Itself, a malformed ID, an empty or over-long reason, a body that isn't the schema.
    assert_eq!(vouch_at(&w, &a, a.id(), "x", NOW).await.unwrap_err().0, 400);
    assert_eq!(
        vouch_at(&w, &a, "Not-An-Id", "x", NOW).await.unwrap_err().0,
        400
    );
    assert_eq!(vouch_at(&w, &a, b.id(), "", NOW).await.unwrap_err().0, 400);
    let long = "é".repeat(201);
    assert_eq!(
        vouch_at(&w, &a, b.id(), &long, NOW).await.unwrap_err().0,
        400
    );
    assert_eq!(
        vouch_at(&w, &a, b.id(), &"é".repeat(200), NOW)
            .await
            .unwrap()
            .0,
        201
    );
    let path = format!("/v1/identity/self/vouches/{}", b.id());
    let sent = write("PUT", path, a.auth(), br#"{"reason":"x","more":1}"#, NOW);
    let reply = w.vouches.give(&sent.call(), b.id(), NOW).await;
    assert_eq!(reply.unwrap_err().status, 400);

    // No proof is the proof's 400; a wrong claim is 401.
    let mut sent = vouch_call(&a, b.id(), "x", NOW);
    sent.proof = None;
    let err = w.vouches.give(&sent.call(), b.id(), NOW).await.unwrap_err();
    assert_eq!(
        (err.status, err.extra.contains_key("required_bits")),
        (400, true)
    );
    let wrong = Agent {
        claim: "x".repeat(43),
        ..a.clone()
    };
    let sent = vouch_call(&wrong, b.id(), "x", NOW);
    assert_eq!(
        w.vouches
            .give(&sent.call(), b.id(), NOW)
            .await
            .unwrap_err()
            .status,
        401
    );
    assert_eq!(
        w.vouches
            .given(wrong.auth().as_deref(), None, NOW)
            .await
            .unwrap_err()
            .status,
        401
    );
    assert_eq!(
        w.vouches
            .withdraw(None, b.id(), NOW)
            .await
            .unwrap_err()
            .status,
        401
    );
}
