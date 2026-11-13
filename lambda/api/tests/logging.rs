//! Logs carry no `Authorization` value: `.stories/2-identity-api/spec.md` "Logging". Every route
//! runs, successes and failures alike, with a store that fails, and the captured logs must hold
//! no claim, no recovery code and no `Authorization` header. Nor does any log line or stored
//! object hold the viewer's raw IP address (`.stories/17-genesis-codes/spec.md` "The IP rule"):
//! births use ordinary genesis codes from a viewer address, so the IP rule's read and write run.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use serde_json::Value;
use sfab_core::credentials::caller_of;
use sfab_core::proof;
use sfab_core::storage::{Bucket, MemoryStore};
use sfab_core::testing::Captured;
use sfab_genesis::testing as genesis;
use tower::ServiceExt;

const BITS: u32 = 4;
const NOW: i64 = 1_793_836_800;
const V4: &str = "198.51.100.7";
const V6: &str = "2001:db8:1:2:aaaa::1";

fn now() -> i64 {
    NOW
}

/// A request from `V4`, as CloudFront forwards it.
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    auth: Option<&str>,
    body: &[u8],
) -> (u16, Value) {
    let viewer = format!("{V4}:46532");
    send_from(app, Some(&viewer), method, path, auth, body).await
}

async fn send_from(
    app: &Router,
    viewer: Option<&str>,
    method: &str,
    path: &str,
    auth: Option<&str>,
    body: &[u8],
) -> (u16, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(v) = viewer {
        request = request.header("cloudfront-viewer-address", v);
    }
    if let Some(a) = auth {
        request = request.header("authorization", a);
    }
    if method != "GET" {
        let caller = caller_of(auth).unwrap();
        let r = proof::Request {
            method,
            path,
            caller: &caller,
            body,
        };
        request = request.header("proof", proof::solve(&r, NOW, BITS));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_vec())).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn birth_body(code: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"name": "Wren", "genesis_code": code})).unwrap()
}

/// A `birth` with `code` from `viewer`, and its status.
async fn birth_from(app: &Router, viewer: Option<&str>, code: &str) -> u16 {
    let path = "/v1/identity/birth";
    send_from(app, viewer, "POST", path, None, &birth_body(code))
        .await
        .0
}

#[tokio::test(flavor = "current_thread")]
async fn logs_carry_no_authorization_value() {
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let store = Arc::new(MemoryStore::new());
    let here = sfab_federation::Here {
        domain: "sfab.example.org".into(),
        contact: "abuse@sfab.example.org".into(),
    };
    let app = sfab_api::app(store.clone(), BITS, genesis::ip_rule(), now, here);
    let code = genesis::seed_code(&*store).await;
    let (status, born) = send(&app, "POST", "/v1/identity/birth", None, &birth_body(&code)).await;
    assert_eq!(status, 201);
    // Too soon from the same address, then from an IPv6 viewer, then with no viewer address.
    let code = genesis::seed_code(&*store).await;
    let (status, _) = send(&app, "POST", "/v1/identity/birth", None, &birth_body(&code)).await;
    assert_eq!(status, 409);
    let v6 = format!("{V6}:443");
    assert_eq!(birth_from(&app, Some(&v6), &code).await, 201);
    let code = genesis::seed_code(&*store).await;
    assert_eq!(birth_from(&app, None, &code).await, 500);
    assert_eq!(birth_from(&app, Some("garbage"), &code).await, 500);
    send(&app, "GET", "/v1/world/genesis-code", None, b"").await;
    let (id, claim, recovery) = (
        born["id"].as_str().unwrap().to_owned(),
        born["claim"].as_str().unwrap().to_owned(),
        born["recovery"].as_str().unwrap().to_owned(),
    );
    let auth = format!("Bearer {id}.{claim}");
    let recovery_auth = format!("Bearer {id}.{recovery}");
    send(&app, "GET", "/v1/identity/self", Some(&auth), b"").await;
    send(
        &app,
        "PUT",
        "/v1/identity/evolve",
        Some(&auth),
        br#"{"whoiam":"x"}"#,
    )
    .await;

    // Now every write fails, so the error paths log too.
    store.fail_writes_under(Bucket::Identities, "");
    store.fail_writes_under(Bucket::Claims, "");
    let (status, _) = send(
        &app,
        "PUT",
        "/v1/identity/evolve",
        Some(&auth),
        br#"{"whoiam":"y"}"#,
    )
    .await;
    assert_eq!(status, 500);
    let (status, _) = send(
        &app,
        "POST",
        "/v1/identity/reclaim",
        Some(&recovery_auth),
        b"",
    )
    .await;
    assert_eq!(status, 500);
    let other_v6 = "2001:db8:9:9::1:443";
    assert_eq!(birth_from(&app, Some(other_v6), &code).await, 500);
    send(
        &app,
        "GET",
        "/v1/identity/self",
        Some("Bearer not-a-token"),
        b"",
    )
    .await;

    let logs = captured.text();
    assert!(
        logs.contains("call failed"),
        "the failures were logged: {logs}"
    );
    for secret in [
        claim.as_str(),
        recovery.as_str(),
        "Bearer",
        "authorization",
        "not-a-token",
    ] {
        assert!(!logs.contains(secret), "the logs hold {secret:?}: {logs}");
    }
    let stored: String = [Bucket::Identities, Bucket::Claims]
        .into_iter()
        .flat_map(|b| store.keys(b).into_iter().map(move |k| (b, k)))
        .map(|(b, k)| {
            let o = store.object(b, &k).unwrap();
            format!(
                "{k} {} {:?}\n",
                String::from_utf8_lossy(&o.body),
                o.metadata
            )
        })
        .collect();
    assert!(
        stored.contains("genesis/ips/"),
        "the IP rule wrote its record: {stored}"
    );
    for address in [V4, V6, "2001:db8:1:2", "2001:db8:9:9", "garbage"] {
        assert!(!logs.contains(address), "the logs hold {address:?}: {logs}");
        assert!(
            !stored.contains(address),
            "storage holds {address:?}: {stored}"
        );
    }
}
