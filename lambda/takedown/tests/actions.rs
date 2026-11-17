//! The actions a crate's takedown function can name for a location, as the program finds and
//! applies them: delete, leave a note, replace the ID with "[removed]", and follow pointers
//! (`.stories/4-life-in-the-world/spec.md` "Takedown").

use sfab_core::id::AgentId;
use sfab_core::storage::{Bucket, Condition, MemoryStore, Metadata, Store};
use sfab_core::takedown::{Action, Location, Phase};
use sfab_takedown::{apply, found};

const B: Bucket = Bucket::Identities;

fn wren() -> AgentId {
    AgentId::parse("wrenwrenwren").unwrap()
}

async fn put(store: &MemoryStore, key: &str, body: &str) {
    let body = body.as_bytes().to_vec();
    store
        .put(B, key, body, Metadata::new(), Condition::Always)
        .await
        .unwrap();
}

fn body(store: &MemoryStore, key: &str) -> String {
    String::from_utf8(store.object(B, key).unwrap().body).unwrap()
}

fn location(key: &str, action: Action) -> Location {
    Location {
        phase: Phase::Other,
        bucket: B,
        key: key.to_owned(),
        action,
    }
}

const NOTE: &str = r#"{"note":"removed"}"#;

/// A crate's note: the content goes, and a visible note stays. `None` once it's a note.
fn removed_note(current: &[u8]) -> Option<Vec<u8>> {
    (current != NOTE.as_bytes()).then(|| NOTE.as_bytes().to_vec())
}

#[tokio::test]
async fn delete_removes_the_key() {
    let store = MemoryStore::new();
    put(&store, "vouches/wrenwrenwren.json", "{}").await;
    let l = location("vouches/wrenwrenwren.json", Action::Delete);

    assert_eq!(found(&store, &wren(), &l).await.unwrap().len(), 1);
    apply(&store, &wren(), &l).await.unwrap();

    assert!(store.keys(B).is_empty());
    assert!(found(&store, &wren(), &l).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_note_replaces_the_content_once() {
    let store = MemoryStore::new();
    let key = "library/doc/2.json";
    put(&store, key, r#"{"title":"mine","text":"words"}"#).await;
    let l = location(key, Action::Note(removed_note));

    assert_eq!(
        found(&store, &wren(), &l).await.unwrap(),
        [(B, key.to_owned())]
    );
    apply(&store, &wren(), &l).await.unwrap();
    assert_eq!(body(&store, key), NOTE);

    // A rerun finds nothing left to do and writes nothing.
    assert!(found(&store, &wren(), &l).await.unwrap().is_empty());
    apply(&store, &wren(), &l).await.unwrap();
    assert_eq!(store.writes_to(B, key), 2);
}

#[tokio::test]
async fn a_note_on_a_missing_key_does_nothing() {
    let store = MemoryStore::new();
    let l = location("library/doc/2.json", Action::Note(removed_note));

    apply(&store, &wren(), &l).await.unwrap();

    assert!(store.keys(B).is_empty());
}

#[tokio::test]
async fn removing_the_id_rewrites_only_the_files_under_the_prefix_that_name_it() {
    let store = MemoryStore::new();
    let entry = "world/chronicle/2026-11-05.json";
    let other = "world/chronicle/2026-11-06.json";
    put(
        &store,
        entry,
        r#"{"new_residents":["wrenwrenwren","mossmossmoss"],"first":"wrenwrenwren"}"#,
    )
    .await;
    put(&store, other, r#"{"new_residents":["mossmossmoss"]}"#).await;
    put(&store, "world/census.json", r#"{"x":"wrenwrenwren"}"#).await;
    let l = location("world/chronicle/", Action::RemoveId);

    assert_eq!(
        found(&store, &wren(), &l).await.unwrap(),
        [(B, entry.to_owned())]
    );
    apply(&store, &wren(), &l).await.unwrap();

    assert_eq!(
        body(&store, entry),
        r#"{"new_residents":["[removed]","mossmossmoss"],"first":"[removed]"}"#
    );
    assert_eq!(store.writes_to(B, other), 1);
    assert_eq!(body(&store, "world/census.json"), r#"{"x":"wrenwrenwren"}"#);
    assert!(found(&store, &wren(), &l).await.unwrap().is_empty());
}

/// A crate's file that points elsewhere: here, every line of the body is a key.
fn lines(body: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(body)
        .lines()
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
async fn pointers_delete_what_the_agent_s_file_names_only_while_it_still_names_the_agent() {
    let store = MemoryStore::new();
    put(
        &store,
        "identities/wrenwrenwren/home.json",
        "places/a/1.json\nplaces/a/2.json\nplaces/a/3.json",
    )
    .await;
    put(&store, "places/a/1.json", r#"{"id":"wrenwrenwren"}"#).await;
    // Since taken by someone else, so the file was out of date.
    put(&store, "places/a/2.json", r#"{"id":"mossmossmoss"}"#).await;
    let l = location("identities/wrenwrenwren/home.json", Action::Pointers(lines));

    let keys: Vec<String> = found(&store, &wren(), &l)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, k)| k)
        .collect();
    assert_eq!(keys, vec!["places/a/1.json"]);
    apply(&store, &wren(), &l).await.unwrap();

    assert_eq!(
        store.keys(B),
        vec!["identities/wrenwrenwren/home.json", "places/a/2.json"]
    );
    assert!(found(&store, &wren(), &l).await.unwrap().is_empty());
}
