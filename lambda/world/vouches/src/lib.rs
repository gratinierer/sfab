//! Vouches and standing: `.stories/4-life-in-the-world/spec.md` "Vouches: trust" and
//! `docs/adr/0002-life-in-the-world.md`. This crate owns the `vouches/` prefix, the daily job's
//! `world/vouches/` and `world/standing/` files and `world/standing.json`, and the founders, from
//! whom all standing flows: compiled in from `world/founders.yaml`, or until ticket 11 commits
//! it, from `world/founders.fixture.yaml` (see `build.rs`). It asks the identity crate who is
//! calling, whether the recipient blocks the giver, and whether an agent publishes `vouches`
//! (`docs/adr/0004-rules-live-in-crates.md`).
//!
//! - `vouches/<id>.json`: the vouches `<id>` gave, up to [`MAX_GIVEN`], one per recipient, each
//!   storing its proof's hash, so a retry with the same `Proof` returns it. Written with
//!   `If-Match`, or `If-None-Match: *` for the first, so the limit needs no counter.
//! - The daily job works out standing in [`daily`], and the routes read what it wrote.
//!
//! Costs at the daily limit of 100,000 calls, each route alone: giving reads the claim,
//! `contacts.json` and the giver's file and writes it, 3 GETs and a PUT, about $0.62 a day;
//! withdrawing about $0.54; reading the given list, standing or `self/vouched-by` 2 to 3 GETs,
//! about $0.12; `{id}/vouched-by` 3 GETs, about $0.12. All under `docs/api-design.md`'s $2.

mod standing;

use std::sync::{Arc, LazyLock};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::config;
use sfab_core::daily::rfc3339;
use sfab_core::files::to_json;
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::AgentId;
use sfab_core::paging::{OFFSET_PAGE, offset_page};
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::hex;
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::takedown::{Action, Location, Phase};
use sfab_identity::{Field, Identity, Relation};

pub use standing::{
    AGE_DAYS, DailyReport, MIN_VOUCHES, STANDING_PREFIX, VOUCHED_BY_PREFIX, daily, with_standing,
};

/// Where each agent's given vouches live, `vouches/<id>.json`. IAM in `cdk/stacks.py` and
/// `cdk/world_job.py` names it too, tied here by a CDK test.
pub const PREFIX: &str = "vouches/";

/// The fewest founders a world may have: with fewer, nobody else could ever gain standing.
/// `cdk/world_config.py` checks the same at synth.
pub const MIN_FOUNDERS: usize = 3;

/// The most vouches an agent may have given at a time; an 11th gets `409` `list-full`.
pub const MAX_GIVEN: usize = 10;

/// The longest reason, in characters of UTF-8. `sfab.yaml`'s `VouchReason` says the same,
/// tied by a test.
pub const MAX_REASON_CHARS: usize = 200;

const FOUNDERS_YAML: &str = include_str!(env!("SFAB_FOUNDERS_FILE"));
const FOUNDERS_FILE: &str = "founders.yaml";
const GIVEN_PATH: &str = "/v1/identity/self/vouches";
const SELF_VOUCHED_BY_PATH: &str = "/v1/identity/self/vouched-by";

static FOUNDERS: LazyLock<Vec<AgentId>> = LazyLock::new(|| {
    parse_founders(FOUNDERS_YAML).expect("the founders file, checked at synth and by its test")
});

/// The compiled-in founders, in the file's order.
pub fn founders() -> &'static [AgentId] {
    &FOUNDERS
}

/// `founders.yaml`: a `founders` list of at least [`MIN_FOUNDERS`] agent IDs, each listed once.
pub fn parse_founders(text: &str) -> Result<Vec<AgentId>, String> {
    let doc = config::document(text, FOUNDERS_FILE)?;
    let items = config::list(&doc, FOUNDERS_FILE, "founders")?;
    let ids = config::strings(items, FOUNDERS_FILE, "founder", |t| {
        AgentId::parse(t).is_some()
    })?;
    if ids.len() < MIN_FOUNDERS {
        return Err(format!(
            "{FOUNDERS_FILE}: needs at least {MIN_FOUNDERS} founders, found {}",
            ids.len()
        ));
    }
    Ok(ids
        .iter()
        .map(|t| AgentId::parse(t).expect("checked above"))
        .collect())
}

fn given_key(id: &AgentId) -> String {
    format!("{PREFIX}{id}.json")
}

