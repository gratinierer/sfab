//! Issuing batches and reading the summary, from `.stories/17-genesis-codes/issues/04-owner-script.md`'s
//! Done when, against `core`'s in-memory fake. What the tool wrote is checked through the
//! `genesis` crate, as `birth` and the daily job see it, since only that crate knows a key.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::json;
use sfab_core::storage::MemoryStore;
use sfab_genesis::{Daily, Genesis, Issue, Summary, daily, is_code, read_summary, testing};
use sfab_genesis_codes::{issue_codes, status};

/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;
/// The next daily run, 2026-11-06T00:05:00Z.
const RUN: i64 = 1_793_923_200 + 300;
const VIEWER: &str = "198.51.100.7:46532";

const SETTINGS: Daily = Daily {
    codes_per_day: 0,
    ip_window_hours: 4,
};

fn owner(label: &str) -> Issue {
    Issue {
        label: label.to_owned(),
        expires: None,
        no_ip_limit: false,
    }
}

fn genesis(store: &Arc<MemoryStore>) -> Genesis<MemoryStore> {
    Genesis::new(store.clone(), testing::ip_rule())
}

/// The labels after the next daily run, as JSON.
async fn labels_after_the_run(store: &MemoryStore) -> serde_json::Value {
    daily(store, RUN, &SETTINGS).await.unwrap();
    let summary: Summary = read_summary(store)
        .await
        .unwrap()
        .expect("a summary")
        .summary;
    serde_json::to_value(summary.labels).unwrap()
}

#[tokio::test]
async fn a_new_batch_gives_distinct_codes_that_work_and_count_under_their_label() {
    let store = Arc::new(MemoryStore::new());

    let codes = issue_codes(&*store, 5, &owner("forum-y")).await.unwrap();

    assert_eq!(codes.len(), 5);
    assert_eq!(codes.iter().collect::<BTreeSet<_>>().len(), 5);
    assert!(codes.iter().all(|c| is_code(c)), "{codes:?}");
    let admitted = genesis(&store).admit(&codes[0], Some(VIEWER), NOW).await;
    assert!(admitted.is_ok(), "an owner code works");
    assert_eq!(
        labels_after_the_run(&store).await,
        json!({"forum-y": {"uploaded": 5, "left": 5}})
    );
}

#[tokio::test]
async fn an_existing_label_adds_to_its_batch() {
    let store = Arc::new(MemoryStore::new());
    let first = issue_codes(&*store, 3, &owner("forum-y")).await.unwrap();
    issue_codes(&*store, 1, &owner("mastodon")).await.unwrap();
    let used = genesis(&store).admit(&first[0], Some(VIEWER), NOW).await;
    genesis(&store).use_up(&used.unwrap(), NOW).await;

    let more = issue_codes(&*store, 2, &owner("forum-y")).await.unwrap();

    assert!(first.iter().all(|c| !more.contains(c)));
    let admitted = genesis(&store)
        .admit(&first[1], Some("198.51.100.8:1"), NOW)
        .await;
    assert!(admitted.is_ok(), "the batch's earlier codes still work");
    assert_eq!(
        labels_after_the_run(&store).await,
        json!({
            "forum-y": {"uploaded": 5, "left": 4},
            "mastodon": {"uploaded": 1, "left": 1},
        })
    );
}

#[tokio::test]
async fn no_ip_limit_codes_skip_the_ip_rule_and_ordinary_ones_do_not() {
    let store = Arc::new(MemoryStore::new());
    let exempt = Issue {
        no_ip_limit: true,
        ..owner("smoke")
    };
    let smoke = issue_codes(&*store, 2, &exempt).await.unwrap();
    let ordinary = issue_codes(&*store, 1, &owner("forum-y")).await.unwrap();

    // The IP rule needs a viewer address; a code that skips it doesn't.
    let admitted = genesis(&store).admit(&smoke[0], None, NOW).await;
    assert!(admitted.is_ok(), "a --no-ip-limit code needs no viewer");
    let refused = genesis(&store).admit(&ordinary[0], None, NOW).await;
    assert_eq!(refused.expect_err("refused").status, 500);

    // Births from one address, one after the other, all pass with exempt codes, which leave no
    // IP record behind, so an ordinary code from that address still passes after them.
    for code in &smoke {
        let admission = genesis(&store).admit(code, Some(VIEWER), NOW).await;
        genesis(&store).use_up(&admission.unwrap(), NOW).await;
    }
    let admission = genesis(&store).admit(&ordinary[0], Some(VIEWER), NOW).await;
    genesis(&store).use_up(&admission.unwrap(), NOW).await;
    // Each still works once.
    let again = genesis(&store).admit(&smoke[0], None, NOW).await;
    assert_eq!(again.expect_err("used").kind, "invalid-genesis-code");
}

#[tokio::test]
async fn an_expiring_batch_stops_working_after_its_date() {
    let store = Arc::new(MemoryStore::new());
    let how = Issue {
        // 2026-11-06T00:00:00Z, as `--expires 2026-11-05` gives.
        expires: Some(1_793_923_200),
        ..owner("forum-y")
    };
    let codes = issue_codes(&*store, 1, &how).await.unwrap();

    let today = genesis(&store)
        .admit(&codes[0], Some(VIEWER), 1_793_923_199)
        .await;
    assert!(today.is_ok(), "works through its last day");
    let after = genesis(&store)
        .admit(&codes[0], Some(VIEWER), 1_793_923_200)
        .await;
    assert_eq!(after.expect_err("expired").kind, "invalid-genesis-code");
}

#[tokio::test]
async fn status_before_the_daily_job_has_run() {
    let store = MemoryStore::new();
    issue_codes(&store, 1, &owner("forum-y")).await.unwrap();

    assert_eq!(
        status(&store).await.unwrap(),
        "No summary yet: the daily job hasn't run since genesis codes arrived.\n"
    );
}

#[tokio::test]
async fn status_prints_the_summary_with_when_the_daily_job_wrote_it() {
    let store = Arc::new(MemoryStore::new());
    let forum = issue_codes(&*store, 12, &owner("forum-y")).await.unwrap();
    issue_codes(&*store, 3, &owner("smoke")).await.unwrap();
    let world = testing::seed_world_codes(&*store, NOW, 100).await;
    for (code, viewer) in [(&forum[0], "198.51.100.1:1"), (&world[0], "198.51.100.2:1")] {
        let admission = genesis(&store).admit(code, Some(viewer), NOW).await;
        genesis(&store).use_up(&admission.unwrap(), NOW).await;
    }
    store.set_time(RUN);
    daily(&*store, RUN, &SETTINGS).await.unwrap();

    assert_eq!(
        status(&*store).await.unwrap(),
        "\
Written by the daily job at 2026-11-06T00:05:00Z; a batch issued since shows after its next run.

Owner batches   uploaded   left
  forum-y             12     11
  smoke                3      3

World days        issued   used
  2026-11-05         100      1
"
    );
}
