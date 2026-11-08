//! Genesis codes, the IP rule and the pool route, from `.stories/17-genesis-codes/spec.md`
//! "What `birth` does", "The IP rule" and "Two sources", against the in-memory fake: issue 01's
//! Done when, at the crate's seam. `identity::birth` calls `admit` before the birth and `use_up`
//! after it.

use std::sync::Arc;

use serde_json::json;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Calls, MemoryStore};
use sfab_genesis::{Genesis, Issue, issue, testing, write_pool};

/// 2026-11-05T12:00:00Z.
const NOW: i64 = 1_793_836_800 + 43_200;
const HOUR: i64 = 3_600;
const DAY: i64 = 86_400;
const V4: &str = "198.51.100.7:46532";
const V4_OTHER: &str = "198.51.100.8:46532";

fn genesis() -> (Arc<MemoryStore>, Genesis<MemoryStore>) {
    let store = Arc::new(MemoryStore::new());
    (store.clone(), Genesis::new(store, testing::ip_rule()))
}

fn problem<T: std::fmt::Debug>(result: Result<T, Problem>) -> (u16, &'static str) {
    let p = result.expect_err("a problem");
    (p.status, p.kind)
}

/// Admits `code` from `viewer`, then uses it up, as a successful birth does.
async fn birth(g: &Genesis<MemoryStore>, code: &str, viewer: Option<&str>, now: i64) {
    let admission = g.admit(code, viewer, now).await.expect("admitted");
    g.use_up(&admission, now).await;
}

/// Every stored body and metadata value, as text, to search for an address.
fn everything(store: &MemoryStore) -> String {
    let mut text = String::new();
    for bucket in [Bucket::Identities, Bucket::Claims] {
        for key in store.keys(bucket) {
            let o = store.object(bucket, &key).unwrap();
            text.push_str(&key);
            text.push_str(&String::from_utf8_lossy(&o.body));
            text.push_str(&format!("{:?}", o.metadata));
        }
    }
    text
}

#[test]
fn a_new_code_is_16_characters_of_lowercase_base32() {
    let a = sfab_genesis::new_code();
    let b = sfab_genesis::new_code();
    assert_eq!(a.len(), 16);
    assert!(
        a.bytes().all(|c| matches!(c, b'a'..=b'z' | b'2'..=b'7')),
        "{a}"
    );
    assert_ne!(a, b);
}

#[tokio::test]
async fn every_refused_code_gets_the_same_403() {
    let (store, g) = genesis();
    let used = testing::seed_code(&*store).await;
    birth(&g, &used, Some(V4), NOW).await;
    let expired = sfab_genesis::new_code();
    let expiring = Issue {
        label: "forum-y".into(),
        expires: Some(NOW),
        no_ip_limit: false,
    };
    issue(&*store, &expired, &expiring).await.unwrap();
    let never_issued = sfab_genesis::new_code();

    let mut answers = Vec::new();
    for code in [
        used.as_str(),
        expired.as_str(),
        never_issued.as_str(),
        "",
        "not-a-code",
        "../../claims/x123",
        "ABCDEFGHIJKLMNOP",
    ] {
        let p = g.admit(code, Some(V4_OTHER), NOW).await.unwrap_err();
        answers.push((p.status, p.kind, p.title.clone(), p.extra.clone()));
        assert_eq!(
            (p.status, p.kind),
            (403, "invalid-genesis-code"),
            "{code:?}"
        );
    }
    assert!(answers.windows(2).all(|w| w[0] == w[1]), "{answers:?}");
    assert!(
        answers[0].3["detail"]
            .as_str()
            .unwrap()
            .contains("GET /v1/world/genesis-code")
    );
}

#[tokio::test]
async fn a_code_that_expires_later_still_works() {
    let (store, g) = genesis();
    let code = sfab_genesis::new_code();
    let later = Issue {
        label: "forum-y".into(),
        expires: Some(NOW + 1),
        no_ip_limit: false,
    };
    issue(&*store, &code, &later).await.unwrap();
    assert!(g.admit(&code, Some(V4), NOW).await.is_ok());
}

