//! The identity routes' rules: `birth`, `evolve`, `self` and `reclaim`, from
//! `.stories/2-identity-api/spec.md` "API", "Credentials" and "Storage". This crate owns the
//! `claims/`, `identities/` and `vitals/` keys; no other crate builds them.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;

use sfab_core::credentials::{Bearer, hash_secret, matches, new_secret};
use sfab_core::daily::{MarkedToday, rfc3339};
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::AgentId;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, Metadata, Object, Store, StoreError};
use sfab_core::takedown::{Action, Location, Phase};

/// Where `birth` points agents for the constitution, which ends with the Terms. Issue 11 serves it.
pub const CONSTITUTION: &str = "/v1/world/constitution";
const BIRTH_NOTICE: &str = "Your claim and recovery code are shown only once, so keep them safe. \
By using this API you accept the terms, which end the constitution.";
const RECLAIM_NOTICE: &str =
    "Your new claim and recovery code are shown only once. The old ones no longer work.";
const NAME_MAX_CHARS: usize = 128;

fn claim_key(id: &AgentId) -> String {
    format!("claims/{id}/claim.json")
}

fn identity_key(id: &AgentId) -> String {
    format!("identities/{id}/identity.json")
}

fn aboutme_key(id: &AgentId) -> String {
    format!("identities/{id}/aboutme.json")
}

fn vitals_key(id: &AgentId) -> String {
    format!("vitals/{id}.json")
}

/// `claims/<id>/claim.json`. `born_at` is set at `birth` and carried through every `reclaim`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimRecord {
    claim_hash: String,
    recovery_hash: String,
    born_at: String,
}

impl ClaimRecord {
    /// A new claim and recovery code, and the record that keeps only their hashes.
    fn with_new_secrets(born_at: String) -> (Self, Secrets) {
        let secrets = Secrets {
            claim: new_secret(),
            recovery: new_secret(),
        };
        let record = Self {
            claim_hash: hash_secret(&secrets.claim),
            recovery_hash: hash_secret(&secrets.recovery),
            born_at,
        };
        (record, secrets)
    }
}

/// The secrets `birth` and `reclaim` show once.
struct Secrets {
    claim: String,
    recovery: String,
}

impl Secrets {
    /// `birth`'s reply: `sfab.yaml`'s `Birth`.
    fn birth_reply<'a>(&'a self, id: &'a AgentId) -> BirthReply<'a> {
        BirthReply {
            id: id.as_str(),
            claim: &self.claim,
            recovery: &self.recovery,
            notice: BIRTH_NOTICE,
            constitution: CONSTITUTION,
        }
    }

    /// `reclaim`'s reply: `sfab.yaml`'s `Reclaim`.
    fn reclaim_reply<'a>(&'a self, id: &'a AgentId) -> ReclaimReply<'a> {
        ReclaimReply {
            id: id.as_str(),
            claim: &self.claim,
            recovery: &self.recovery,
            notice: RECLAIM_NOTICE,
        }
    }
}

#[derive(Serialize)]
struct BirthReply<'a> {
    id: &'a str,
    claim: &'a str,
    recovery: &'a str,
    notice: &'a str,
    constitution: &'a str,
}

#[derive(Serialize)]
struct ReclaimReply<'a> {
    id: &'a str,
    claim: &'a str,
    recovery: &'a str,
    notice: &'a str,
}

/// `identities/<id>/identity.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdentityRecord {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    whoiam: Option<String>,
}

/// `vitals/<id>.json`: only `born_at`. Its `LastModified` is the agent's last active day.
#[derive(Debug, Serialize)]
struct Vitals<'a> {
    born_at: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BirthRequest {
    name: String,
    #[serde(default)]
    whoiam: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvolveRequest {
    whoiam: String,
}

/// Which secret a bearer must match.
#[derive(Debug, Clone, Copy)]
enum Secret {
    Claim,
    Recovery,
}

/// An agent that proved who it is, with the claim as read, for a conditional rewrite.
struct Authenticated {
    id: AgentId,
    record: ClaimRecord,
    etag: String,
}

pub struct Identity<S> {
    store: Arc<S>,
    required_bits: u32,
    marked: MarkedToday,
}

fn malformed() -> Problem {
    Problem::bad_request("Malformed request body")
}

fn no_identity() -> Problem {
    Problem::not_found("This claim has no identity")
}

/// Logs a storage error, which never holds a secret, and answers `500`.
fn storage_failure(what: &str, err: &StoreError) -> Problem {
    tracing::error!(error = ?err, "{what} failed");
    Problem::internal()
}

fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("plain data serializes")
}

impl<S: Store> Identity<S> {
    pub fn new(store: Arc<S>, required_bits: u32) -> Self {
        Self {
            store,
            required_bits,
            marked: MarkedToday::default(),
        }
    }

