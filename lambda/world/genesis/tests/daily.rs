//! The daily world job's genesis step: `.stories/17-genesis-codes/spec.md` "Two sources" (the
//! world), "Tracking" and "Cost at the daily limit", issue 02's Done when, at the crate's seam.

use std::sync::Arc;

use sfab_core::storage::{Calls, MemoryStore};
use sfab_genesis::{Daily, Genesis, Issue, Summary, daily, issue, new_code, read_summary, testing};

/// 2026-11-05T00:05:00Z, when the daily job runs.
const RUN: i64 = 1_793_836_800 + 300;
const DAY: i64 = 86_400;
const HOUR: i64 = 3_600;

const SETTINGS: Daily = Daily {
    codes_per_day: 100,
    ip_window_hours: 4,
};

fn genesis(store: &Arc<MemoryStore>) -> Genesis<MemoryStore> {
    Genesis::new(store.clone(), testing::ip_rule())
}

/// The summary as JSON, so a test can compare it with a literal.
async fn summary(store: &MemoryStore) -> serde_json::Value {
    let summary: Summary = read_summary(store)
        .await
        .unwrap()
        .expect("a summary")
        .summary;
    serde_json::to_value(summary).unwrap()
}

#[tokio::test]
async fn a_fresh_world_gets_its_first_pool_and_an_empty_summary() {
    let store = Arc::new(MemoryStore::new());

    daily(&*store, RUN, &SETTINGS).await.unwrap();

    let reply = genesis(&store).pool_code(RUN + HOUR).await.unwrap();
    let code = reply.body["genesis_code"].as_str().unwrap().to_owned();
    let admitted = genesis(&store)
        .admit(&code, Some("198.51.100.7:1"), RUN + HOUR)
        .await;
    assert!(admitted.is_ok(), "a pool code works today");
    assert_eq!(
        summary(&store).await,
        serde_json::json!({"labels": {}, "world": {}})
    );
}

/// Births with `n` codes from today's pool, each from its own address.
async fn use_world_codes(store: &Arc<MemoryStore>, now: i64, n: usize) {
    let g = genesis(store);
    for i in 0..n {
        let code = g.pool_code(now).await.unwrap().body["genesis_code"]
            .as_str()
            .unwrap()
            .to_owned();
        let viewer = format!("198.51.100.{i}:1");
        let admission = g.admit(&code, Some(&viewer), now).await.unwrap();
        g.use_up(&admission, now).await;
    }
}

/// Unused codes in storage, whatever their source: the codes that pile up if nothing deletes them.
fn unused_codes(store: &MemoryStore) -> usize {
    store
        .keys(sfab_core::storage::Bucket::Identities)
        .iter()
        .filter(|k| k.starts_with("genesis/codes/"))
        .count()
}

#[tokio::test]
async fn the_next_run_counts_yesterdays_world_codes_and_deletes_the_leftovers() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, RUN, &SETTINGS).await.unwrap();
    use_world_codes(&store, RUN + HOUR, 3).await;

    daily(&*store, RUN + DAY, &SETTINGS).await.unwrap();

    assert_eq!(unused_codes(&store), 100, "only today's codes are left");
    assert_eq!(
        summary(&store).await,
        serde_json::json!({
            "labels": {},
            "world": {"2026-11-05": {"issued": 100, "used": 3}},
        })
    );
    let reply = genesis(&store).pool_code(RUN + DAY + HOUR).await;
    assert!(reply.is_ok(), "today's pool is there");
}

#[tokio::test]
async fn a_run_after_a_missed_day_still_counts_the_day_before_it() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, RUN, &SETTINGS).await.unwrap();
    use_world_codes(&store, RUN + HOUR, 2).await;

    daily(&*store, RUN + 2 * DAY, &SETTINGS).await.unwrap();

    assert_eq!(unused_codes(&store), 100, "only today's codes are left");
    assert_eq!(
        summary(&store).await["world"],
        serde_json::json!({"2026-11-05": {"issued": 100, "used": 2}})
    );
}

#[tokio::test]
async fn the_world_history_keeps_30_days() {
    let store = Arc::new(MemoryStore::new());
    for day in 0..32 {
        daily(&*store, RUN + day * DAY, &SETTINGS).await.unwrap();
    }

    let world = summary(&store).await["world"].clone();
    let dates: Vec<&String> = world.as_object().unwrap().keys().collect();
    assert_eq!(dates.len(), 30);
    assert_eq!(dates.first().unwrap().as_str(), "2026-11-06");
    assert_eq!(dates.last().unwrap().as_str(), "2026-12-05");
}

