//! The daily world job end to end, against the in-memory fake: it counts every claim apart from
//! takedown markers, and stamps the census with the run's time.

use std::sync::Arc;

use serde_json::json;
use sfab_census::Census;
use sfab_core::id::AgentId;
use sfab_core::storage::{Bucket, Condition, MemoryStore, Store};
use sfab_core::takedown::Action;
use sfab_core::testing::Captured;
use sfab_federation::{Fetch, Notify};
use sfab_genesis::{Daily, Genesis, IpRule, Window};

/// 2026-11-07T00:05:00Z.
const NOW: i64 = 1_794_009_600 + 300;

/// `cdk/stacks.py`'s placeholder, so no one needs age yet.
const AGE_RULE_STARTS: &str = "2099-12-31T00:00:00Z";

const GENESIS: Daily = Daily {
    codes_per_day: 3,
    ip_window: Window::hours(4),
};

/// No server answers, and no email is sent: this file tests the census, and `outbound.rs` and
/// the federation crate test the rest.
struct Offline;

impl Fetch for Offline {
    async fn fetch(&self, _: &str, _: &str) -> Option<Vec<u8>> {
        None
    }
}

impl Notify for Offline {
    async fn notify(&self, _: &str, _: &str) -> Result<(), String> {
        Err("no email expected".into())
    }
}

async fn claim(store: &MemoryStore, id: &str) {
    // A claim as `birth` leaves it; the job never opens one, so its content doesn't matter.
    let key = sfab_identity::takedown(&AgentId::parse(id).unwrap())
        .into_iter()
        .find(|l| matches!(l.action, Action::Marker(_)))
        .unwrap()
        .key;
    store
        .put(
            Bucket::Claims,
            &key,
            b"{}".to_vec(),
            Default::default(),
            Condition::Always,
        )
        .await
        .unwrap();
}

