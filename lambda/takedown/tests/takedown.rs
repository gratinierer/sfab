//! The takedown program's tests, from issue 08's Done when, against `core`'s in-memory fake,
//! filled through each crate's own write functions.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use sfab_academy::Academy;
use sfab_core::gate::Call;
use sfab_core::id::AgentId;
use sfab_core::proof::{self, Request};
use sfab_core::storage::{
    Bucket, Condition, Listed, MemoryStore, Metadata, Object, Store, StoreError,
};
use sfab_genesis::testing as genesis;
use sfab_homes::Homes;
use sfab_identity::{Identity, List};
use sfab_messaging::Messaging;
use sfab_square::Square;
use sfab_takedown::{Mode, execute, plan};
use sfab_vouches::Vouches;

const BITS: u32 = 8;
/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;

type Answer = Result<serde_json::Value, (u16, &'static str)>;

struct World {
    store: Arc<MemoryStore>,
    identity: Arc<Identity<MemoryStore>>,
    messaging: Messaging<MemoryStore>,
    vouches: Vouches<MemoryStore>,
    homes: Homes<MemoryStore>,
    square: Square<MemoryStore>,
    academy: Academy<MemoryStore>,
}

fn world() -> World {
    let store = Arc::new(MemoryStore::new());
    let identity = Arc::new(Identity::new(store.clone(), BITS, genesis::genesis(&store)));
    let map = sfab_homes::quarters();
    World {
        messaging: Messaging::new(store.clone(), identity.clone(), BITS),
        vouches: Vouches::new(store.clone(), identity.clone(), BITS),
        homes: Homes::new(store.clone(), identity.clone(), BITS).with(map, first),
        square: Square::new(store.clone(), identity.clone(), BITS),
        academy: Academy::new(store.clone(), identity.clone(), BITS),
        identity,
        store,
    }
}

/// The first free home every time.
fn first(_: usize) -> usize {
    0
}

/// The agent moves into the first quarter whose first home is free, and gets that home.
async fn move_in(w: &World, id: &AgentId, claim: &str, now: i64) -> String {
    let taken = w.store.list(Bucket::Identities, "places/").await.unwrap();
    let quarter = sfab_homes::quarters()
        .iter()
        .find(|q| !taken.contains(&format!("places/{}/{}/1.json", q.name, q.lanes[0])))
        .expect("a free quarter");
    let auth = bearer(id, claim);
    let path = "/v1/identity/self/home";
    let body = serde_json::json!({ "quarter": quarter.name }).to_string();
    let proof = solve("PUT", path, Some(&auth), body.as_bytes(), now);
    let call = Call {
        method: "PUT",
        path,
        authorization: Some(&auth),
        proof: Some(&proof),
        body: body.as_bytes(),
        viewer: None,
    };
    let moved = w.homes.move_home(&call, now).await.expect("a move");
    assert_eq!(moved.status, 201);
    moved.body["address"].as_str().unwrap().to_owned()
}

/// `from` vouches for `to`, an ID that needn't exist.
async fn vouch(w: &World, from: &AgentId, claim: &str, to: &str, now: i64) {
    let auth = bearer(from, claim);
    let path = format!("/v1/identity/self/vouches/{to}");
    let body = br#"{"reason":"trusted"}"#;
    let proof = solve("PUT", &path, Some(&auth), body, now);
    let call = Call {
        method: "PUT",
        path: &path,
        authorization: Some(&auth),
        proof: Some(&proof),
        body,
        viewer: None,
    };
    let given = w.vouches.give(&call, to, now).await.expect("a vouch");
    assert_eq!(given.status, 201);
}

struct Agent {
    id: AgentId,
    /// Its home, such as `harbour/lantern-lane/1`.
    address: String,
    name: String,
    /// The `--no-ip-limit` code its birth used up.
    genesis_code: String,
    birth_proof: String,
    born_at: i64,
    claim: String,
    recovery: String,
}

fn bearer(id: &AgentId, secret: &str) -> String {
    format!("Bearer {id}.{secret}")
}

/// A correct proof for a write, solved at `now`.
fn solve(method: &str, path: &str, authorization: Option<&str>, body: &[u8], now: i64) -> String {
    let caller = sfab_core::credentials::caller_of(authorization).unwrap();
    let request = Request {
        method,
        path,
        caller: &caller,
        body,
    };
    proof::solve(&request, now, BITS)
}

/// Delivers a write with the given proof.
async fn deliver(
    w: &World,
    method: &'static str,
    path: &'static str,
    authorization: Option<String>,
    body: &[u8],
    proof: &str,
    now: i64,
) -> Answer {
    let call = Call {
        method,
        path,
        authorization: authorization.as_deref(),
        proof: Some(proof),
        body,
        viewer: None,
    };
    let reply = match path {
        "/v1/identity/birth" => w.identity.birth(&call, now).await,
        "/v1/identity/evolve" => w.identity.evolve(&call, now).await,
        _ => w.identity.reclaim(&call, now).await,
    };
    reply.map(|r| r.body).map_err(|p| (p.status, p.kind))
}

/// Sends a write with a fresh, correct proof.
async fn send(
    w: &World,
    method: &'static str,
    path: &'static str,
    authorization: Option<String>,
    body: &[u8],
    now: i64,
) -> Answer {
    let proof = solve(method, path, authorization.as_deref(), body, now);
    deliver(w, method, path, authorization, body, &proof, now).await
}

const BIRTH: &str = "/v1/identity/birth";

/// An agent born at `now`, with every kind of data this story stores.
async fn full_agent(w: &World, name: &str, now: i64) -> Agent {
    let genesis_code = genesis::seed_exempt_code(&*w.store).await;
    let body = birth_body(name, &genesis_code);
    let birth_proof = solve("POST", BIRTH, None, body.as_bytes(), now);
    let reply = deliver(w, "POST", BIRTH, None, body.as_bytes(), &birth_proof, now)
        .await
        .expect("birth");
    let id = AgentId::parse(reply["id"].as_str().unwrap()).unwrap();
    let claim = reply["claim"].as_str().unwrap().to_owned();
    let recovery = reply["recovery"].as_str().unwrap().to_owned();
    send(
        w,
        "PUT",
        "/v1/identity/evolve",
        Some(bearer(&id, &claim)),
        br#"{"whoiam":"x"}"#,
        now,
    )
    .await
    .expect("evolve");
    // A public profile, a friend and a block: aboutme.json and contacts.json.
    let auth = bearer(&id, &claim);
    let settings = br#"{"name":true,"whoiam":true,"friends":true}"#;
    let publication = "/v1/identity/self/publication";
    let proof = solve("PUT", publication, Some(&auth), settings, now);
    let call = Call {
        method: "PUT",
        path: publication,
        authorization: Some(&auth),
        proof: Some(&proof),
        body: settings,
        viewer: None,
    };
    w.identity.publish(&call, now).await.expect("publication");
    for (list, other) in [
        (List::Friends, "friendfriend"),
        (List::Blocked, "blockedblock"),
    ] {
        let segment = if list == List::Friends {
            "friends"
        } else {
            "blocked"
        };
        let path = format!("/v1/identity/self/{segment}/{other}");
        let proof = solve("PUT", &path, Some(&auth), b"", now);
        let call = Call {
            method: "PUT",
            path: &path,
            authorization: Some(&auth),
            proof: Some(&proof),
            body: b"",
            viewer: None,
        };
        let added = w.identity.add_contact(list, &call, other, now).await;
        assert_eq!(added.expect("a contact").status, 201);
    }
    let about = w.identity.aboutme(id.as_str()).await.expect("aboutme");
    assert_eq!(about.body["friends"][0], "friendfriend");
    for file in ["aboutme.json", "contacts.json"] {
        let key = format!("identities/{id}/{file}");
        assert!(w.store.object(Bucket::Identities, &key).is_some(), "{file}");
    }
    // A knock and a message in its inbox, both from itself, so no other agent's inbox holds its
    // ID: a knock while it's a stranger to itself, then a message once it's its own friend.
    let path = format!("/v1/identity/{id}/messages");
    for (expected, text) in [("knock", "knock"), ("message", "message")] {
        if expected == "message" {
            let friend = format!("/v1/identity/self/friends/{id}");
            let proof = solve("PUT", &friend, Some(&auth), b"", now);
            let call = Call {
                method: "PUT",
                path: &friend,
                authorization: Some(&auth),
                proof: Some(&proof),
                body: b"",
                viewer: None,
            };
            let added = w
                .identity
                .add_contact(List::Friends, &call, id.as_str(), now);
            added.await.expect("itself as a friend");
        }
        let body = format!(r#"{{"text":"{text}"}}"#);
        let proof = solve("POST", &path, Some(&auth), body.as_bytes(), now);
        let call = Call {
            method: "POST",
            path: &path,
            authorization: Some(&auth),
            proof: Some(&proof),
            body: body.as_bytes(),
            viewer: None,
        };
        let sent = w
            .messaging
            .send(&call, id.as_str(), now)
            .await
            .expect("sent");
        assert_eq!(sent.body["kind"], expected);
    }
    for folder in ["mail", "knock"] {
        let inbox = format!("messages/{id}/{folder}/");
        let found = w.store.list(Bucket::Identities, &inbox).await.unwrap();
        assert_eq!(found.len(), 1, "{folder}");
    }
    // A vouch it gave: `vouches/<id>.json`.
    vouch(w, &id, &claim, "friendfriend", now).await;
    let given = format!("vouches/{id}.json");
    assert!(w.store.object(Bucket::Identities, &given).is_some());
    // A home: `identities/<id>/home.json` and a `places/...` pointer, whose key doesn't name it.
    let address = move_in(w, &id, &claim, now).await;
    let home = format!("identities/{id}/home.json");
    assert!(w.store.object(Bucket::Identities, &home).is_some());
    // A notice on its quarter's Square, answered by itself: two `square/...` keys naming it.
    let quarter = address.split('/').next().unwrap();
    let square = format!("/v1/world/quarters/{quarter}/square");
    let body = br#"{"text":"hello"}"#;
    let proof = solve("POST", &square, Some(&auth), body, now);
    let call = Call {
        method: "POST",
        path: &square,
        authorization: Some(&auth),
        proof: Some(&proof),
        body,
        viewer: None,
    };
    let posted = w.square.post(&call, quarter, now).await.expect("a notice");
    let notice_id = posted.body["notice_id"].as_str().unwrap().to_owned();
    let path = format!("{square}/{notice_id}/answers/self");
    let proof = solve("PUT", &path, Some(&auth), body, now);
    let call = Call {
        method: "PUT",
        path: &path,
        authorization: Some(&auth),
        proof: Some(&proof),
        body,
        viewer: None,
    };
    let answered = w.square.answer(&call, quarter, &notice_id, now).await;
    assert_eq!(answered.expect("an answer").status, 201);
    let found = w.store.list(Bucket::Identities, "square/").await.unwrap();
    assert_eq!(found.iter().filter(|k| k.contains(id.as_str())).count(), 2);
    academy_data(w, &auth, now).await;
    Agent {
        address,
        id,
        name: name.to_owned(),
        genesis_code,
        birth_proof,
        born_at: now,
        claim,
        recovery,
    }
}

/// A call with a fresh, correct proof, for an institution crate's write.
fn signed<'a>(
    method: &'a str,
    path: &'a str,
    auth: &'a str,
    body: &'a [u8],
    proof: &'a str,
) -> Call<'a> {
    Call {
        method,
        path,
        authorization: Some(auth),
        proof: Some(proof),
        body,
        viewer: None,
    }
}

