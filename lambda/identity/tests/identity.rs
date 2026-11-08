//! The identity unit tests from `.stories/2-identity-api/spec.md` "Testing", against the in-memory
//! fake: issue 06's Done when.

use std::sync::Arc;

use serde_json::Value;
use sfab_core::gate::{Call, MAX_BODY_BYTES};
use sfab_core::id::{AgentId, Caller};
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::{self, Request};
use sfab_core::storage::{Bucket, Condition, MemoryStore, Store};
use sfab_core::takedown::{Action, Phase};
use sfab_genesis::testing as genesis;
use sfab_identity::{Identity, takedown};

/// Enough bits that a proof for another request almost never passes by chance.
const BITS: u32 = 12;
/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;
const DAY: i64 = 86_400;

struct World {
    store: Arc<MemoryStore>,
    identity: Identity<MemoryStore>,
}

fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    World {
        identity: Identity::new(store.clone(), BITS, genesis::ip_rule()),
        store,
    }
}

struct Sent {
    method: &'static str,
    path: &'static str,
    authorization: Option<String>,
    proof: Option<String>,
    body: Vec<u8>,
}

impl Sent {
    fn call(&self) -> Call<'_> {
        Call {
            method: self.method,
            path: self.path,
            authorization: self.authorization.as_deref(),
            proof: self.proof.as_deref(),
            body: &self.body,
            viewer: None,
        }
    }
}

/// A write with a correct proof, solved at `now`.
fn write(
    method: &'static str,
    path: &'static str,
    auth: Option<String>,
    body: &[u8],
    now: i64,
) -> Sent {
    let caller = sfab_core::credentials::caller_of(auth.as_deref()).expect("a well-formed header");
    let request = Request {
        method,
        path,
        caller: &caller,
        body,
    };
    Sent {
        method,
        path,
        proof: Some(proof::solve(&request, now, BITS)),
        authorization: auth,
        body: body.to_vec(),
    }
}

fn bearer(id: &str, secret: &str) -> Option<String> {
    Some(format!("Bearer {id}.{secret}"))
}

/// A `birth` with a fresh `--no-ip-limit` genesis code seeded in `w`'s store.
async fn birth_call(w: &World, name: &str, now: i64) -> Sent {
    let code = genesis::seed_exempt_code(&*w.store).await;
    birth_with(name, &code, now)
}

fn birth_with(name: &str, code: &str, now: i64) -> Sent {
    let body = serde_json::json!({ "name": name, "genesis_code": code });
    write(
        "POST",
        "/v1/identity/birth",
        None,
        &serde_json::to_vec(&body).unwrap(),
        now,
    )
}

fn evolve_call(id: &str, claim: &str, whoiam: &str, now: i64) -> Sent {
    let body = serde_json::to_vec(&serde_json::json!({ "whoiam": whoiam })).unwrap();
    write("PUT", "/v1/identity/evolve", bearer(id, claim), &body, now)
}

fn reclaim_call(id: &str, recovery: &str, now: i64) -> Sent {
    write(
        "POST",
        "/v1/identity/reclaim",
        bearer(id, recovery),
        b"",
        now,
    )
}

struct Born {
    id: String,
    claim: String,
    recovery: String,
}

async fn born(w: &World, name: &str) -> Born {
    let reply = w
        .identity
        .birth(&birth_call(w, name, NOW).await.call(), NOW)
        .await
        .expect("birth");
    assert_eq!(reply.status, 201);
    let s = |k: &str| reply.body[k].as_str().unwrap().to_owned();
    Born {
        id: s("id"),
        claim: s("claim"),
        recovery: s("recovery"),
    }
}

fn status(result: &Result<Reply, Problem>) -> u16 {
    match result {
        Ok(reply) => reply.status,
        Err(problem) => problem.status,
    }
}

/// Every key in both buckets with its body, to prove nothing was stored.
fn snapshot(store: &MemoryStore) -> Vec<(String, Vec<u8>)> {
    [Bucket::Claims, Bucket::Identities]
        .into_iter()
        .flat_map(|b| {
            store
                .keys(b)
                .into_iter()
                .map(move |k| (k.clone(), store.object(b, &k).unwrap().body))
        })
        .collect()
}

