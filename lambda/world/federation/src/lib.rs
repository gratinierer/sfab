//! Federation: `.stories/2-identity-api/spec.md` "Federation" and
//! `docs/adr/0001-initial-architecture.md`. This crate owns the `federation/` keys, the compiled-in
//! directory (`federation/world.yaml` and `federation/worlds.yaml`), the request route, and the
//! daily world job's verification and email ([`daily`]).
//!
//! **Nothing that handles a request calls out.** The routes only read and write S3. The one
//! outbound call, fetching a requester's token, is the daily world job's, made through [`Fetch`],
//! which only the job implements.
//!
//! **Cost at the daily limit**, 100,000 calls a day, if every call were this route
//! (`docs/api-design.md` "Cost"): `GET /federation/world` and `/federation/worlds` read nothing
//! from S3. `POST /federation/requests` makes at most one GET, one LIST of at most about 100 keys
//! and one PUT, about $1.04 a day with S3's prices from memory, and a repeat only the GET.

mod daily;
mod directory;
mod domain;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;

use sfab_core::daily::{next_daily_run, rfc3339};
use sfab_core::gate::{Call, admit_write};
use sfab_core::problem::{ABOUT_BLANK, Problem, Reply};
use sfab_core::proof::hex;
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};

pub use daily::{DailyReport, Fetch, Notify, daily, matches};
pub use directory::{
    Approved, WORLDS_PATH, approved, page as worlds_page, parse_world, parse_worlds, world_name,
};
pub use domain::valid_domain;

/// The most requests waiting for verification; another gets `503` until the next daily run.
pub const MAX_PENDING: usize = 100;
/// How long a request waits for its token: the `federation/pending/` lifecycle rule's 3 days.
pub const PENDING_DAYS: i64 = 3;
/// The requester's manual, which the `202`'s instructions link.
pub const MANUAL: &str = "https://github.com/gratinierer/sfab/blob/main/docs/federation.md";
/// Where the requester publishes the token, under its domain.
pub const TOKEN_PATH: &str = "/.well-known/sfab-federation/";

const PENDING_PREFIX: &str = "federation/pending/";
const VERIFIED_PREFIX: &str = "federation/verified/";
const DAY: i64 = 86_400;

fn pending_key(domain: &str) -> String {
    format!("{PENDING_PREFIX}{domain}.json")
}

fn verified_key(domain: &str) -> String {
    format!("{VERIFIED_PREFIX}{domain}.json")
}

/// This world, as `GET /federation/world` publishes it. The domain and contact come from the
/// owner's config, through the CDK; the name is compiled in from `federation/world.yaml`.
#[derive(Debug, Clone)]
pub struct Here {
    pub domain: String,
    pub contact: String,
}

/// `federation/pending/<domain>.json`, and, once verified, `federation/verified/<domain>.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Stored {
    domain: String,
    token: String,
    requested_at: String,
    expires_at: String,
    /// The proof's hash, as every create with a natural key stores (spec.md "Proof of work").
    proof_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    verified_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestBody {
    domain: String,
}

pub struct Federation<S> {
    store: Arc<S>,
    required_bits: u32,
    here: Here,
}

impl<S: Store> Federation<S> {
    pub fn new(store: Arc<S>, required_bits: u32, here: Here) -> Self {
        Self {
            store,
            required_bits,
            here,
        }
    }

    /// `GET /federation/world`: this world's domain, name, constitution link and contact.
    pub fn world(&self) -> Reply {
        let api = format!("https://api.{}/v1", self.here.domain);
        Reply::ok(json!({
            "domain": self.here.domain,
            "name": world_name(),
            "constitution": format!("{api}/world/constitution"),
            "api": format!("{api}/openapi.yaml"),
            "contact": self.here.contact,
        }))
    }

    /// `GET /federation/worlds`: the approved worlds, 20 a page.
    pub fn worlds(&self, page: Option<&str>) -> Result<Reply, Problem> {
        directory::page(approved(), page)
    }

    /// `POST /federation/requests`: the proof, then a repeat, then the cap, then the write.
    pub async fn request(&self, call: &Call<'_>, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let body: RequestBody = serde_json::from_slice(call.body).map_err(|_| {
            Problem::bad_request(r#"The body must be {"domain": "<hostname>"}"#)
        })?;
        let domain = body.domain;
        if !valid_domain(&domain) {
            return Err(Problem::bad_request(
                "domain must be a lowercase public hostname, such as world.example.org, with no \
                 port, path or trailing dot",
            ));
        }
        let key = pending_key(&domain);
        if let Some(existing) = self.read(&key).await? {
            return Ok(Reply::ok(reply_body(&existing)));
        }
        let waiting = self
            .store
            .list(Bucket::Identities, PENDING_PREFIX)
            .await
            .map_err(internal)?;
        if waiting.len() >= MAX_PENDING {
            return Err(Problem::new(
                503,
                ABOUT_BLANK,
                "100 federation requests are already waiting for verification",
            )
            .with("retry_after", rfc3339(next_daily_run(now))));
        }
        let stored = Stored {
            domain,
            token: new_token(),
            requested_at: rfc3339(now),
            expires_at: rfc3339(now + PENDING_DAYS * DAY),
            proof_hash: hex(&admitted.hash),
            verified_at: None,
        };
        let bytes = serde_json::to_vec(&stored).expect("a request serializes");
        match self
            .store
            .put(Bucket::Identities, &key, bytes, Metadata::new(), Condition::IfAbsent)
            .await
        {
            Ok(()) => Ok(Reply {
                status: 202,
                body: reply_body(&stored),
            }),
            // A concurrent request for the same domain won: answer with its token.
            Err(StoreError::ConditionFailed) => match self.read(&key).await? {
                Some(existing) => Ok(Reply::ok(reply_body(&existing))),
                None => Err(Problem::internal()),
            },
            Err(e) => Err(internal(e)),
        }
    }

    async fn read(&self, key: &str) -> Result<Option<Stored>, Problem> {
        let Some(object) = self.store.get(Bucket::Identities, key).await.map_err(internal)? else {
            return Ok(None);
        };
        serde_json::from_slice(&object.body).map(Some).map_err(|_| {
            tracing::error!("{key} in storage doesn't parse");
            Problem::internal()
        })
    }
}

fn internal(e: StoreError) -> Problem {
    tracing::error!(error = ?e, "federation storage failed");
    Problem::internal()
}

/// 128 random bits, as lowercase hex, so it's safe as a file name.
fn new_token() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the operating system's random generator");
    hex(&bytes)
}

fn reply_body(stored: &Stored) -> serde_json::Value {
    let url = format!("https://{}{TOKEN_PATH}{}", stored.domain, stored.token);
    json!({
        "domain": stored.domain,
        "token": stored.token,
        "url": url,
        "expires_at": stored.expires_at,
        "instructions": format!(
            "Publish the token as the whole body of {url}, over HTTPS, with no redirect, before \
             expires_at; one trailing newline is allowed. An sfab world commits it as \
             website/.well-known/sfab-federation/{token} and deploys, because a file copied in by \
             hand is deleted by the next deploy. This world checks once a day, just after 00:05 \
             UTC, and its owner then decides. See the manual.",
            token = stored.token
        ),
        "manual": MANUAL,
    })
}