/// One vouch an agent gave.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Vouch {
    /// The recipient.
    id: String,
    reason: String,
    given_at: String,
    /// The proof's hash in lowercase hex, so a retry is told apart from a new reason.
    proof: String,
}

impl Vouch {
    /// What the routes show: everything but the proof.
    fn shown(&self) -> Value {
        json!({"id": self.id, "reason": self.reason, "given_at": self.given_at})
    }
}

/// `vouches/<id>.json`, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Given {
    #[serde(default)]
    vouches: Vec<Vouch>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VouchRequest {
    reason: String,
}

/// `items` paged 20 at a time along `path`, as story 2's "Lists" says.
fn page_of(items: &[Value], token: Option<&str>, path: &str) -> Result<Reply, Problem> {
    let page = offset_page(items.len(), token, OFFSET_PAGE, path)?;
    Ok(Reply::ok(
        json!({"items": items[page.range].to_vec(), "next": page.next}),
    ))
}

/// A given list as stored, with its ETag, read by the API or the daily job.
async fn read_given<S: Store>(
    store: &S,
    id: &AgentId,
) -> Result<(Given, Option<String>), StoreError> {
    let Some(object) = store.get(Bucket::Identities, &given_key(id)).await? else {
        return Ok((Given::default(), None));
    };
    let given = serde_json::from_slice(&object.body)
        .map_err(|_| StoreError::Other(format!("{} doesn't parse", given_key(id))))?;
    Ok((given, Some(object.etag)))
}

pub struct Vouches<S> {
    store: Arc<S>,
    identity: Arc<Identity<S>>,
    required_bits: u32,
}

impl<S: Store> Vouches<S> {
    pub fn new(store: Arc<S>, identity: Arc<Identity<S>>, required_bits: u32) -> Self {
        Self {
            store,
            identity,
            required_bits,
        }
    }

    /// `PUT /identity/self/vouches/{id}`, in `docs/adr/0003-proof-on-every-write.md`'s order:
    /// the proof, the caller, the recipient's one read of `contacts.json` for a block, a retry
    /// with the same proof, which returns the vouch with `200`, then the limit of
    /// [`MAX_GIVEN`], then a conditional write. `201` for a new vouch, `200` for a new reason.
    pub async fn give(&self, call: &Call<'_>, to: &str, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let giver = self.identity.caller(call.authorization, now).await?;
        let to = AgentId::parse(to).ok_or_else(Problem::bad_id)?;
        if to == giver {
            return Err(Problem::bad_request("An agent can't vouch for itself"));
        }
        let request: VouchRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        let chars = request.reason.chars().count();
        if chars == 0 || chars > MAX_REASON_CHARS {
            return Err(Problem::bad_request("reason must be 1 to 200 characters"));
        }
        // An unknown recipient is `None`: the vouch is stored and never counts, so IDs can't
        // be probed.
        if self.identity.relation(&to, &giver).await? == Some(Relation::Blocked) {
            return Err(Problem::forbidden(
                "blocked",
                "The recipient blocks you, so no vouch was stored",
            ));
        }
        let (mut given, etag) = read_given(&*self.store, &giver)
            .await
            .map_err(|e| Problem::storage("reading given vouches", &e))?;
        let proof = hex(&admitted.hash);
        let at = given.vouches.iter().position(|v| v.id == to.as_str());
        if let Some(at) = at
            && given.vouches[at].proof == proof
        {
            return Ok(Reply::ok(given.vouches[at].shown()));
        }
        if at.is_none() && given.vouches.len() >= MAX_GIVEN {
            return Err(Problem::conflict(
                "list-full",
                format!("You already vouch for {MAX_GIVEN} agents; withdraw one first"),
            ));
        }
        let vouch = Vouch {
            id: to.as_str().to_owned(),
            reason: request.reason,
            given_at: rfc3339(now),
            proof,
        };
        let shown = vouch.shown();
        match at {
            Some(at) => given.vouches[at] = vouch,
            None => given.vouches.push(vouch),
        }
        self.save(&giver, &given, etag).await?;
        Ok(match at {
            Some(_) => Reply::ok(shown),
            None => Reply::created(shown),
        })
    }

