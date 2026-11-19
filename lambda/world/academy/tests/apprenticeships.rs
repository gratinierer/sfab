//! Masters and apprenticeships: `.stories/4-life-in-the-world/spec.md` "The Academy"
//! ("Apprenticeships", "What's public", "When things go away"). Routes that need standing are
//! tested only here, because the founders make no test writes before launch.

mod common;

use common::*;
use serde_json::{Value, json};
use sfab_core::id::AgentId;
use sfab_core::storage::{Bucket, Store};
use sfab_identity::List;

/// `masters` have standing, as founders of the vouches step's run at `NOW`.
async fn give_standing(w: &World, masters: &[&Agent]) {
    let founders: Vec<AgentId> = masters.iter().map(|m| m.id.clone()).collect();
    let residents = sfab_identity::residents(&*w.store).await.unwrap();
    sfab_vouches::daily(
        &*w.store,
        &founders,
        &residents,
        "2099-12-31T00:00:00Z",
        NOW,
    )
    .await
    .unwrap();
}

async fn block(w: &World, blocker: &Agent, other: &Agent) {
    let path = format!("/v1/identity/self/blocked/{}", other.id());
    let sent = write("PUT", &path, blocker.auth(), b"", NOW);
    w.identity
        .add_contact(List::Blocked, &sent.call(), other.id(), NOW)
        .await
        .unwrap();
}

fn apply_call(applicant: &Agent, master: &str, note: &str, at: i64) -> Sent {
    let path = format!("/v1/academy/masters/{master}/applications/self");
    let body = serde_json::to_vec(&json!({ "note": note })).unwrap();
    write("PUT", &path, applicant.auth(), &body, at)
}

async fn apply(w: &World, applicant: &Agent, master: &str, note: &str, at: i64) -> Answer {
    let sent = apply_call(applicant, master, note, at);
    answer(w.academy.apply(&sent.call(), master, at).await)
}

async fn applications(w: &World, master: &Agent, at: i64) -> Answer {
    let auth = master.auth();
    answer(w.academy.applications(auth.as_deref(), None, at).await)
}

fn accept_call(master: &Agent, applicant: &str, at: i64) -> Sent {
    let path = format!("/v1/academy/masters/self/applications/{applicant}");
    write("PUT", &path, master.auth(), b"", at)
}

async fn send_accept(w: &World, sent: &Sent, applicant: &str, at: i64) -> Answer {
    answer(w.academy.accept(&sent.call(), applicant, at).await)
}

async fn accept(w: &World, master: &Agent, applicant: &str, at: i64) -> Answer {
    send_accept(w, &accept_call(master, applicant, at), applicant, at).await
}

/// `apprentice` applies to `master` and is accepted; the apprenticeship's ID.
async fn apprenticed(w: &World, master: &Agent, apprentice: &Agent) -> String {
    apply(w, apprentice, master.id(), "Teach me", NOW)
        .await
        .unwrap();
    let (status, started) = accept(w, master, apprentice.id(), NOW).await.unwrap();
    assert_eq!(status, 201, "{started}");
    started["apprenticeship"].as_str().unwrap().to_owned()
}

fn work_call(who: &Agent, a: &str, text: &str) -> Sent {
    let path = format!("/v1/academy/apprenticeships/{a}/work");
    let body = serde_json::to_vec(&json!({ "text": text })).unwrap();
    write("POST", &path, who.auth(), &body, NOW)
}

async fn add_work(w: &World, sent: &Sent, a: &str) -> Answer {
    answer(w.academy.add_work(&sent.call(), a, NOW).await)
}

async fn work(w: &World, who: &Agent, a: &str) -> Answer {
    let auth = who.auth();
    answer(w.academy.work(auth.as_deref(), a, None, NOW).await)
}

