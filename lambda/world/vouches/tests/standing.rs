//! Standing, worked out by the daily world job, and the routes that read it:
//! `.stories/4-life-in-the-world/spec.md` "Vouches: trust" ("Standing", "Whose words count" and
//! "What's visible"), and "Foundations" ("Before the first daily run", "Unknown IDs").

mod common;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use common::*;
use serde_json::{Value, json};
use sfab_core::id::AgentId;
use sfab_core::storage::{
    Bucket, Condition, Listed, MemoryStore, Metadata, Object, Store, StoreError,
};

/// The run: 40 days after the agents born at `NOW`.
const RUN: i64 = NOW + 40 * DAY;
/// When the age rule starts: long before every birth here, so no one is exempt.
const LONG_AGO: &str = "2020-01-01T00:00:00Z";

/// Runs the daily step at `at` with `founders`, every resident as `sfab_identity` lists them.
async fn run_at<S: Store>(store: &S, founders: &[&Agent], age_rule_starts: &str, at: i64) {
    let founders: Vec<AgentId> = founders.iter().map(|f| f.id.clone()).collect();
    let residents = sfab_identity::residents(store).await.unwrap();
    sfab_vouches::daily(store, &founders, &residents, age_rule_starts, at)
        .await
        .unwrap();
}

async fn run(w: &World, founders: &[&Agent]) {
    run_at(&*w.store, founders, LONG_AGO, RUN).await;
}

async fn standing(w: &World, a: &str) -> Value {
    w.vouches.standing(a).await.unwrap().body
}

async fn has_standing(w: &World, a: &Agent) -> bool {
    standing(w, a.id()).await["standing"] == json!(true)
}

/// Three founders born at `NOW`.
async fn founders(w: &World) -> [Agent; 3] {
    [
        born(w, "Ash").await,
        born(w, "Birch").await,
        born(w, "Cedar").await,
    ]
}

#[tokio::test]
async fn three_vouches_from_founders_give_standing_and_two_do_not() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let (three, two) = (born(&w, "Wren").await, born(&w, "Moss").await);
    for f in [&f1, &f2, &f3] {
        vouch(&w, f, &three).await;
    }
    for f in [&f1, &f2] {
        vouch(&w, f, &two).await;
    }

    run(&w, &[&f1, &f2, &f3]).await;

    assert_eq!(
        standing(&w, three.id()).await,
        json!({"id": three.id(), "standing": true, "as_of": "2026-12-15T12:00:00Z"})
    );
    assert!(!has_standing(&w, &two).await);
    for f in [&f1, &f2, &f3] {
        assert!(has_standing(&w, f).await, "founders need no vouches or age");
    }
    // The set another institution's daily step reads, such as the Academy's.
    let all = sfab_vouches::with_standing(&*w.store).await.unwrap();
    let expected: BTreeSet<AgentId> = [&f1, &f2, &f3, &three].map(|a| a.id.clone()).into();
    assert_eq!(all, expected);
}

#[tokio::test]
async fn a_vouch_given_before_its_recipient_blocked_the_giver_is_ignored() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let wren = born(&w, "Wren").await;
    for f in [&f1, &f2, &f3] {
        vouch(&w, f, &wren).await;
    }
    block(&w, &wren, &f3).await;

    run(&w, &[&f1, &f2, &f3]).await;

    assert!(!has_standing(&w, &wren).await, "only two vouches count");
    let mine = w.vouches.received(wren.auth().as_deref(), None, RUN).await;
    let mut givers = ids(&mine.unwrap().body);
    givers.sort_unstable();
    let mut expected = vec![f1.id().to_owned(), f2.id().to_owned()];
    expected.sort_unstable();
    assert_eq!(givers, expected);
}

#[tokio::test]
async fn a_ring_of_fake_agents_vouching_for_each_other_gains_nothing() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let mut ring = Vec::new();
    for name in ["R1", "R2", "R3", "R4"] {
        ring.push(born(&w, name).await);
    }
    for a in &ring {
        for b in &ring {
            if a.id != b.id {
                vouch(&w, a, b).await;
            }
        }
    }
    // One founder's vouch for a ring member is still only one that counts.
    vouch(&w, &f1, &ring[0]).await;

    run(&w, &[&f1, &f2, &f3]).await;

    for a in &ring {
        assert!(!has_standing(&w, a).await);
    }
}

