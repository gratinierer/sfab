//! Moving in, moving, moving out and the description: `.stories/4-life-in-the-world/spec.md`
//! "Homes and quarters" ("A home", "What's visible" and "Storage"), against the in-memory fake.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use common::*;
use serde_json::{Value, json};
use sfab_core::storage::{
    Bucket, Condition, Listed, MemoryStore, Metadata, Object, Store, StoreError,
};

#[tokio::test]
async fn moving_in_without_free_homes_takes_one_on_the_quarter_s_first_lane() {
    let w = world();
    let a = born(&w, "Wren").await;

    let (status, body) = move_to(&w, &a, "harbour", NOW).await.unwrap();

    assert_eq!(status, 201);
    assert_eq!(
        body,
        json!({"address": "harbour/lantern-lane/1", "moved_at": "2026-11-05T12:00:00Z", "description": null})
    );
    assert_eq!(own_home(&w, &a).await, body);
    let home = home_at(&w, "harbour/lantern-lane/1").await.unwrap();
    assert_eq!(home["id"], a.id());
    assert_eq!(home["standing"], false);
    let lane = w.homes.lane("harbour", "lantern-lane", None).await.unwrap();
    assert_eq!(
        lane.body,
        json!({"items": [{"home": 1, "id": a.id()}], "next": null})
    );
}

#[tokio::test]
async fn an_agent_that_never_moved_in_has_no_address() {
    let w = world();
    let a = born(&w, "Wren").await;
    assert_eq!(
        own_home(&w, &a).await,
        json!({"address": null, "moved_at": null, "description": null})
    );
    let out = move_out(&w, &a, NOW).await.unwrap();
    assert_eq!(out.0, 200);
}

#[tokio::test]
async fn an_unknown_quarter_or_a_missing_proof_is_refused_with_nothing_stored() {
    let w = world();
    let a = born(&w, "Wren").await;
    let before = w.store.keys(Bucket::Identities);

    assert_eq!(
        move_to(&w, &a, "atlantis", NOW).await,
        Err((400, "about:blank"))
    );
    let mut sent = move_call(&a, "harbour", NOW);
    sent.proof = None;
    assert_eq!(send_move(&w, &sent, NOW).await.unwrap_err().0, 400);
    let mut sent = move_call(&a, "harbour", NOW);
    sent.authorization = Some(format!("Bearer {}.wrong", a.id()));
    assert_eq!(send_move(&w, &sent, NOW).await.unwrap_err().0, 401);
    assert_eq!(w.store.keys(Bucket::Identities), before);
}

#[tokio::test]
async fn two_agents_moving_to_the_same_home_at_once_one_wins_and_the_other_tries_later() {
    let w = world();
    let (a, b) = (born(&w, "Wren").await, born(&w, "Moss").await);

    let first = move_to(&w, &a, "harbour", NOW).await.unwrap();
    let second = w
        .homes
        .move_home(&move_call(&b, "harbour", NOW).call(), NOW)
        .await
        .unwrap_err();

    assert_eq!(first.1["address"], "harbour/lantern-lane/1");
    assert_eq!((second.status, second.kind), (409, "try-later"));
    assert_eq!(second.extra["retry_after"], "2026-11-06T00:05:00Z");
    assert_eq!(own_home(&w, &b).await["address"], Value::Null);
    let home = home_at(&w, "harbour/lantern-lane/1").await.unwrap();
    assert_eq!(home["id"], a.id());
}

