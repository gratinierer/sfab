//! Public profiles, friends and blocks, from `.stories/2-identity-api/spec.md` "Public profile",
//! "Friends and blocks" and "Testing", against the in-memory fake: issue 13's Done when.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::{self, Request};
use sfab_core::storage::{
    Bucket, Condition, Listed, MemoryStore, Metadata, Object, Store, StoreError,
};
use sfab_identity::{Identity, List};

const BITS: u32 = 8;
/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;

struct World {
    store: Arc<MemoryStore>,
    identity: Identity<MemoryStore>,
}

fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    World {
        identity: Identity::new(store.clone(), BITS),
        store,
    }
}

struct Sent {
    method: &'static str,
    path: String,
    authorization: Option<String>,
    proof: Option<String>,
    body: Vec<u8>,
}

impl Sent {
    fn call(&self) -> Call<'_> {
        Call {
            method: self.method,
            path: &self.path,
            authorization: self.authorization.as_deref(),
            proof: self.proof.as_deref(),
            body: &self.body,
        }
    }
}

/// A write with a correct proof.
fn write(method: &'static str, path: String, auth: Option<String>, body: &[u8]) -> Sent {
    let caller = sfab_core::credentials::caller_of(auth.as_deref()).unwrap();
    let request = Request {
        method,
        path: &path,
        caller: &caller,
        body,
    };
    Sent {
        method,
        proof: Some(proof::solve(&request, NOW, BITS)),
        path,
        authorization: auth,
        body: body.to_vec(),
    }
}

struct Agent {
    id: String,
    claim: String,
}

impl Agent {
    fn auth(&self) -> Option<String> {
        Some(format!("Bearer {}.{}", self.id, self.claim))
    }
}

