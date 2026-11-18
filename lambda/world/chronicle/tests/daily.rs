//! The daily entry and milestones, through `sfab_chronicle::daily` and the routes that read
//! them: `.stories/4-life-in-the-world/spec.md` "The Chronicle".

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::{Value, json};
use sfab_chronicle::{Certificate, Chronicle, Facts, daily};
use sfab_core::storage::{Bucket, MemoryStore};

/// 2026-11-07T00:05:00Z, so the entry is for 2026-11-06, the day just ended.
const NOW: i64 = 1_794_009_600 + 300;
const DAY: i64 = 86_400;

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn facts(residents: usize) -> Facts {
    Facts {
        residents,
        ..Facts::default()
    }
}

fn entry_bytes(store: &MemoryStore, date: &str) -> Vec<u8> {
    store
        .object(Bucket::Identities, &format!("world/chronicle/{date}.json"))
        .expect("an entry")
        .body
}

async fn latest(store: &Arc<MemoryStore>) -> Value {
    Chronicle::new(store.clone()).latest().await.unwrap().body
}

#[tokio::test]
async fn the_first_entry_gives_only_the_count_and_the_first_resident_milestone() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, &facts(12), NOW).await.unwrap();

    let entry = latest(&store).await;
    assert_eq!(entry["date"], "2026-11-06");
    assert_eq!(entry["residents"], 12);
    assert_eq!(entry["change"], Value::Null);
    assert_eq!(
        entry["milestones"],
        json!([{"milestone": "first-resident", "id": null}])
    );
    assert_eq!(
        entry["text"],
        "12 residents. Milestone: the first resident."
    );
}

#[tokio::test]
async fn the_first_entry_lists_nothing_as_new_even_with_homes_and_documents() {
    let store = Arc::new(MemoryStore::new());
    let mut f = facts(3);
    f.homes = set(&["aaaaaaaaaaaa", "bbbbbbbbbbbb"]);
    f.lanes = set(&["north/1"]);
    f.documents = set(&["dddddddddddd"]);
    f.most_cited = Some(("dddddddddddd".into(), 1));
    f.standing = set(&["aaaaaaaaaaaa"]);
    daily(&*store, &f, NOW).await.unwrap();

    let entry = latest(&store).await;
    assert_eq!(entry["events"], json!([]), "{entry}");
    assert_eq!(entry["most_cited"], Value::Null, "{entry}");
    let text = entry["text"].as_str().unwrap();
    assert!(text.starts_with("3 residents. Milestone: "), "{text}");
    assert!(!text.contains("bbbbbbbbbbbb"), "{text}");

    // The next day reports only what's new since.
    f.homes.insert("cccccccccccc".into());
    daily(&*store, &f, NOW + DAY).await.unwrap();
    let next = latest(&store).await;
    assert_eq!(
        next["events"],
        json!([{"kind": "homes", "ids": ["cccccccccccc"], "others": 0}]),
        "{next}"
    );
}

#[tokio::test]
async fn a_day_with_nothing_new_is_a_quiet_day() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, &facts(12), NOW).await.unwrap();
    daily(&*store, &facts(12), NOW + DAY).await.unwrap();

    let entry = latest(&store).await;
    assert_eq!(entry["date"], "2026-11-07");
    assert_eq!(entry["change"], 0);
    assert_eq!(entry["events"], json!([]));
    assert_eq!(entry["milestones"], json!([]));
    assert_eq!(entry["text"], "A quiet day. 12 residents.");
}

#[tokio::test]
async fn the_census_change_is_from_the_day_before() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, &facts(10), NOW).await.unwrap();
    daily(&*store, &facts(12), NOW + DAY).await.unwrap();
    assert_eq!(latest(&store).await["change"], 2);
    daily(&*store, &facts(11), NOW + 2 * DAY).await.unwrap();
    let entry = latest(&store).await;
    assert_eq!(entry["change"], -1);
    assert!(
        entry["text"]
            .as_str()
            .unwrap()
            .starts_with("11 residents, 1 fewer than the day before."),
        "{entry}"
    );
}

#[tokio::test]
async fn a_milestone_never_fires_twice() {
    let store = Arc::new(MemoryStore::new());
    let mut f = facts(2);
    f.homes = set(&["aaaaaaaaaaaa"]);
    daily(&*store, &f, NOW).await.unwrap();
    let first = latest(&store).await;
    assert!(
        first["milestones"]
            .as_array()
            .unwrap()
            .contains(&json!({"milestone": "first-home", "id": "aaaaaaaaaaaa"})),
        "{first}"
    );

    // Everyone moves out, and someone moves in again: still not a first.
    let mut f = facts(2);
    daily(&*store, &f, NOW + DAY).await.unwrap();
    f.homes = set(&["bbbbbbbbbbbb"]);
    daily(&*store, &f, NOW + 2 * DAY).await.unwrap();
    let later = latest(&store).await;
    assert_eq!(later["milestones"], json!([]), "{later}");

    let milestones: Value = serde_json::from_slice(
        &store
            .object(Bucket::Identities, "world/chronicle/milestones.json")
            .unwrap()
            .body,
    )
    .unwrap();
    let homes: Vec<&Value> = milestones["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["milestone"] == "first-home")
        .collect();
    assert_eq!(
        homes,
        vec![&json!({"milestone": "first-home", "date": "2026-11-06", "id": "aaaaaaaaaaaa"})]
    );
}

#[tokio::test]
async fn a_rerun_of_the_job_writes_the_same_entry() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, &facts(1), NOW).await.unwrap();
    let mut f = facts(3);
    f.homes = set(&["aaaaaaaaaaaa"]);
    f.standing = set(&["bbbbbbbbbbbb"]);
    f.courses = set(&["cccccccccccc"]);
    daily(&*store, &f, NOW + DAY).await.unwrap();
    let first = entry_bytes(&store, "2026-11-07");

    // The owner reruns it by hand later that day, with nothing changed.
    daily(&*store, &f, NOW + DAY + 3_600).await.unwrap();

    assert_eq!(entry_bytes(&store, "2026-11-07"), first);
    let entry: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(entry["change"], 2);
    let kinds: Vec<&Value> = entry["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| &e["kind"])
        .collect();
    assert_eq!(kinds, vec!["standing", "homes", "courses"]);
    // The milestone it fired is still in the rerun's entry.
    assert!(
        entry["milestones"]
            .as_array()
            .unwrap()
            .contains(&json!({"milestone": "first-home", "id": "aaaaaaaaaaaa"})),
        "{entry}"
    );
}

