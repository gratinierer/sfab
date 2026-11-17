//! Sitting exams and marking them: `.stories/4-life-in-the-world/spec.md` "The Academy"
//! ("Exams", "Who marks"), against the in-memory fake.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use serde_json::json;
use sfab_academy::Academy;
use sfab_core::storage::{
    Bucket, Condition, Listed, MemoryStore, Metadata, Object, Store, StoreError,
};
use sfab_genesis::testing as genesis;
use sfab_identity::Identity;

#[tokio::test]
async fn an_attempt_needs_an_enrolment() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 2, 0, 50)).await;

    assert_eq!(
        start(&w, &s, &course, NOW).await,
        Err((403, "not-enrolled"))
    );
    let none = format!("academy/courses/{course}/attempts/");
    assert!(
        w.store
            .keys(Bucket::Identities)
            .iter()
            .all(|k| !k.starts_with(&none))
    );
}

#[tokio::test]
async fn an_open_attempt_is_returned_and_a_retake_within_7_days_is_too_soon() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 2, 0, 100)).await;
    enrol(&w, &s, &course, NOW).await.unwrap();

    let attempt = started(&w, &s, &course, NOW).await;
    assert_eq!(attempt, format!("{}-{TODAY}", s.id()));
    let (status, open) = start(&w, &s, &course, NOW + 60).await.unwrap();
    assert_eq!((status, open["attempt"].clone()), (200, json!(attempt)));
    submit(&w, &s, &course, &attempt, NOW).await.unwrap();

    let refused = w
        .academy
        .start(
            &start_call(&s, &course, NOW + 6 * DAY).call(),
            &course,
            NOW + 6 * DAY,
        )
        .await
        .unwrap_err();
    assert_eq!((refused.status, refused.kind), (409, "too-soon"));
    assert_eq!(refused.extra["retry_after"], "2026-11-12T00:00:00Z");
    let (status, again) = start(&w, &s, &course, NOW + 7 * DAY).await.unwrap();
    assert_eq!(
        (status, again["attempt"].clone()),
        (201, json!(format!("{}-2026-11-12", s.id())))
    );
}

#[tokio::test]
async fn the_set_is_fixed_by_the_student_s_id_and_an_edit_during_an_attempt_leaves_it() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let body = json!({"title": "Pool", "pass": 50, "set_size": 5, "tasks":
        (1..=30).map(|i| json!({"kind": "check", "text": format!("Task {i}"), "answer": format!("a{i}")}))
            .collect::<Vec<_>>()});
    let course = published(&w, &t, &body).await;
    enrol(&w, &s, &course, NOW).await.unwrap();
    let attempt = started(&w, &s, &course, NOW).await;
    let (_, before) = read_attempt(&w, &s, &course, &attempt).await.unwrap();
    assert_eq!(before["tasks"].as_array().unwrap().len(), 5);

    edit(&w, &t, &course, &course_body("Other", 3, 0, 50), NOW + 10)
        .await
        .unwrap();

    let (_, after) = read_attempt(&w, &s, &course, &attempt).await.unwrap();
    assert_eq!(after["tasks"], before["tasks"]);
    // A retake starts from the pool as it now stands.
    answer_all(&w, &s, &course, &attempt).await;
    submit(&w, &s, &course, &attempt, NOW).await.unwrap();
    let (_, retake) = start(&w, &s, &course, NOW + 7 * DAY).await.unwrap();
    let texts: Vec<_> = retake["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["text"].clone())
        .collect();
    assert_eq!(
        texts,
        vec![json!("Task 1"), json!("Task 2"), json!("Task 3")]
    );
}

#[tokio::test]
async fn an_attempt_never_shows_its_answer_hashes_and_only_its_student_reads_it() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 2, 1, 50)).await;
    enrol(&w, &s, &course, NOW).await.unwrap();
    let attempt = started(&w, &s, &course, NOW).await;
    answer_all(&w, &s, &course, &attempt).await;

    let (status, read) = read_attempt(&w, &s, &course, &attempt).await.unwrap();

    assert_eq!(status, 200);
    let text = read.to_string();
    assert!(!text.contains("hash"), "{read}");
    // The task's answer hash and the student's own hashed answer alike.
    let stored = w
        .store
        .object(
            Bucket::Identities,
            &format!("academy/courses/{course}/attempts/{attempt}.json"),
        )
        .unwrap()
        .body;
    let stored: serde_json::Value = serde_json::from_slice(&stored).unwrap();
    let hash = stored["tasks"][0]["hash"].as_str().unwrap();
    assert!(!text.contains(hash));
    assert!(
        !text.contains("\"a1\""),
        "an answer check's answer is stored only as its hash"
    );
    assert_eq!(
        read_attempt(&w, &t, &course, &attempt).await,
        Err((403, "not-a-party"))
    );
}

