//! The one outbound call to a third party: the daily world job fetching a federation requester's
//! token (`.stories/2-identity-api/spec.md` "Federation" ("Outbound calls") and
//! `docs/adr/0001-initial-architecture.md`). HTTPS on port 443 only, no redirects, private and
//! link-local addresses refused after resolving, a 3-second timeout and a 1 KB limit, and the
//! response is only compared with the token, by `sfab_federation`.
//!
//! Nothing that handles a request reaches this: only the job's binary builds an [`Outbound`].
//! It connects only to the addresses it checked, so a name that resolves differently a second
//! time can't slip past the check.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Limited};
use hyper_util::rt::TokioIo;
use rustls::ClientConfig;
use rustls::pki_types::{CertificateDer, ServerName};
use sfab_federation::Fetch;
use tokio::net::TcpStream;

/// HTTPS's port, the only one fetched.
pub const PORT: u16 = 443;
/// The whole fetch, from resolving to the last byte.
pub const TIMEOUT: Duration = Duration::from_secs(3);
/// The most body bytes read. A token is 32 characters.
pub const LIMIT: usize = 1024;
const USER_AGENT: &str =
    "sfab-federation-check (+https://github.com/gratinierer/sfab/blob/main/docs/federation.md)";

/// Where a domain's addresses come from.
#[derive(Debug, Clone)]
pub enum Resolve {
    /// The system resolver, as in production.
    Dns,
    /// These addresses, for tests.
    Fixed(Vec<IpAddr>),
}

/// The outbound check. [`Outbound::production`] has every limit; the builder methods exist for
/// tests against a local fake server.
#[derive(Clone)]
pub struct Outbound {
    tls: Arc<ClientConfig>,
    resolve: Resolve,
    port: u16,
    timeout: Duration,
    allowed: fn(IpAddr) -> bool,
}

/// Why a fetch gave nothing, for the job's log, which reads the fields through `Debug`.
#[derive(Debug)]
#[allow(dead_code)]
enum Refusal {
    NoAddress,
    NotPublic(IpAddr),
    Connect,
    Tls,
    Http,
    Status(u16),
    TooLarge,
}

impl Outbound {
    /// HTTPS on 443 with the system's trusted roots, public addresses only, 3 seconds, 1 KB.
    pub fn production() -> Self {
        let found = rustls_native_certs::load_native_certs();
        if !found.errors.is_empty() {
            tracing::error!(
                errors = found.errors.len(),
                "some system certificates didn't load"
            );
        }
        Self::with_roots(found.certs)
    }

    /// Production's limits, trusting only `roots`.
    pub fn with_roots(roots: Vec<CertificateDer<'static>>) -> Self {
        let mut store = rustls::RootCertStore::empty();
        store.add_parsable_certificates(roots);
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let mut tls = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("the provider supports the default versions")
            .with_root_certificates(store)
            .with_no_client_auth();
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        Self {
            tls: Arc::new(tls),
            resolve: Resolve::Dns,
            port: PORT,
            timeout: TIMEOUT,
            allowed: is_public,
        }
    }

    pub fn resolving(mut self, resolve: Resolve) -> Self {
        self.resolve = resolve;
        self
    }