async fn comment(w: &World, who: &Agent, a: &str, item: &str, text: &str) -> Answer {
    let path = format!("/v1/academy/apprenticeships/{a}/work/{item}/comments");
    let body = serde_json::to_vec(&json!({ "text": text })).unwrap();
    let sent = write("POST", &path, who.auth(), &body, NOW);
    answer(w.academy.comment(&sent.call(), a, item, NOW).await)
}

async fn comments(w: &World, who: &Agent, a: &str, item: &str) -> Answer {
    let auth = who.auth();
    answer(
        w.academy
            .comments(auth.as_deref(), a, item, None, NOW)
            .await,
    )
}

async fn end(w: &World, who: &Agent, a: &str) -> Answer {
    let path = format!("/v1/academy/apprenticeships/{a}/ending");
    let sent = write("PUT", &path, who.auth(), b"", NOW);
    answer(w.academy.end(&sent.call(), a, NOW).await)
}

async fn award(w: &World, who: &Agent, a: &str) -> Answer {
    let path = format!("/v1/academy/apprenticeships/{a}/certificate");
    let sent = write("POST", &path, who.auth(), b"", NOW);
    answer(w.academy.award(&sent.call(), a, NOW).await)
}

#[tokio::test]
async fn a_master_has_up_to_3_apprentices_and_ending_one_frees_a_place() {
    let w = world();
    let m = born(&w, "Wren").await;
    let students = [
        born(&w, "Moss").await,
        born(&w, "Kite").await,
        born(&w, "Gull").await,
        born(&w, "Fern").await,
    ];
    give_standing(&w, &[&m]).await;
    let mut started = Vec::new();
    for s in &students[..3] {
        started.push(apprenticed(&w, &m, s).await);
    }
    apply(&w, &students[3], m.id(), "Me too", NOW)
        .await
        .unwrap();

    assert_eq!(
        accept(&w, &m, students[3].id(), NOW).await,
        Err((409, "list-full"))
    );
    let (_, ended) = end(&w, &students[0], &started[0]).await.unwrap();
    assert_eq!(
        (ended["ended"].clone(), ended["ended_by"].clone()),
        (json!(true), json!(students[0].id()))
    );
    assert_eq!(end(&w, &m, &started[0]).await.unwrap().0, 200, "again");
    let (status, fourth) = accept(&w, &m, students[3].id(), NOW).await.unwrap();
    assert_eq!(status, 201);
    assert_eq!(
        fourth["apprenticeship"],
        format!("{}-{}-{TODAY}", m.id(), students[3].id())
    );
}

#[tokio::test]
async fn an_apprenticeship_a_takedown_removed_frees_the_master_s_place() {
    let w = world();
    let m = born(&w, "Wren").await;
    let students = [
        born(&w, "Moss").await,
        born(&w, "Kite").await,
        born(&w, "Gull").await,
        born(&w, "Fern").await,
    ];
    give_standing(&w, &[&m]).await;
    let mut started = Vec::new();
    for s in &students[..3] {
        started.push(apprenticed(&w, &m, s).await);
    }
    // As the apprentice's takedown leaves it: the record gone, the master's counter untouched.
    let record = format!("academy/apprenticeships/{}/apprenticeship.json", started[1]);
    w.store.delete(Bucket::Identities, &record).await.unwrap();
    apply(&w, &students[3], m.id(), "Me too", NOW)
        .await
        .unwrap();

    assert_eq!(accept(&w, &m, students[3].id(), NOW).await.unwrap().0, 201);
}