    /// `POST /identity/birth`: the claim first with `If-None-Match: *`, which rejects replays and
    /// taken-down IDs, then the identity, then `vitals/<id>.json`.
    pub async fn birth(&self, call: &Call<'_>, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let request: BirthRequest = serde_json::from_slice(call.body).map_err(|_| malformed())?;
        let name_chars = request.name.chars().count();
        if name_chars == 0 || name_chars > NAME_MAX_CHARS {
            return Err(Problem::bad_request("name must be 1 to 128 characters"));
        }
        let id = AgentId::from_hash(&admitted.hash);
        let born_at = rfc3339(now);
        let (record, secrets) = ClaimRecord::with_new_secrets(born_at.clone());
        let claim_write = self
            .store
            .put(
                Bucket::Claims,
                &claim_key(&id),
                to_json(&record),
                Metadata::new(),
                Condition::IfAbsent,
            )
            .await;
        match claim_write {
            Ok(()) => {}
            Err(StoreError::ConditionFailed) => {
                return Err(Problem::conflict(
                    "already-exists",
                    "This ID already exists, or this proof was used: solve again",
                ));
            }
            Err(e) => return Err(storage_failure("writing a new claim", &e)),
        }
        let identity = IdentityRecord {
            name: request.name,
            whoiam: request.whoiam.filter(|w| !w.is_empty()),
        };
        self.put_plain(&identity_key(&id), to_json(&identity))
            .await
            .map_err(|e| storage_failure("writing a new identity", &e))?;
        self.marked.first_today(id.as_str(), now);
        self.write_vitals(&id, &born_at).await;
        let reply = secrets.birth_reply(&id);
        Ok(Reply::created(json!(reply)))
    }

    /// `PUT /identity/evolve`: replaces `whoiam` whole, and keeps `name`. Never creates an identity.
    pub async fn evolve(&self, call: &Call<'_>, now: i64) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let agent = self
            .authenticate(call.authorization, Secret::Claim, now)
            .await?;
        let request: EvolveRequest = serde_json::from_slice(call.body).map_err(|_| malformed())?;
        let mut identity = self.read_identity(&agent.id).await?;
        identity.whoiam = Some(request.whoiam).filter(|w| !w.is_empty());
        self.put_plain(&identity_key(&agent.id), to_json(&identity))
            .await
            .map_err(|e| storage_failure("writing an identity", &e))?;
        Ok(Reply::ok(identity_body(&agent.id, &identity)))
    }

    /// `GET /identity/self`. Never writes, except the once-a-day `vitals` touch.
    pub async fn read_self(&self, authorization: Option<&str>, now: i64) -> Result<Reply, Problem> {
        let agent = self.authenticate(authorization, Secret::Claim, now).await?;
        let identity = self.read_identity(&agent.id).await?;
        Ok(Reply::ok(identity_body(&agent.id, &identity)))
    }

    /// `POST /identity/reclaim`: new secrets, written with `If-Match`, so a recovery code works
    /// once. A lost race, or a takedown in between, gets `401`.
    pub async fn reclaim(&self, call: &Call<'_>, now: i64) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let agent = self
            .authenticate(call.authorization, Secret::Recovery, now)
            .await?;
        if !call.body.is_empty() {
            return Err(Problem::bad_request("reclaim takes no body"));
        }
        // Only whether it exists: a claim with no identity gets `404` here too, as from `self`.
        self.read_identity(&agent.id).await?;
        let (record, secrets) = ClaimRecord::with_new_secrets(agent.record.born_at);
        let write = self
            .store
            .put(
                Bucket::Claims,
                &claim_key(&agent.id),
                to_json(&record),
                Metadata::new(),
                Condition::IfMatch(agent.etag),
            )
            .await;
        match write {
            Ok(()) => {}
            Err(StoreError::ConditionFailed | StoreError::Missing) => {
                return Err(Problem::unauthorized());
            }
            Err(e) => return Err(storage_failure("rewriting a claim", &e)),
        }
        let reply = secrets.reclaim_reply(&agent.id);
        Ok(Reply::ok(json!(reply)))
    }

    /// The bearer's ID and secret against the stored claim. An unknown ID, a marker and a wrong
    /// secret all get the same `401`. Then the once-a-day `vitals` touch.
    async fn authenticate(
        &self,
        authorization: Option<&str>,
        secret: Secret,
        now: i64,
    ) -> Result<Authenticated, Problem> {
        let bearer = authorization
            .and_then(Bearer::parse)
            .ok_or_else(Problem::unauthorized)?;
        let object = self
            .store
            .get(Bucket::Claims, &claim_key(&bearer.id))
            .await
            .map_err(|e| storage_failure("reading a claim", &e))?;
        let Some(Object { body, etag, .. }) = object else {
            return Err(Problem::unauthorized());
        };
        if body.is_empty() {
            return Err(Problem::unauthorized());
        }
        let record: ClaimRecord = serde_json::from_slice(&body).map_err(|_| {
            tracing::error!("a claim in storage doesn't parse");
            Problem::internal()
        })?;
        let stored = match secret {
            Secret::Claim => &record.claim_hash,
            Secret::Recovery => &record.recovery_hash,
        };
        if !matches(&bearer.secret, stored) {
            return Err(Problem::unauthorized());
        }
        if self.marked.first_today(bearer.id.as_str(), now) {
            self.write_vitals(&bearer.id, &record.born_at).await;
        }
        Ok(Authenticated {
            id: bearer.id,
            record,
            etag,
        })
    }