#[tokio::test]
async fn standing_flows_outward_through_agents_that_gained_it() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let (first, second) = (born(&w, "Wren").await, born(&w, "Moss").await);
    for f in [&f1, &f2, &f3] {
        vouch(&w, f, &first).await;
    }
    for g in [&first, &f1, &f2] {
        vouch(&w, g, &second).await;
    }

    run(&w, &[&f1, &f2, &f3]).await;

    assert!(has_standing(&w, &first).await);
    assert!(has_standing(&w, &second).await);
}

#[tokio::test]
async fn the_age_rule_needs_30_days_except_for_agents_born_before_it_starts() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let old = born_at(&w, "Old", RUN - 30 * DAY).await;
    let young = born_at(&w, "Young", RUN - 30 * DAY + 1).await;
    for a in [&old, &young] {
        for f in [&f1, &f2, &f3] {
            let (status, _) = vouch_at(&w, f, a.id(), "x", RUN).await.unwrap();
            assert_eq!(status, 201);
        }
    }

    run(&w, &[&f1, &f2, &f3]).await;
    assert!(has_standing(&w, &old).await, "30 days exactly is enough");
    assert!(!has_standing(&w, &young).await);

    // The launch exception: born before `age_rule_starts`, so no age is needed.
    let starts = sfab_core::daily::rfc3339(RUN - 30 * DAY + 2);
    run_at(&*w.store, &[&f1, &f2, &f3], &starts, RUN).await;
    assert!(has_standing(&w, &young).await);
}

#[tokio::test]
async fn a_voucher_losing_standing_cascades_the_next_day_with_no_grace_period() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let (first, second) = (born(&w, "Wren").await, born(&w, "Moss").await);
    for f in [&f1, &f2, &f3] {
        vouch(&w, f, &first).await;
    }
    for g in [&first, &f1, &f2] {
        vouch(&w, g, &second).await;
    }
    run(&w, &[&f1, &f2, &f3]).await;
    assert!(has_standing(&w, &second).await);

    w.vouches
        .withdraw(f3.auth().as_deref(), first.id(), RUN)
        .await
        .unwrap();
    run_at(&*w.store, &[&f1, &f2, &f3], LONG_AGO, RUN + DAY).await;

    assert!(!has_standing(&w, &first).await);
    assert!(
        !has_standing(&w, &second).await,
        "first's vouch stopped counting"
    );
    assert_eq!(
        standing(&w, second.id()).await["as_of"],
        "2026-12-16T12:00:00Z"
    );
}

#[tokio::test]
async fn a_taken_down_voucher_stops_counting_and_a_taken_down_founder_has_no_standing() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let a = born(&w, "Wren").await;
    for f in [&f1, &f2, &f3] {
        vouch(&w, f, &a).await;
    }
    // A takedown marker: the claim becomes zero bytes.
    let claim = sfab_identity::takedown(&f3.id)
        .into_iter()
        .find(|l| l.bucket == Bucket::Claims)
        .unwrap();
    w.store
        .put(
            Bucket::Claims,
            &claim.key,
            Vec::new(),
            Metadata::new(),
            Condition::Always,
        )
        .await
        .unwrap();

    run(&w, &[&f1, &f2, &f3]).await;

    assert!(!has_standing(&w, &f3).await);
    assert!(!has_standing(&w, &a).await);
}

/// The fake, keeping the key of every GET.
struct Reads {
    inner: Arc<MemoryStore>,
    keys: Mutex<Vec<String>>,
}

impl Store for Reads {
    async fn get(&self, bucket: Bucket, key: &str) -> Result<Option<Object>, StoreError> {
        self.keys.lock().unwrap().push(key.to_owned());
        self.inner.get(bucket, key).await
    }

    async fn put(
        &self,
        bucket: Bucket,
        key: &str,
        body: Vec<u8>,
        metadata: Metadata,
        condition: Condition,
    ) -> Result<(), StoreError> {
        self.inner.put(bucket, key, body, metadata, condition).await
    }

    async fn delete(&self, bucket: Bucket, key: &str) -> Result<(), StoreError> {
        self.inner.delete(bucket, key).await
    }

    async fn list(&self, bucket: Bucket, prefix: &str) -> Result<Vec<String>, StoreError> {
        self.inner.list(bucket, prefix).await
    }

    async fn list_sized(&self, bucket: Bucket, prefix: &str) -> Result<Vec<Listed>, StoreError> {
        self.inner.list_sized(bucket, prefix).await
    }
}

