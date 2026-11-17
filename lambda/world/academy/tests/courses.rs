//! Publishing, editing, retiring and reading courses, and enrolling: `.stories/4-life-in-the-world/
//! spec.md` "The Academy", against the in-memory fake.

mod common;

use common::*;
use serde_json::json;
use sfab_core::storage::Bucket;

#[tokio::test]
async fn anyone_publishes_a_course_whose_id_comes_from_the_proof() {
    let w = world();
    let t = born(&w, "Wren").await;

    let (status, course) = publish(&w, &t, &course_body("Knots", 2, 1, 60), NOW)
        .await
        .unwrap();

    assert_eq!(status, 201);
    let id = course["course"].as_str().unwrap();
    assert_eq!(id.len(), 12);
    assert_eq!(course["teacher"], t.id());
    assert_eq!(course["title"], "Knots");
    assert_eq!(course["set_size"], 3);
    assert_eq!(course["retired"], false);
    let key = format!("academy/courses/{id}/course.json");
    assert!(w.store.object(Bucket::Identities, &key).is_some());
    // Answers are stored only as hashes.
    let stored = w.store.object(Bucket::Identities, &key).unwrap().body;
    assert!(!String::from_utf8(stored).unwrap().contains("\"a1\""));
}

#[tokio::test]
async fn the_same_proof_again_returns_the_course_with_200() {
    let w = world();
    let t = born(&w, "Wren").await;
    let sent = publish_call(&t, &course_body("Knots", 1, 0, 100), NOW);

    let (_, first) = send_publish(&w, &sent, NOW).await.unwrap();
    let again = send_publish(&w, &sent, NOW + 1).await.unwrap();

    assert_eq!(again, (200, first));
    let counter = format!("identities/{}/academy.json", t.id());
    assert_eq!(w.store.writes_to(Bucket::Identities, &counter), 1);
}

#[tokio::test]
async fn a_sixth_course_gets_list_full() {
    let w = world();
    let t = born(&w, "Wren").await;
    for i in 0..5 {
        let body = course_body(&format!("Course {i}"), 1, 0, 100);
        assert_eq!(publish(&w, &t, &body, NOW + i).await.unwrap().0, 201);
    }

    let refused = publish(&w, &t, &course_body("Sixth", 1, 0, 100), NOW + 9).await;

    assert_eq!(refused, Err((409, "list-full")));
}

#[tokio::test]
async fn a_course_must_fit_its_limits() {
    let w = world();
    let t = born(&w, "Wren").await;
    let long_title =
        json!({"title": "x".repeat(129), "pass": 50, "tasks": [{"kind": "work", "text": "x"}]});
    let long_task =
        json!({"title": "x", "pass": 50, "tasks": [{"kind": "work", "text": "x".repeat(301)}]});
    let no_tasks = json!({"title": "x", "pass": 50, "tasks": []});
    let pool: Vec<_> = (0..51)
        .map(|_| json!({"kind": "work", "text": "x"}))
        .collect();
    let big_pool = json!({"title": "x", "pass": 50, "tasks": pool});
    let check_without_answer =
        json!({"title": "x", "pass": 50, "tasks": [{"kind": "check", "text": "x"}]});
    let work_with_answer =
        json!({"title": "x", "pass": 50, "tasks": [{"kind": "work", "text": "x", "answer": "y"}]});
    let no_pass = json!({"title": "x", "pass": 0, "tasks": [{"kind": "work", "text": "x"}]});
    let big_set =
        json!({"title": "x", "pass": 50, "set_size": 2, "tasks": [{"kind": "work", "text": "x"}]});
    let long_prep = json!({"title": "x", "pass": 50, "preparation": "x".repeat(16 * 1024 + 1), "tasks": [{"kind": "work", "text": "x"}]});
    for (i, body) in [
        long_title,
        long_task,
        no_tasks,
        big_pool,
        check_without_answer,
        work_with_answer,
        no_pass,
        big_set,
        long_prep,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            publish(&w, &t, body, NOW).await,
            Err((400, "about:blank")),
            "{i}"
        );
    }
}