#[tokio::test]
async fn a_second_run_on_one_day_makes_no_new_codes() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, RUN, &SETTINGS).await.unwrap();
    use_world_codes(&store, RUN + HOUR, 3).await;

    let report = daily(&*store, RUN + 2 * HOUR, &SETTINGS).await.unwrap();

    assert_eq!(report.issued, 0);
    assert_eq!(unused_codes(&store), 97);
    assert!(genesis(&store).pool_code(RUN + 2 * HOUR).await.is_ok());
}

/// `n` codes in an owner batch.
async fn owner_batch(
    store: &MemoryStore,
    label: &str,
    n: usize,
    expires: Option<i64>,
) -> Vec<String> {
    let how = Issue {
        label: label.into(),
        expires,
        no_ip_limit: false,
    };
    let mut codes = Vec::new();
    for _ in 0..n {
        let code = new_code();
        issue(store, &code, &how).await.unwrap();
        codes.push(code);
    }
    codes
}

#[tokio::test]
async fn an_owner_batch_half_used_counts_as_uploaded_and_left() {
    let store = Arc::new(MemoryStore::new());
    let codes = owner_batch(&store, "forum-y", 4, None).await;
    let g = genesis(&store);
    for (i, code) in codes.iter().take(2).enumerate() {
        let viewer = format!("198.51.100.{i}:1");
        let admission = g.admit(code, Some(&viewer), RUN - HOUR).await.unwrap();
        g.use_up(&admission, RUN - HOUR).await;
    }

    daily(&*store, RUN, &SETTINGS).await.unwrap();

    assert_eq!(
        summary(&store).await["labels"],
        serde_json::json!({"forum-y": {"uploaded": 4, "left": 2}})
    );
}

#[tokio::test]
async fn an_expired_owner_batch_still_counts_as_left_and_is_never_read() {
    let store = Arc::new(MemoryStore::new());
    owner_batch(&store, "old-post", 3, Some(RUN - 10 * DAY)).await;
    let before = store.calls();

    daily(&*store, RUN, &SETTINGS).await.unwrap();

    assert_eq!(store.calls().reads, before.reads, "no HEAD, nor any read");
    assert_eq!(unused_codes(&store), 3 + 100, "expired owner codes stay");
    assert_eq!(
        summary(&store).await["labels"],
        serde_json::json!({"old-post": {"uploaded": 3, "left": 3}})
    );
}

/// `spec.md` "Cost at the daily limit": 100 PUTs for codes and 100 for their batch entries, one
/// pool write, the summary's LIST, GET and PUT, one listing each of `genesis/codes/`,
/// `genesis/batches/` and `genesis/ips/`, and free DELETEs. `MemoryStore` counts a listing as one
/// call however many pages S3 would need.
#[tokio::test]
async fn a_run_makes_the_s3_calls_the_spec_costs() {
    let store = Arc::new(MemoryStore::new());
    daily(&*store, RUN, &SETTINGS).await.unwrap();
    use_world_codes(&store, RUN + HOUR, 3).await;
    let before = store.calls();

    daily(&*store, RUN + DAY, &SETTINGS).await.unwrap();

    let after = store.calls();
    let run = Calls {
        reads: after.reads - before.reads,
        puts: after.puts - before.puts,
        lists: after.lists - before.lists,
        deletes: after.deletes - before.deletes,
    };
    assert_eq!(
        run,
        Calls {
            reads: 1,
            puts: 100 + 100 + 1 + 1,
            lists: 4,
            // Yesterday's 97 leftovers, and the 3 births' IP records, now older than the window.
            deletes: 97 + 3,
        }
    );
}

fn ip_records(store: &MemoryStore) -> usize {
    store
        .keys(sfab_core::storage::Bucket::Identities)
        .iter()
        .filter(|k| k.starts_with("genesis/ips/"))
        .count()
}

#[tokio::test]
async fn the_run_deletes_ip_records_older_than_the_window_and_keeps_newer_ones() {
    let store = Arc::new(MemoryStore::new());
    let g = genesis(&store);
    for (viewer, at) in [
        ("198.51.100.1:1", RUN - 5 * HOUR),
        ("198.51.100.2:1", RUN - 3 * HOUR),
    ] {
        store.set_time(at);
        let code = testing::seed_code(&*store).await;
        let admission = g.admit(&code, Some(viewer), at).await.unwrap();
        g.use_up(&admission, at).await;
    }
    store.set_time(RUN);

    daily(&*store, RUN, &SETTINGS).await.unwrap();

    assert_eq!(ip_records(&store), 1, "the 5-hour-old record is gone");
    let code = testing::seed_code(&*store).await;
    let newer = g.admit(&code, Some("198.51.100.2:1"), RUN).await;
    assert_eq!(
        newer.unwrap_err().kind,
        "too-soon",
        "the newer record stays"
    );
}