#[tokio::test]
async fn a_second_birth_from_one_address_inside_the_window_gets_409_too_soon() {
    let (store, g) = genesis();
    let first = testing::seed_code(&*store).await;
    birth(&g, &first, Some(V4), NOW).await;

    let second = testing::seed_code(&*store).await;
    let p = g
        .admit(&second, Some(V4), NOW + 4 * HOUR - 1)
        .await
        .unwrap_err();
    assert_eq!((p.status, p.kind), (409, "too-soon"));
    assert_eq!(p.extra["retry_after"], json!("2026-11-05T16:00:00Z"));
    assert!(g.admit(&second, Some(V4_OTHER), NOW + 1).await.is_ok());
    assert!(g.admit(&second, Some(V4), NOW + 4 * HOUR).await.is_ok());
}

#[tokio::test]
async fn an_exempt_code_skips_the_ip_rule_and_records_nothing() {
    let (store, g) = genesis();
    let first = testing::seed_code(&*store).await;
    birth(&g, &first, Some(V4), NOW).await;

    let exempt = testing::seed_exempt_code(&*store).await;
    birth(&g, &exempt, Some(V4), NOW + HOUR).await;
    let ordinary = testing::seed_code(&*store).await;
    assert_eq!(
        problem(g.admit(&ordinary, Some(V4), NOW + 2 * HOUR).await),
        (409, "too-soon")
    );
    // The window still runs from the ordinary birth: the exempt one recorded nothing.
    assert!(g.admit(&ordinary, Some(V4), NOW + 4 * HOUR).await.is_ok());
    // And a used exempt code is gone like any other.
    assert_eq!(
        problem(g.admit(&exempt, Some(V4_OTHER), NOW).await),
        (403, "invalid-genesis-code")
    );
}

#[tokio::test]
async fn ipv6_addresses_in_one_64_share_a_record() {
    let (store, g) = genesis();
    let first = testing::seed_code(&*store).await;
    birth(&g, &first, Some("2001:db8:1:2:aaaa::1:443"), NOW).await;

    let code = testing::seed_code(&*store).await;
    let same_64 = g
        .admit(&code, Some("[2001:db8:1:2:bbbb::9]:443"), NOW + 1)
        .await;
    assert_eq!(problem(same_64), (409, "too-soon"));
    let other_64 = g.admit(&code, Some("2001:db8:1:3::1:443"), NOW + 1).await;
    assert!(other_64.is_ok());
}

#[tokio::test]
async fn no_raw_address_is_stored() {
    let (store, g) = genesis();
    for (viewer, ip) in [
        (V4, "198.51.100.7"),
        ("2001:db8:1:2:aaaa::1:443", "2001:db8"),
    ] {
        let code = testing::seed_code(&*store).await;
        birth(&g, &code, Some(viewer), NOW).await;
        assert!(!everything(&store).contains(ip), "{ip} stored");
    }
}

#[tokio::test]
async fn an_ordinary_code_without_a_usable_viewer_address_gets_500_and_writes_nothing() {
    let (store, g) = genesis();
    let code = testing::seed_code(&*store).await;
    for viewer in [None, Some(""), Some("not an address"), Some("198.51.100.7")] {
        let before = store.calls();
        assert_eq!(
            problem(g.admit(&code, viewer, NOW).await),
            (500, "about:blank"),
            "{viewer:?}"
        );
        let after = store.calls();
        assert_eq!((after.puts, after.deletes), (before.puts, before.deletes));
    }
}

#[tokio::test]
async fn an_exempt_code_without_a_viewer_address_proceeds() {
    let (store, g) = genesis();
    let code = testing::seed_exempt_code(&*store).await;
    birth(&g, &code, None, NOW).await;
    let code = testing::seed_exempt_code(&*store).await;
    birth(&g, &code, Some("garbage"), NOW).await;
}

