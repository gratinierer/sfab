//! The job's outbound check against a fake HTTPS server, through the federation step: a match,
//! the token plus one trailing newline matching, any other extra content not matching, a wrong
//! body, a redirect, a private address, a slow response and an oversized one
//! (`.stories/2-identity-api/spec.md` "Federation" ("Outbound calls") and "Testing").

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde_json::{Value, json};
use sfab_core::gate::Call;
use sfab_core::id::Caller;
use sfab_core::proof::{self, Request};
use sfab_core::storage::{Bucket, MemoryStore};
use sfab_federation::{Federation, Fetch, Here, Notify, daily};
use sfab_job::outbound::{Outbound, Resolve, is_public};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const DOMAIN: &str = "fake-world.example.org";
const BITS: u32 = 4;
/// 2026-11-05T12:00:00Z, and the next run.
const NOW: i64 = 1_793_836_800 + 43_200;
const RUN: i64 = 1_793_836_800 + 86_400 + 300;
/// Short, so the slow case doesn't slow the suite; production's is 3 seconds.
const TIMEOUT: Duration = Duration::from_millis(500);

/// One HTTPS server on 127.0.0.1 with a certificate for [`DOMAIN`], answering every request with
/// `answer`, after `delay`. It counts connections and records each request's head.
struct FakeServer {
    port: u16,
    root: CertificateDer<'static>,
    connections: Arc<AtomicUsize>,
    heads: Arc<Mutex<Vec<String>>>,
}

async fn serve(answer: Vec<u8>, delay: Duration) -> FakeServer {
    let key = rcgen::generate_simple_self_signed(vec![DOMAIN.to_owned()]).unwrap();
    let root = key.cert.der().clone();
    let private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()));
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![root.clone()], private)
        .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let connections = Arc::new(AtomicUsize::new(0));
    let heads = Arc::new(Mutex::new(Vec::new()));
    let (count, seen) = (connections.clone(), heads.clone());
    tokio::spawn(async move {
        loop {
            let (tcp, _) = listener.accept().await.unwrap();
            count.fetch_add(1, Ordering::SeqCst);
            let (acceptor, answer, seen) = (acceptor.clone(), answer.clone(), seen.clone());
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(tcp).await else {
                    return;
                };
                let mut head = Vec::new();
                let mut buf = [0u8; 1024];
                while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                    match tls.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => head.extend_from_slice(&buf[..n]),
                    }
                }
                seen.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&head).into_owned());
                tokio::time::sleep(delay).await;
                let _ = tls.write_all(&answer).await;
                let _ = tls.shutdown().await;
            });
        }
    });
    FakeServer {
        port,
        root,
        connections,
        heads,
    }
}

fn ok(body: &[u8]) -> Vec<u8> {
    let mut answer = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    answer.extend_from_slice(body);
    answer
}

/// The outbound check pointed at the fake server, with every production limit but the timeout.
fn outbound_to(server: &FakeServer, at: IpAddr) -> Outbound {
    Outbound::with_roots(vec![server.root.clone()])
        .resolving(Resolve::Fixed(vec![at]))
        .on_port(server.port)
        .timing_out_after(TIMEOUT)
}

/// Allows the fake server's loopback address, which production refuses.
fn loopback_allowed(server: &FakeServer) -> Outbound {
    outbound_to(server, IpAddr::V4(Ipv4Addr::LOCALHOST)).allowing(|_| true)
}

struct NoEmail;