async fn marker(store: &MemoryStore, id: &str) {
    let location = sfab_identity::takedown(&AgentId::parse(id).unwrap())
        .into_iter()
        .find(|l| matches!(l.action, Action::Marker(_)))
        .unwrap();
    store
        .put(
            Bucket::Claims,
            &location.key,
            Vec::new(),
            Default::default(),
            Condition::Always,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn the_job_counts_residents_without_markers_as_of_its_run() {
    let store = Arc::new(MemoryStore::new());
    claim(&store, "aaaaaaaaaaaa").await;
    claim(&store, "bbbbbbbbbbbb").await;
    marker(&store, "cccccccccccc").await;

    let report = sfab_job::run(&*store, &Offline, &Offline, &GENESIS, AGE_RULE_STARTS, NOW)
        .await
        .unwrap();

    assert_eq!(report.census.residents, 2);
    let census = Census::new(store.clone());
    assert_eq!(
        census.census().await.unwrap().body,
        json!({"residents": 2, "as_of": "2026-11-07T00:05:00Z"})
    );
    assert_eq!(
        census.residents(None).await.unwrap().body,
        json!({"items": ["aaaaaaaaaaaa", "bbbbbbbbbbbb"], "next": null})
    );
}

#[tokio::test]
async fn the_job_gives_the_compiled_in_founders_standing_unless_taken_down() {
    // The fixture founders: `world/founders.fixture.yaml`.
    let store = Arc::new(MemoryStore::new());
    claim(&store, "foundera2222").await;
    claim(&store, "founderb2222").await;
    marker(&store, "founderc2222").await;

    let report = sfab_job::run(&*store, &Offline, &Offline, &GENESIS, AGE_RULE_STARTS, NOW)
        .await
        .unwrap();

    assert_eq!(report.standing.standing, 2);
    let rule = IpRule::new("anykeyanykeyanykeyanykeyanykey12", Window::hours(4));
    let genesis = Arc::new(Genesis::new(store.clone(), rule));
    let identity = Arc::new(sfab_identity::Identity::new(store.clone(), 8, genesis));
    let vouches = sfab_vouches::Vouches::new(store.clone(), identity, 8);
    for (id, standing) in [("foundera2222", true), ("founderc2222", false)] {
        assert_eq!(
            vouches.standing(id).await.unwrap().body,
            json!({"id": id, "standing": standing, "as_of": "2026-11-07T00:05:00Z"})
        );
    }
}

#[tokio::test]
async fn the_job_deletes_a_ghost_pointer_and_counts_every_quarter_s_homes() {
    let store = Arc::new(MemoryStore::new());
    // A pointer naming an agent with no home, as a takedown or a skipped delete leaves it.
    let ghost = "places/harbour/lantern-lane/1.json";
    store
        .put(
            Bucket::Identities,
            ghost,
            br#"{"id":"aaaaaaaaaaaa"}"#.to_vec(),
            Default::default(),
            Condition::Always,
        )
        .await
        .unwrap();

    let report = sfab_job::run(&*store, &Offline, &Offline, &GENESIS, AGE_RULE_STARTS, NOW)
        .await
        .unwrap();

    assert_eq!((report.homes.residents, report.homes.ghosts), (0, 1));
    assert!(store.object(Bucket::Identities, ghost).is_none());
    let rule = IpRule::new("anykeyanykeyanykeyanykeyanykey12", Window::hours(4));
    let genesis = Arc::new(Genesis::new(store.clone(), rule));
    let identity = Arc::new(sfab_identity::Identity::new(store.clone(), 8, genesis));
    let homes = sfab_homes::Homes::new(store.clone(), identity, 8);
    let quarter = homes.quarter_counts("harbour").await.unwrap().body;
    assert_eq!(
        quarter,
        json!({"quarter": "harbour", "residents": 0, "free": 100, "as_of": "2026-11-07T00:05:00Z"})
    );
}

#[tokio::test]
async fn the_job_makes_the_days_genesis_codes() {
    let store = Arc::new(MemoryStore::new());

    let report = sfab_job::run(&*store, &Offline, &Offline, &GENESIS, AGE_RULE_STARTS, NOW)
        .await
        .unwrap();

    assert_eq!(report.genesis.issued, 3);
    let rule = IpRule::new("anykeyanykeyanykeyanykeyanykey12", Window::hours(4));
    let genesis = Genesis::new(store.clone(), rule);
    assert!(genesis.pool_code(NOW).await.is_ok());
}

#[tokio::test]
async fn a_failed_census_still_makes_the_days_genesis_codes_and_fails_the_run() {
    // A census fault mustn't leave `GET /world/genesis-code` answering `try-later` all day.
    let store = Arc::new(MemoryStore::new());
    claim(&store, "aaaaaaaaaaaa").await;
    store.fail_writes_under(Bucket::Identities, "world/");

    let run = sfab_job::run(&*store, &Offline, &Offline, &GENESIS, AGE_RULE_STARTS, NOW).await;

    assert!(run.is_err(), "the census's failure still fails the run");
    let rule = IpRule::new("anykeyanykeyanykeyanykeyanykey12", Window::hours(4));
    let genesis = Genesis::new(store.clone(), rule);
    assert!(genesis.pool_code(NOW).await.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn when_the_census_and_genesis_both_fail_the_genesis_error_is_logged() {
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let store = Arc::new(MemoryStore::new());
    claim(&store, "aaaaaaaaaaaa").await;
    store.fail_writes_under(Bucket::Identities, "world/");
    store.fail_writes_under(Bucket::Identities, "genesis/");

    let run = sfab_job::run(&*store, &Offline, &Offline, &GENESIS, AGE_RULE_STARTS, NOW).await;

    let error = format!("{:?}", run.unwrap_err());
    assert!(
        !error.contains("genesis/"),
        "the run returns the census's error: {error}"
    );
    let logs = captured.text();
    assert!(logs.contains("injected failure on genesis/"), "{logs}");
}
