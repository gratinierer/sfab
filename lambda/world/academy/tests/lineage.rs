//! Certificates, and what the daily job builds from them: `.stories/4-life-in-the-world/spec.md`
//! "The Academy" ("Who marks", "Lineage", "What's public", "When things go away") and
//! "Foundations" ("Before the first daily run", "Unknown IDs").

mod common;

use common::*;
use serde_json::json;
use sfab_core::storage::{Bucket, Condition, Metadata, Store};
use sfab_core::takedown::Action;

/// `agent` gets a takedown marker, as the takedown program's first step leaves.
async fn mark_taken_down(w: &World, agent: &Agent) {
    for l in sfab_identity::takedown(&agent.id) {
        if let Action::Marker(_) = l.action {
            let put = w.store.put(
                l.bucket,
                &l.key,
                Vec::new(),
                Metadata::new(),
                Condition::Always,
            );
            put.await.unwrap();
        }
    }
}

#[tokio::test]
async fn the_teacher_revokes_a_certificate_and_a_graduate_who_marked_it_cannot() {
    let w = world();
    let t = born(&w, "Wren").await;
    let grad = born(&w, "Moss").await;
    let s = born(&w, "Kite").await;
    let course = published(&w, &t, &course_body("Work", 0, 1, 100)).await;
    for (student, marker) in [(&grad, &t), (&s, &grad)] {
        enrol(&w, student, &course, NOW).await.unwrap();
        let attempt = started(&w, student, &course, NOW).await;
        answer_all(&w, student, &course, &attempt).await;
        submit(&w, student, &course, &attempt, NOW).await.unwrap();
        mark(&w, marker, &course, &attempt, &json!({"accepted": [1]}))
            .await
            .unwrap();
    }
    let id = format!("exam-{course}-{}", s.id());

    assert_eq!(revoke(&w, &grad, &id).await, Err((403, "not-the-author")));
    let (status, revoked) = revoke(&w, &t, &id).await.unwrap();
    assert_eq!((status, revoked["revoked"].clone()), (200, json!(true)));
    assert_eq!(revoke(&w, &t, &id).await.unwrap().0, 200);

    let (_, shown) = certificate(&w, &id).await.unwrap();
    assert_eq!(shown["revoked"], true);
    assert_eq!(shown["kind"], "exam");
    assert_eq!(shown["marker"]["id"], grad.id());
    assert_eq!(certificate(&w, "exam-x").await, Err((400, "about:blank")));
    let unknown = format!("exam-{course}-aaaaaaaaaaaa");
    assert_eq!(certificate(&w, &unknown).await, Err((404, "about:blank")));
}

#[tokio::test]
async fn a_certificate_whose_teacher_was_taken_down_shows_it_removed() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 1, 0, 100)).await;
    let id = graduate(&w, &s, &course).await;

    mark_taken_down(&w, &t).await;

    let (_, shown) = certificate(&w, &id).await.unwrap();
    assert_eq!(shown["teacher"], json!({"id": t.id(), "removed": true}));
    assert_eq!(shown["marker"], json!(null));
}

#[tokio::test]
async fn daily_routes_are_not_built_yet_before_the_first_run_and_then_empty_for_an_unknown_id() {
    let w = world();
    let course = "aaaaaaaaaaaa";
    let id = "bbbbbbbbbbbb";
    let reads = || async {
        vec![
            answer(w.academy.catalogue(None).await),
            answer(w.academy.enrolled(course, None).await),
            answer(w.academy.graduates(course, None).await),
            answer(w.academy.teacher(id).await),
            answer(w.academy.teacher_graduates(id, None).await),
            answer(w.academy.enrolments(id, None).await),
            answer(w.academy.certificates(id, None).await),
        ]
    };
    for read in reads().await {
        assert_eq!(read, Err((404, "not-built-yet")));
    }

    daily(&w, &[], NOW + DAY).await;

    let empty = json!({"items": [], "next": null});
    for (i, read) in reads().await.into_iter().enumerate() {
        let (status, body) = read.unwrap();
        assert_eq!(status, 200, "{i}");
        if i == 3 {
            assert_eq!(body["courses"], json!([]));
            assert_eq!(body["graduates"], 0);
        } else {
            assert_eq!(body, empty, "{i}");
        }
    }
}

