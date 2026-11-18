//! The actions a crate's takedown function can name for a location, as the program finds and
//! applies them: delete, leave a note, replace the ID with "[removed]", follow pointers
//! and search a prefix (`.stories/4-life-in-the-world/spec.md` "Takedown").

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

/// A crate's search: the keys whose last segment starts with the agent's ID.
fn named_last(key: &str, id: &AgentId) -> bool {
    key.rsplit('/')
        .next()
        .is_some_and(|k| k.starts_with(id.as_str()))
}

#[tokio::test]
async fn a_search_deletes_only_the_keys_under_the_prefix_it_says_are_the_agent_s() {
    let store = MemoryStore::new();
    put(&store, "square/a/wrenwrenwren-1.json", "{}").await;
    put(&store, "square/a/mossmossmoss-1.json", "{}").await;
    put(&store, "square/b/x/wrenwrenwren.json", "{}").await;
    put(&store, "places/wrenwrenwren.json", "{}").await;
    let l = location("square/", Action::Search(named_last));

    let keys: Vec<String> = found(&store, &wren(), &l)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, k)| k)
        .collect();
    assert_eq!(
        keys,
        vec![
            "square/a/wrenwrenwren-1.json",
            "square/b/x/wrenwrenwren.json"
        ]
    );
    apply(&store, &wren(), &l).await.unwrap();

    assert_eq!(
        store.keys(B),
        vec!["places/wrenwrenwren.json", "square/a/mossmossmoss-1.json"]
    );
    assert!(found(&store, &wren(), &l).await.unwrap().is_empty());
}

#[tokio::test]
async fn folders_delete_all_beside_a_file_the_agent_s_file_names_while_it_names_the_agent() {
    let store = MemoryStore::new();
    put(
        &store,
        "identities/wrenwrenwren/academy.json",
        "academy/courses/aaaa/course.json\nacademy/courses/bbbb/course.json",
    )
    .await;
    let mine = r#"{"teacher":"wrenwrenwren"}"#;
    put(&store, "academy/courses/aaaa/course.json", mine).await;
    put(
        &store,
        "academy/courses/aaaa/enrolled/mossmossmoss.json",
        "{}",
    )
    .await;
    // Someone else's course, so the file was out of date.
    let theirs = r#"{"teacher":"mossmossmoss"}"#;
    put(&store, "academy/courses/bbbb/course.json", theirs).await;
    put(
        &store,
        "academy/courses/bbbb/enrolled/kitekitekite.json",
        "{}",
    )
    .await;
    let l = location(
        "identities/wrenwrenwren/academy.json",
        Action::Folders(lines),
    );

    let keys: Vec<String> = found(&store, &wren(), &l)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, k)| k)
        .collect();
    assert_eq!(
        keys,
        vec![
            "academy/courses/aaaa/enrolled/mossmossmoss.json",
            "academy/courses/aaaa/course.json",
        ]
    );
    apply(&store, &wren(), &l).await.unwrap();

    assert_eq!(
        store.keys(B),
        vec![
            "academy/courses/bbbb/course.json",
            "academy/courses/bbbb/enrolled/kitekitekite.json",
            "identities/wrenwrenwren/academy.json",
        ]
    );
    assert!(found(&store, &wren(), &l).await.unwrap().is_empty());
}

/// A note that keeps the author, so the anchor still names the agent on a rerun.
fn removed_keeping_author(current: &[u8]) -> Option<Vec<u8>> {
    let text = String::from_utf8_lossy(current);
    (!text.contains("removed")).then(|| text.replace("words", "removed").into_bytes())
}

#[tokio::test]
async fn note_folders_note_all_beside_a_file_the_agent_s_file_names_the_named_one_last() {
    let store = MemoryStore::new();
    put(
        &store,
        "identities/wrenwrenwren/library.json",
        "library/aaaa/document.json\nlibrary/bbbb/document.json",
    )
    .await;
    let mine = r#"{"author":"wrenwrenwren","words":1}"#;
    put(&store, "library/aaaa/document.json", mine).await;
    put(&store, "library/aaaa/2.json", r#"{"words":2}"#).await;
    // Someone else's document, so the list was out of date.
    let theirs = r#"{"author":"mossmossmoss","words":1}"#;
    put(&store, "library/bbbb/document.json", theirs).await;
    let l = location(
        "identities/wrenwrenwren/library.json",
        Action::NoteFolders(lines, removed_keeping_author),
    );

    let keys: Vec<String> = found(&store, &wren(), &l)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, k)| k)
        .collect();
    assert_eq!(
        keys,
        vec!["library/aaaa/2.json", "library/aaaa/document.json"]
    );
    apply(&store, &wren(), &l).await.unwrap();

    assert_eq!(body(&store, "library/aaaa/2.json"), r#"{"removed":2}"#);
    assert_eq!(
        body(&store, "library/aaaa/document.json"),
        r#"{"author":"wrenwrenwren","removed":1}"#
    );
    assert_eq!(body(&store, "library/bbbb/document.json"), theirs);
    assert!(found(&store, &wren(), &l).await.unwrap().is_empty());
}
