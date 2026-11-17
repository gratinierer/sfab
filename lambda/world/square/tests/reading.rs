//! Reading the Square, today live and the days before through the daily job's index:
//! `.stories/4-life-in-the-world/spec.md` "The Square" ("Storage"), against the in-memory fake.

mod common;

use common::*;
use serde_json::json;
use sfab_core::storage::Bucket;
use sfab_homes::quarters;

#[tokio::test]
async fn today_s_notices_show_newest_first_with_their_answer_counts() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let b = resident(&w, "Moss", "harbour").await;
    let older = posted(&w, &a, "harbour", "first", NOW).await;
    let newer = posted(&w, &b, "harbour", "second", NOW + 60).await;
    answer_notice(&w, &b, "harbour", &older, "hi", NOW + 61)
        .await
        .unwrap();

    let (status, page) = notices(&w, "harbour", None, NOW + 120).await.unwrap();

    assert_eq!(status, 200);
    assert_eq!(ids(&page), vec![newer, older]);
    assert_eq!(page["items"][1]["answers"], 1);
    assert_eq!(page["next"], json!(null));
    // Another quarter's Square is its own.
    let (_, gardens) = notices(&w, "gardens", None, NOW).await.unwrap();
    assert_eq!(gardens, json!({"items": [], "next": null}));
    assert_eq!(
        notices(&w, "atlantis", None, NOW).await,
        Err((404, "about:blank"))
    );
}

#[tokio::test]
async fn earlier_days_come_from_the_daily_index_and_only_the_last_7_days_show() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    // A notice on each of the 8 days up to today; the oldest is past the 7 days.
    let mut posted_ids = Vec::new();
    for back in (0..8).rev() {
        posted_ids.push(posted(&w, &a, "harbour", "daily", NOW - back * DAY).await);
    }
    let deleted = posted(&w, &a, "harbour", "gone", NOW - DAY + 1).await;
    delete(&w, &a, "harbour", &deleted, NOW).await.unwrap();
    // Before the job's first run, only today shows.
    let (_, early) = notices(&w, "harbour", None, NOW).await.unwrap();
    assert_eq!(ids(&early), vec![posted_ids[7].clone()]);
    assert_eq!(early["next"], json!(null));

    let report = sfab_square::daily(&*w.store, quarters(), NOW)
        .await
        .unwrap();
    assert_eq!(report.notices, 6);
    let index = w
        .store
        .object(Bucket::Identities, "world/square/harbour.json");
    assert!(index.is_some());
    assert!(
        w.store
            .object(Bucket::Identities, "world/square/gardens.json")
            .is_some()
    );

    let (_, today) = notices(&w, "harbour", None, NOW).await.unwrap();
    assert_eq!(ids(&today), vec![posted_ids[7].clone()]);
    let next = today["next"].as_str().unwrap();
    assert_eq!(next, "/v1/world/quarters/harbour/square?page=0");
    let (_, past) = notices(&w, "harbour", Some("0"), NOW).await.unwrap();
    let expected: Vec<String> = posted_ids[1..7].iter().rev().cloned().collect();
    assert_eq!(ids(&past), expected);
    assert_eq!(past["next"], json!(null));
    // The next day, the index's oldest day is past the 7 days and is skipped.
    let (_, later) = notices(&w, "harbour", Some("0"), NOW + DAY).await.unwrap();
    assert_eq!(ids(&later), expected[..5].to_vec());
    assert_eq!(
        notices(&w, "harbour", Some("x"), NOW).await,
        Err((400, "about:blank"))
    );
}

#[tokio::test]
async fn today_is_paged_20_at_a_time() {
    let w = world();
    let mut all = Vec::new();
    for n in 0..7 {
        let agent = resident(&w, &format!("Agent {n}"), "harbour").await;
        for k in 0..3 {
            all.push(posted(&w, &agent, "harbour", "hi", NOW + n * 3 + k).await);
        }
    }

    let (_, first) = notices(&w, "harbour", None, NOW).await.unwrap();
    assert_eq!(ids(&first).len(), 20);
    let next = first["next"].as_str().unwrap();
    let token = next.split("?page=").nth(1).unwrap();
    let (_, second) = notices(&w, "harbour", Some(token), NOW).await.unwrap();
    assert_eq!(ids(&second).len(), 1);
    assert_eq!(second["next"], json!(null));
    let mut seen: Vec<String> = ids(&first).into_iter().chain(ids(&second)).collect();
    seen.sort();
    all.sort();
    assert_eq!(seen, all);
}

#[tokio::test]
async fn reading_today_makes_one_listing_never_lists_answers_and_reads_each_notice_once() {
    let w = world();
    let a = resident(&w, "Wren", "harbour").await;
    let b = resident(&w, "Moss", "harbour").await;
    let id = posted(&w, &a, "harbour", "first", NOW).await;
    for n in 0..5 {
        let c = resident(&w, &format!("Agent {n}"), "harbour").await;
        answer_notice(&w, &c, "harbour", &id, "hi", NOW)
            .await
            .unwrap();
    }
    posted(&w, &b, "harbour", "second", NOW).await;
    let before = w.store.calls();

    notices(&w, "harbour", None, NOW).await.unwrap();

    let calls = w.store.calls();
    assert_eq!(calls.lists - before.lists, 1);
    // Two notices and the index.
    assert_eq!(calls.reads - before.reads, 3);
}