async fn born(w: &World, name: &str) -> Agent {
    let body = format!(r#"{{"name":"{name}"}}"#);
    let sent = write("POST", "/v1/identity/birth".into(), None, body.as_bytes());
    let reply = w.identity.birth(&sent.call(), NOW).await.unwrap();
    Agent {
        id: reply.body["id"].as_str().unwrap().to_owned(),
        claim: reply.body["claim"].as_str().unwrap().to_owned(),
    }
}

fn status(result: &Result<Reply, Problem>) -> u16 {
    match result {
        Ok(reply) => reply.status,
        Err(problem) => problem.status,
    }
}

fn publication_call(a: &Agent, settings: Value) -> Sent {
    let body = serde_json::to_vec(&settings).unwrap();
    write(
        "PUT",
        "/v1/identity/self/publication".into(),
        a.auth(),
        &body,
    )
}

async fn publish(w: &World, a: &Agent, settings: Value) -> Result<Reply, Problem> {
    w.identity
        .publish(&publication_call(a, settings).call(), NOW)
        .await
}

async fn evolve(w: &World, a: &Agent, whoiam: &str) {
    let body = serde_json::to_vec(&json!({ "whoiam": whoiam })).unwrap();
    let sent = write("PUT", "/v1/identity/evolve".into(), a.auth(), &body);
    w.identity.evolve(&sent.call(), NOW).await.unwrap();
}

/// `aboutme`'s body, or its status when it's an error.
async fn aboutme(w: &World, id: &str) -> Result<Value, u16> {
    w.identity
        .aboutme(id)
        .await
        .map(|r| r.body)
        .map_err(|p| p.status)
}

#[tokio::test]
async fn every_field_starts_private_and_publishing_shows_only_the_public_ones() {
    let w = world();
    let a = born(&w, "Wren").await;
    evolve(&w, &a, "a bird").await;

    let settings = w.identity.read_publication(a.auth().as_deref(), NOW).await;
    assert_eq!(
        settings.unwrap().body,
        json!({"name": false, "whoiam": false, "friends": false})
    );
    assert_eq!(aboutme(&w, &a.id).await, Err(404));

    let reply = publish(&w, &a, json!({"name": true})).await.unwrap();
    assert_eq!(
        reply.body,
        json!({"name": true, "whoiam": false, "friends": false}),
        "a field left out is private"
    );
    assert_eq!(
        aboutme(&w, &a.id).await,
        Ok(json!({"id": a.id, "name": "Wren"}))
    );

    publish(&w, &a, json!({"whoiam": true})).await.unwrap();
    assert_eq!(
        aboutme(&w, &a.id).await,
        Ok(json!({"id": a.id, "whoiam": "a bird"}))
    );
    publish(&w, &a, json!({})).await.unwrap();
    assert_eq!(aboutme(&w, &a.id).await, Err(404));
}

fn identity_key(a: &Agent) -> String {
    format!("identities/{}/identity.json", a.id)
}

fn aboutme_key(a: &Agent) -> String {
    format!("identities/{}/aboutme.json", a.id)
}

/// Whether `whoiam` is visible to anyone: in the copy.
async fn whoiam_shown(w: &World, a: &Agent) -> bool {
    aboutme(w, &a.id)
        .await
        .is_ok_and(|body| body.get("whoiam").is_some())
}

#[tokio::test]
async fn making_a_field_public_saves_the_setting_first_so_a_failure_leaves_it_private() {
    for failing in ["identity", "aboutme"] {
        let w = world();
        let a = born(&w, "Wren").await;
        evolve(&w, &a, "a secret").await;
        let key = if failing == "identity" {
            identity_key(&a)
        } else {
            aboutme_key(&a)
        };
        w.store.fail_writes_under(Bucket::Identities, &key);

        let result = publish(&w, &a, json!({"whoiam": true})).await;

        assert_eq!(status(&result), 500, "{failing}");
        assert!(!whoiam_shown(&w, &a).await, "{failing}: whoiam shown");
    }
}

#[tokio::test]
async fn making_a_field_private_drops_it_from_the_copy_first_so_a_failure_leaves_it_private() {
    // The identity write failing: the copy was already rewritten without whoiam.
    let w = world();
    let a = born(&w, "Wren").await;
    evolve(&w, &a, "a secret").await;
    publish(&w, &a, json!({"name": true, "whoiam": true}))
        .await
        .unwrap();
    w.store
        .fail_writes_under(Bucket::Identities, &identity_key(&a));
    let result = publish(&w, &a, json!({"name": true})).await;
    assert_eq!(status(&result), 500);
    assert!(!whoiam_shown(&w, &a).await);
    assert_eq!(aboutme(&w, &a.id).await.unwrap()["name"], "Wren");

    // The copy write failing: nothing changes, so the setting still says public and the agent
    // retries; no state shows a field its setting calls private.
    let w = world();
    let a = born(&w, "Wren").await;
    evolve(&w, &a, "a secret").await;
    publish(&w, &a, json!({"whoiam": true})).await.unwrap();
    w.store
        .fail_writes_under(Bucket::Identities, &aboutme_key(&a));
    let result = publish(&w, &a, json!({})).await;
    assert_eq!(status(&result), 500);
    let settings = w
        .identity
        .read_publication(a.auth().as_deref(), NOW)
        .await
        .unwrap();
    assert_eq!(settings.body["whoiam"], true);
}

/// The fake, where another call rewrites `identity.json` just before each of the first `races`
/// conditional writes to it, as a concurrent `evolve` would.
struct Racing {
    inner: Arc<MemoryStore>,
    races: AtomicUsize,
    whoiam: &'static str,
}

impl Store for Racing {
    async fn get(&self, bucket: Bucket, key: &str) -> Result<Option<Object>, StoreError> {
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
        if key.ends_with("/identity.json")
            && matches!(condition, Condition::IfMatch(_))
            && self.races.load(Ordering::SeqCst) > 0
        {
            self.races.fetch_sub(1, Ordering::SeqCst);
            let current = self.inner.get(bucket, key).await?.unwrap();
            let mut theirs: Value = serde_json::from_slice(&current.body).unwrap();
            theirs["whoiam"] = json!(self.whoiam);
            let theirs = serde_json::to_vec(&theirs).unwrap();
            self.inner
                .put(bucket, key, theirs, Metadata::new(), Condition::Always)
                .await?;
        }
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

fn racing(w: &World, races: usize) -> Arc<Racing> {
    Arc::new(Racing {
        inner: w.store.clone(),
        races: races.into(),
        whoiam: "theirs",
    })
}

#[tokio::test]
async fn a_conflict_on_identity_json_retries_and_the_copy_comes_from_a_fresh_read() {
    let w = world();
    let a = born(&w, "Wren").await;
    evolve(&w, &a, "old").await;
    let store = racing(&w, 2);
    let identity = Identity::new(store.clone(), BITS);

    let reply = identity
        .publish(&publication_call(&a, json!({"whoiam": true})).call(), NOW)
        .await
        .unwrap();

    assert_eq!(reply.body["whoiam"], true);
    assert_eq!(store.races.load(Ordering::SeqCst), 0);
    // The concurrent whoiam survived, and the copy shows it, not the one read before the race.
    assert_eq!(aboutme(&w, &a.id).await.unwrap()["whoiam"], "theirs");
}

#[tokio::test]
async fn evolve_retries_a_conflict_without_undoing_a_publication_and_rewrites_the_copy() {
    let w = world();
    let a = born(&w, "Wren").await;
    publish(&w, &a, json!({"whoiam": true})).await.unwrap();
    evolve(&w, &a, "new").await;
    assert_eq!(aboutme(&w, &a.id).await.unwrap()["whoiam"], "new");

    let identity = Identity::new(racing(&w, 1), BITS);
    let body = serde_json::to_vec(&json!({"whoiam": "mine"})).unwrap();
    let sent = write("PUT", "/v1/identity/evolve".into(), a.auth(), &body);
    identity.evolve(&sent.call(), NOW).await.unwrap();

    assert_eq!(aboutme(&w, &a.id).await.unwrap()["whoiam"], "mine");
    let settings = w
        .identity
        .read_publication(a.auth().as_deref(), NOW)
        .await
        .unwrap();
    assert_eq!(settings.body["whoiam"], true, "evolve kept the publication");
}

#[tokio::test]
async fn endless_conflicts_end_in_500_with_nothing_published() {
    let w = world();
    let a = born(&w, "Wren").await;
    let identity = Identity::new(racing(&w, 100), BITS);
    let result = identity
        .publish(&publication_call(&a, json!({"name": true})).call(), NOW)
        .await;
    assert_eq!(status(&result), 500);
    assert_eq!(aboutme(&w, &a.id).await, Err(404));
}

#[tokio::test]
async fn publication_refuses_unknown_fields_and_non_booleans() {
    let w = world();
    let a = born(&w, "Wren").await;
    for bad in [
        json!({"nope": true}),
        json!({"name": "yes"}),
        json!([true]),
        json!(null),
    ] {
        let result = publish(&w, &a, bad.clone()).await;
        assert_eq!(status(&result), 400, "{bad}");
    }
}

#[tokio::test]
async fn an_unpublished_unknown_malformed_or_taken_down_agent_gets_the_same_404() {
    let w = world();
    let published = born(&w, "Wren").await;
    publish(&w, &published, json!({"name": true}))
        .await
        .unwrap();
    let unpublished = born(&w, "Moss").await;
    // A takedown deletes the copy straight away.
    let gone = born(&w, "Ash").await;
    publish(&w, &gone, json!({"name": true})).await.unwrap();
    w.store
        .delete(Bucket::Identities, &aboutme_key(&gone))
        .await
        .unwrap();

    let mut answers = Vec::new();
    for id in [
        unpublished.id.as_str(),
        "aaaaaaaaaaaa",
        "NOT-AN-ID",
        gone.id.as_str(),
    ] {
        let problem = w.identity.aboutme(id).await.unwrap_err();
        answers.push((problem.status, problem.kind, problem.title));
    }
    assert!(answers.iter().all(|a| a == &answers[0]), "{answers:?}");
    assert_eq!(answers[0].0, 404);
    assert!(w.identity.aboutme(&published.id).await.is_ok());
}

fn list_path(list: List) -> &'static str {
    match list {
        List::Friends => "friends",
        List::Blocked => "blocked",
    }
}

fn add_call(a: &Agent, list: List, other: &str) -> Sent {
    let path = format!("/v1/identity/self/{}/{other}", list_path(list));
    write("PUT", path, a.auth(), b"")
}

async fn add(w: &World, a: &Agent, list: List, other: &str) -> Result<Reply, Problem> {
    w.identity
        .add_contact(list, &add_call(a, list, other).call(), other, NOW)
        .await
}

async fn remove(w: &World, a: &Agent, list: List, other: &str) -> Result<Reply, Problem> {
    w.identity
        .remove_contact(list, a.auth().as_deref(), other, NOW)
        .await
}

/// Every ID in a list, following `next` to the end.
async fn listed(w: &World, a: &Agent, list: List) -> Vec<String> {
    let mut ids = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let reply = w
            .identity
            .list_contacts(list, a.auth().as_deref(), page.as_deref(), NOW)
            .await
            .unwrap();
        let items = reply.body["items"].as_array().unwrap();
        assert!(items.len() <= 20);
        ids.extend(items.iter().map(|e| e["id"].as_str().unwrap().to_owned()));
        match reply.body["next"].as_str() {
            Some(next) => page = Some(next.split_once("?page=").unwrap().1.to_owned()),
            None => return ids,
        }
    }
}

/// The `n`th of a run of distinct, well-formed IDs.
fn other_id(n: usize) -> String {
    let alphabet = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut id = *b"zzzzzzzzzzzz";
    id[10] = alphabet[n / 32];
    id[11] = alphabet[n % 32];
    String::from_utf8(id.to_vec()).unwrap()
}

#[tokio::test]
async fn adding_a_friend_is_safe_to_retry_and_removing_one_is_too() {
    let w = world();
    let a = born(&w, "Wren").await;
    let friend = other_id(0);

    let first = add(&w, &a, List::Friends, &friend).await.unwrap();
    assert_eq!(first.status, 201);
    assert_eq!(first.body["id"], friend.as_str());
    assert_eq!(first.body["added_at"], "2026-11-05T12:00:00Z");
    let again = add(&w, &a, List::Friends, &friend).await.unwrap();
    assert_eq!((again.status, again.body), (200, first.body));
    assert_eq!(listed(&w, &a, List::Friends).await, vec![friend.clone()]);

    for _ in 0..2 {
        let removed = remove(&w, &a, List::Friends, &friend).await.unwrap();
        assert_eq!(removed.status, 200);
    }
    assert_eq!(listed(&w, &a, List::Friends).await, Vec::<String>::new());
}

#[tokio::test]
async fn blocking_again_still_removes_a_friend_added_after_the_first_block() {
    let w = world();
    let a = born(&w, "Wren").await;
    let other = other_id(0);
    add(&w, &a, List::Blocked, &other).await.unwrap();
    add(&w, &a, List::Friends, &other).await.unwrap();

    let again = add(&w, &a, List::Blocked, &other).await.unwrap();

    assert_eq!(again.status, 200);
    assert_eq!(listed(&w, &a, List::Friends).await, Vec::<String>::new());
    assert_eq!(listed(&w, &a, List::Blocked).await, vec![other]);
}

#[tokio::test]
async fn each_list_holds_100_and_a_101st_gets_409_but_a_retry_still_200() {
    for list in [List::Friends, List::Blocked] {
        let w = world();
        let a = born(&w, "Wren").await;
        for n in 0..100 {
            assert_eq!(status(&add(&w, &a, list, &other_id(n)).await), 201);
        }
        let full = add(&w, &a, list, &other_id(100)).await.unwrap_err();
        assert_eq!((full.status, full.kind), (409, "list-full"));
        assert_eq!(status(&add(&w, &a, list, &other_id(99)).await), 200);

        let ids = listed(&w, &a, list).await;
        assert_eq!(ids, (0..100).map(other_id).collect::<Vec<_>>());
    }
}

#[tokio::test]
async fn blocking_a_published_friend_removes_it_from_the_list_and_the_copy() {
    let w = world();
    let a = born(&w, "Wren").await;
    let (friend, other) = (other_id(0), other_id(1));
    add(&w, &a, List::Friends, &friend).await.unwrap();
    publish(&w, &a, json!({"friends": true})).await.unwrap();
    add(&w, &a, List::Friends, &other).await.unwrap();
    assert_eq!(
        aboutme(&w, &a.id).await.unwrap()["friends"],
        json!([friend, other])
    );

    add(&w, &a, List::Blocked, &friend).await.unwrap();

    assert_eq!(aboutme(&w, &a.id).await.unwrap()["friends"], json!([other]));
    assert_eq!(listed(&w, &a, List::Friends).await, vec![other.clone()]);
    assert_eq!(listed(&w, &a, List::Blocked).await, vec![friend.clone()]);

    remove(&w, &a, List::Friends, &other).await.unwrap();
    assert_eq!(aboutme(&w, &a.id).await.unwrap()["friends"], json!([]));
}

#[tokio::test]
async fn a_leaving_friend_leaves_the_copy_first_so_a_failure_keeps_it_out() {
    let w = world();
    let a = born(&w, "Wren").await;
    let friend = other_id(0);
    add(&w, &a, List::Friends, &friend).await.unwrap();
    publish(&w, &a, json!({"friends": true})).await.unwrap();
    w.store.fail_writes_under(
        Bucket::Identities,
        &format!("identities/{}/contacts.json", a.id),
    );

    let result = add(&w, &a, List::Blocked, &friend).await;

    assert_eq!(status(&result), 500);
    assert_eq!(aboutme(&w, &a.id).await.unwrap()["friends"], json!([]));
}

#[tokio::test]
async fn a_new_friend_reaches_the_copy_only_after_it_is_saved() {
    let w = world();
    let a = born(&w, "Wren").await;
    publish(&w, &a, json!({"friends": true})).await.unwrap();
    w.store.fail_writes_under(
        Bucket::Identities,
        &format!("identities/{}/contacts.json", a.id),
    );
    let result = add(&w, &a, List::Friends, &other_id(0)).await;
    assert_eq!(status(&result), 500);
    assert_eq!(aboutme(&w, &a.id).await.unwrap()["friends"], json!([]));
}

#[tokio::test]
async fn unpublished_friends_stay_out_of_the_copy() {
    let w = world();
    let a = born(&w, "Wren").await;
    publish(&w, &a, json!({"name": true})).await.unwrap();
    add(&w, &a, List::Friends, &other_id(0)).await.unwrap();
    assert_eq!(
        aboutme(&w, &a.id).await,
        Ok(json!({"id": a.id, "name": "Wren"}))
    );
}

#[tokio::test]
async fn a_malformed_id_gets_400_and_contacts_need_credentials() {
    let w = world();
    let a = born(&w, "Wren").await;
    for list in [List::Friends, List::Blocked] {
        assert_eq!(status(&add(&w, &a, list, "NOT-AN-ID").await), 400);
        assert_eq!(status(&remove(&w, &a, list, "short").await), 400);
        let anonymous = w.identity.list_contacts(list, None, None, NOW).await;
        assert_eq!(status(&anonymous), 401);
        let removed = w
            .identity
            .remove_contact(list, None, &other_id(0), NOW)
            .await;
        assert_eq!(status(&removed), 401);
    }
}

#[tokio::test]
async fn a_contacts_write_without_a_valid_proof_gets_400_with_nothing_stored() {
    let w = world();
    let a = born(&w, "Wren").await;
    let before = w.store.keys(Bucket::Identities);
    for list in [List::Friends, List::Blocked] {
        let mut sent = add_call(&a, list, &other_id(0));
        sent.proof = None;
        let result = w
            .identity
            .add_contact(list, &sent.call(), &other_id(0), NOW)
            .await;
        assert_eq!(result.unwrap_err().extra["required_bits"], BITS);
        // Made for another ID, which the path names.
        let mut sent = add_call(&a, list, &other_id(1));
        sent.path = add_call(&a, list, &other_id(0)).path;
        let result = w
            .identity
            .add_contact(list, &sent.call(), &other_id(0), NOW)
            .await;
        assert_eq!(result.unwrap_err().extra["required_bits"], BITS);
    }
    let mut sent = publication_call(&a, json!({"name": true}));
    sent.proof = None;
    let result = w.identity.publish(&sent.call(), NOW).await;
    assert_eq!(result.unwrap_err().extra["required_bits"], BITS);
    assert_eq!(w.store.keys(Bucket::Identities), before);
}
