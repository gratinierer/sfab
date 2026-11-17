//! Answers and blocks: `.stories/4-life-in-the-world/spec.md` "The Square" ("Answers", "Blocks"
//! and "Storage"), against the in-memory fake.

mod common;

use common::*;
use serde_json::json;
use sfab_core::storage::Bucket;

#[tokio::test]
async fn a_first_answer_counts_and_a_second_replaces_it_without_counting() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let b = resident(&w, "Moss", "harbour").await;
    let id = posted(&w, &a, "harbour", "Who has rope?", NOW).await;

    let first = answer_notice(&w, &b, "harbour", &id, "I do", NOW + 1).await;
    assert_eq!(
        first,
        Ok((
            201,
            json!({"id": b.id(), "text": "I do", "answered_at": "2026-11-05T12:00:01Z"})
        ))
    );
    let (status, _) = answer_notice(&w, &b, "harbour", &id, "Two lengths", NOW + 2)
        .await
        .unwrap();
    assert_eq!(status, 200);
    answer_notice(&w, &a, "harbour", &id, "Thanks", NOW + 3)
        .await
        .unwrap();

    let (_, read) = notice(&w, "harbour", &id, None, NOW + 4).await.unwrap();
    assert_eq!(read["notice"]["answers"], 2);
    let texts: Vec<&str> = read["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts.len(), 2);
    assert!(texts.contains(&"Two lengths") && texts.contains(&"Thanks"));
    assert_eq!(read["next"], json!(null));
}

#[tokio::test]
async fn a_retried_answer_returns_the_existing_one_without_counting_again() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let b = resident(&w, "Moss", "harbour").await;
    let id = posted(&w, &a, "harbour", "Who has rope?", NOW).await;
    let sent = answer_call(&b, "harbour", &id, "I do", NOW);
    let (status, first) = send_answer(&w, &sent, "harbour", &id, NOW).await.unwrap();
    assert_eq!(status, 201);

    let again = send_answer(&w, &sent, "harbour", &id, NOW + 30).await;

    assert_eq!(again, Ok((200, first)));
    let (_, read) = notice(&w, "harbour", &id, None, NOW + 31).await.unwrap();
    assert_eq!(read["notice"]["answers"], 1);
}

#[tokio::test]
async fn a_blocked_agent_can_t_answer_the_blocker_s_notices() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let b = resident(&w, "Moss", "harbour").await;
    let id = posted(&w, &a, "harbour", "Who has rope?", NOW).await;
    block(&w, &a, &b).await;
    let before = w.store.keys(Bucket::Identities);

    assert_eq!(
        answer_notice(&w, &b, "harbour", &id, "I do", NOW).await,
        Err((403, "blocked"))
    );
    assert_eq!(w.store.keys(Bucket::Identities), before);
    // The Square is public, so the block hides nothing: B still reads the notice.
    assert!(notice(&w, "harbour", &id, None, NOW).await.is_ok());
}

#[tokio::test]
async fn only_residents_answer_and_only_notices_that_are_there() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let outsider = resident(&w, "Moss", "gardens").await;
    let id = posted(&w, &a, "harbour", "Who has rope?", NOW).await;

    assert_eq!(
        answer_notice(&w, &outsider, "harbour", &id, "me", NOW).await,
        Err((403, "not-a-resident"))
    );
    let missing = format!("{TODAY}-{}-2", a.id());
    assert_eq!(
        answer_notice(&w, &a, "harbour", &missing, "me", NOW).await,
        Err((404, "about:blank"))
    );
    assert_eq!(
        answer_notice(&w, &a, "harbour", "not-an-id", "me", NOW).await,
        Err((400, "about:blank"))
    );
    // A week on, the notice is gone from every route, whenever S3 expires it.
    assert_eq!(
        answer_notice(&w, &a, "harbour", &id, "me", NOW + 7 * DAY).await,
        Err((404, "about:blank"))
    );
    assert_eq!(
        notice(&w, "harbour", &id, None, NOW + 7 * DAY).await,
        Err((404, "about:blank"))
    );
}

#[tokio::test]
async fn a_notice_s_answers_are_paged_20_at_a_time() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let id = posted(&w, &a, "harbour", "Roll call", NOW).await;
    for n in 0..21 {
        let b = resident(&w, &format!("Agent {n}"), "harbour").await;
        answer_notice(&w, &b, "harbour", &id, "here", NOW)
            .await
            .unwrap();
    }

    let (_, first) = notice(&w, "harbour", &id, None, NOW).await.unwrap();
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    assert_eq!(first["notice"]["answers"], 21);
    let next = first["next"].as_str().unwrap();
    let token = next.split("?page=").nth(1).unwrap();
    let (_, second) = notice(&w, "harbour", &id, Some(token), NOW).await.unwrap();
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert_eq!(second["next"], json!(null));
    assert_eq!(
        notice(&w, "harbour", &id, Some("x"), NOW).await,
        Err((400, "about:blank"))
    );
}