    /// `DELETE /identity/self/vouches/{id}`: `200` whether or not the vouch was there, so a
    /// retry is safe. Its place is free at once.
    pub async fn withdraw(
        &self,
        authorization: Option<&str>,
        to: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let giver = self.identity.caller(authorization, now).await?;
        let to = AgentId::parse(to).ok_or_else(Problem::bad_id)?;
        let (mut given, etag) = read_given(&*self.store, &giver)
            .await
            .map_err(|e| Problem::storage("reading given vouches", &e))?;
        let before = given.vouches.len();
        given.vouches.retain(|v| v.id != to.as_str());
        if given.vouches.len() < before {
            self.save(&giver, &given, etag).await?;
        }
        Ok(Reply::ok(json!({"id": to.as_str()})))
    }

    /// `GET /identity/self/vouches`: every vouch the caller gave, waiting ones included, oldest
    /// first, 20 a page.
    pub async fn given(
        &self,
        authorization: Option<&str>,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let giver = self.identity.caller(authorization, now).await?;
        let (given, _) = read_given(&*self.store, &giver)
            .await
            .map_err(|e| Problem::storage("reading given vouches", &e))?;
        let items: Vec<Value> = given.vouches.iter().map(Vouch::shown).collect();
        page_of(&items, page, GIVEN_PATH)
    }

    /// `GET /identity/self/vouched-by`: the vouches the caller received that count, as the
    /// daily job last worked them out, 20 a page.
    pub async fn received(
        &self,
        authorization: Option<&str>,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let id = self.identity.caller(authorization, now).await?;
        let items = standing::counted(&*self.store, &id).await?;
        page_of(&items, page, SELF_VOUCHED_BY_PATH)
    }

    /// `GET /identity/{id}/vouched-by`: the same, if the agent publishes `vouches`, and
    /// otherwise an empty page, as for an unknown or taken-down ID. The daily job's file is
    /// read first, so before its first run every ID alike gets `not-built-yet` and none is
    /// confirmed to exist.
    pub async fn vouched_by(&self, id: &str, page: Option<&str>) -> Result<Reply, Problem> {
        let id = AgentId::parse(id).ok_or_else(Problem::bad_id)?;
        let path = format!("/v1/identity/{id}/vouched-by");
        let mut items = standing::counted(&*self.store, &id).await?;
        if !self.identity.publishes(&id, Field::Vouches).await? {
            items.clear();
        }
        page_of(&items, page, &path)
    }

    /// `GET /identity/{id}/standing`: whether the agent has standing, `as_of` the daily job's
    /// last run. An unknown or taken-down ID, and every ID before the first run, gets `false`,
    /// the last with `as_of: null`, so it can't be used to probe which IDs exist.
    pub async fn standing(&self, id: &str) -> Result<Reply, Problem> {
        let id = AgentId::parse(id).ok_or_else(Problem::bad_id)?;
        let (standing, as_of) = standing::read(&*self.store, &id).await?;
        Ok(Reply::ok(
            json!({"id": id.as_str(), "standing": standing, "as_of": as_of}),
        ))
    }

    /// Saves the given list only if it's unchanged since it was read.
    async fn save(
        &self,
        giver: &AgentId,
        given: &Given,
        etag: Option<String>,
    ) -> Result<(), Problem> {
        let condition = match etag {
            Some(etag) => Condition::IfMatch(etag),
            None => Condition::IfAbsent,
        };
        let write = self
            .store
            .put(
                Bucket::Identities,
                &given_key(giver),
                to_json(given),
                Metadata::new(),
                condition,
            )
            .await;
        match write {
            Ok(()) => Ok(()),
            Err(StoreError::ConditionFailed | StoreError::Missing) => {
                Err(Problem::lost_race("vouches"))
            }
            Err(e) => Err(Problem::storage("writing given vouches", &e)),
        }
    }
}

/// Whether `id` has standing, and `as_of` the daily job's last run, `(false, None)` before the
/// first, for another institution's page about an agent, such as a home.
pub async fn has_standing<S: Store>(
    store: &S,
    id: &AgentId,
) -> Result<(bool, Option<String>), Problem> {
    standing::read(store, id).await
}

/// This crate's locations for an agent, for the takedown program: its given vouches, and the
/// daily job's standing and counted vouches for it, which end public reading at once. Vouches
/// others gave it stay, because the daily job skips markers, and each voucher can withdraw to
/// free its place.
pub fn takedown(id: &AgentId) -> Vec<Location> {
    let delete = |phase, key| Location {
        phase,
        bucket: Bucket::Identities,
        key,
        action: Action::Delete,
    };
    vec![
        delete(Phase::First, standing::standing_key(id)),
        delete(Phase::First, standing::vouched_by_key(id)),
        delete(Phase::Other, given_key(id)),
    ]
}