impl Notify for NoEmail {
    async fn notify(&self, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
}

/// A pending request for [`DOMAIN`], and its token.
async fn pending(store: &Arc<MemoryStore>) -> String {
    let here = Here {
        domain: "sfab.example.org".into(),
        contact: "abuse@sfab.example.org".into(),
    };
    let f = Federation::new(store.clone(), BITS, here);
    let body = serde_json::to_vec(&json!({ "domain": DOMAIN })).unwrap();
    let path = "/v1/federation/requests";
    let request = Request {
        method: "POST",
        path,
        caller: &Caller::Anonymous,
        body: &body,
    };
    let proof = proof::solve(&request, NOW, BITS);
    let call = Call {
        method: "POST",
        path,
        authorization: None,
        proof: Some(&proof),
        body: &body,
    };
    let reply: Value = f.request(&call, NOW).await.unwrap().body;
    reply["token"].as_str().unwrap().to_owned()
}

/// Runs the federation step against a server answering `answer(token)`, and says whether the
/// request was verified.
async fn verified_when_serving(answer: impl Fn(&str) -> Vec<u8>, delay: Duration) -> bool {
    let store = Arc::new(MemoryStore::new());
    let token = pending(&store).await;
    let server = serve(answer(&token), delay).await;
    let report = daily(&*store, &loopback_allowed(&server), &NoEmail, RUN)
        .await
        .unwrap();
    let moved = store
        .object(
            Bucket::Identities,
            &format!("federation/verified/{DOMAIN}.json"),
        )
        .is_some();
    assert_eq!(report.verified == 1, moved);
    moved
}

#[tokio::test]
async fn the_token_as_the_whole_body_matches() {
    assert!(verified_when_serving(|t| ok(t.as_bytes()), Duration::ZERO).await);
}

#[tokio::test]
async fn the_token_plus_one_trailing_newline_matches() {
    assert!(verified_when_serving(|t| ok(format!("{t}\n").as_bytes()), Duration::ZERO).await);
}

#[tokio::test]
async fn any_other_extra_content_does_not_match() {
    for extra in ["{t}\n\n", "{t}\r\n", "{t} ", "<p>{t}</p>", "token: {t}"] {
        let verified =
            verified_when_serving(|t| ok(extra.replace("{t}", t).as_bytes()), Duration::ZERO).await;
        assert!(!verified, "{extra:?}");
    }
}

#[tokio::test]
async fn a_wrong_body_does_not_match() {
    let wrong = |_: &str| ok(b"0123456789abcdef0123456789abcdef");
    assert!(!verified_when_serving(wrong, Duration::ZERO).await);
}

#[tokio::test]
async fn the_token_on_another_status_does_not_match() {
    let not_found = |t: &str| {
        format!(
            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{t}",
            t.len()
        )
        .into_bytes()
    };
    assert!(!verified_when_serving(not_found, Duration::ZERO).await);
}

#[tokio::test]
async fn a_redirect_is_not_followed() {
    let store = Arc::new(MemoryStore::new());
    let token = pending(&store).await;
    let redirect = format!(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: https://{DOMAIN}/.well-known/sfab-federation/{token}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{token}",
        token.len()
    );
    let server = serve(redirect.into_bytes(), Duration::ZERO).await;

    let report = daily(&*store, &loopback_allowed(&server), &NoEmail, RUN)
        .await
        .unwrap();

    assert_eq!(report.verified, 0);
    assert_eq!(
        server.connections.load(Ordering::SeqCst),
        1,
        "one request, not followed"
    );
}

#[tokio::test]
async fn a_private_address_is_refused_without_connecting() {
    for at in [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
    ] {
        let store = Arc::new(MemoryStore::new());
        let token = pending(&store).await;
        let server = serve(ok(token.as_bytes()), Duration::ZERO).await;

        // Production's address check, with the fake server's loopback address.
        let report = daily(&*store, &outbound_to(&server, at), &NoEmail, RUN)
            .await
            .unwrap();

        assert_eq!(report.verified, 0, "{at}");
        assert_eq!(server.connections.load(Ordering::SeqCst), 0, "{at}");
    }
}

#[tokio::test]
async fn one_private_address_among_public_ones_refuses_the_domain() {
    let store = Arc::new(MemoryStore::new());
    let token = pending(&store).await;
    let server = serve(ok(token.as_bytes()), Duration::ZERO).await;
    let both = Resolve::Fixed(vec![
        IpAddr::V4(Ipv4Addr::new(93, 184, 215, 14)),
        IpAddr::V4(Ipv4Addr::LOCALHOST),
    ]);
    let outbound = outbound_to(&server, IpAddr::V4(Ipv4Addr::LOCALHOST)).resolving(both);

    let report = daily(&*store, &outbound, &NoEmail, RUN).await.unwrap();

    assert_eq!(report.verified, 0);
    assert_eq!(server.connections.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_slow_response_times_out() {
    let started = std::time::Instant::now();
    let slow = |t: &str| ok(t.as_bytes());
    assert!(!verified_when_serving(slow, TIMEOUT * 4).await);
    assert!(started.elapsed() < TIMEOUT * 3, "gave up at the timeout");
}

#[tokio::test]
async fn an_oversized_response_does_not_match() {
    // The token first, then padding past 1 KB.
    let oversized = |t: &str| ok(format!("{t}\n{}", "x".repeat(1024)).as_bytes());
    assert!(!verified_when_serving(oversized, Duration::ZERO).await);
}

#[tokio::test]
async fn at_most_1_kb_is_read_with_or_without_a_content_length() {
    let path = "/.well-known/sfab-federation/x";
    let streamed = |body: &[u8]| {
        let mut answer = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        answer.extend_from_slice(body);
        answer
    };
    let fits = vec![b'a'; 1024];
    let over = vec![b'a'; 1025];
    for (answer, expected) in [
        (ok(&fits), Some(fits.clone())),
        (ok(&over), None),
        (streamed(&fits), Some(fits.clone())),
        (streamed(&over), None),
    ] {
        let server = serve(answer, Duration::ZERO).await;
        let got = loopback_allowed(&server).fetch(DOMAIN, path).await;
        assert_eq!(got.map(|b| b.len()), expected.map(|b| b.len()));
    }
}

#[tokio::test]
async fn it_asks_for_the_one_fixed_path_over_https_with_the_domain_as_host() {
    let store = Arc::new(MemoryStore::new());
    let token = pending(&store).await;
    let server = serve(ok(token.as_bytes()), Duration::ZERO).await;

    daily(&*store, &loopback_allowed(&server), &NoEmail, RUN)
        .await
        .unwrap();

    let heads = server.heads.lock().unwrap().clone();
    assert_eq!(heads.len(), 1);
    let head = heads[0].to_lowercase();
    assert!(
        head.starts_with(&format!(
            "get /.well-known/sfab-federation/{token} http/1.1\r\n"
        )),
        "{head}"
    );
    assert!(head.contains(&format!("\r\nhost: {DOMAIN}\r\n")), "{head}");
}

#[tokio::test]
async fn a_certificate_for_another_name_is_refused() {
    let store = Arc::new(MemoryStore::new());
    pending(&store).await;
    // Trusts nothing the server can show.
    let other = rcgen::generate_simple_self_signed(vec!["other.example.org".to_owned()]).unwrap();
    let server = serve(ok(b"x"), Duration::ZERO).await;
    let outbound = Outbound::with_roots(vec![other.cert.der().clone()])
        .resolving(Resolve::Fixed(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]))
        .on_port(server.port)
        .timing_out_after(TIMEOUT)
        .allowing(|_| true);

    let report = daily(&*store, &outbound, &NoEmail, RUN).await.unwrap();

    assert_eq!(report.verified, 0);
    assert!(
        server.heads.lock().unwrap().is_empty(),
        "no request without a trusted certificate"
    );
}

#[test]
fn only_public_addresses_may_be_fetched() {
    for public in ["93.184.215.14", "8.8.8.8", "2606:4700::1111"] {
        assert!(is_public(public.parse().unwrap()), "{public}");
    }
    for private in [
        "0.0.0.0",
        "10.1.2.3",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "172.31.255.255",
        "192.0.0.8",
        "192.0.2.1",
        "192.168.1.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "240.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "64:ff9b::a00:1",
        "fc00::1",
        "fd12:3456::1",
        "fe80::1",
        "ff02::1",
        "2001:db8::1",
    ] {
        assert!(!is_public(private.parse().unwrap()), "{private}");
    }
}

#[test]
fn production_uses_https_on_443_with_a_3_second_timeout_and_a_1_kb_limit() {
    let production = Outbound::production();
    assert_eq!(production.port(), 443);
    assert_eq!(production.timeout(), Duration::from_secs(3));
    assert_eq!(production.limit(), 1024);
}