#[tokio::test]
async fn the_pool_route_hands_out_todays_codes_and_loses_used_ones() {
    let (store, g) = genesis();
    let codes = testing::seed_world_codes(&*store, NOW, 2).await;

    let reply: Reply = g.pool_code(NOW).await.unwrap();
    assert_eq!(reply.status, 200);
    let given = reply.body["genesis_code"].as_str().unwrap().to_owned();
    assert!(codes.contains(&given));

    birth(&g, &codes[0], Some(V4), NOW).await;
    for _ in 0..20 {
        let reply = g.pool_code(NOW).await.unwrap();
        assert_eq!(reply.body["genesis_code"], json!(codes[1]));
    }
    birth(&g, &codes[1], Some(V4_OTHER), NOW).await;
    let p = g.pool_code(NOW).await.unwrap_err();
    assert_eq!((p.status, p.kind), (409, "try-later"));
    assert_eq!(p.extra["retry_after"], json!("2026-11-06T00:05:00Z"));
}

#[tokio::test]
async fn the_pool_route_answers_not_built_yet_before_the_first_pool() {
    let (_, g) = genesis();
    assert_eq!(problem(g.pool_code(NOW).await), (404, "not-built-yet"));
}

#[tokio::test]
async fn a_pool_from_an_earlier_day_counts_as_empty() {
    let (store, g) = genesis();
    testing::seed_world_codes(&*store, NOW - DAY, 3).await;
    let p = g.pool_code(NOW).await.unwrap_err();
    assert_eq!((p.status, p.kind), (409, "try-later"));
    assert_eq!(p.extra["retry_after"], json!("2026-11-06T00:05:00Z"));
}

#[tokio::test]
async fn an_empty_pool_answers_try_later() {
    let (store, g) = genesis();
    write_pool(&*store, NOW, &[]).await.unwrap();
    assert_eq!(problem(g.pool_code(NOW).await), (409, "try-later"));
}

#[tokio::test]
async fn a_world_code_is_valid_only_for_its_utc_day() {
    let (store, g) = genesis();
    let codes = testing::seed_world_codes(&*store, NOW, 1).await;
    let midnight = (NOW / DAY + 1) * DAY;
    assert!(g.admit(&codes[0], Some(V4), midnight - 1).await.is_ok());
    assert_eq!(
        problem(g.admit(&codes[0], Some(V4), midnight).await),
        (403, "invalid-genesis-code")
    );
}

#[tokio::test]
async fn a_lost_race_on_the_pool_is_ignored() {
    let (store, g) = genesis();
    let codes = testing::seed_world_codes(&*store, NOW, 2).await;
    store.fail_writes_under(Bucket::Identities, "genesis/pool.json");
    birth(&g, &codes[0], Some(V4), NOW).await;
    assert_eq!(
        problem(g.admit(&codes[0], Some(V4_OTHER), NOW).await),
        (403, "invalid-genesis-code")
    );
}

#[tokio::test]
async fn the_checks_cost_what_the_spec_works_out() {
    let (store, g) = genesis();
    let used = testing::seed_code(&*store).await;
    birth(&g, &used, Some(V4), NOW).await;
    testing::seed_world_codes(&*store, NOW, 3).await;

    let cost = |before: Calls| {
        let after = store.calls();
        (
            after.reads - before.reads,
            after.puts - before.puts,
            after.lists - before.lists,
            after.deletes - before.deletes,
        )
    };
    let before = store.calls();
    g.admit(&sfab_genesis::new_code(), Some(V4), NOW)
        .await
        .unwrap_err();
    assert_eq!(cost(before), (1, 0, 0, 0), "a bad code: one HEAD");

    let code = testing::seed_code(&*store).await;
    let before = store.calls();
    g.admit(&code, Some(V4), NOW).await.unwrap_err();
    assert_eq!(cost(before), (2, 0, 0, 0), "blocked: one HEAD and one GET");

    let before = store.calls();
    g.pool_code(NOW).await.unwrap();
    assert_eq!(cost(before), (1, 0, 0, 0), "the route: one GET");

    let world = testing::seed_world_codes(&*store, NOW, 1).await;
    let before = store.calls();
    birth(&g, &world[0], Some(V4_OTHER), NOW).await;
    assert_eq!(
        cost(before),
        (3, 2, 0, 1),
        "a world code: HEAD, IP GET, pool GET; IP and pool PUT; the code's DELETE"
    );
}