#[tokio::test]
async fn the_catalogue_puts_teachers_with_standing_first_by_graduates_then_the_rest_newest_first() {
    let w = world();
    let founder = born(&w, "Ash").await;
    let other_founder = born(&w, "Birch").await;
    let nobody = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let few = published(&w, &founder, &course_body("Few", 1, 0, 100)).await;
    let many = published(&w, &other_founder, &course_body("Many", 1, 0, 100)).await;
    let older = published(&w, &nobody, &course_body("Older", 1, 0, 100)).await;
    let (_, newer) = publish(&w, &nobody, &course_body("Newer", 1, 0, 100), NOW + 60)
        .await
        .unwrap();
    let newer = newer["course"].as_str().unwrap().to_owned();
    let (_, gone) = publish(&w, &nobody, &course_body("Retired", 1, 0, 100), NOW + 61)
        .await
        .unwrap();
    let gone = gone["course"].as_str().unwrap().to_owned();
    retire(&w, &nobody, &gone, NOW + 62).await.unwrap();
    for c in [&many, &older, &newer] {
        graduate(&w, &s, c).await;
    }

    daily(&w, &[&founder, &other_founder], NOW + DAY).await;

    let (_, page) = answer(w.academy.catalogue(None).await).unwrap();
    let order: Vec<_> = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["course"].clone())
        .collect();
    assert_eq!(
        order,
        vec![json!(many), json!(few), json!(newer), json!(older)]
    );
    assert_eq!(page["items"][0]["teacher_standing"], true);
    assert_eq!(page["items"][0]["graduates"], 1);
    assert_eq!(page["items"][0]["enrolled"], 1);
}

#[tokio::test]
async fn the_job_builds_the_lists_teacher_pages_and_which_certificates_count() {
    let w = world();
    let founder = born(&w, "Ash").await;
    let nobody = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let taken = born(&w, "Kite").await;
    let course = published(&w, &founder, &course_body("Knots", 1, 0, 100)).await;
    let unvouched = published(&w, &nobody, &course_body("Ropes", 1, 0, 100)).await;
    let by_founder = graduate(&w, &s, &course).await;
    let by_nobody = graduate(&w, &s, &unvouched).await;
    // Moss goes on to teach.
    published(&w, &s, &course_body("Splices", 1, 0, 100)).await;
    graduate(&w, &taken, &course).await;
    mark_taken_down(&w, &taken).await;

    let report = daily(&w, &[&founder, &s], NOW + DAY).await;
    assert_eq!(
        (report.courses, report.certificates, report.counting),
        (3, 2, 1)
    );

    let (_, enrolled) = answer(w.academy.enrolled(&course, None).await).unwrap();
    assert_eq!(
        enrolled["items"],
        json!([s.id()]),
        "a taken-down student leaves the lists"
    );
    let (_, graduates) = answer(w.academy.graduates(&course, None).await).unwrap();
    assert_eq!(graduates["items"], json!([s.id()]));
    let (_, teacher) = answer(w.academy.teacher(founder.id()).await).unwrap();
    assert_eq!(teacher["graduates"], 1);
    assert_eq!(teacher["graduates_with_standing"], 1);
    assert_eq!(teacher["graduates_teaching"], 1);
    assert_eq!(teacher["courses"][0]["course"], json!(course));
    assert!(!teacher.to_string().contains("score"));
    let (_, theirs) = answer(w.academy.teacher_graduates(founder.id(), None).await).unwrap();
    assert_eq!(theirs["items"], json!([s.id()]));
    let (_, mine) = answer(w.academy.enrolments(s.id(), None).await).unwrap();
    assert_eq!(mine["items"].as_array().unwrap().len(), 2);
    let (_, held) = answer(w.academy.certificates(s.id(), None).await).unwrap();
    let mut expected = vec![by_founder.clone(), by_nobody.clone()];
    expected.sort();
    assert_eq!(held["items"], json!(expected));

    let (_, counted) = certificate(&w, &by_founder).await.unwrap();
    assert_eq!(
        (counted["counts"].clone(), counted["as_of"].clone()),
        (json!(true), json!("2026-11-06T12:00:00Z"))
    );
    let (_, waiting) = certificate(&w, &by_nobody).await.unwrap();
    assert_eq!(waiting["counts"], false);
    // A teacher who gains standing makes its earlier certificates count at the next run.
    daily(&w, &[&founder, &s, &nobody], NOW + 2 * DAY).await;
    assert_eq!(certificate(&w, &by_nobody).await.unwrap().1["counts"], true);

    let (_, page) = answer(w.academy.course(&course).await).unwrap();
    assert_eq!(
        (page["enrolled"].clone(), page["graduates"].clone()),
        (json!(1), json!(1))
    );
    assert_eq!(page["lineage"]["graduates_teaching"], 1);
}

#[tokio::test]
async fn the_job_leaves_the_takedown_index_beside_the_agents_pages() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 1, 0, 100)).await;
    enrol(&w, &s, &course, NOW).await.unwrap();
    let index = format!("world/agents/{}.json", s.id());
    let put = w.store.put(
        Bucket::Identities,
        &index,
        b"{}".to_vec(),
        Metadata::new(),
        Condition::Always,
    );
    put.await.unwrap();
    daily(&w, &[], NOW + DAY).await;

    leave(&w, &s, &course, NOW + DAY).await.unwrap();
    daily(&w, &[], NOW + 2 * DAY).await;

    assert!(w.store.object(Bucket::Identities, &index).is_some());
    let pages = format!("world/agents/{}/", s.id());
    assert!(
        w.store
            .keys(Bucket::Identities)
            .iter()
            .all(|k| !k.starts_with(&pages))
    );
    let (_, mine) = answer(w.academy.enrolments(s.id(), None).await).unwrap();
    assert_eq!(mine["items"], json!([]));
}