    pub fn on_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn timing_out_after(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Replaces the address check, so a test can reach its server on loopback.
    pub fn allowing(mut self, allowed: fn(IpAddr) -> bool) -> Self {
        self.allowed = allowed;
        self
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn limit(&self) -> usize {
        LIMIT
    }

    async fn get(&self, domain: &str, path: &str) -> Result<Vec<u8>, Refusal> {
        let addresses: Vec<IpAddr> = match &self.resolve {
            Resolve::Dns => tokio::net::lookup_host((domain, self.port))
                .await
                .map_err(|_| Refusal::NoAddress)?
                .map(|a| a.ip())
                .collect(),
            Resolve::Fixed(found) => found.clone(),
        };
        if addresses.is_empty() {
            return Err(Refusal::NoAddress);
        }
        // Every address must be public, so the answer can't depend on which one is tried.
        if let Some(bad) = addresses.iter().find(|ip| !(self.allowed)(**ip)) {
            return Err(Refusal::NotPublic(*bad));
        }
        let mut tcp = None;
        for ip in &addresses {
            if let Ok(stream) = TcpStream::connect(SocketAddr::new(*ip, self.port)).await {
                tcp = Some(stream);
                break;
            }
        }
        let tcp = tcp.ok_or(Refusal::Connect)?;
        let name = ServerName::try_from(domain.to_owned()).map_err(|_| Refusal::Tls)?;
        let tls = tokio_rustls::TlsConnector::from(self.tls.clone())
            .connect(name, tcp)
            .await
            .map_err(|_| Refusal::Tls)?;

        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
            .await
            .map_err(|_| Refusal::Http)?;
        let driver = AbortOnDrop(tokio::spawn(connection));
        let request = http::Request::get(path)
            .header(http::header::HOST, domain)
            .header(http::header::USER_AGENT, USER_AGENT)
            .header(http::header::CONNECTION, "close")
            .body(Empty::<Bytes>::new())
            .map_err(|_| Refusal::Http)?;
        // One request, and its answer as it is: a redirect is just a status that isn't 200.
        let response = sender
            .send_request(request)
            .await
            .map_err(|_| Refusal::Http)?;
        if response.status() != http::StatusCode::OK {
            return Err(Refusal::Status(response.status().as_u16()));
        }
        let body = Limited::new(response.into_body(), LIMIT)
            .collect()
            .await
            .map_err(|e| {
                if e.is::<http_body_util::LengthLimitError>() {
                    Refusal::TooLarge
                } else {
                    Refusal::Http
                }
            })?
            .to_bytes();
        drop(driver);
        Ok(body.to_vec())
    }
}

impl Fetch for Outbound {
    async fn fetch(&self, domain: &str, path: &str) -> Option<Vec<u8>> {
        match tokio::time::timeout(self.timeout, self.get(domain, path)).await {
            Ok(Ok(body)) => Some(body),
            Ok(Err(refusal)) => {
                tracing::info!(domain, ?refusal, "federation token not fetched");
                None
            }
            Err(_) => {
                tracing::info!(domain, "federation token fetch timed out");
                None
            }
        }
    }
}

/// Stops the connection's task when the fetch ends, finished or timed out.
struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Whether an address is on the public internet: not private, loopback, link-local, shared,
/// documentation, benchmarking, multicast, reserved or unspecified, in IPv4 or IPv6, including
/// IPv4 inside IPv6.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_v4(v4);
            }
            let s = v6.segments();
            !(v6.is_unspecified()
                || v6.is_loopback()
                || v6.is_multicast()
                // Unique local, fc00::/7, and link-local, fe80::/10.
                || (s[0] & 0xfe00) == 0xfc00
                || (s[0] & 0xffc0) == 0xfe80
                // Documentation, 2001:db8::/32.
                || (s[0] == 0x2001 && s[1] == 0x0db8)
                // NAT64, 64:ff9b::/96, which can reach private IPv4.
                || (s[0] == 0x0064 && s[1] == 0xff9b)
                // IPv4-compatible, ::/96.
                || s[..6].iter().all(|&x| x == 0))
        }
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_unspecified()
        || ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        // "This network", 0.0.0.0/8.
        || a == 0
        // Shared address space, 100.64.0.0/10.
        || (a == 100 && (64..128).contains(&b))
        // IETF protocol assignments, 192.0.0.0/24.
        || (a == 192 && b == 0 && c == 0)
        // Benchmarking, 198.18.0.0/15.
        || (a == 198 && (18..20).contains(&b))
        // Reserved, 240.0.0.0/4.
        || a >= 240)
}
