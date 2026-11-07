//! The census and resident pages: the daily world job's snapshot and the routes that read it,
//! from `.stories/2-identity-api/spec.md` "World" and "Testing", against the in-memory fake.

use std::sync::Arc;

use serde_json::{Value, json};
use sfab_census::{Census, snapshot};
use sfab_core::id::AgentId;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, MemoryStore, Metadata, Object, Store, StoreError};

const AS_OF: &str = "2026-11-07T00:05:00Z";

/// S3 as the job sees it: it may list only `world/` in the identities bucket, so asking for a
/// missing key there gets `403`, not `404`.
struct ListOnlyWorld(MemoryStore);

impl Store for ListOnlyWorld {
    async fn get(&self, bucket: Bucket, key: &str) -> Result<Option<Object>, StoreError> {
        match self.0.get(bucket, key).await? {
            None => Err(StoreError::Other(format!("403 on missing {key}"))),
            found => Ok(found),
        }
    }

    async fn put(
        &self,
        bucket: Bucket,
        key: &str,
        body: Vec<u8>,
        metadata: Metadata,
        condition: Condition,
    ) -> Result<(), StoreError> {
        self.0.put(bucket, key, body, metadata, condition).await
    }

    async fn delete(&self, bucket: Bucket, key: &str) -> Result<(), StoreError> {
        self.0.delete(bucket, key).await
    }

    async fn list(&self, bucket: Bucket, prefix: &str) -> Result<Vec<String>, StoreError> {
        self.0.list(bucket, prefix).await
    }
}

#[tokio::test]
async fn the_snapshot_never_asks_for_a_missing_key() {
    let store = ListOnlyWorld(MemoryStore::new());

    snapshot(&store, &ids(1_500), AS_OF).await.unwrap();
    snapshot(&store, &ids(2_500), NEXT_DAY).await.unwrap();
}
const NEXT_DAY: &str = "2026-11-08T00:05:00Z";

/// `n` distinct, sorted resident IDs.
fn ids(n: usize) -> Vec<AgentId> {
    let alphabet = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut ids: Vec<AgentId> = (0..n)
        .map(|i| {
            let id: String = (0..12)
                .rev()
                .map(|d| alphabet[(i >> (5 * d)) & 31] as char)
                .collect();
            AgentId::parse(&id).unwrap()
        })
        .collect();
    ids.sort();
    ids
}

fn page_keys(store: &MemoryStore) -> Vec<String> {
    store
        .keys(Bucket::Identities)
        .into_iter()
        .filter(|k| k.starts_with("world/residents/"))
        .collect()
}

fn ok(result: Result<Reply, Problem>) -> Value {
    let reply = result.expect("a reply");
    assert_eq!(reply.status, 200);
    reply.body
}

fn problem(result: Result<Reply, Problem>) -> (u16, &'static str) {
    let problem = result.expect_err("a problem");
    (problem.status, problem.kind)
}

/// Every resident, following `next` from the first page.
async fn read_all(census: &Census<MemoryStore>) -> Vec<String> {
    let mut found = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let body = ok(census.residents(page.as_deref()).await);
        for item in body["items"].as_array().unwrap() {
            found.push(item.as_str().unwrap().to_owned());
        }
        match body["next"].as_str() {
            None => return found,
            Some(next) => {
                let token = next.strip_prefix("/v1/world/residents?page=").unwrap();
                page = Some(token.to_owned());
            }
        }
    }
}

#[tokio::test]
async fn before_the_first_run_both_routes_say_not_built_yet() {
    let census = Census::new(Arc::new(MemoryStore::new()));

    assert_eq!(problem(census.census().await), (404, "not-built-yet"));
    assert_eq!(
        problem(census.residents(None).await),
        (404, "not-built-yet")
    );
    assert_eq!(
        problem(census.residents(Some("2")).await),
        (404, "not-built-yet")
    );
}