#[tokio::test]
async fn applying_needs_a_master_with_standing_that_doesn_t_block_you() {
    let w = world();
    let m = born(&w, "Wren").await;
    let plain = born(&w, "Moss").await;
    let s = born(&w, "Kite").await;
    let blocked = born(&w, "Gull").await;
    give_standing(&w, &[&m]).await;
    block(&w, &m, &blocked).await;

    assert_eq!(
        apply(&w, &s, plain.id(), "Teach me", NOW).await,
        Err((403, "not-a-master"))
    );
    assert_eq!(
        apply(&w, &blocked, m.id(), "Teach me", NOW).await,
        Err((403, "blocked"))
    );
    assert_eq!(
        apply(&w, &m, m.id(), "Me", NOW).await,
        Err((400, "about:blank"))
    );
    let long = "x".repeat(201);
    assert_eq!(
        apply(&w, &s, m.id(), &long, NOW).await,
        Err((400, "about:blank"))
    );
    let sent = apply_call(&s, m.id(), "Teach me", NOW);
    let first = answer(w.academy.apply(&sent.call(), m.id(), NOW).await).unwrap();
    assert_eq!(first.0, 201);
    let retry = answer(w.academy.apply(&sent.call(), m.id(), NOW).await).unwrap();
    assert_eq!(retry, (200, first.1));
    let (status, replaced) = apply(&w, &s, m.id(), "Please", NOW + 60).await.unwrap();
    assert_eq!((status, replaced["note"].clone()), (200, json!("Please")));

    let (_, list) = applications(&w, &m, NOW + 60).await.unwrap();
    assert_eq!(
        list["items"],
        json!([{"applicant": s.id(), "note": "Please", "applied_at": "2026-11-05T12:01:00Z"}])
    );
    // Past 30 days it's hidden, though S3 hasn't expired it yet.
    let (_, later) = applications(&w, &m, NOW + 31 * DAY).await.unwrap();
    assert_eq!(later["items"], json!([]));
}

#[tokio::test]
async fn reading_and_accepting_applications_need_standing() {
    let w = world();
    let m = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    give_standing(&w, &[]).await;
    assert_eq!(applications(&w, &m, NOW).await, Err((403, "no-standing")));
    assert_eq!(accept(&w, &m, s.id(), NOW).await, Err((403, "no-standing")));
    give_standing(&w, &[&m]).await;
    assert_eq!(
        accept(&w, &m, s.id(), NOW).await,
        Err((404, "about:blank")),
        "no application"
    );
}

#[tokio::test]
async fn a_retried_accept_returns_the_apprenticeship_and_leaves_one_place_taken() {
    let w = world();
    let m = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    give_standing(&w, &[&m]).await;
    apply(&w, &s, m.id(), "Teach me", NOW).await.unwrap();
    let sent = accept_call(&m, s.id(), NOW);
    let (status, first) = send_accept(&w, &sent, s.id(), NOW).await.unwrap();
    assert_eq!(status, 201);

    assert_eq!(
        send_accept(&w, &sent, s.id(), NOW).await,
        Ok((200, first.clone()))
    );
    // A new proof the next day, while it's current, finds the same one.
    assert_eq!(
        accept(&w, &m, s.id(), NOW + DAY).await,
        Ok((200, first.clone()))
    );
    let counter = format!("identities/{}/academy.json", m.id());
    let counter = w.store.object(Bucket::Identities, &counter).unwrap().body;
    let counter: Value = serde_json::from_slice(&counter).unwrap();
    assert_eq!(counter["apprentices"], json!([first["apprenticeship"]]));
    let left = format!("academy/applications/{}/", m.id());
    assert!(
        w.store
            .list(Bucket::Identities, &left)
            .await
            .unwrap()
            .is_empty()
    );

    // Ended the same day, the pair starts again tomorrow.
    let a = first["apprenticeship"].as_str().unwrap();
    end(&w, &m, a).await.unwrap();
    apply(&w, &s, m.id(), "Again", NOW).await.unwrap();
    assert_eq!(accept(&w, &m, s.id(), NOW).await, Err((409, "too-soon")));
}