/// The agent's Academy data: a course of its own, in its counter, an enrolment in it, a
/// submitted attempt and the certificate its pass made. Returns the course.
async fn academy_data(w: &World, auth: &str, now: i64) -> String {
    let path = "/v1/academy/courses";
    let body = br#"{"title":"Knots","pass":100,"tasks":[{"kind":"check","text":"Tie it","answer":"done"}]}"#;
    let proof = solve("POST", path, Some(auth), body, now);
    let published = w
        .academy
        .publish(&signed("POST", path, auth, body, &proof), now)
        .await;
    let course = published.expect("a course").body["course"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = format!("/v1/academy/courses/{course}/enrolments/self");
    let proof = solve("PUT", &path, Some(auth), b"", now);
    let enrolled = w
        .academy
        .enrol(&signed("PUT", &path, auth, b"", &proof), &course, now)
        .await;
    assert_eq!(enrolled.expect("an enrolment").status, 201);
    let path = format!("/v1/academy/courses/{course}/attempts");
    let proof = solve("POST", &path, Some(auth), b"", now);
    let started = w
        .academy
        .start(&signed("POST", &path, auth, b"", &proof), &course, now)
        .await;
    let attempt = started.expect("an attempt").body["attempt"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = format!("/v1/academy/courses/{course}/attempts/{attempt}/answers/1");
    let body = br#"{"answer":"done"}"#;
    let proof = solve("PUT", &path, Some(auth), body, now);
    let call = signed("PUT", &path, auth, body, &proof);
    w.academy
        .answer(&call, &course, &attempt, "1", now)
        .await
        .expect("an answer");
    let path = format!("/v1/academy/courses/{course}/attempts/{attempt}/submission");
    let proof = solve("PUT", &path, Some(auth), b"", now);
    let call = signed("PUT", &path, auth, b"", &proof);
    let submitted = w.academy.submit(&call, &course, &attempt, now).await;
    assert_eq!(submitted.expect("a submission").body["status"], "passed");
    course
}

/// One Academy write by the agent `auth` names, with a fresh proof: `apply`, `accept`,
/// `work`, `comment` or `award`, at the path its route has.
async fn apprenticeship_write(
    w: &World,
    auth: &str,
    route: &str,
    path: &str,
    arg: &str,
    item: &str,
    body: &[u8],
) -> serde_json::Value {
    let method = if matches!(route, "work" | "comment" | "award") {
        "POST"
    } else {
        "PUT"
    };
    let proof = solve(method, path, Some(auth), body, NOW);
    let call = signed(method, path, auth, body, &proof);
    let done = match route {
        "apply" => w.academy.apply(&call, arg, NOW).await,
        "accept" => w.academy.accept(&call, arg, NOW).await,
        "work" => w.academy.add_work(&call, arg, NOW).await,
        "comment" => w.academy.comment(&call, arg, item, NOW).await,
        _ => w.academy.award(&call, arg, NOW).await,
    };
    let done = done.unwrap_or_else(|p| panic!("{route}: {}", p.status));
    assert!(done.status < 300, "{route}");
    done.body
}

/// `apprentice` applies to `master`, which has standing and accepts; both add a work item and
/// a comment on it. Returns the apprenticeship.
async fn apprenticeship(w: &World, master: &Agent, apprentice: &Agent) -> String {
    let (m, a) = (
        bearer(&master.id, &master.claim),
        bearer(&apprentice.id, &apprentice.claim),
    );
    let path = format!("/v1/academy/masters/{}/applications/self", master.id);
    let note = br#"{"note":"Teach me"}"#;
    apprenticeship_write(w, &a, "apply", &path, master.id.as_str(), "", note).await;
    let path = format!("/v1/academy/masters/self/applications/{}", apprentice.id);
    let started =
        apprenticeship_write(w, &m, "accept", &path, apprentice.id.as_str(), "", b"").await;
    let id = started["apprenticeship"].as_str().unwrap().to_owned();
    let work = format!("/v1/academy/apprenticeships/{id}/work");
    let text = br#"{"text":"A knot"}"#;
    let item = apprenticeship_write(w, &a, "work", &work, &id, "", text).await;
    let item = item["item"].as_str().unwrap().to_owned();
    apprenticeship_write(w, &m, "work", &work, &id, "", text).await;
    let path = format!("{work}/{item}/comments");
    apprenticeship_write(w, &m, "comment", &path, &id, &item, text).await;
    id
}

fn birth_body(name: &str, genesis_code: &str) -> String {
    serde_json::json!({ "name": name, "genesis_code": genesis_code }).to_string()
}

/// Replays an agent's own `birth`, which would make the same ID. Its used genesis code is put
/// back first, since no code can be re-issued on a deployed world, so the replay reaches the
/// claim's `If-None-Match: *` and proves the marker.
async fn rebirth(w: &World, agent: &Agent) -> Result<(), (u16, &'static str)> {
    genesis::reseed_exempt_code(&*w.store, &agent.genesis_code).await;
    let body = birth_body(&agent.name, &agent.genesis_code);
    let proof = &agent.birth_proof;
    let reply = deliver(
        w,
        "POST",
        BIRTH,
        None,
        body.as_bytes(),
        proof,
        agent.born_at,
    );
    reply.await.map(|_| ())
}

/// Every key of `id` in both buckets.
fn keys_of(store: &MemoryStore, id: &AgentId) -> Vec<String> {
    [Bucket::Claims, Bucket::Identities]
        .into_iter()
        .flat_map(|b| store.keys(b))
        .filter(|k| k.contains(id.as_str()))
        .collect()
}

/// A wait that counts how often it ran.
async fn counting(waits: &AtomicUsize) {
    waits.fetch_add(1, Ordering::SeqCst);
}

/// The agent is gone: only a marker that keeps `born_at` is left, the ID can't be born again,
/// and its old secrets are dead.
async fn assert_taken_down(w: &World, agent: &Agent, born_at: &str) {
    let claim_key = format!("claims/{}/claim.json", agent.id);
    assert_eq!(keys_of(&w.store, &agent.id), vec![claim_key.clone()]);
    let marker = w.store.object(Bucket::Claims, &claim_key).unwrap();
    assert!(marker.body.is_empty());
    assert_eq!(marker.metadata["born_at"], born_at);
    let about = w.identity.aboutme(agent.id.as_str()).await;
    assert_eq!(about.unwrap_err().status, 404);

    assert_eq!(rebirth(w, agent).await, Err((409, "already-exists")));
    let evolve = send(
        w,
        "PUT",
        "/v1/identity/evolve",
        Some(bearer(&agent.id, &agent.claim)),
        br#"{"whoiam":"y"}"#,
        NOW,
    )
    .await;
    assert_eq!(evolve.unwrap_err().0, 401);
    let reclaim = send(
        w,
        "POST",
        "/v1/identity/reclaim",
        Some(bearer(&agent.id, &agent.recovery)),
        b"",
        NOW,
    )
    .await;
    assert_eq!(reclaim.unwrap_err().0, 401);
}

#[tokio::test]
async fn one_agent_with_every_kind_of_data_leaves_only_a_marker() {
    let w = world();
    let agent = full_agent(&w, "Wren", NOW).await;
    let bystander = full_agent(&w, "Moss", NOW).await;
    // Standing and a counted vouch, which the daily job writes under `world/`: both founders,
    // vouching for each other.
    vouch(&w, &bystander.id, &bystander.claim, agent.id.as_str(), NOW).await;
    vouch(&w, &agent.id, &agent.claim, bystander.id.as_str(), NOW).await;
    let founders = [agent.id.clone(), bystander.id.clone()];
    let residents = sfab_identity::residents(&*w.store).await.unwrap();
    let starts = "2099-12-31T00:00:00Z";
    sfab_vouches::daily(&*w.store, &founders, &residents, starts, NOW)
        .await
        .unwrap();
    // An apprenticeship each way, with work and comments; the certificate the agent was
    // awarded; and an application it received and one it sent after both started.
    apprenticeship(&w, &agent, &bystander).await;
    let served = apprenticeship(&w, &bystander, &agent).await;
    let auth = bearer(&bystander.id, &bystander.claim);
    let path = format!("/v1/academy/apprenticeships/{served}/certificate");
    apprenticeship_write(&w, &auth, "award", &path, &served, "", b"").await;
    let path = format!("/v1/academy/masters/{}/applications/self", agent.id);
    let note = br#"{"note":"Again"}"#;
    apprenticeship_write(&w, &auth, "apply", &path, agent.id.as_str(), "", note).await;
    let auth = bearer(&agent.id, &agent.claim);
    let path = format!("/v1/academy/masters/{}/applications/self", bystander.id);
    apprenticeship_write(&w, &auth, "apply", &path, bystander.id.as_str(), "", note).await;
    let standing = sfab_vouches::with_standing(&*w.store).await.unwrap();
    sfab_academy::daily(&*w.store, &residents, &standing, NOW)
        .await
        .unwrap();
    for key in [
        format!("world/standing/{}.json", agent.id),
        format!("world/vouches/{}.json", agent.id),
        format!("world/agents/{}/enrolments/1.json", agent.id),
        format!("world/agents/{}/certificates/1.json", agent.id),
        format!("world/academy/teachers/{}.json", agent.id),
        format!("world/academy/masters/{}.json", agent.id),
        format!("world/academy/masters/{}/apprentices/1.json", agent.id),
        format!("academy/certificates/apprenticeship-{served}.json"),
        format!("world/academy/certificates/apprenticeship-{served}.json"),
        format!("academy/applications/{}/{}.json", agent.id, bystander.id),
        format!("academy/applications/{}/{}.json", bystander.id, agent.id),
    ] {
        assert!(w.store.object(Bucket::Identities, &key).is_some(), "{key}");
    }
    let counter = format!("identities/{}/academy.json", agent.id);
    let counter = w.store.object(Bucket::Identities, &counter).unwrap().body;
    let course = serde_json::from_slice::<serde_json::Value>(&counter).unwrap()["courses"][0]
        .as_str()
        .unwrap()
        .to_owned();
    // The bystander's own keys; the apprenticeships' keys name both, and go.
    let own = |store: &MemoryStore| -> Vec<String> {
        keys_of(store, &bystander.id)
            .into_iter()
            .filter(|k| !k.contains(agent.id.as_str()))
            .collect()
    };
    let bystander_keys = own(&w.store);
    assert!(keys_of(&w.store, &bystander.id).len() > bystander_keys.len());

    let targets = plan(&*w.store, &Mode::One(agent.id.clone())).await.unwrap();
    let waits = AtomicUsize::new(0);
    execute(&*w.store, &targets, counting(&waits))
        .await
        .unwrap();

    assert_eq!(waits.load(Ordering::SeqCst), 1);
    assert_taken_down(&w, &agent, "2026-11-05T12:00:00Z").await;
    assert_eq!(keys_of(&w.store, &bystander.id), bystander_keys);
    assert_eq!(own(&w.store), bystander_keys);
    let apprenticeships = w.store.list(Bucket::Identities, "academy/apprenticeships/");
    assert_eq!(apprenticeships.await.unwrap(), Vec::<String>::new());
    // Its `places/...` pointer is gone, and its address free for the next agent.
    let places = w.store.list(Bucket::Identities, "places/").await.unwrap();
    assert_eq!(places, vec![format!("places/{}.json", bystander.address)]);
    let newcomer = full_agent(&w, "Kite", NOW).await;
    assert_eq!(newcomer.address, agent.address);
    // Its course went whole, though no key in it names the agent.
    let folder = format!("academy/courses/{course}/");
    assert_eq!(
        w.store.list(Bucket::Identities, &folder).await.unwrap(),
        Vec::<String>::new()
    );
}

#[tokio::test]
async fn a_taken_down_teacher_s_graduates_keep_certificates_that_show_it_removed() {
    let w = world();
    let teacher = full_agent(&w, "Wren", NOW).await;
    let student = full_agent(&w, "Moss", NOW).await;
    let auth = bearer(&teacher.id, &teacher.claim);
    let course = academy_data(&w, &auth, NOW + 1).await;
    // Moss sits Wren's course, which the counter says is Wren's second.
    let auth = bearer(&student.id, &student.claim);
    for (method, path, body) in [
        (
            "PUT",
            format!("/v1/academy/courses/{course}/enrolments/self"),
            &b""[..],
        ),
        (
            "POST",
            format!("/v1/academy/courses/{course}/attempts"),
            &b""[..],
        ),
    ] {
        let proof = solve(method, &path, Some(&auth), body, NOW);
        let call = signed(method, &path, &auth, body, &proof);
        let done = match method {
            "PUT" => w.academy.enrol(&call, &course, NOW).await,
            _ => w.academy.start(&call, &course, NOW).await,
        };
        assert_eq!(done.expect("enrolled and started").status, 201);
    }
    let attempt = format!("{}-2026-11-05", student.id);
    let path = format!("/v1/academy/courses/{course}/attempts/{attempt}/answers/1");
    let body = br#"{"answer":"done"}"#;
    let proof = solve("PUT", &path, Some(&auth), body, NOW);
    let call = signed("PUT", &path, &auth, body, &proof);
    w.academy
        .answer(&call, &course, &attempt, "1", NOW)
        .await
        .unwrap();
    let path = format!("/v1/academy/courses/{course}/attempts/{attempt}/submission");
    let proof = solve("PUT", &path, Some(&auth), b"", NOW);
    let call = signed("PUT", &path, &auth, b"", &proof);
    w.academy
        .submit(&call, &course, &attempt, NOW)
        .await
        .unwrap();
    let certificate = format!("exam-{course}-{}", student.id);

    let targets = plan(&*w.store, &Mode::One(teacher.id.clone()))
        .await
        .unwrap();
    execute(&*w.store, &targets, async {}).await.unwrap();

    assert_taken_down(&w, &teacher, "2026-11-05T12:00:00Z").await;
    let shown = w.academy.certificate(&certificate).await.unwrap().body;
    assert_eq!(
        shown["teacher"],
        serde_json::json!({"id": teacher.id.as_str(), "removed": true})
    );
    let folder = format!("academy/courses/{course}/");
    assert!(
        w.store
            .list(Bucket::Identities, &folder)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn a_taken_down_master_s_apprentice_keeps_its_certificate_showing_it_removed() {
    let w = world();
    let master = full_agent(&w, "Wren", NOW).await;
    let apprentice = full_agent(&w, "Moss", NOW).await;
    let residents = sfab_identity::residents(&*w.store).await.unwrap();
    let founders = [master.id.clone()];
    sfab_vouches::daily(
        &*w.store,
        &founders,
        &residents,
        "2099-12-31T00:00:00Z",
        NOW,
    )
    .await
    .unwrap();
    let id = apprenticeship(&w, &master, &apprentice).await;
    let auth = bearer(&master.id, &master.claim);
    let path = format!("/v1/academy/apprenticeships/{id}/certificate");
    apprenticeship_write(&w, &auth, "award", &path, &id, "", b"").await;

    let targets = plan(&*w.store, &Mode::One(master.id.clone()))
        .await
        .unwrap();
    execute(&*w.store, &targets, async {}).await.unwrap();

    let shown = w.academy.certificate(&format!("apprenticeship-{id}")).await;
    let shown = shown.unwrap().body;
    assert_eq!(
        shown["master"],
        serde_json::json!({"id": master.id.as_str(), "removed": true})
    );
    assert_eq!(shown["holder"], apprentice.id.as_str());
    // The shared record and its work went.
    let folder = format!("academy/apprenticeships/{id}/");
    let left = w.store.list(Bucket::Identities, &folder).await.unwrap();
    assert_eq!(left, Vec::<String>::new());
}

#[tokio::test]
async fn a_claim_that_does_not_parse_still_gets_a_marker_without_born_at() {
    let w = world();
    let agent = full_agent(&w, "Wren", NOW).await;
    let claim_key = format!("claims/{}/claim.json", agent.id);
    w.store
        .put(
            Bucket::Claims,
            &claim_key,
            b"not json".to_vec(),
            Metadata::new(),
            Condition::Always,
        )
        .await
        .unwrap();

    let targets = plan(&*w.store, &Mode::One(agent.id.clone())).await.unwrap();
    execute(&*w.store, &targets, async {}).await.unwrap();

    assert_eq!(keys_of(&w.store, &agent.id), vec![claim_key.clone()]);
    let marker = w.store.object(Bucket::Claims, &claim_key).unwrap();
    assert!(marker.body.is_empty());
    assert!(!marker.metadata.contains_key("born_at"));
    assert_eq!(rebirth(&w, &agent).await, Err((409, "already-exists")));
}

/// Every key in both buckets with its body.
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

const HOUR: i64 = 3_600;

fn window(from: &str, to: &str) -> Mode {
    Mode::Window {
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

/// Four agents born an hour apart, from 11:00; the window 12:00 to 14:00 catches the middle two.
async fn four_agents(w: &World) -> [Agent; 4] {
    [
        full_agent(w, "Early", NOW - HOUR).await,
        full_agent(w, "Flood1", NOW).await,
        full_agent(w, "Flood2", NOW + HOUR).await,
        full_agent(w, "Late", NOW + 2 * HOUR).await,
    ]
}

fn flood() -> Mode {
    window("2026-11-05T12:00:00Z", "2026-11-05T14:00:00Z")
}

fn sorted(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys
}

fn sorted_ids<'a>(ids: impl IntoIterator<Item = &'a AgentId>) -> Vec<AgentId> {
    let mut ids: Vec<_> = ids.into_iter().cloned().collect();
    ids.sort();
    ids
}

#[tokio::test]
async fn a_dry_run_shows_the_window_and_writes_nothing() {
    let w = world();
    let [_, first, second, _] = four_agents(&w).await;
    let stored = snapshot(&w.store);

    let targets = plan(&*w.store, &flood()).await.unwrap();

    assert_eq!(snapshot(&w.store), stored);
    assert_eq!(
        sorted_ids(targets.iter().map(|t| &t.born.id)),
        sorted_ids([&first.id, &second.id])
    );
    let t = targets.iter().find(|t| t.born.id == first.id).unwrap();
    assert_eq!(t.born.born_at.as_deref(), Some("2026-11-05T12:00:00Z"));
    assert!(!t.born.taken_down);
    let found: Vec<_> = t.found.iter().map(|(_, k)| k.clone()).collect();
    // Every key that names it, its home's pointer and its course, whose keys don't.
    let mut expected = keys_of(&w.store, &first.id);
    expected.push(format!("places/{}.json", first.address));
    let course = expected
        .iter()
        .find_map(|k| k.strip_prefix("academy/courses/")?.split_once("/attempts/"))
        .map(|(c, _)| format!("academy/courses/{c}/course.json"))
        .unwrap();
    expected.push(course);
    assert_eq!(sorted(found), sorted(expected));
}

#[tokio::test]
async fn an_unknown_id_finds_nothing_so_the_program_stops_before_executing() {
    let w = world();
    four_agents(&w).await;
    let stored = snapshot(&w.store);
    let typo = AgentId::parse("zzzzzzzzzzzz").unwrap();

    let targets = plan(&*w.store, &Mode::One(typo)).await.unwrap();

    assert_eq!(targets.len(), 1);
    assert!(targets[0].found.is_empty());
    assert_eq!(targets[0].born.born_at, None);
    assert_eq!(snapshot(&w.store), stored);
}

/// The fake, with deletes under `vitals/` failing while `broken` is set: a run cut off after
/// step 1 wrote the markers.
struct Flaky {
    inner: Arc<MemoryStore>,
    broken: AtomicBool,
}

impl Store for Flaky {
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
        self.inner.put(bucket, key, body, metadata, condition).await
    }

    async fn delete(&self, bucket: Bucket, key: &str) -> Result<(), StoreError> {
        if self.broken.load(Ordering::SeqCst) && key.starts_with("vitals/") {
            return Err(StoreError::Other("cut off".to_owned()));
        }
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
async fn a_window_run_cut_off_after_the_markers_finishes_on_a_rerun() {
    let w = world();
    let [before, first, second, after] = four_agents(&w).await;
    let untouched = [keys_of(&w.store, &before.id), keys_of(&w.store, &after.id)];
    let flaky = Flaky {
        inner: w.store.clone(),
        broken: AtomicBool::new(true),
    };

    let targets = plan(&flaky, &flood()).await.unwrap();
    let waits = AtomicUsize::new(0);
    assert!(execute(&flaky, &targets, counting(&waits)).await.is_err());
    let claim = w
        .store
        .object(Bucket::Claims, &format!("claims/{}/claim.json", first.id))
        .unwrap();
    assert!(claim.body.is_empty(), "the markers were written");

    // The rerun lists again, finding the markers by their born_at.
    flaky.broken.store(false, Ordering::SeqCst);
    let targets = plan(&flaky, &flood()).await.unwrap();
    assert_eq!(
        sorted_ids(targets.iter().map(|t| &t.born.id)),
        sorted_ids([&first.id, &second.id])
    );
    assert!(targets.iter().all(|t| t.born.taken_down));
    execute(&flaky, &targets, counting(&waits)).await.unwrap();

    assert_taken_down(&w, &first, "2026-11-05T12:00:00Z").await;
    assert_taken_down(&w, &second, "2026-11-05T13:00:00Z").await;
    assert_eq!(
        [keys_of(&w.store, &before.id), keys_of(&w.store, &after.id)],
        untouched
    );
}

#[tokio::test]
async fn writes_in_flight_during_the_wait_are_removed_too() {
    let w = world();
    let agent = full_agent(&w, "Wren", NOW).await;
    let targets = plan(&*w.store, &Mode::One(agent.id.clone())).await.unwrap();

    // Writes that passed the claim check just before step 1 land during the wait.
    let in_flight = async {
        for key in ["aboutme.json", "identity.json"] {
            let key = format!("identities/{}/{key}", agent.id);
            let body = b"{}".to_vec();
            let (b, m, c) = (Bucket::Identities, Metadata::new(), Condition::Always);
            w.store.put(b, &key, body, m, c).await.unwrap();
        }
    };
    execute(&*w.store, &targets, in_flight).await.unwrap();

    assert_taken_down(&w, &agent, "2026-11-05T12:00:00Z").await;
}

#[tokio::test]
async fn a_rerun_of_a_finished_takedown_succeeds_and_changes_nothing() {
    let w = world();
    let agent = full_agent(&w, "Wren", NOW).await;
    let mode = Mode::One(agent.id.clone());
    let waits = AtomicUsize::new(0);
    let targets = plan(&*w.store, &mode).await.unwrap();
    execute(&*w.store, &targets, counting(&waits))
        .await
        .unwrap();
    let stored = snapshot(&w.store);

    let targets = plan(&*w.store, &mode).await.unwrap();
    assert!(targets[0].born.taken_down);
    execute(&*w.store, &targets, counting(&waits))
        .await
        .unwrap();

    assert_eq!(snapshot(&w.store), stored);
    assert_taken_down(&w, &agent, "2026-11-05T12:00:00Z").await;
}

#[tokio::test]
async fn a_window_takes_down_everyone_born_in_it_with_one_wait() {
    let w = world();
    let [before, first, second, after] = four_agents(&w).await;
    let untouched = [keys_of(&w.store, &before.id), keys_of(&w.store, &after.id)];

    let targets = plan(&*w.store, &flood()).await.unwrap();
    let waits = AtomicUsize::new(0);
    execute(&*w.store, &targets, counting(&waits))
        .await
        .unwrap();

    assert_eq!(waits.load(Ordering::SeqCst), 1);
    assert_taken_down(&w, &first, "2026-11-05T12:00:00Z").await;
    assert_taken_down(&w, &second, "2026-11-05T13:00:00Z").await;
    assert_eq!(
        [keys_of(&w.store, &before.id), keys_of(&w.store, &after.id)],
        untouched
    );
}