#[tokio::test]
async fn birth_creates_the_identity_from_the_proofs_hash_and_stores_only_hashes() {
    let w = world();
    let sent = birth_call(&w, "Wren", NOW).await;
    let reply = w.identity.birth(&sent.call(), NOW).await.unwrap();
    assert_eq!(reply.status, 201);
    let body = &reply.body;

    let request = Request {
        method: "POST",
        path: "/v1/identity/birth",
        caller: &Caller::Anonymous,
        body: &sent.body,
    };
    let parsed = proof::Proof::parse(sent.proof.as_deref().unwrap()).unwrap();
    let expected = AgentId::from_hash(&proof::solution_hash(&request, &parsed));
    assert_eq!(body["id"], expected.as_str());
    assert_eq!(body["constitution"], "/v1/world/constitution");
    assert!(body["notice"].as_str().unwrap().contains("only once"));

    let id = body["id"].as_str().unwrap();
    let claim = String::from_utf8(
        w.store
            .object(Bucket::Claims, &format!("claims/{id}/claim.json"))
            .unwrap()
            .body,
    )
    .unwrap();
    for secret in ["claim", "recovery"] {
        assert!(
            !claim.contains(body[secret].as_str().unwrap()),
            "{secret} stored in clear"
        );
    }
    assert!(claim.contains(r#""born_at":"2026-11-05T12:00:00Z""#));
    assert!(
        w.store
            .object(
                Bucket::Identities,
                &format!("identities/{id}/identity.json")
            )
            .is_some()
    );
    assert!(
        w.store
            .object(Bucket::Identities, &format!("vitals/{id}.json"))
            .is_some()
    );
}

#[tokio::test]
async fn a_birth_replay_gets_409_and_changes_nothing() {
    let w = world();
    let code = genesis::seed_exempt_code(&*w.store).await;
    let sent = birth_with("Wren", &code, NOW);
    w.identity.birth(&sent.call(), NOW).await.unwrap();
    // The used code put back, so the replay reaches the claim's `If-None-Match: *`.
    genesis::reseed_exempt_code(&*w.store, &code).await;
    let before = snapshot(&w.store);
    let replay = w.identity.birth(&sent.call(), NOW).await.unwrap_err();
    assert_eq!((replay.status, replay.kind), (409, "already-exists"));
    assert_eq!(snapshot(&w.store), before);
}

#[tokio::test]
async fn a_plain_birth_replay_meets_its_used_code_and_gets_403() {
    let w = world();
    let sent = birth_call(&w, "Wren", NOW).await;
    w.identity.birth(&sent.call(), NOW).await.unwrap();
    let before = snapshot(&w.store);
    let replay = w.identity.birth(&sent.call(), NOW).await.unwrap_err();
    assert_eq!((replay.status, replay.kind), (403, "invalid-genesis-code"));
    assert_eq!(snapshot(&w.store), before);
}

#[tokio::test]
async fn a_birth_with_a_bad_code_writes_nothing() {
    let w = world();
    let before = snapshot(&w.store);
    let sent = birth_with("Wren", &sfab_genesis::new_code(), NOW);
    let err = w.identity.birth(&sent.call(), NOW).await.unwrap_err();
    assert_eq!((err.status, err.kind), (403, "invalid-genesis-code"));
    assert_eq!(snapshot(&w.store), before);
}

#[tokio::test]
async fn a_birth_too_soon_from_one_address_gets_409_and_writes_nothing() {
    let w = world();
    let viewer = Some("198.51.100.7:46532");
    let first = genesis::seed_code(&*w.store).await;
    let sent = birth_with("Wren", &first, NOW);
    let call = Call {
        viewer,
        ..sent.call()
    };
    assert_eq!(w.identity.birth(&call, NOW).await.unwrap().status, 201);

    let second = genesis::seed_code(&*w.store).await;
    let sent = birth_with("Robin", &second, NOW + 60);
    let before = snapshot(&w.store);
    let call = Call {
        viewer,
        ..sent.call()
    };
    let err = w.identity.birth(&call, NOW + 60).await.unwrap_err();
    assert_eq!((err.status, err.kind), (409, "too-soon"));
    assert_eq!(snapshot(&w.store), before);
}

#[tokio::test]
async fn an_ordinary_code_without_a_viewer_address_gets_500_and_writes_nothing() {
    let w = world();
    let code = genesis::seed_code(&*w.store).await;
    let before = snapshot(&w.store);
    let sent = birth_with("Wren", &code, NOW);
    for viewer in [None, Some("not-an-address")] {
        let call = Call {
            viewer,
            ..sent.call()
        };
        assert_eq!(
            status(&w.identity.birth(&call, NOW).await),
            500,
            "{viewer:?}"
        );
        assert_eq!(snapshot(&w.store), before);
    }
    let exempt = birth_call(&w, "Robin", NOW).await;
    assert_eq!(status(&w.identity.birth(&exempt.call(), NOW).await), 201);
}

#[tokio::test]
async fn self_reads_the_identity_and_evolve_replaces_whoiam_keeping_the_name() {
    let w = world();
    let a = born(&w, "Wren").await;
    let auth = bearer(&a.id, &a.claim);

    let me = w.identity.read_self(auth.as_deref(), NOW).await.unwrap();
    assert_eq!(me.body, serde_json::json!({"id": a.id, "name": "Wren"}));

    let evolved = w
        .identity
        .evolve(&evolve_call(&a.id, &a.claim, "a bird", NOW).call(), NOW)
        .await
        .unwrap();
    assert_eq!(evolved.body["whoiam"], "a bird");
    let me = w.identity.read_self(auth.as_deref(), NOW).await.unwrap();
    assert_eq!(
        (me.body["name"].clone(), me.body["whoiam"].clone()),
        ("Wren".into(), "a bird".into())
    );

    w.identity
        .evolve(&evolve_call(&a.id, &a.claim, "", NOW).call(), NOW)
        .await
        .unwrap();
    let me = w.identity.read_self(auth.as_deref(), NOW).await.unwrap();
    assert_eq!(me.body.get("whoiam"), None, "an empty whoiam clears it");
}

#[tokio::test]
async fn wrong_or_unknown_credentials_get_401() {
    let w = world();
    let a = born(&w, "Wren").await;
    let other = sfab_core::credentials::new_secret();
    for auth in [
        None,
        Some("Bearer junk".to_owned()),
        bearer(&a.id, &other),
        bearer(&a.id, &a.recovery),
        bearer("aaaaaaaaaaaa", &a.claim),
    ] {
        assert_eq!(
            status(&w.identity.read_self(auth.as_deref(), NOW).await),
            401,
            "{auth:?}"
        );
    }
}

#[tokio::test]
async fn reclaim_invalidates_the_old_secrets_and_the_code_works_once() {
    let w = world();
    let a = born(&w, "Wren").await;
    let reply = w
        .identity
        .reclaim(&reclaim_call(&a.id, &a.recovery, NOW).call(), NOW)
        .await
        .unwrap();
    assert_eq!(reply.status, 200);
    assert_eq!(reply.body["id"], a.id.as_str());
    let (claim, recovery) = (
        reply.body["claim"].as_str().unwrap(),
        reply.body["recovery"].as_str().unwrap(),
    );

    assert_eq!(
        status(
            &w.identity
                .read_self(bearer(&a.id, &a.claim).as_deref(), NOW)
                .await
        ),
        401
    );
    assert_eq!(
        status(
            &w.identity
                .read_self(bearer(&a.id, claim).as_deref(), NOW)
                .await
        ),
        200
    );
    // A second use of the old code, with a fresh proof.
    let again = reclaim_call(&a.id, &a.recovery, NOW + 1);
    assert_eq!(
        status(&w.identity.reclaim(&again.call(), NOW + 1).await),
        401
    );
    // The recovery code can do nothing but reclaim.
    assert_eq!(
        status(
            &w.identity
                .read_self(bearer(&a.id, recovery).as_deref(), NOW)
                .await
        ),
        401
    );
    let next = reclaim_call(&a.id, recovery, NOW + 2);
    assert_eq!(
        status(&w.identity.reclaim(&next.call(), NOW + 2).await),
        200
    );
}

#[tokio::test]
async fn reclaim_keeps_born_at() {
    let w = world();
    let a = born(&w, "Wren").await;
    let later = NOW + 5 * DAY;
    w.identity
        .reclaim(&reclaim_call(&a.id, &a.recovery, later).call(), later)
        .await
        .unwrap();
    let claim = w
        .store
        .object(Bucket::Claims, &format!("claims/{}/claim.json", a.id))
        .unwrap();
    assert!(
        String::from_utf8(claim.body)
            .unwrap()
            .contains("2026-11-05T12:00:00Z")
    );
}

#[tokio::test]
async fn every_write_without_a_valid_proof_gets_400_with_nothing_stored() {
    let w = world();
    let a = born(&w, "Wren").await;
    let before = snapshot(&w.store);
    let claim_auth = bearer(&a.id, &a.claim);
    let b = born(&w, "Robin").await;
    let before_b = snapshot(&w.store);
    assert_ne!(before, before_b);

    let mut cases: Vec<(&str, Sent)> = Vec::new();
    for (name, mut sent) in [
        ("birth", birth_call(&w, "Finch", NOW).await),
        ("evolve", evolve_call(&a.id, &a.claim, "x", NOW)),
        ("reclaim", reclaim_call(&a.id, &a.recovery, NOW)),
    ] {
        let good_proof = sent.proof.clone();
        sent.proof = None;
        cases.push((
            name,
            Sent {
                proof: None,
                ..clone(&sent)
            },
        ));
        // Made for another body.
        let mut other_body = clone(&sent);
        other_body.proof = good_proof.clone();
        other_body.body.extend_from_slice(b" ");
        cases.push((name, other_body));
        // Made for another route.
        let mut other_path = clone(&sent);
        other_path.proof = good_proof.clone();
        other_path.path = "/v1/identity/other";
        cases.push((name, other_path));
        // A stale one.
        let mut stale = clone(&sent);
        stale.proof = Some(format!("{}:0", NOW - 301));
        cases.push((name, stale));
    }
    // Made for another caller: b's proof under a's credentials, with the same body.
    let for_b = evolve_call(&b.id, &b.claim, "x", NOW);
    cases.push((
        "evolve",
        Sent {
            authorization: claim_auth.clone(),
            ..clone(&for_b)
        },
    ));

    let before_b = snapshot(&w.store);
    for (name, sent) in &cases {
        let result = match *name {
            "birth" => w.identity.birth(&sent.call(), NOW).await,
            "evolve" => w.identity.evolve(&sent.call(), NOW).await,
            _ => w.identity.reclaim(&sent.call(), NOW).await,
        };
        let problem = result.expect_err(name);
        assert_eq!(problem.status, 400, "{name}");
        assert_eq!(problem.extra["required_bits"], BITS, "{name}");
        assert_eq!(problem.extra["server_time"], NOW, "{name}");
        assert_eq!(snapshot(&w.store), before_b, "{name} stored something");
    }
}

fn clone(s: &Sent) -> Sent {
    Sent {
        method: s.method,
        path: s.path,
        authorization: s.authorization.clone(),
        proof: s.proof.clone(),
        body: s.body.clone(),
    }
}

#[tokio::test]
async fn the_largest_whoiam_that_fits_32_kb_is_stored_and_one_byte_more_gets_400() {
    let w = world();
    let a = born(&w, "Wren").await;
    let overhead = br#"{"whoiam":""}"#.len();
    let fits = "a".repeat(MAX_BODY_BYTES - overhead);
    let ok = w
        .identity
        .evolve(&evolve_call(&a.id, &a.claim, &fits, NOW).call(), NOW)
        .await
        .unwrap();
    assert_eq!(ok.body["whoiam"].as_str().unwrap().len(), fits.len());

    let over = format!("{fits}a");
    let err = w
        .identity
        .evolve(&evolve_call(&a.id, &a.claim, &over, NOW).call(), NOW)
        .await
        .unwrap_err();
    assert_eq!(err.status, 400);
    assert!(
        !err.extra.contains_key("required_bits"),
        "it isn't the proof's fault"
    );
}

#[tokio::test]
async fn who_is_calling_is_checked_before_the_body() {
    let w = world();
    let a = born(&w, "Wren").await;
    let wrong = bearer(&a.id, &sfab_core::credentials::new_secret());
    let evolve = write(
        "PUT",
        "/v1/identity/evolve",
        wrong.clone(),
        b"not json",
        NOW,
    );
    assert_eq!(status(&w.identity.evolve(&evolve.call(), NOW).await), 401);
    let reclaim = write("POST", "/v1/identity/reclaim", wrong, b"{}", NOW);
    assert_eq!(status(&w.identity.reclaim(&reclaim.call(), NOW).await), 401);
}

#[tokio::test]
async fn malformed_bodies_get_400() {
    let w = world();
    let a = born(&w, "Wren").await;
    for body in [
        &b"not json"[..],
        br#"{}"#,
        br#"{"name":""}"#,
        br#"{"name":"x","extra":1}"#,
    ] {
        let sent = write("POST", "/v1/identity/birth", None, body, NOW);
        assert_eq!(status(&w.identity.birth(&sent.call(), NOW).await), 400);
    }
    let long = "n".repeat(129);
    let sent = birth_call(&w, &long, NOW).await;
    assert_eq!(status(&w.identity.birth(&sent.call(), NOW).await), 400);
    let evolve = write(
        "PUT",
        "/v1/identity/evolve",
        bearer(&a.id, &a.claim),
        br#"{"name":"x"}"#,
        NOW,
    );
    assert_eq!(status(&w.identity.evolve(&evolve.call(), NOW).await), 400);
    let reclaim = write(
        "POST",
        "/v1/identity/reclaim",
        bearer(&a.id, &a.recovery),
        b"{}",
        NOW,
    );
    assert_eq!(status(&w.identity.reclaim(&reclaim.call(), NOW).await), 400);
}

/// Runs this crate's takedown list the simple way, in phase order. Issue 08's program adds the
/// wait, the rerun and the dry run.
async fn take_down(store: &MemoryStore, id: &AgentId) {
    let mut locations = takedown(id);
    locations.sort_by_key(|l| l.phase);
    for l in locations {
        match l.action {
            Action::Delete => store.delete(l.bucket, &l.key).await.unwrap(),
            Action::DeletePrefix => {
                for key in store.list(l.bucket, &l.key).await.unwrap() {
                    store.delete(l.bucket, &key).await.unwrap();
                }
            }
            Action::Marker(metadata_of) => {
                let current = store.get(l.bucket, &l.key).await.unwrap().unwrap();
                let metadata = metadata_of(&current.body).expect("a claim, not a marker yet");
                store
                    .put(l.bucket, &l.key, Vec::new(), metadata, Condition::Always)
                    .await
                    .unwrap();
            }
        }
    }
}

#[tokio::test]
async fn the_takedown_list_leaves_only_a_marker_that_keeps_born_at() {
    let w = world();
    let code = genesis::seed_exempt_code(&*w.store).await;
    let sent = birth_with("Wren", &code, NOW);
    let reply = w.identity.birth(&sent.call(), NOW).await.unwrap();
    let id = AgentId::parse(reply.body["id"].as_str().unwrap()).unwrap();
    let claim = reply.body["claim"].as_str().unwrap();
    w.identity
        .evolve(&evolve_call(id.as_str(), claim, "x", NOW).call(), NOW)
        .await
        .unwrap();
    // A public copy, which the takedown list must cover; `tests/profile.rs` and the takedown
    // program's tests write it through `publish`.
    w.store
        .put(
            Bucket::Identities,
            &format!("identities/{id}/aboutme.json"),
            b"{}".to_vec(),
            Default::default(),
            Condition::Always,
        )
        .await
        .unwrap();

    let phases: Vec<Phase> = takedown(&id).iter().map(|l| l.phase).collect();
    assert_eq!(phases.first(), Some(&Phase::First));
    assert_eq!(phases.last(), Some(&Phase::Last));

    take_down(&w.store, &id).await;
    // Only the genesis code's batch entry stays, which holds nothing of the agent.
    let left: Vec<String> = w.store.keys(Bucket::Identities);
    assert_eq!(left.len(), 1, "{left:?}");
    assert!(left[0].starts_with("genesis/batches/"), "{left:?}");
    assert_eq!(
        w.store.keys(Bucket::Claims),
        vec![format!("claims/{id}/claim.json")]
    );
    let marker = w
        .store
        .object(Bucket::Claims, &format!("claims/{id}/claim.json"))
        .unwrap();
    assert!(marker.body.is_empty());
    assert_eq!(marker.metadata["born_at"], "2026-11-05T12:00:00Z");

    // A marker gets 401 everywhere, and birth on it gets 409, so the ID can't be recreated.
    assert_eq!(
        status(
            &w.identity
                .read_self(bearer(id.as_str(), claim).as_deref(), NOW)
                .await
        ),
        401
    );
    assert_eq!(
        status(
            &w.identity
                .evolve(&evolve_call(id.as_str(), claim, "y", NOW).call(), NOW)
                .await
        ),
        401
    );
    let recovery = reply.body["recovery"].as_str().unwrap();
    assert_eq!(
        status(
            &w.identity
                .reclaim(&reclaim_call(id.as_str(), recovery, NOW).call(), NOW)
                .await
        ),
        401
    );
    // The used code put back, so the replay reaches the marker.
    genesis::reseed_exempt_code(&*w.store, &code).await;
    let again = w.identity.birth(&sent.call(), NOW).await.unwrap_err();
    assert_eq!((again.status, again.kind), (409, "already-exists"));
}

#[tokio::test]
async fn a_claim_with_no_identity_gets_404() {
    let w = world();
    let a = born(&w, "Wren").await;
    w.store
        .delete(
            Bucket::Identities,
            &format!("identities/{}/identity.json", a.id),
        )
        .await
        .unwrap();
    assert_eq!(
        status(
            &w.identity
                .read_self(bearer(&a.id, &a.claim).as_deref(), NOW)
                .await
        ),
        404
    );
    assert_eq!(
        status(
            &w.identity
                .evolve(&evolve_call(&a.id, &a.claim, "x", NOW).call(), NOW)
                .await
        ),
        404
    );
    assert_eq!(
        status(
            &w.identity
                .reclaim(&reclaim_call(&a.id, &a.recovery, NOW).call(), NOW)
                .await
        ),
        404
    );
    assert!(
        w.store
            .object(
                Bucket::Identities,
                &format!("identities/{}/identity.json", a.id)
            )
            .is_none(),
        "evolve never creates one"
    );
}

#[tokio::test]
async fn the_first_call_of_a_day_updates_vitals_and_later_ones_dont() {
    let w = world();
    let a = born(&w, "Wren").await;
    let key = format!("vitals/{}.json", a.id);
    let auth = bearer(&a.id, &a.claim);
    assert_eq!(
        w.store.writes_to(Bucket::Identities, &key),
        1,
        "birth writes it"
    );

    w.identity
        .read_self(auth.as_deref(), NOW + 60)
        .await
        .unwrap();
    w.identity
        .evolve(
            &evolve_call(&a.id, &a.claim, "x", NOW + 120).call(),
            NOW + 120,
        )
        .await
        .unwrap();
    assert_eq!(w.store.writes_to(Bucket::Identities, &key), 1, "same day");

    let tomorrow = NOW + DAY;
    w.identity
        .read_self(auth.as_deref(), tomorrow)
        .await
        .unwrap();
    w.identity
        .read_self(auth.as_deref(), tomorrow + 60)
        .await
        .unwrap();
    assert_eq!(w.store.writes_to(Bucket::Identities, &key), 2);
    let vitals: Value =
        serde_json::from_slice(&w.store.object(Bucket::Identities, &key).unwrap().body).unwrap();
    assert_eq!(
        vitals,
        serde_json::json!({"born_at": "2026-11-05T12:00:00Z"})
    );
}

#[tokio::test]
async fn a_failed_vitals_write_doesnt_fail_the_call() {
    let w = world();
    w.store.fail_writes_under(Bucket::Identities, "vitals/");
    let a = born(&w, "Wren").await;
    let auth = bearer(&a.id, &a.claim);
    let tomorrow = NOW + DAY;
    assert_eq!(
        status(&w.identity.read_self(auth.as_deref(), tomorrow).await),
        200
    );
    assert_eq!(
        w.store
            .writes_to(Bucket::Identities, &format!("vitals/{}.json", a.id)),
        2
    );
}

#[tokio::test]
async fn a_failed_claim_write_stores_nothing_else() {
    let w = world();
    w.store.fail_writes_under(Bucket::Claims, "claims/");
    let sent = birth_call(&w, "Wren", NOW).await;
    let before = w.store.keys(Bucket::Identities);
    let err = w.identity.birth(&sent.call(), NOW).await.unwrap_err();
    assert_eq!(err.status, 500);
    assert_eq!(w.store.keys(Bucket::Identities), before);
}

#[tokio::test]
async fn the_residents_are_every_claim_except_takedown_markers() {
    let w = world();
    let wren = born(&w, "Wren").await;
    let robin = born(&w, "Robin").await;
    let gone = born(&w, "Gone").await;
    take_down(&w.store, &AgentId::parse(&gone.id).unwrap()).await;
    // A claim orphaned by a failed birth still counts (spec.md "World").
    w.store
        .delete(
            Bucket::Identities,
            &format!("identities/{}/identity.json", robin.id),
        )
        .await
        .unwrap();

    let residents = sfab_identity::residents(&*w.store).await.unwrap();

    let mut expected = vec![wren.id, robin.id];
    expected.sort();
    let found: Vec<String> = residents.iter().map(|id| id.as_str().to_owned()).collect();
    assert_eq!(found, expected);
}