#[tokio::test]
async fn work_items_and_comments_stay_private_to_the_two_after_ending() {
    let w = world();
    let m = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let other = born(&w, "Kite").await;
    give_standing(&w, &[&m]).await;
    let a = apprenticed(&w, &m, &s).await;

    let sent = work_call(&s, &a, "My first knot");
    let (status, item) = add_work(&w, &sent, &a).await.unwrap();
    assert_eq!((status, item["comments"].clone()), (201, json!(0)));
    assert_eq!(
        add_work(&w, &sent, &a).await,
        Ok((200, item.clone())),
        "a retry"
    );
    let id = item["item"].as_str().unwrap().to_owned();
    let (status, _) = comment(&w, &m, &a, &id, "Tighter").await.unwrap();
    assert_eq!(status, 201);
    comment(&w, &s, &a, &id, "Like this?").await.unwrap();
    let big = "x".repeat(2049);
    assert_eq!(
        comment(&w, &m, &a, &id, &big).await,
        Err((400, "about:blank"))
    );
    end(&w, &m, &a).await.unwrap();

    for who in [&m, &s] {
        let (_, list) = work(&w, who, &a).await.unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 1);
        assert_eq!(
            list["items"][0]["comments"], 2,
            "the count, not the comments"
        );
        assert_eq!(list["items"][0]["text"], "My first knot");
        let (_, said) = comments(&w, who, &a, &id).await.unwrap();
        // Both at the same second, so in order of author ID, which is random.
        let mut texts: Vec<&str> = said["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["text"].as_str().unwrap())
            .collect();
        texts.sort_unstable();
        assert_eq!(texts, ["Like this?", "Tighter"]);
    }
    assert_eq!(work(&w, &other, &a).await, Err((403, "not-a-party")));
    assert_eq!(
        comments(&w, &other, &a, &id).await,
        Err((403, "not-a-party"))
    );
    let theirs = work_call(&other, &a, "Me too");
    assert_eq!(add_work(&w, &theirs, &a).await, Err((403, "not-a-party")));
    assert_eq!(end(&w, &other, &a).await, Err((403, "not-a-party")));
    let unknown = format!("{}-{}-2026-11-01", m.id(), s.id());
    assert_eq!(work(&w, &s, &unknown).await, Err((404, "about:blank")));
}

#[tokio::test]
async fn the_master_awards_a_certificate_that_counts_by_its_standing_and_can_revoke_it() {
    let w = world();
    let m = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    give_standing(&w, &[&m]).await;
    let a = apprenticed(&w, &m, &s).await;

    assert_eq!(award(&w, &s, &a).await, Err((403, "not-the-author")));
    let (status, awarded) = award(&w, &m, &a).await.unwrap();
    let id = format!("apprenticeship-{a}");
    assert_eq!((status, awarded["certificate"].clone()), (201, json!(id)));
    assert_eq!(award(&w, &m, &a).await.unwrap().0, 200, "again");

    let standing: Vec<&Agent> = vec![&m];
    daily(&w, &standing, NOW).await;
    let (_, shown) = certificate(&w, &id).await.unwrap();
    assert_eq!(shown["kind"], "apprenticeship");
    assert_eq!(shown["master"], json!({"id": m.id(), "removed": false}));
    assert_eq!(shown["holder"], s.id());
    assert_eq!(shown["counts"], true);
    let (_, held) = answer(w.academy.certificates(s.id(), None).await).unwrap();
    assert_eq!(held["items"], json!([id]));

    let (_, page) = answer(w.academy.master(m.id()).await).unwrap();
    assert_eq!(
        page,
        json!({"id": m.id(), "current": [a], "apprentices": 1, "apprenticeships": 1, "certificates": 1})
    );
    let (_, list) = answer(w.academy.master_apprentices(m.id(), None).await).unwrap();
    assert_eq!(list, json!({"items": [a], "next": null}));
    let (_, nobody) = answer(w.academy.master(s.id()).await).unwrap();
    assert_eq!(nobody["apprenticeships"], 0);

    assert_eq!(revoke(&w, &s, &id).await, Err((403, "not-the-author")));
    assert_eq!(revoke(&w, &m, &id).await.unwrap().0, 200);
    end(&w, &s, &a).await.unwrap();
    daily(&w, &standing, NOW).await;
    let (_, shown) = certificate(&w, &id).await.unwrap();
    assert_eq!(
        (shown["revoked"].clone(), shown["counts"].clone()),
        (json!(true), json!(false))
    );
    let (_, page) = answer(w.academy.master(m.id()).await).unwrap();
    assert_eq!(
        (page["current"].clone(), page["certificates"].clone()),
        (json!([]), json!(0))
    );
}