#[tokio::test]
async fn the_public_page_never_shows_tasks_or_answer_hashes() {
    let w = world();
    let t = born(&w, "Wren").await;
    let course = published(&w, &t, &course_body("Knots", 2, 1, 60)).await;

    let (status, page) = answer(w.academy.course(&course).await).unwrap();

    assert_eq!(status, 200);
    assert_eq!(page["title"], "Knots");
    assert_eq!(page["preparation"], "Read this first.");
    let text = page.to_string();
    assert!(page.get("tasks").is_none(), "{page}");
    assert!(!text.contains("Task 1") && !text.contains("hash"), "{page}");
    // Before the daily job's first run its counts are unknown.
    assert_eq!(page["enrolled"], json!(null));
    assert_eq!(
        answer(w.academy.course("aaaaaaaaaaaa").await),
        Err((404, "about:blank"))
    );
    assert_eq!(
        answer(w.academy.course("../x").await),
        Err((400, "about:blank"))
    );
}

#[tokio::test]
async fn only_the_teacher_reads_the_task_pool_with_its_hashes() {
    let w = world();
    let t = born(&w, "Wren").await;
    let other = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 21, 2, 60)).await;

    let auth = t.auth();
    let (status, first) =
        answer(w.academy.tasks(auth.as_deref(), &course, None, NOW).await).unwrap();
    assert_eq!(status, 200);
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    assert_eq!(first["items"][0]["n"], 1);
    assert_eq!(first["items"][0]["kind"], "check");
    assert_eq!(first["items"][0]["answer_hash"].as_str().unwrap().len(), 64);
    let next = first["next"].as_str().unwrap();
    let token = next.split("?page=").nth(1).unwrap();
    let (_, last) = answer(
        w.academy
            .tasks(auth.as_deref(), &course, Some(token), NOW)
            .await,
    )
    .unwrap();
    assert_eq!(last["items"].as_array().unwrap().len(), 3);
    assert_eq!(last["items"][2]["answer_hash"], json!(null));

    let theirs = other.auth();
    let refused = w.academy.tasks(theirs.as_deref(), &course, None, NOW).await;
    assert_eq!(answer(refused), Err((403, "not-the-author")));
}

#[tokio::test]
async fn the_teacher_edits_and_retires_its_course_and_no_one_else_can() {
    let w = world();
    let t = born(&w, "Wren").await;
    let other = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 1, 0, 100)).await;

    let (status, edited) = edit(
        &w,
        &t,
        &course,
        &course_body("Better knots", 2, 0, 50),
        NOW + 5,
    )
    .await
    .unwrap();
    assert_eq!(
        (status, edited["title"].clone()),
        (200, json!("Better knots"))
    );
    assert_eq!(
        edit(&w, &other, &course, &course_body("Mine", 1, 0, 50), NOW).await,
        Err((403, "not-the-author"))
    );
    assert_eq!(
        retire(&w, &other, &course, NOW).await,
        Err((403, "not-the-author"))
    );

    let (status, retired) = retire(&w, &t, &course, NOW + 6).await.unwrap();
    assert_eq!((status, retired["retired"].clone()), (200, json!(true)));
    // Again is safe, and it stays readable.
    assert_eq!(retire(&w, &t, &course, NOW + 7).await.unwrap().0, 200);
    assert_eq!(answer(w.academy.course(&course).await).unwrap().0, 200);
}

#[tokio::test]
async fn enrolling_is_safe_to_retry_and_a_retired_course_takes_no_new_students() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let late = born(&w, "Kite").await;
    let course = published(&w, &t, &course_body("Knots", 1, 0, 100)).await;

    assert_eq!(enrol(&w, &s, &course, NOW).await.unwrap().0, 201);
    assert_eq!(enrol(&w, &s, &course, NOW + 1).await.unwrap().0, 200);
    retire(&w, &t, &course, NOW + 2).await.unwrap();

    assert_eq!(
        enrol(&w, &late, &course, NOW + 3).await,
        Err((409, "retired"))
    );
    // Already enrolled, it stays enrolled.
    assert_eq!(enrol(&w, &s, &course, NOW + 4).await.unwrap().0, 200);
    assert_eq!(
        enrol(&w, &s, "aaaaaaaaaaaa", NOW).await,
        Err((404, "about:blank"))
    );

    let (status, left) = leave(&w, &s, &course, NOW).await.unwrap();
    assert_eq!((status, left["course"].clone()), (200, json!(course)));
    let key = format!("academy/courses/{course}/enrolled/{}.json", s.id());
    assert!(w.store.object(Bucket::Identities, &key).is_none());
    assert_eq!(leave(&w, &s, &course, NOW).await.unwrap().0, 200);
}