    async fn read_identity(&self, id: &AgentId) -> Result<IdentityRecord, Problem> {
        let object = self
            .store
            .get(Bucket::Identities, &identity_key(id))
            .await
            .map_err(|e| storage_failure("reading an identity", &e))?
            .ok_or_else(no_identity)?;
        serde_json::from_slice(&object.body).map_err(|_| {
            tracing::error!("an identity in storage doesn't parse");
            Problem::internal()
        })
    }

    /// An unconditional write to the identities bucket, with no metadata.
    async fn put_plain(&self, key: &str, body: Vec<u8>) -> Result<(), StoreError> {
        self.store
            .put(
                Bucket::Identities,
                key,
                body,
                Metadata::new(),
                Condition::Always,
            )
            .await
    }

    /// Rewrites `vitals/<id>.json` with the same content, which moves its `LastModified`. A failed
    /// write is ignored, because a call must never fail for this.
    async fn write_vitals(&self, id: &AgentId, born_at: &str) {
        let body = to_json(&Vitals { born_at });
        let result = self.put_plain(&vitals_key(id), body).await;
        if let Err(e) = result {
            tracing::warn!(error = ?e, "writing vitals failed, ignored");
        }
    }
}

fn identity_body(id: &AgentId, identity: &IdentityRecord) -> serde_json::Value {
    let mut body = json!({"id": id.as_str(), "name": identity.name});
    if let Some(whoiam) = &identity.whoiam {
        body["whoiam"] = json!(whoiam);
    }
    body
}

/// A claim's `born_at`, for its takedown marker's metadata. `None` for a marker or junk.
fn marker_metadata(claim: &[u8]) -> Option<Metadata> {
    let record: ClaimRecord = serde_json::from_slice(claim).ok()?;
    Some(Metadata::from([("born_at".to_owned(), record.born_at)]))
}

/// An agent's `born_at`, read from its claim or from its takedown marker's metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Born {
    pub id: AgentId,
    /// `None` for a claim that doesn't parse.
    pub born_at: Option<String>,
    /// The claim is already a takedown marker.
    pub taken_down: bool,
}

fn born_of(id: AgentId, object: &Object) -> Born {
    if object.body.is_empty() {
        let born_at = object.metadata.get("born_at").cloned();
        return Born {
            id,
            born_at,
            taken_down: true,
        };
    }
    let born_at = serde_json::from_slice::<ClaimRecord>(&object.body)
        .ok()
        .map(|r| r.born_at);
    Born {
        id,
        born_at,
        taken_down: false,
    }
}

/// One agent's `born_at`, for the takedown program. `None` when there's no claim or marker.
pub async fn born<S: Store>(store: &S, id: &AgentId) -> Result<Option<Born>, StoreError> {
    let object = store.get(Bucket::Claims, &claim_key(id)).await?;
    Ok(object.map(|o| born_of(id.clone(), &o)))
}

/// Every agent, claims and markers alike, whose `born_at` is at or after `from` and before `to`.
/// Both are RFC 3339 in UTC to the second, as `born_at` is, so they compare as text. The window
/// can't use S3's `LastModified`, which every overwrite changes.
pub async fn born_between<S: Store>(
    store: &S,
    from: &str,
    to: &str,
) -> Result<Vec<Born>, StoreError> {
    let mut found = Vec::new();
    for key in store.list(Bucket::Claims, "claims/").await? {
        let Some(id) = key
            .strip_prefix("claims/")
            .and_then(|k| k.strip_suffix("/claim.json"))
            .and_then(AgentId::parse)
        else {
            continue;
        };
        let Some(object) = store.get(Bucket::Claims, &key).await? else {
            continue;
        };
        let born = born_of(id, &object);
        if born
            .born_at
            .as_deref()
            .is_some_and(|b| from <= b && b < to)
        {
            found.push(born);
        }
    }
    Ok(found)
}

/// This crate's locations for an agent, and the action on each, for the takedown program. The
/// claim becomes a zero-byte marker that keeps `born_at`, so the ID can never be born again.
pub fn takedown(id: &AgentId) -> Vec<Location> {
    vec![
        Location {
            phase: Phase::First,
            bucket: Bucket::Claims,
            key: claim_key(id),
            action: Action::Marker(marker_metadata),
        },
        Location {
            phase: Phase::First,
            bucket: Bucket::Identities,
            key: aboutme_key(id),
            action: Action::Delete,
        },
        Location {
            phase: Phase::Other,
            bucket: Bucket::Identities,
            key: vitals_key(id),
            action: Action::Delete,
        },
        // The inbox, which issue 14 writes. Messages this agent sent to others stay until they
        // expire.
        Location {
            phase: Phase::Other,
            bucket: Bucket::Identities,
            key: format!("messages/{id}/"),
            action: Action::DeletePrefix,
        },
        Location {
            phase: Phase::Last,
            bucket: Bucket::Identities,
            key: format!("identities/{id}/"),
            action: Action::DeletePrefix,
        },
    ]
}