/// The fake, except that the first pointer written also rewrites the agent's `home.json`, as a
/// second move of the same agent winning the race would.
struct Racing {
    inner: MemoryStore,
    raced: AtomicBool,
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
        let pointer = key.starts_with("places/");
        self.inner
            .put(bucket, key, body, metadata, condition)
            .await?;
        if pointer && !self.raced.swap(true, Ordering::SeqCst) {
            let homes: Vec<String> = self
                .inner
                .keys(Bucket::Identities)
                .into_iter()
                .filter(|k| k.ends_with("/home.json"))
                .collect();
            for home in homes {
                let current = self.inner.object(Bucket::Identities, &home).unwrap();
                let put = self.inner.put(
                    Bucket::Identities,
                    &home,
                    current.body,
                    Metadata::new(),
                    Condition::Always,
                );
                put.await.unwrap();
            }
        }
        Ok(())
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
async fn of_two_moves_of_one_agent_at_once_the_one_that_loses_home_json_takes_back_its_pointer() {
    let store = Arc::new(Racing {
        inner: MemoryStore::new(),
        raced: AtomicBool::new(true),
    });
    let w = world_on(store.clone(), sfab_homes::quarters(), first);
    let a = born(&w, "Wren").await;
    move_to(&w, &a, "harbour", NOW).await.unwrap();
    store.raced.store(false, Ordering::SeqCst);

    let lost = move_to(&w, &a, "gardens", NOW + 31 * DAY).await;

    assert_eq!(lost, Err((409, "try-later")));
    let pointers = store
        .inner
        .list(Bucket::Identities, "places/")
        .await
        .unwrap();
    assert_eq!(pointers, vec!["places/harbour/lantern-lane/1.json"]);
    assert_eq!(own_home(&w, &a).await["address"], "harbour/lantern-lane/1");
}

#[tokio::test]
async fn a_retried_move_returns_the_move_with_200_and_rewrites_aboutme() {
    let w = world();
    let a = born(&w, "Wren").await;
    publish(&w, &a, json!({"name": true, "address": true})).await;
    let sent = move_call(&a, "harbour", NOW);
    let (status, first) = send_move(&w, &sent, NOW).await.unwrap();
    assert_eq!(status, 201);
    // The answer was lost, and so was the copy's write.
    let key = format!("identities/{}/aboutme.json", a.id());
    w.store
        .put(
            Bucket::Identities,
            &key,
            br#"{"id":"x"}"#.to_vec(),
            Metadata::new(),
            Condition::Always,
        )
        .await
        .unwrap();

    let again = send_move(&w, &sent, NOW + 10).await.unwrap();

    assert_eq!(again, (200, first));
    assert_eq!(
        aboutme(&w, &a).await.unwrap(),
        json!({"id": a.id(), "name": "Wren", "address": "harbour/lantern-lane/1"})
    );
    let pointers = w.store.list(Bucket::Identities, "places/").await.unwrap();
    assert_eq!(pointers.len(), 1, "no second home");
}

#[tokio::test]
async fn moving_again_within_30_days_is_too_soon_and_after_them_frees_the_old_address() {
    let w = world();
    let a = born(&w, "Wren").await;
    move_to(&w, &a, "harbour", NOW).await.unwrap();

    let early = w
        .homes
        .move_home(&move_call(&a, "gardens", NOW + DAY).call(), NOW + DAY)
        .await
        .unwrap_err();
    assert_eq!((early.status, early.kind), (409, "too-soon"));
    assert_eq!(early.extra["retry_after"], "2026-12-05T12:00:00Z");

    let later = NOW + 30 * DAY;
    let (status, moved) = move_to(&w, &a, "gardens", later).await.unwrap();
    assert_eq!(status, 200);
    assert_eq!(moved["address"], "gardens/rose-walk/1");
    assert_eq!(home_at(&w, "harbour/lantern-lane/1").await, Err(404));
    let pointers = w.store.list(Bucket::Identities, "places/").await.unwrap();
    assert_eq!(pointers, vec!["places/gardens/rose-walk/1.json"]);
}

#[tokio::test]
async fn moving_out_keeps_home_json_frees_the_address_and_moving_back_in_within_30_days_is_too_soon()
 {
    let w = world();
    let (a, b) = (born(&w, "Wren").await, born(&w, "Moss").await);
    move_to(&w, &a, "harbour", NOW).await.unwrap();

    let (status, out) = move_out(&w, &a, NOW + 40 * DAY).await.unwrap();

    assert_eq!(status, 200);
    assert_eq!(
        out,
        json!({"address": null, "moved_at": "2026-12-15T12:00:00Z", "description": null})
    );
    assert_eq!(own_home(&w, &a).await, out);
    assert_eq!(home_at(&w, "harbour/lantern-lane/1").await, Err(404));
    let back = move_to(&w, &a, "harbour", NOW + 41 * DAY).await;
    assert_eq!(back, Err((409, "too-soon")));
    // Its old address is free at once.
    let (_, taken) = move_to(&w, &b, "harbour", NOW + 41 * DAY).await.unwrap();
    assert_eq!(taken["address"], "harbour/lantern-lane/1");
    // And a retry of the move out changes nothing.
    assert_eq!(move_out(&w, &a, NOW + 42 * DAY).await.unwrap(), (200, out));
}

#[tokio::test]
async fn moving_changes_a_published_address_and_moving_out_removes_it() {
    let w = world();
    let a = born(&w, "Wren").await;
    publish(&w, &a, json!({"address": true})).await;
    assert_eq!(aboutme(&w, &a).await.unwrap(), json!({"id": a.id()}));

    move_to(&w, &a, "harbour", NOW).await.unwrap();
    assert_eq!(
        aboutme(&w, &a).await.unwrap()["address"],
        "harbour/lantern-lane/1"
    );
    move_to(&w, &a, "gardens", NOW + 30 * DAY).await.unwrap();
    assert_eq!(
        aboutme(&w, &a).await.unwrap()["address"],
        "gardens/rose-walk/1"
    );
    move_out(&w, &a, NOW + 60 * DAY).await.unwrap();
    assert_eq!(aboutme(&w, &a).await.unwrap(), json!({"id": a.id()}));
}

#[tokio::test]
async fn an_unpublished_address_stays_off_aboutme_but_publishing_it_later_shows_it() {
    let w = world();
    let a = born(&w, "Wren").await;
    publish(&w, &a, json!({"name": true})).await;
    move_to(&w, &a, "harbour", NOW).await.unwrap();
    assert_eq!(
        aboutme(&w, &a).await.unwrap(),
        json!({"id": a.id(), "name": "Wren"})
    );

    publish(&w, &a, json!({"name": true, "address": true})).await;

    assert_eq!(
        aboutme(&w, &a).await.unwrap()["address"],
        "harbour/lantern-lane/1"
    );
}

fn describe_call(a: &Agent, text: &str) -> Sent {
    describe_at(a, text, NOW)
}

fn describe_at(a: &Agent, text: &str, at: i64) -> Sent {
    let body = serde_json::to_vec(&json!({ "description": text })).unwrap();
    write(
        "PUT",
        "/v1/identity/self/home/description",
        a.auth(),
        &body,
        at,
    )
}

#[tokio::test]
async fn a_description_needs_a_home_is_at_most_2_kb_and_shows_on_the_home() {
    let w = world();
    let a = born(&w, "Wren").await;
    let refused = w
        .homes
        .describe(&describe_call(&a, "a quiet room").call(), NOW)
        .await
        .unwrap_err();
    assert_eq!((refused.status, refused.kind), (403, "no-home"));

    move_to(&w, &a, "harbour", NOW).await.unwrap();
    let too_long = "x".repeat(sfab_homes::MAX_DESCRIPTION_BYTES + 1);
    let long = w
        .homes
        .describe(&describe_call(&a, &too_long).call(), NOW)
        .await;
    assert_eq!(long.unwrap_err().status, 400);
    let reply = w
        .homes
        .describe(&describe_call(&a, "a quiet room").call(), NOW)
        .await
        .unwrap();

    assert_eq!(reply.body["description"], "a quiet room");
    let home = home_at(&w, "harbour/lantern-lane/1").await.unwrap();
    assert_eq!(home["description"], "a quiet room");
    // Kept across a move, and gone with the address once moved out.
    move_to(&w, &a, "gardens", NOW + 30 * DAY).await.unwrap();
    assert_eq!(own_home(&w, &a).await["description"], "a quiet room");
    move_out(&w, &a, NOW + 60 * DAY).await.unwrap();
    let refused = w
        .homes
        .describe(
            &describe_at(&a, "again", NOW + 60 * DAY).call(),
            NOW + 60 * DAY,
        )
        .await
        .unwrap_err();
    assert_eq!(refused.kind, "no-home");
}

#[tokio::test]
async fn a_home_whose_pointer_and_home_json_disagree_is_404_and_unknown_places_are_404() {
    let w = world();
    let a = born(&w, "Wren").await;
    move_to(&w, &a, "harbour", NOW).await.unwrap();
    // A ghost: a pointer naming Wren on another lane.
    w.store
        .put(
            Bucket::Identities,
            "places/harbour/rope-walk/3.json",
            json!({"id": a.id()}).to_string().into_bytes(),
            Metadata::new(),
            Condition::Always,
        )
        .await
        .unwrap();

    assert_eq!(home_at(&w, "harbour/rope-walk/3").await, Err(404));
    for address in [
        "harbour/lantern-lane/2",
        "harbour/lantern-lane/21",
        "harbour/lantern-lane/01",
        "harbour/nowhere/1",
        "atlantis/lantern-lane/1",
    ] {
        assert_eq!(home_at(&w, address).await, Err(404), "{address}");
    }
    for (quarter, lane) in [("harbour", "tide-street"), ("harbour", "x"), ("x", "x")] {
        let found = w.homes.lane(quarter, lane, None).await;
        assert_eq!(found.unwrap_err().status, 404, "{quarter}/{lane}");
    }
}

#[tokio::test]
async fn a_home_shows_the_resident_s_aboutme_when_published() {
    let w = world();
    let a = born(&w, "Wren").await;
    move_to(&w, &a, "harbour", NOW).await.unwrap();
    assert!(
        home_at(&w, "harbour/lantern-lane/1")
            .await
            .unwrap()
            .get("aboutme")
            .is_none()
    );

    publish(&w, &a, json!({"name": true})).await;

    let home = home_at(&w, "harbour/lantern-lane/1").await.unwrap();
    assert_eq!(home["aboutme"], json!({"id": a.id(), "name": "Wren"}));
}
