//! The daily world job end to end, against the in-memory fake: it counts every claim apart from
//! takedown markers, and stamps the census with the run's time.

use std::sync::Arc;

use serde_json::json;
use sfab_census::Census;
use sfab_core::id::AgentId;
use sfab_core::storage::{Bucket, Condition, MemoryStore, Store};
use sfab_core::takedown::Action;
use sfab_federation::{Fetch, Notify};
use sfab_genesis::{Daily, Genesis, IpRule};

/// 2026-11-07T00:05:00Z.
const NOW: i64 = 1_794_009_600 + 300;

const GENESIS: Daily = Daily {
    codes_per_day: 3,
    ip_window_hours: 4,
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

    let report = sfab_job::run(&*store, &Offline, &Offline, &GENESIS, NOW)
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
async fn the_job_makes_the_days_genesis_codes() {
    let store = Arc::new(MemoryStore::new());

    let report = sfab_job::run(&*store, &Offline, &Offline, &GENESIS, NOW)
        .await
        .unwrap();

    assert_eq!(report.genesis.issued, 3);
    let rule = IpRule::new("anykeyanykeyanykeyanykeyanykey12", 4);
    let genesis = Genesis::new(store.clone(), rule);
    assert!(genesis.pool_code(NOW).await.is_ok());
}