#[tokio::test]
async fn the_snapshot_pages_residents_1000_at_a_time() {
    let store = Arc::new(MemoryStore::new());
    let census = Census::new(store.clone());
    let residents = ids(2_500);

    snapshot(&*store, &residents, AS_OF).await.unwrap();

    assert_eq!(
        ok(census.census().await),
        json!({"residents": 2_500, "as_of": AS_OF})
    );
    let first = ok(census.residents(None).await);
    assert_eq!(first["items"].as_array().unwrap().len(), 1_000);
    assert_eq!(first["items"][0], residents[0].as_str());
    assert_eq!(first["next"], "/v1/world/residents?page=2");
    assert_eq!(ok(census.residents(Some("1")).await), first);
    let last = ok(census.residents(Some("3")).await);
    assert_eq!(last["items"].as_array().unwrap().len(), 500);
    assert_eq!(last["next"], Value::Null);
    let expected: Vec<String> = residents.iter().map(|i| i.as_str().to_owned()).collect();
    assert_eq!(read_all(&census).await, expected);
}

#[tokio::test]
async fn a_page_past_the_last_gets_404_and_a_malformed_token_400() {
    let store = Arc::new(MemoryStore::new());
    let census = Census::new(store.clone());
    snapshot(&*store, &ids(1_500), AS_OF).await.unwrap();

    assert_eq!(
        problem(census.residents(Some("3")).await),
        (404, "about:blank")
    );
    for bad in ["0", "01", "x", "-1", "", "99999999999999999999"] {
        assert_eq!(problem(census.residents(Some(bad)).await).0, 400, "{bad:?}");
    }
}

#[tokio::test]
async fn an_empty_world_has_one_empty_page() {
    let store = Arc::new(MemoryStore::new());
    let census = Census::new(store.clone());

    snapshot(&*store, &[], AS_OF).await.unwrap();

    assert_eq!(ok(census.census().await)["residents"], 0);
    assert_eq!(
        ok(census.residents(None).await),
        json!({"items": [], "next": null})
    );
}

#[tokio::test]
async fn running_twice_changes_nothing_more() {
    let store = Arc::new(MemoryStore::new());
    let residents = ids(2_500);
    snapshot(&*store, &residents, AS_OF).await.unwrap();
    let before: Vec<_> = store
        .keys(Bucket::Identities)
        .iter()
        .map(|k| (k.clone(), store.writes_to(Bucket::Identities, k)))
        .collect();

    let report = snapshot(&*store, &residents, AS_OF).await.unwrap();

    let after: Vec<_> = store
        .keys(Bucket::Identities)
        .iter()
        .map(|k| (k.clone(), store.writes_to(Bucket::Identities, k)))
        .collect();
    assert_eq!(after, before);
    assert_eq!((report.written, report.deleted), (0, 0));
}

#[tokio::test]
async fn a_new_day_writes_only_what_changed() {
    let store = Arc::new(MemoryStore::new());
    let census = Census::new(store.clone());
    let all = ids(2_501);
    // The newest resident sorts last, so only the last page changes.
    snapshot(&*store, &all[..2_500], AS_OF).await.unwrap();

    let report = snapshot(&*store, &all, NEXT_DAY).await.unwrap();

    assert_eq!(
        store.writes_to(Bucket::Identities, "world/residents/1.json"),
        1
    );
    assert_eq!(
        store.writes_to(Bucket::Identities, "world/residents/2.json"),
        1
    );
    assert_eq!(
        store.writes_to(Bucket::Identities, "world/residents/3.json"),
        2
    );
    // The census and the last page.
    assert_eq!(report.written, 2);
    assert_eq!(
        ok(census.census().await),
        json!({"residents": 2_501, "as_of": NEXT_DAY})
    );
}

#[tokio::test]
async fn pages_beyond_the_last_are_deleted() {
    let store = Arc::new(MemoryStore::new());
    let census = Census::new(store.clone());
    let all = ids(2_500);
    snapshot(&*store, &all, AS_OF).await.unwrap();

    let report = snapshot(&*store, &all[..900], NEXT_DAY).await.unwrap();

    assert_eq!(report.deleted, 2);
    assert_eq!(page_keys(&store), ["world/residents/1.json"]);
    assert_eq!(ok(census.residents(None).await)["next"], Value::Null);
    assert_eq!(
        problem(census.residents(Some("2")).await),
        (404, "about:blank")
    );
    // A removed resident leaves the public list on the next run.
    assert_eq!(read_all(&census).await.len(), 900);
}

#[tokio::test]
async fn a_failed_write_is_an_error() {
    let store = Arc::new(MemoryStore::new());
    store.fail_writes_under(Bucket::Identities, "world/");

    assert!(snapshot(&*store, &ids(10), AS_OF).await.is_err());
}
