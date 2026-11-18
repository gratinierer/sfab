//! Publishing documents and editions, and reading them: `.stories/4-life-in-the-world/spec.md`
//! "The Library", against the in-memory fake. Routes that need standing are tested only here,
//! because the founders make no test writes before launch.

mod common;

use common::*;
use serde_json::json;
use sfab_core::storage::Bucket;

#[tokio::test]
async fn an_author_with_standing_publishes_a_document_at_the_id_it_picked() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");

    let (status, page) = publish(&w, &a, &id, &document_body("craft", "Knots", &[]))
        .await
        .unwrap();

    assert_eq!(status, 201);
    assert_eq!(page["document"], id.as_str());
    assert_eq!(page["author"], a.id());
    assert_eq!(page["shelf"], "craft");
    assert_eq!(page["title"], "Knots");
    assert_eq!(page["editions"][0]["n"], 1);
    let key = format!("library/{id}/document.json");
    assert!(w.store.object(Bucket::Identities, &key).is_some());
    let list = format!("identities/{}/library.json", a.id());
    assert!(w.store.object(Bucket::Identities, &list).is_some());

    let (status, read) = read_edition(&w, &id, "1").await.unwrap();
    assert_eq!(status, 200);
    assert_eq!(read["content"], "The words of Knots.");
    assert_eq!(read["newer_edition"], json!(null));
    assert_eq!(read["note"], json!(null));
}

#[tokio::test]
async fn an_author_without_standing_gets_no_standing_and_nothing_is_written() {
    let w = world();
    let a = born(&w, "Wren").await;
    give_standing(&w, &[]).await;
    let id = doc("knots");

    let refused = publish(&w, &a, &id, &document_body("craft", "Knots", &[])).await;

    assert_eq!(refused, Err((403, "no-standing")));
    assert!(
        !w.store
            .keys(Bucket::Identities)
            .iter()
            .any(|k| k.starts_with("library/"))
    );
    // Before the daily job's first run, nobody has standing either.
    let w = world();
    let a = born(&w, "Wren").await;
    let refused = publish(&w, &a, &id, &document_body("craft", "Knots", &[])).await;
    assert_eq!(refused, Err((403, "no-standing")));
}

#[tokio::test]
async fn a_retried_publish_returns_the_document_with_200_and_writes_nothing_more() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");
    let sent = publish_call(&a, &id, &document_body("craft", "Knots", &[]), NOW);

    let (_, first) = send_publish(&w, &sent, &id, NOW).await.unwrap();
    let again = send_publish(&w, &sent, &id, NOW + 1).await.unwrap();

    assert_eq!(again, (200, first));
    let key = format!("library/{id}/document.json");
    assert_eq!(w.store.writes_to(Bucket::Identities, &key), 1);
    let list = format!("identities/{}/library.json", a.id());
    assert_eq!(w.store.writes_to(Bucket::Identities, &list), 1);
}

#[tokio::test]
async fn another_write_to_a_taken_id_gets_already_exists() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let b = author(&w, "Moss", &[&a]).await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;

    let mine_again = publish(&w, &a, &id, &document_body("craft", "Other", &[])).await;
    let theirs = publish(&w, &b, &id, &document_body("craft", "Knots", &[])).await;

    assert_eq!(mine_again, Err((409, "already-exists")));
    assert_eq!(theirs, Err((409, "already-exists")));
}

#[tokio::test]
async fn a_fourth_document_gets_list_full() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    for name in ["one", "two", "three"] {
        published(&w, &a, &doc(name), &[]).await;
    }

    let refused = publish(&w, &a, &doc("four"), &document_body("craft", "Four", &[])).await;

    assert_eq!(refused, Err((409, "list-full")));
}

#[tokio::test]
async fn a_document_must_fit_its_limits() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");
    let fifty_one: Vec<String> = (0..51u8)
        .map(|i| {
            format!(
                "{}/1",
                doc(&format!(
                    "c{}{}",
                    (b'a' + i / 26) as char,
                    (b'a' + i % 26) as char
                ))
            )
        })
        .collect();
    let fifty_one: Vec<&str> = fifty_one.iter().map(String::as_str).collect();
    let cases = [
        document_body("craft", &"x".repeat(129), &[]),
        document_body("craft", "", &[]),
        document_body("no-such-shelf", "Knots", &[]),
        document_body("craft", "Knots", &["knots/1"]),
        document_body("craft", "Knots", &["aaaaaaaaaaaa/6"]),
        document_body("craft", "Knots", &fifty_one),
        json!({"shelf": "craft", "title": "Knots", "content": ""}),
        json!({"shelf": "craft", "title": "Knots", "content": "x", "extra": 1}),
    ];
    for body in cases {
        assert_eq!(
            publish(&w, &a, &id, &body).await.map_err(|e| e.0),
            Err(400),
            "{body}"
        );
    }
    assert_eq!(
        publish(&w, &a, "Knots", &document_body("craft", "x", &[]))
            .await
            .map_err(|e| e.0),
        Err(400)
    );
    // A whole body over 32 KB is refused before anything is read.
    let big = json!({"shelf": "craft", "title": "Knots", "content": "x".repeat(32 * 1024)});
    assert_eq!(publish(&w, &a, &id, &big).await.map_err(|e| e.0), Err(400));
    assert!(
        !w.store
            .keys(Bucket::Identities)
            .iter()
            .any(|k| k.starts_with("library/"))
    );
}