#[tokio::test]
async fn a_master_s_pages_answer_not_built_yet_before_the_first_run() {
    let w = world();
    let m = born(&w, "Wren").await;
    assert_eq!(
        answer(w.academy.master(m.id()).await),
        Err((404, "not-built-yet"))
    );
    assert_eq!(
        answer(w.academy.master_apprentices(m.id(), None).await),
        Err((404, "not-built-yet"))
    );
    assert_eq!(
        answer(w.academy.master("Nope").await),
        Err((400, "about:blank"))
    );
}

#[tokio::test]
async fn a_block_stops_the_blocked_party_adding_work_or_comments() {
    let w = world();
    let m = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    give_standing(&w, &[&m]).await;
    let a = apprenticed(&w, &m, &s).await;
    let (_, item) = add_work(&w, &work_call(&s, &a, "My first knot"), &a)
        .await
        .unwrap();
    let id = item["item"].as_str().unwrap().to_owned();

    block(&w, &m, &s).await;
    assert_eq!(
        add_work(&w, &work_call(&s, &a, "Another knot"), &a).await,
        Err((403, "blocked"))
    );
    assert_eq!(
        comment(&w, &s, &a, &id, "Like this?").await,
        Err((403, "blocked"))
    );
    let (_, list) = work(&w, &m, &a).await.unwrap();
    assert_eq!(list["items"].as_array().unwrap().len(), 1, "nothing stored");
    assert_eq!(list["items"][0]["comments"], 0);
    let (status, _) = comment(&w, &m, &a, &id, "Tighter").await.unwrap();
    assert_eq!(status, 201, "the blocker still writes");
}

#[tokio::test]
async fn a_block_stops_the_blocked_master_accepting_or_awarding_but_not_ending() {
    let w = world();
    let m = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    let t = born(&w, "Fern").await;
    give_standing(&w, &[&m]).await;
    apply(&w, &s, m.id(), "Teach me", NOW).await.unwrap();
    block(&w, &s, &m).await;
    assert_eq!(accept(&w, &m, s.id(), NOW).await, Err((403, "blocked")));
    let (_, list) = applications(&w, &m, NOW).await.unwrap();
    assert_eq!(list["items"].as_array().unwrap().len(), 1, "nothing stored");

    let a = apprenticed(&w, &m, &t).await;
    block(&w, &t, &m).await;
    assert_eq!(award(&w, &m, &a).await, Err((403, "blocked")));
    let (status, _) = end(&w, &m, &a).await.unwrap();
    assert_eq!(status, 200, "ending is a way out");
}

#[tokio::test]
async fn a_retried_accept_redoes_a_missing_record_even_after_a_block() {
    let w = world();
    let m = born(&w, "Wren").await;
    let s = born(&w, "Moss").await;
    give_standing(&w, &[&m]).await;
    let a = apprenticed(&w, &m, &s).await;
    // As a first try that wrote only the counter leaves it.
    let record = format!("academy/apprenticeships/{a}/apprenticeship.json");
    w.store.delete(Bucket::Identities, &record).await.unwrap();
    block(&w, &s, &m).await;
    let (status, found) = accept(&w, &m, s.id(), NOW).await.unwrap();
    assert_eq!(
        (status, found["apprenticeship"].as_str()),
        (200, Some(a.as_str()))
    );
    assert!(
        w.store
            .get(Bucket::Identities, &record)
            .await
            .unwrap()
            .is_some()
    );
}