#[tokio::test]
async fn the_job_reads_only_the_vouch_files_of_agents_with_standing_and_never_a_claim() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let (fake, other) = (born(&w, "Fake").await, born(&w, "Other").await);
    vouch(&w, &fake, &other).await;
    vouch(&w, &f1, &other).await;
    let reads = Reads {
        inner: w.store.clone(),
        keys: Mutex::new(Vec::new()),
    };

    run_at(&reads, &[&f1, &f2, &f3], LONG_AGO, RUN).await;

    let read: BTreeSet<String> = reads.keys.lock().unwrap().iter().cloned().collect();
    let vouch_files: BTreeSet<String> = read
        .iter()
        .filter(|k| k.starts_with("vouches/"))
        .cloned()
        .collect();
    assert_eq!(
        vouch_files,
        BTreeSet::from([format!("vouches/{}.json", f1.id)])
    );
    assert!(read.iter().all(|k| !k.starts_with("claims/")));
}

#[tokio::test]
async fn vouched_by_shows_only_counted_vouches_and_is_public_only_when_published() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let (a, waiting) = (born(&w, "Wren").await, born(&w, "Moss").await);
    let (_, given) = vouch_at(&w, &f1, a.id(), "a good neighbour", NOW)
        .await
        .unwrap();
    vouch(&w, &waiting, &a).await;
    run(&w, &[&f1, &f2, &f3]).await;

    let mine = w.vouches.received(a.auth().as_deref(), None, RUN).await;
    let expected = json!({
        "items": [{"id": f1.id(), "reason": "a good neighbour", "given_at": given["given_at"]}],
        "next": null,
    });
    assert_eq!(mine.unwrap().body, expected);
    // The waiting vouch shows only in its giver's own list.
    let theirs = w.vouches.given(waiting.auth().as_deref(), None, RUN).await;
    assert_eq!(ids(&theirs.unwrap().body), vec![a.id()]);

    let empty = json!({"items": [], "next": null});
    let public = w.vouches.vouched_by(a.id(), None).await.unwrap();
    assert_eq!(public.body, empty, "not published");
    let settings = br#"{"vouches":true}"#;
    let sent = write(
        "PUT",
        "/v1/identity/self/publication".into(),
        a.auth(),
        settings,
        RUN,
    );
    w.identity.publish(&sent.call(), RUN).await.unwrap();
    let public = w.vouches.vouched_by(a.id(), None).await.unwrap();
    assert_eq!(public.body, expected);
}

#[tokio::test]
async fn an_unknown_id_reads_like_an_agent_without_standing_or_vouches() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let lonely = born(&w, "Wren").await;
    run(&w, &[&f1, &f2, &f3]).await;

    let unknown = "unknownagent";
    assert_eq!(
        standing(&w, unknown).await,
        json!({"id": unknown, "standing": false, "as_of": "2026-12-15T12:00:00Z"})
    );
    assert_eq!(
        standing(&w, lonely.id()).await["standing"],
        standing(&w, unknown).await["standing"]
    );
    let empty = json!({"items": [], "next": null});
    let mine = w
        .vouches
        .received(lonely.auth().as_deref(), None, RUN)
        .await;
    assert_eq!(mine.unwrap().body, empty);
    assert_eq!(
        w.vouches.vouched_by(unknown, None).await.unwrap().body,
        empty
    );
}

#[tokio::test]
async fn before_the_first_run_standing_is_false_with_no_as_of_and_vouched_by_is_not_built_yet() {
    let w = world();
    let a = born(&w, "Wren").await;
    assert_eq!(
        standing(&w, a.id()).await,
        json!({"id": a.id(), "standing": false, "as_of": null})
    );
    let mine = w.vouches.received(a.auth().as_deref(), None, NOW).await;
    assert_eq!(mine.unwrap_err().kind, "not-built-yet");
    assert_eq!(w.vouches.standing("Bad").await.unwrap_err().status, 400);
}

#[tokio::test]
async fn a_second_run_with_nothing_changed_writes_only_its_time() {
    let w = world();
    let [f1, f2, f3] = founders(&w).await;
    let a = born(&w, "Wren").await;
    for f in [&f1, &f2, &f3] {
        vouch(&w, f, &a).await;
    }
    run(&w, &[&f1, &f2, &f3]).await;
    let before = w.store.calls().puts;
    run_at(&*w.store, &[&f1, &f2, &f3], LONG_AGO, RUN + DAY).await;
    assert_eq!(w.store.calls().puts - before, 1);
}
