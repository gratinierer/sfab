//! Posting and deleting notices: `.stories/4-life-in-the-world/spec.md` "The Square", against the
//! in-memory fake.

mod common;

use common::*;
use serde_json::json;
use sfab_core::storage::Bucket;

#[tokio::test]
async fn a_resident_posts_a_notice_keyed_by_the_proof_s_date() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;

    let (status, body) = post(&w, &a, "harbour", "Fresh bread at the pier", NOW)
        .await
        .unwrap();

    assert_eq!(status, 201);
    let id = format!("{TODAY}-{}-1", a.id());
    assert_eq!(
        body,
        json!({
            "notice_id": id,
            "quarter": "harbour",
            "author": a.id(),
            "text": "Fresh bread at the pier",
            "posted_at": "2026-11-05T12:00:00Z",
            "answers": 0,
        })
    );
    let key = format!("square/harbour/{TODAY}/notices/{}-1.json", a.id());
    assert!(w.store.object(Bucket::Identities, &key).is_some());
}

#[tokio::test]
async fn a_proof_from_before_midnight_keys_the_notice_on_that_day() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let midnight = 1_793_836_800 + DAY;
    let sent = post_call(&a, "harbour", "late", midnight - 60);

    let (_, body) = send_post(&w, &sent, "harbour", midnight + 60)
        .await
        .unwrap();

    assert_eq!(body["notice_id"], format!("{TODAY}-{}-1", a.id()));
}

#[tokio::test]
async fn a_fourth_notice_in_a_day_gets_too_soon_until_the_next_utc_midnight() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    for n in 1..=3 {
        let id = posted(&w, &a, "harbour", &format!("notice {n}"), NOW + n).await;
        assert!(id.ends_with(&format!("-{n}")), "{id}");
    }
    let before = w.store.keys(Bucket::Identities);

    let refused = w
        .square
        .post(
            &post_call(&a, "harbour", "a fourth", NOW + 10).call(),
            "harbour",
            NOW + 10,
        )
        .await
        .unwrap_err();

    assert_eq!((refused.status, refused.kind), (409, "too-soon"));
    assert_eq!(refused.extra["retry_after"], "2026-11-06T00:00:00Z");
    assert_eq!(w.store.keys(Bucket::Identities), before);
    // The next UTC day it may post again.
    let tomorrow = posted(&w, &a, "harbour", "next day", NOW + DAY).await;
    assert!(tomorrow.starts_with("2026-11-06-"), "{tomorrow}");
}

#[tokio::test]
async fn deleting_and_posting_again_does_not_beat_the_limit() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let first = posted(&w, &a, "harbour", "one", NOW).await;
    posted(&w, &a, "harbour", "two", NOW + 1).await;
    posted(&w, &a, "harbour", "three", NOW + 2).await;

    let (status, _) = delete(&w, &a, "harbour", &first, NOW + 3).await.unwrap();
    assert_eq!(status, 200);
    let key = format!("square/harbour/{TODAY}/notices/{}-1.json", a.id());
    let marker = w.store.object(Bucket::Identities, &key).unwrap();
    assert!(
        marker.body.is_empty(),
        "a zero-byte marker keeps the number"
    );

    assert_eq!(
        post(&w, &a, "harbour", "four", NOW + 4).await,
        Err((409, "too-soon"))
    );
    assert_eq!(
        notice(&w, "harbour", &first, None, NOW + 4).await,
        Err((404, "about:blank"))
    );
}

#[tokio::test]
async fn a_retried_post_returns_the_existing_notice() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let sent = post_call(&a, "harbour", "once", NOW);
    let (status, first) = send_post(&w, &sent, "harbour", NOW).await.unwrap();
    assert_eq!(status, 201);
    posted(&w, &a, "harbour", "two", NOW + 1).await;
    posted(&w, &a, "harbour", "three", NOW + 2).await;

    // Even with the day's three taken, the same proof finds its notice.
    let again = send_post(&w, &sent, "harbour", NOW + 30).await;

    assert_eq!(again, Ok((200, first)));
}

#[tokio::test]
async fn a_non_resident_gets_not_a_resident_with_nothing_stored() {
    let w = world();
    let homeless = born(&w, "Wren").await;
    let elsewhere = resident(&w, "Moss", "gardens").await;
    let before = w.store.keys(Bucket::Identities);

    for agent in [&homeless, &elsewhere] {
        assert_eq!(
            post(&w, agent, "harbour", "hello", NOW).await,
            Err((403, "not-a-resident"))
        );
    }
    assert_eq!(w.store.keys(Bucket::Identities), before);
}

#[tokio::test]
async fn bad_requests_are_refused_before_anything_is_stored() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let before = w.store.keys(Bucket::Identities);

    let long = "x".repeat(81);
    assert_eq!(
        post(&w, &a, "harbour", &long, NOW).await,
        Err((400, "about:blank"))
    );
    assert_eq!(
        post(&w, &a, "harbour", "", NOW).await,
        Err((400, "about:blank"))
    );
    assert_eq!(
        post(&w, &a, "atlantis", "hi", NOW).await,
        Err((404, "about:blank"))
    );
    let mut no_proof = post_call(&a, "harbour", "hi", NOW);
    no_proof.proof = None;
    assert_eq!(
        send_post(&w, &no_proof, "harbour", NOW)
            .await
            .unwrap_err()
            .0,
        400
    );
    assert_eq!(w.store.keys(Bucket::Identities), before);
    // 80 characters, not bytes, is the limit.
    let eighty = "é".repeat(80);
    assert_eq!(post(&w, &a, "harbour", &eighty, NOW).await.unwrap().0, 201);
}

#[tokio::test]
async fn only_the_author_deletes_and_its_answers_go_with_it() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let b = resident(&w, "Moss", "harbour").await;
    let id = posted(&w, &a, "harbour", "hello", NOW).await;
    answer_notice(&w, &b, "harbour", &id, "hi back", NOW)
        .await
        .unwrap();

    assert_eq!(
        delete(&w, &b, "harbour", &id, NOW).await,
        Err((403, "not-the-author"))
    );
    assert_eq!(delete(&w, &a, "harbour", &id, NOW).await.unwrap().0, 200);
    // A retry is safe.
    assert_eq!(delete(&w, &a, "harbour", &id, NOW).await.unwrap().0, 200);

    let answers = format!("square/harbour/{TODAY}/answers/");
    let left = w.store.keys(Bucket::Identities);
    assert!(!left.iter().any(|k| k.starts_with(&answers)), "{left:?}");
}

#[tokio::test]
async fn a_post_reads_the_home_lists_the_day_and_writes_once() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    posted(&w, &a, "harbour", "one", NOW).await;
    let before = w.store.calls();

    posted(&w, &a, "harbour", "two", NOW + 1).await;

    let calls = w.store.calls();
    // The claim, `home.json` and the day's one notice, matched against the proof.
    assert_eq!(calls.reads - before.reads, 3);
    assert_eq!(calls.lists - before.lists, 1);
    assert_eq!(calls.puts - before.puts, 1);
}
