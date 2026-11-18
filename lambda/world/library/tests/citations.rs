//! Citations, `cited-by` and shelves, built by the daily job: `.stories/4-life-in-the-world/
//! spec.md` "The Library" ("Citations", "Shelves") and "Foundations".

mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn before_the_first_run_shelves_and_cited_by_are_not_built_yet() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let id = doc("knots");
    published(&w, &a, &id, &[]).await;

    let shelf = w.library.shelf("craft", None).await.map_err(|p| p.kind);
    let cited = w.library.cited_by(&id, None).await.map_err(|p| p.kind);

    assert_eq!(shelf.err(), Some("not-built-yet"));
    assert_eq!(cited.err(), Some("not-built-yet"));
    assert_eq!(
        read_document(&w, &id).await.unwrap().1["cited_by"],
        json!(null)
    );
}

#[tokio::test]
async fn self_citations_don_t_count_and_others_count_once_per_document() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let b = author(&w, "Moss", &[&a]).await;
    let (target, own, other) = (doc("target"), doc("own"), doc("other"));
    published(&w, &a, &target, &[]).await;
    let t1 = format!("{target}/1");
    published(&w, &a, &own, &[&t1]).await;
    published(&w, &b, &other, &[&t1]).await;
    // A second edition citing it again, and an edition that doesn't exist, count nothing more.
    let t2 = format!("{target}/2");
    edition(&w, &b, &other, "2", &edition_body("Again", &[&t1, &t2]))
        .await
        .unwrap();

    let report = daily(&w, NOW + DAY).await;

    assert_eq!(report.citations, 1);
    let (_, cited) = answer(w.library.cited_by(&target, None).await).unwrap();
    assert_eq!(cited, json!({"items": [other.as_str()], "next": null}));
    assert_eq!(read_document(&w, &target).await.unwrap().1["cited_by"], 1);
    // An uncited document reads 0 once the job has run, and an unknown one an empty page.
    assert_eq!(read_document(&w, &own).await.unwrap().1["cited_by"], 0);
    let (_, none) = answer(w.library.cited_by(&doc("none"), None).await).unwrap();
    assert_eq!(none, json!({"items": [], "next": null}));
}

#[tokio::test]
async fn a_shelf_lists_its_documents_most_cited_first() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let b = author(&w, "Moss", &[&a]).await;
    let (quiet, famous, cite) = (doc("quiet"), doc("famous"), doc("cite"));
    published(&w, &a, &quiet, &[]).await;
    published(&w, &a, &famous, &[]).await;
    let f1 = format!("{famous}/1");
    let body = document_body("science", "Citing", &[&f1]);
    publish(&w, &b, &cite, &body).await.unwrap();

    daily(&w, NOW + DAY).await;

    let (_, craft) = answer(w.library.shelf("craft", None).await).unwrap();
    let order: Vec<&str> = craft["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["document"].as_str().unwrap())
        .collect();
    assert_eq!(order, [famous.as_str(), quiet.as_str()]);
    assert_eq!(craft["items"][0]["cited_by"], 1);
    let (_, science) = answer(w.library.shelf("science", None).await).unwrap();
    assert_eq!(science["items"][0]["document"], cite.as_str());
    // An empty shelf is one empty page; an unknown shelf is 404.
    let (_, empty) = answer(w.library.shelf("history", None).await).unwrap();
    assert_eq!(empty, json!({"items": [], "next": null}));
    assert_eq!(
        w.library.shelf("nowhere", None).await.unwrap_err().status,
        404
    );
}

#[tokio::test]
async fn a_withdrawn_citing_edition_stops_counting_at_the_next_run() {
    let w = world();
    let a = author(&w, "Wren", &[]).await;
    let b = author(&w, "Moss", &[&a]).await;
    let (target, other) = (doc("target"), doc("other"));
    published(&w, &a, &target, &[]).await;
    let t1 = format!("{target}/1");
    published(&w, &b, &other, &[&t1]).await;
    daily(&w, NOW + DAY).await;
    assert_eq!(read_document(&w, &target).await.unwrap().1["cited_by"], 1);

    withdraw(&w, &b, &other, "1").await.unwrap();
    daily(&w, NOW + 2 * DAY).await;

    assert_eq!(read_document(&w, &target).await.unwrap().1["cited_by"], 0);
    // A document with no title left leaves its shelf.
    let (_, craft) = answer(w.library.shelf("craft", None).await).unwrap();
    assert_eq!(craft["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn the_shelves_are_the_compiled_in_list() {
    let w = world();
    let (_, page) = answer(w.library.shelves(None)).unwrap();
    assert_eq!(page["items"][0], json!({"shelf": "agents"}));
    assert_eq!(
        page["items"].as_array().unwrap().len(),
        sfab_library::shelves().len()
    );
    assert_eq!(w.library.shelves(Some("x")).unwrap_err().status, 400);
}