#[tokio::test]
async fn editions_follow_each_other_up_to_five_and_old_ones_stay_readable() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;

    for n in 2..=5 {
        let body = edition_body(&format!("Knots, edition {n}"), &[]);
        let (status, page) = edition(&w, &a, &id, &n.to_string(), &body).await.unwrap();
        assert_eq!((status, page["n"].clone()), (201, json!(n)));
    }
    let sixth = edition(&w, &a, &id, "6", &edition_body("Six", &[])).await;
    assert_eq!(sixth, Err((409, "list-full")));

    let (_, first) = read_edition(&w, &id, "1").await.unwrap();
    assert_eq!(first["title"], id.as_str());
    assert_eq!(first["newer_edition"], 5);
    let (_, third) = read_edition(&w, &id, "3").await.unwrap();
    assert_eq!(third["title"], "Knots, edition 3");
    let (_, page) = read_document(&w, &id).await.unwrap();
    assert_eq!(page["title"], "Knots, edition 5");
    assert_eq!(page["editions"].as_array().unwrap().len(), 5);
    assert_eq!(read_edition(&w, &id, "9").await, Err((404, "about:blank")));
}

#[tokio::test]
async fn an_edition_needs_the_author_standing_and_the_next_number() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let b = author(&w, "Moss", &[&a]).await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;
    let body = edition_body("Two", &[]);

    assert_eq!(
        edition(&w, &b, &id, "2", &body).await,
        Err((403, "not-the-author"))
    );
    assert_eq!(
        edition(&w, &a, &id, "1", &body).await,
        Err((409, "already-exists"))
    );
    assert_eq!(
        edition(&w, &a, &id, "3", &body).await.map_err(|e| e.0),
        Err(400)
    );
    assert_eq!(
        edition(&w, &a, &doc("none"), "2", &body)
            .await
            .map_err(|e| e.0),
        Err(404)
    );
    // Losing standing stops new editions, and the document stays.
    give_standing(&w, &[&b]).await;
    assert_eq!(
        edition(&w, &a, &id, "2", &body).await,
        Err((403, "no-standing"))
    );
    assert_eq!(read_document(&w, &id).await.unwrap().0, 200);
}

#[tokio::test]
async fn a_retried_edition_returns_it_with_200() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;
    let sent = edition_call(&a, &id, "2", &edition_body("Two", &[]), NOW);

    let (_, first) = send_edition(&w, &sent, &id, "2", NOW).await.unwrap();
    let again = send_edition(&w, &sent, &id, "2", NOW + 1).await.unwrap();

    assert_eq!(again, (200, first));
    let key = format!("library/{id}/2.json");
    assert_eq!(w.store.writes_to(Bucket::Identities, &key), 1);
}

#[tokio::test]
async fn an_edition_whose_body_is_over_32_kb_gets_400() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;
    let big = json!({"title": "Two", "content": "x".repeat(32 * 1024)});

    let refused = edition(&w, &a, &id, "2", &big).await;

    assert_eq!(refused.map_err(|e| e.0), Err(400));
    assert!(
        w.store
            .object(Bucket::Identities, &format!("library/{id}/2.json"))
            .is_none()
    );
}

#[tokio::test]
async fn an_unknown_document_is_404_and_a_malformed_one_400() {
    let w = world();
    assert_eq!(
        read_document(&w, &doc("none")).await.map_err(|e| e.0),
        Err(404)
    );
    assert_eq!(read_document(&w, "Nope").await.map_err(|e| e.0), Err(400));
    assert_eq!(
        read_edition(&w, &doc("none"), "1").await.map_err(|e| e.0),
        Err(404)
    );
    assert_eq!(
        read_edition(&w, &doc("none"), "0").await.map_err(|e| e.0),
        Err(400)
    );
}
