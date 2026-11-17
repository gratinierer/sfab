//! The daily world job's shared rules and the routes that read what it built:
//! `.stories/4-life-in-the-world/spec.md` "Foundations" ("What the daily job writes", "Before the
//! first daily run", "Unknown IDs").

use std::collections::BTreeMap;

use serde_json::json;

use sfab_core::storage::{Bucket, Condition, MemoryStore, Metadata, Store};
use sfab_core::world::{self, Rebuilt};

const B: Bucket = Bucket::Identities;

async fn put(store: &MemoryStore, key: &str, body: &str) {
    let body = body.as_bytes().to_vec();
    store
        .put(B, key, body, Metadata::new(), Condition::Always)
        .await
        .unwrap();
}

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.as_bytes().to_vec()))
        .collect()
}

#[tokio::test]
async fn a_rebuild_writes_only_what_changed_and_deletes_what_is_no_longer_in_the_set() {
    let store = MemoryStore::new();
    put(&store, "world/library/shelves/craft/1.json", "same").await;
    put(&store, "world/library/shelves/craft/2.json", "old").await;
    // Pages past the end of a list that shrank.
    put(&store, "world/library/shelves/craft/3.json", "gone").await;
    put(&store, "world/library/shelves/craft/4.json", "gone").await;
    // Beside the prefix, not under it.
    put(&store, "world/library/shelves/crafts/1.json", "other").await;

    let report = world::rebuild(
        &store,
        "world/library/shelves/craft/",
        files(&[
            ("world/library/shelves/craft/1.json", "same"),
            ("world/library/shelves/craft/2.json", "new"),
        ]),
    )
    .await
    .unwrap();

    assert_eq!(
        report,
        Rebuilt {
            written: 1,
            unchanged: 1,
            deleted: 2
        }
    );
    assert_eq!(
        store.keys(B),
        [
            "world/library/shelves/craft/1.json",
            "world/library/shelves/craft/2.json",
            "world/library/shelves/crafts/1.json",
        ]
    );
    assert_eq!(store.writes_to(B, "world/library/shelves/craft/1.json"), 1);
    let page = store
        .object(B, "world/library/shelves/craft/2.json")
        .unwrap();
    assert_eq!(page.body, b"new");
}

#[tokio::test]
async fn a_second_run_with_the_same_set_writes_and_deletes_nothing() {
    let store = MemoryStore::new();
    let set = files(&[("world/square/harbour.json", "{}")]);
    world::rebuild(&store, "world/square/", set.clone())
        .await
        .unwrap();

    let again = world::rebuild(&store, "world/square/", set).await.unwrap();

    assert_eq!(
        again,
        Rebuilt {
            written: 0,
            unchanged: 1,
            deleted: 0
        }
    );
}

#[tokio::test]
async fn a_new_file_costs_no_read() {
    let store = MemoryStore::new();

    world::rebuild(
        &store,
        "world/square/",
        files(&[("world/square/a.json", "{}")]),
    )
    .await
    .unwrap();

    // One listing and one write: the job may list only `world/`, so it never GETs a key it
    // didn't see listed.
    let calls = store.calls();
    assert_eq!((calls.reads, calls.lists, calls.puts), (0, 1, 1));
}

#[tokio::test]
async fn a_rebuild_never_deletes_the_chronicle_s_entries_or_milestones() {
    let store = MemoryStore::new();
    put(&store, "world/chronicle/2026-11-05.json", "entry").await;
    put(&store, "world/chronicle/milestones.json", "milestones").await;
    put(&store, "world/chronicle/days/1.json", "page").await;
    put(&store, "world/chronicle/days/2.json", "past the end").await;

    let report = world::rebuild(
        &store,
        "world/chronicle/",
        files(&[("world/chronicle/days/1.json", "page")]),
    )
    .await
    .unwrap();

    assert_eq!(report.deleted, 1);
    assert_eq!(
        store.keys(B),
        [
            "world/chronicle/2026-11-05.json",
            "world/chronicle/days/1.json",
            "world/chronicle/milestones.json",
        ]
    );
}

#[tokio::test]
async fn a_rebuild_outside_world_or_with_a_file_outside_its_prefix_changes_nothing() {
    let store = MemoryStore::new();
    put(&store, "square/harbour/n1.json", "a notice").await;
    put(&store, "world/square/harbour.json", "{}").await;

    let outside = world::rebuild(&store, "square/", BTreeMap::new()).await;
    let stray = world::rebuild(
        &store,
        "world/square/",
        files(&[("world/library/x.json", "{}")]),
    )
    .await;
    let bare = world::rebuild(&store, "world/square", BTreeMap::new()).await;

    assert!(outside.is_err() && stray.is_err() && bare.is_err());
    assert_eq!(
        store.keys(B),
        ["square/harbour/n1.json", "world/square/harbour.json"]
    );
}

#[tokio::test]
async fn a_route_answers_not_built_yet_until_its_file_exists() {
    let store = MemoryStore::new();

    let before = world::read_built(&store, "world/square/harbour.json").await;
    assert_eq!(before.unwrap_err().kind, "not-built-yet");
    assert_eq!(
        world::read_built(&store, "world/square/harbour.json")
            .await
            .unwrap_err()
            .status,
        404
    );

    put(&store, "world/square/harbour.json", r#"{"items":[]}"#).await;
    let after = world::read_built(&store, "world/square/harbour.json").await;
    assert_eq!(after.unwrap(), json!({"items": []}));
}

#[tokio::test]
async fn a_file_that_does_not_parse_is_a_500() {
    let store = MemoryStore::new();
    put(&store, "world/square/harbour.json", "not json").await;

    let read = world::read_built(&store, "world/square/harbour.json").await;

    assert_eq!(read.unwrap_err().status, 500);
}

#[tokio::test]
async fn an_unknown_or_taken_down_id_reads_like_an_agent_with_nothing_to_show() {
    let store = MemoryStore::new();
    let built = "world/census.json";
    let empty = json!({"items": [], "next": null});
    let shown = json!({"items": ["x"], "next": null});
    let read = |key: &'static str| world::read_for_agent(&store, key, built, empty.clone());

    // Before the first run, every ID alike.
    assert_eq!(
        read("world/academy/masters/aaaaaaaaaaaa.json")
            .await
            .unwrap_err()
            .kind,
        "not-built-yet"
    );

    put(&store, built, "{}").await;
    put(
        &store,
        "world/academy/masters/bbbbbbbbbbbb.json",
        &shown.to_string(),
    )
    .await;
    // An agent with nothing to show has no file, like an unknown or taken-down ID.
    assert_eq!(
        read("world/academy/masters/aaaaaaaaaaaa.json")
            .await
            .unwrap(),
        empty
    );
    assert_eq!(
        read("world/academy/masters/bbbbbbbbbbbb.json")
            .await
            .unwrap(),
        shown
    );
}