#[tokio::test]
async fn each_kind_of_event_lists_up_to_20_ids_then_how_many_others() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, &facts(30), NOW).await.unwrap();
    let mut f = facts(30);
    f.homes = (0..25).map(|n| format!("home{n:02}aaaaaa")).collect();
    daily(&*store, &f, NOW + DAY).await.unwrap();

    let entry = latest(&store).await;
    let homes = &entry["events"][0];
    assert_eq!(homes["kind"], "homes");
    assert_eq!(homes["ids"].as_array().unwrap().len(), 20);
    assert_eq!(homes["others"], 5);
    assert!(
        entry["text"].as_str().unwrap().contains("and 5 others."),
        "{entry}"
    );
}

#[tokio::test]
async fn only_ids_enter_an_entry() {
    let store = Arc::new(MemoryStore::new());
    let mut f = facts(1);
    // Anything that isn't shaped like an ID is dropped, so no agent's text can slip in.
    f.homes = set(&["aaaaaaaaaaaa", "Hello, I am Wren!"]);
    daily(&*store, &f, NOW).await.unwrap();
    let text = String::from_utf8(entry_bytes(&store, "2026-11-06")).unwrap();
    assert!(text.contains("aaaaaaaaaaaa"));
    assert!(!text.contains("Wren"), "{text}");
}

#[tokio::test]
async fn the_first_third_generation_teacher_is_a_milestone() {
    let store = Arc::new(MemoryStore::new());
    let cert = |id: &str, holder: &str, issuer: &str| Certificate {
        id: id.to_owned(),
        exam: true,
        holder: holder.to_owned(),
        issuer: issuer.to_owned(),
    };
    let mut f = facts(3);
    f.teachers = set(&["aaaaaaaaaaaa", "bbbbbbbbbbbb"]);
    f.certificates = vec![cert("exam-cccc-bbbb", "bbbbbbbbbbbb", "aaaaaaaaaaaa")];
    daily(&*store, &f, NOW).await.unwrap();
    let names = |entry: &Value| -> Vec<String> {
        entry["milestones"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["milestone"].as_str().unwrap().to_owned())
            .collect()
    };
    assert!(!names(&latest(&store).await).contains(&"first-third-generation-teacher".into()));

    f.teachers.insert("cccccccccccc".into());
    f.certificates
        .push(cert("exam-dddd-cccc", "cccccccccccc", "bbbbbbbbbbbb"));
    daily(&*store, &f, NOW + DAY).await.unwrap();
    let entry = latest(&store).await;
    assert!(
        entry["milestones"].as_array().unwrap().contains(
            &json!({"milestone": "first-third-generation-teacher", "id": "cccccccccccc"})
        ),
        "{entry}"
    );
}

#[tokio::test]
async fn the_routes_answer_not_built_yet_before_the_first_run() {
    let store = Arc::new(MemoryStore::new());
    let chronicle = Chronicle::new(store.clone());
    assert_eq!(chronicle.latest().await.unwrap_err().kind, "not-built-yet");
    assert_eq!(
        chronicle.days(None).await.unwrap_err().kind,
        "not-built-yet"
    );
    assert_eq!(
        chronicle.day("2026-11-06").await.unwrap_err().kind,
        "not-built-yet"
    );
}

#[tokio::test]
async fn the_days_are_newest_first_and_each_day_is_readable() {
    let store = Arc::new(MemoryStore::new());
    for n in 0..3 {
        daily(&*store, &facts(1), NOW + n * DAY).await.unwrap();
    }
    let chronicle = Chronicle::new(store.clone());
    assert_eq!(
        chronicle.days(None).await.unwrap().body,
        json!({"items": ["2026-11-08", "2026-11-07", "2026-11-06"], "next": null})
    );
    let day = chronicle.day("2026-11-07").await.unwrap().body;
    assert_eq!(day["date"], "2026-11-07");
    let missing = chronicle.day("2026-01-01").await.unwrap_err();
    assert_eq!(missing.status, 404);
    for bad in ["2026-13-01", "2026-11-6", "latest", "2026-11-06.json"] {
        assert_eq!(chronicle.day(bad).await.unwrap_err().status, 400, "{bad}");
    }
}
