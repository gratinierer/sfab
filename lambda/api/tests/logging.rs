//! Logs carry no `Authorization` value: `.stories/2-identity-api/spec.md` "Logging". Every route
//! runs, successes and failures alike, with a store that fails, and the captured logs must hold
//! no claim, no recovery code and no `Authorization` header.

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use serde_json::Value;
use sfab_core::credentials::caller_of;
use sfab_core::proof;
use sfab_core::storage::{Bucket, MemoryStore};
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;

const BITS: u32 = 4;
const NOW: i64 = 1_793_836_800;

fn now() -> i64 {
    NOW
}

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Captured;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    auth: Option<&str>,
    body: &[u8],
) -> (u16, Value) {
    let mut request = Request::builder().method(method).uri(path);
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
    let app = sfab_api::app(store.clone(), BITS, now, here);
    let (_, born) = send(
        &app,
        "POST",
        "/v1/identity/birth",
        None,
        br#"{"name":"Wren"}"#,
    )
    .await;
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
    send(
        &app,
        "GET",
        "/v1/identity/self",
        Some("Bearer not-a-token"),
        b"",
    )
    .await;

    let logs = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
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
}