#[tokio::test]
async fn an_answer_after_submission_gets_submitted_and_a_retried_submission_returns_the_first() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 2, 0, 50)).await;
    enrol(&w, &s, &course, NOW).await.unwrap();
    let attempt = started(&w, &s, &course, NOW).await;
    answer_all(&w, &s, &course, &attempt).await;

    let (status, first) = submit(&w, &s, &course, &attempt, NOW).await.unwrap();
    assert_eq!(
        (status, first["status"].clone(), first["score"].clone()),
        (200, json!("passed"), json!(100))
    );
    let again = submit(&w, &s, &course, &attempt, NOW + 30).await.unwrap();
    assert_eq!(again, (200, first));

    assert_eq!(
        hand_in(&w, &s, &course, &attempt, "1", "changed").await,
        Err((409, "submitted"))
    );
    let certificate = format!("academy/certificates/exam-{course}-{}.json", s.id());
    assert!(w.store.object(Bucket::Identities, &certificate).is_some());
}

#[tokio::test]
async fn a_retried_answer_with_the_same_proof_is_returned() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 0, 1, 50)).await;
    enrol(&w, &s, &course, NOW).await.unwrap();
    let attempt = started(&w, &s, &course, NOW).await;
    let sent = answer_call(&s, &course, &attempt, "1", "work", NOW);
    let first = send_answer(&w, &sent, &course, &attempt, "1", NOW)
        .await
        .unwrap();
    let key = format!("academy/courses/{course}/attempts/{attempt}.json");
    let writes = w.store.writes_to(Bucket::Identities, &key);

    assert_eq!(
        send_answer(&w, &sent, &course, &attempt, "1", NOW)
            .await
            .unwrap(),
        first
    );
    assert_eq!(w.store.writes_to(Bucket::Identities, &key), writes);
    assert_eq!(
        hand_in(&w, &s, &course, &attempt, "2", "x").await,
        Err((404, "about:blank"))
    );
    let big = "x".repeat(8 * 1024 + 1);
    assert_eq!(
        hand_in(&w, &s, &course, &attempt, "1", &big).await,
        Err((400, "about:blank"))
    );
    let t_answers = hand_in(&w, &t, &course, &attempt, "1", "x").await;
    assert_eq!(t_answers, Err((403, "not-a-party")));
}

/// A store that, on the first `If-Match` write to an attempt, first lets another answer through
/// the same attempt, as a second request racing the first would.
struct Racing {
    inner: Arc<MemoryStore>,
    other: Academy<MemoryStore>,
    pending: Mutex<Option<(Sent, String, String)>>,
}

impl Store for Racing {
    async fn get(&self, b: Bucket, k: &str) -> Result<Option<Object>, StoreError> {
        self.inner.get(b, k).await
    }
    async fn put(
        &self,
        b: Bucket,
        k: &str,
        body: Vec<u8>,
        m: Metadata,
        c: Condition,
    ) -> Result<(), StoreError> {
        let racing = match c {
            Condition::IfMatch(_) if k.contains("/attempts/") => {
                self.pending.lock().unwrap().take()
            }
            _ => None,
        };
        if let Some((sent, course, attempt)) = racing {
            self.other
                .answer(&sent.call(), &course, &attempt, "2", NOW)
                .await
                .expect("the racing answer");
        }
        self.inner.put(b, k, body, m, c).await
    }
    async fn delete(&self, b: Bucket, k: &str) -> Result<(), StoreError> {
        self.inner.delete(b, k).await
    }
    async fn list(&self, b: Bucket, p: &str) -> Result<Vec<String>, StoreError> {
        self.inner.list(b, p).await
    }
    async fn list_sized(&self, b: Bucket, p: &str) -> Result<Vec<Listed>, StoreError> {
        self.inner.list_sized(b, p).await
    }
}

#[tokio::test]
async fn two_answers_handed_in_at_once_are_both_kept() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Knots", 0, 2, 50)).await;
    enrol(&w, &s, &course, NOW).await.unwrap();
    let attempt = started(&w, &s, &course, NOW).await;
    let second = answer_call(&s, &course, &attempt, "2", "second", NOW);
    let racing = Arc::new(Racing {
        inner: w.store.clone(),
        other: Academy::new(w.store.clone(), w.identity.clone(), BITS),
        pending: Mutex::new(Some((second, course.clone(), attempt.clone()))),
    });
    let identity = Arc::new(Identity::new(
        racing.clone(),
        BITS,
        genesis::genesis(&racing),
    ));
    let academy = Academy::new(racing.clone(), identity, BITS);

    let first = answer_call(&s, &course, &attempt, "1", "first", NOW);
    academy
        .answer(&first.call(), &course, &attempt, "1", NOW)
        .await
        .unwrap();

    let (_, read) = read_attempt(&w, &s, &course, &attempt).await.unwrap();
    assert_eq!(read["tasks"][0]["answer"], "first");
    assert_eq!(read["tasks"][1]["answer"], "second");
}

