//! The daily world job's homes step and the quarter routes that read what it wrote:
//! `.stories/4-life-in-the-world/spec.md` "Homes and quarters", against the in-memory fake.

mod common;

use common::*;
use serde_json::json;
use sfab_core::storage::{Bucket, Condition, MemoryStore, Metadata, Store};
use sfab_homes::{Quarter, daily, parse_quarters, quarters};

fn last(n: usize) -> usize {
    n - 1
}

/// One quarter of six lanes, 120 homes, more than a day's 100 free.
fn six_lanes() -> &'static [Quarter] {
    let text =
        "quarters:\n  - name: dock\n    lanes: [a-lane, b-lane, c-lane, d-lane, e-lane, f-lane]\n";
    Box::leak(parse_quarters(text).unwrap().into_boxed_slice())
}

async fn pointer(w: &World, key: &str, id: &str) {
    w.store
        .put(
            Bucket::Identities,
            key,
            json!({ "id": id }).to_string().into_bytes(),
            Metadata::new(),
            Condition::Always,
        )
        .await
        .unwrap();
}

async fn pointers(w: &World) -> Vec<String> {
    w.store.list(Bucket::Identities, "places/").await.unwrap()
}

#[tokio::test]
async fn the_job_deletes_ghost_pointers_and_keeps_those_home_json_agrees_with() {
    let w = world();
    let (a, gone) = (born(&w, "Wren").await, born(&w, "Moss").await);
    move_to(&w, &a, "harbour", NOW).await.unwrap();
    // An old pointer a retried move skipped, one naming an agent with no home, and junk.
    pointer(&w, "places/harbour/rope-walk/2.json", a.id()).await;
    pointer(&w, "places/harbour/rope-walk/3.json", gone.id()).await;
    pointer(&w, "places/harbour/rope-walk/4.json", "Not An Id").await;

    let report = daily(&*w.store, quarters(), NOW + 60).await.unwrap();

    assert_eq!((report.residents, report.ghosts), (1, 3));
    assert_eq!(
        pointers(&w).await,
        vec!["places/harbour/lantern-lane/1.json"]
    );
    let again = daily(&*w.store, quarters(), NOW + 120).await.unwrap();
    assert_eq!((again.residents, again.ghosts), (1, 0));
}

#[tokio::test]
async fn a_pointer_written_in_the_last_minutes_is_left_alone_as_a_move_in_flight() {
    let w = world();
    let a = born(&w, "Wren").await;
    w.store.set_time(NOW);
    pointer(&w, "places/harbour/lantern-lane/5.json", a.id()).await;

    let report = daily(&*w.store, quarters(), NOW + 60).await.unwrap();

    assert_eq!((report.residents, report.ghosts), (1, 0));
    let later = daily(&*w.store, quarters(), NOW + sfab_homes::FRESH_SECONDS + 1).await;
    assert_eq!(later.unwrap().ghosts, 1);
    assert!(pointers(&w).await.is_empty());
}

#[tokio::test]
async fn the_quarter_routes_are_not_built_yet_before_the_first_run_and_then_count() {
    let w = world();
    let a = born(&w, "Wren").await;
    move_to(&w, &a, "harbour", NOW).await.unwrap();
    assert_eq!(
        w.homes.quarters(None).await.unwrap_err().kind,
        "not-built-yet"
    );
    assert_eq!(
        w.homes.quarter_counts("harbour").await.unwrap_err().kind,
        "not-built-yet"
    );
    assert_eq!(
        w.homes.lanes("harbour", None).await.unwrap_err().kind,
        "not-built-yet"
    );

    daily(&*w.store, quarters(), NOW + 60).await.unwrap();

    let quarter = w.homes.quarter_counts("harbour").await.unwrap().body;
    assert_eq!(
        quarter,
        json!({"quarter": "harbour", "residents": 1, "free": 99, "as_of": "2026-11-05T12:01:00Z"})
    );
    let lanes = w.homes.lanes("harbour", None).await.unwrap().body;
    assert_eq!(
        lanes,
        json!({"items": [{"lane": "lantern-lane", "residents": 1}], "next": null})
    );
    let all = w.homes.quarters(None).await.unwrap().body;
    assert_eq!(all["items"].as_array().unwrap().len(), quarters().len());
    assert_eq!(
        all["items"][0],
        json!({"quarter": "harbour", "residents": 1, "free": 99})
    );
    assert_eq!(all["items"][1]["residents"], 0);
    assert_eq!(
        w.homes.quarter_counts("atlantis").await.unwrap_err().status,
        404
    );
}

#[tokio::test]
async fn free_homes_offer_gaps_first_then_the_lanes_in_order_up_to_100() {
    let w = world_on(std::sync::Arc::new(MemoryStore::new()), six_lanes(), last);
    let (a, b) = (born(&w, "Wren").await, born(&w, "Moss").await);
    // Without free homes: the first lane, its last home.
    let (_, home) = move_to(&w, &a, "dock", NOW).await.unwrap();
    assert_eq!(home["address"], "dock/a-lane/20");

    daily(&*w.store, six_lanes(), NOW + 60).await.unwrap();

    // The 100th free home: a-lane's 19 gaps, then b-lane to e-lane, then f-lane's first.
    let (_, home) = move_to(&w, &b, "dock", NOW + 120).await.unwrap();
    assert_eq!(home["address"], "dock/f-lane/1");
}

#[tokio::test]
async fn a_full_quarter_answers_quarter_full() {
    let w = world_on(std::sync::Arc::new(MemoryStore::new()), small_map(), first);
    for i in 0..40 {
        let agent = born(&w, &format!("Agent {i}")).await;
        let moved = move_to(&w, &agent, "dock", NOW).await;
        assert_eq!(moved.unwrap().0, 201, "agent {i}");
        daily(&*w.store, small_map(), NOW + 60).await.unwrap();
    }
    let late = born(&w, "Late").await;

    assert_eq!(
        move_to(&w, &late, "dock", NOW).await,
        Err((409, "quarter-full"))
    );
    let counts = w.homes.quarter_counts("dock").await.unwrap().body;
    assert_eq!(
        (counts["residents"].clone(), counts["free"].clone()),
        (json!(40), json!(0))
    );
}
