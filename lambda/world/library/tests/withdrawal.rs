//! Withdrawing an edition and a takedown's note: `.stories/4-life-in-the-world/spec.md` "The
//! Library" ("Removal, despite permanent").

mod common;

use common::*;
use serde_json::json;
use sfab_core::storage::{Bucket, Store};
use sfab_core::takedown::Action;

#[tokio::test]
async fn a_withdrawn_edition_keeps_its_note_and_loses_its_content() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;
    let cited = format!("{}/1", doc("other"));
    edition(&w, &a, &id, "2", &edition_body("Two", &[&cited]))
        .await
        .unwrap();

    let (status, withdrawn) = withdraw(&w, &a, &id, "2").await.unwrap();

    assert_eq!(status, 200);
    assert_eq!(withdrawn["note"], "withdrawn by the author");
    let (_, read) = read_edition(&w, &id, "2").await.unwrap();
    assert_eq!(read["note"], "withdrawn by the author");
    assert_eq!(read["content"], json!(null));
    assert_eq!(read["title"], json!(null));
    assert_eq!(read["citations"], json!([]));
    let stored = w
        .store
        .object(Bucket::Identities, &format!("library/{id}/2.json"))
        .unwrap();
    assert!(
        !String::from_utf8(stored.body)
            .unwrap()
            .contains("The words of Two")
    );
    // The page shows the note, and its title is the newest edition left.
    let (_, page) = read_document(&w, &id).await.unwrap();
    assert_eq!(page["editions"][1]["note"], "withdrawn by the author");
    assert_eq!(page["title"], id.as_str());
    // Again is 200, and it frees no place.
    assert_eq!(withdraw(&w, &a, &id, "2").await.unwrap().0, 200);
    assert_eq!(
        edition(&w, &a, &id, "2", &edition_body("Two again", &[]))
            .await
            .map_err(|e| e.1),
        Err("already-exists")
    );
}

#[tokio::test]
async fn withdrawing_the_first_edition_needs_no_standing_and_only_the_author() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let b = born(&w, "Moss").await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;
    give_standing(&w, &[]).await;

    assert_eq!(
        withdraw(&w, &b, &id, "1").await,
        Err((403, "not-the-author"))
    );
    assert_eq!(withdraw(&w, &a, &id, "2").await.map_err(|e| e.0), Err(404));
    let (status, done) = withdraw(&w, &a, &id, "1").await.unwrap();

    assert_eq!(
        (status, done["note"].clone()),
        (200, json!("withdrawn by the author"))
    );
    let (_, page) = read_document(&w, &id).await.unwrap();
    assert_eq!(page["title"], json!(null));
    assert_eq!(
        read_edition(&w, &id, "1").await.unwrap().1["content"],
        json!(null)
    );
}

/// Runs the library crate's takedown list as the program does, without the program.
async fn take_down(w: &World, who: &Agent) {
    for l in sfab_library::takedown(&who.id) {
        let Action::NoteFolders(pointers_of, note_of) = l.action else {
            panic!("the Library's list notes folders");
        };
        // The author's list, or the takedown index, which no daily run has built here.
        let Some(list) = w.store.get(l.bucket, &l.key).await.unwrap() else {
            continue;
        };
        for anchor in pointers_of(&list.body) {
            let folder = &anchor[..anchor.rfind('/').unwrap() + 1];
            for key in w.store.list(l.bucket, folder).await.unwrap() {
                let current = w.store.get(l.bucket, &key).await.unwrap().unwrap();
                if let Some(body) = note_of(&current.body) {
                    w.store
                        .put(
                            l.bucket,
                            &key,
                            body,
                            Default::default(),
                            sfab_core::storage::Condition::Always,
                        )
                        .await
                        .unwrap();
                }
            }
        }
    }
}

#[tokio::test]
async fn a_takedown_notes_every_edition_removed_and_a_withdrawn_one_keeps_its_note() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;
    edition(&w, &a, &id, "2", &edition_body("Two", &[]))
        .await
        .unwrap();
    edition(&w, &a, &id, "3", &edition_body("Three", &[]))
        .await
        .unwrap();
    withdraw(&w, &a, &id, "3").await.unwrap();

    take_down(&w, &a).await;

    for (n, note) in [
        ("1", "removed"),
        ("2", "removed"),
        ("3", "withdrawn by the author"),
    ] {
        let (_, read) = read_edition(&w, &id, n).await.unwrap();
        assert_eq!(
            (read["note"].as_str(), read["content"].clone()),
            (Some(note), json!(null)),
            "{n}"
        );
    }
    let (_, page) = read_document(&w, &id).await.unwrap();
    assert_eq!(page["title"], json!(null));
    // A second run finds nothing left to note.
    let writes = w
        .store
        .writes_to(Bucket::Identities, &format!("library/{id}/document.json"));
    take_down(&w, &a).await;
    assert_eq!(
        w.store
            .writes_to(Bucket::Identities, &format!("library/{id}/document.json")),
        writes
    );
}