#[tokio::test]
async fn handed_in_work_is_marked_by_the_teacher_or_a_graduate_and_a_pass_is_a_certificate() {
    let w = world();
    let t = born(&w, "Wren").await;
    let grad = born(&w, "Moss").await;
    let s = born(&w, "Kite").await;
    let stranger = born(&w, "Lark").await;
    let checks = published(&w, &t, &course_body("Checks", 1, 0, 100)).await;
    let course = published(&w, &t, &course_body("Work", 1, 1, 100)).await;
    // Moss graduates from the work course, marked by the teacher.
    enrol(&w, &grad, &course, NOW).await.unwrap();
    let first = started(&w, &grad, &course, NOW).await;
    answer_all(&w, &grad, &course, &first).await;
    let (_, waiting) = submit(&w, &grad, &course, &first, NOW).await.unwrap();
    assert_eq!(waiting["status"], "awaiting-mark");
    let (_, queue_page) = queue(&w, &t, &course, None).await.unwrap();
    assert_eq!(
        queue_page["items"],
        json!([{"attempt": first, "date": TODAY}])
    );
    let (_, work) = marking(&w, &t, &course, &first).await.unwrap();
    assert_eq!(work["tasks"][1]["answer"], "My work.");
    assert_eq!(work["tasks"][0]["correct"], true);
    assert!(!work.to_string().contains("hash"));
    let (status, marked) = mark(
        &w,
        &t,
        &course,
        &first,
        &json!({"accepted": [2], "comment": "Good."}),
    )
    .await
    .unwrap();
    assert_eq!((status, marked["status"].clone()), (200, json!("passed")));
    assert_eq!(
        queue(&w, &t, &course, None).await.unwrap().1["items"],
        json!([])
    );

    // Now Kite's attempt: Moss may mark it, a stranger and a graduate of another course not.
    enrol(&w, &s, &course, NOW).await.unwrap();
    let attempt = started(&w, &s, &course, NOW).await;
    answer_all(&w, &s, &course, &attempt).await;
    submit(&w, &s, &course, &attempt, NOW).await.unwrap();
    assert_eq!(
        queue(&w, &stranger, &course, None).await,
        Err((403, "not-a-graduate"))
    );
    graduate(&w, &stranger, &checks).await;
    assert_eq!(
        marking(&w, &stranger, &course, &attempt).await,
        Err((403, "not-a-graduate"))
    );
    let refused = mark(&w, &stranger, &course, &attempt, &json!({"accepted": [2]})).await;
    assert_eq!(refused, Err((403, "not-a-graduate")));
    assert_eq!(
        mark(&w, &grad, &course, &attempt, &json!({"accepted": [1]})).await,
        Err((400, "about:blank")),
        "an answer check can't be accepted"
    );
    let (_, failed) = mark(&w, &grad, &course, &attempt, &json!({"accepted": []}))
        .await
        .unwrap();
    assert_eq!(
        (failed["status"].clone(), failed["score"].clone()),
        (json!("failed"), json!(50))
    );
    let key = format!("academy/certificates/exam-{course}-{}.json", s.id());
    assert!(w.store.object(Bucket::Identities, &key).is_none());
    // A second mark returns the first.
    let (_, again) = mark(&w, &grad, &course, &attempt, &json!({"accepted": [2]}))
        .await
        .unwrap();
    assert_eq!(again["status"], "failed");

    let (_, cert) = certificate(&w, &format!("exam-{course}-{}", grad.id()))
        .await
        .unwrap();
    assert_eq!(cert["teacher"], json!({"id": t.id(), "removed": false}));
    assert_eq!(cert["marker"], json!({"id": t.id(), "removed": false}));
    let (_, read) = read_attempt(&w, &grad, &course, &first).await.unwrap();
    assert_eq!(read["mark"]["comment"], "Good.");
}

#[tokio::test]
async fn no_one_marks_its_own_attempt() {
    let w = world();
    let t = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let course = published(&w, &t, &course_body("Work", 1, 1, 100)).await;
    graduate_by_mark(&w, &t, &s, &course).await;
    // A retake, marked by itself as a graduate.
    let attempt = started(&w, &s, &course, NOW + 7 * DAY).await;
    answer_all(&w, &s, &course, &attempt).await;
    submit(&w, &s, &course, &attempt, NOW + 7 * DAY)
        .await
        .unwrap();

    let refused = mark(&w, &s, &course, &attempt, &json!({"accepted": [2]})).await;

    assert_eq!(refused, Err((403, "not-a-graduate")));
}

/// `student` sits the work `course` and the teacher passes it.
async fn graduate_by_mark(w: &World, teacher: &Agent, student: &Agent, course: &str) {
    enrol(w, student, course, NOW).await.unwrap();
    let attempt = started(w, student, course, NOW).await;
    answer_all(w, student, course, &attempt).await;
    submit(w, student, course, &attempt, NOW).await.unwrap();
    let (_, marked) = mark(w, teacher, course, &attempt, &json!({"accepted": [2]}))
        .await
        .unwrap();
    assert_eq!(marked["status"], "passed");
}
