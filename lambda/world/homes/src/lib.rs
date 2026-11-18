//! Homes, quarters and lanes: `.stories/4-life-in-the-world/spec.md` "Homes and quarters". This
//! crate owns the `places/` prefix, the rules and format of `identities/<id>/home.json`, whose key
//! the identity crate spells because it owns that folder ([`sfab_identity::home_key`]), the daily
//! job's `world/quarters/` files, and the map, compiled in from `world/quarters.yaml`.
//!
//! - `identities/<id>/home.json` is the truth: the address or `null`, `moved_at`, the last move's
//!   proof hash, so a retry with the same `Proof` returns the move, and the description.
//! - `places/<quarter>/<lane>/<number>.json` names the resident, `{"id": ...}`. Moving in takes it
//!   with `If-None-Match: *`, so nobody can squat on an address, and a pointer counts only if
//!   `home.json` agrees.
//! - The daily job, in [`daily`], deletes pointers that disagree and writes each quarter's counts
//!   and up to [`MAX_FREE`] free addresses.
//!
//! Costs at the daily limit of 100,000 calls, each route alone, S3 only: a move reads the claim,
//! `home.json`, `free.json` and `identity.json` (and `contacts.json` when `friends` is public),
//! and writes the pointer, `home.json` and `aboutme.json` when `address` is public, 4 to 5 GETs
//! and 3 PUTs, about $1.70 a day; a refused one 3 GETs and a PUT, about $0.62; moving out 3 GETs
//! and 2 PUTs, about $1.12; a description 2 GETs and a PUT, about $0.58; a lane one LIST and up to
//! 20 GETs, about $1.30; a home 5 GETs, about $0.20; the quarters list up to 20 GETs, about $0.80.

mod daily;

use std::sync::{Arc, LazyLock};

use saphyr::Yaml;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::config;
use sfab_core::daily::{DAY, next_daily_run, parse_rfc3339, rfc3339};
use sfab_core::files::{read_file, to_json};
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::AgentId;
use sfab_core::paging::{OFFSET_PAGE, offset_page};
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::hex;
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::takedown::{Action, Location, Phase};
use sfab_core::world;
use sfab_identity::{Identity, home_key};

pub use daily::{DailyReport, FRESH_SECONDS, daily};

/// Where the lane pointers live, `places/<quarter>/<lane>/<number>.json`. IAM in
/// `cdk/stacks.py` names it too, tied here by a CDK test.
pub const PREFIX: &str = "places/";

const QUARTERS_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../world/quarters.yaml"
));
const QUARTERS_FILE: &str = "quarters.yaml";

/// One quarter, with the lane names the world opens in this order as it fills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarter {
    pub name: String,
    pub lanes: Vec<String>,
}

static QUARTERS: LazyLock<Vec<Quarter>> = LazyLock::new(|| {
    parse_quarters(QUARTERS_YAML).expect("world/quarters.yaml, checked at synth and by its test")
});

/// The compiled-in quarters, in the file's order.
pub fn quarters() -> &'static [Quarter] {
    &QUARTERS
}

/// `quarters.yaml`: a `quarters` list of `{name, lanes}`, each name a [`config::is_name`],
/// each quarter listed once with a non-empty list of lanes, each listed once in it.
pub fn parse_quarters(text: &str) -> Result<Vec<Quarter>, String> {
    let doc = config::document(text, QUARTERS_FILE)?;
    let mut found: Vec<Quarter> = Vec::new();
    for item in config::list(&doc, QUARTERS_FILE, "quarters")? {
        let name = item.as_mapping_get("name").map(Yaml::as_str);
        let name = match name {
            Some(Some(name)) if config::is_name(name) => name.to_owned(),
            Some(Some(name)) => {
                return Err(format!("{QUARTERS_FILE}: quarter {name:?} isn't valid"));
            }
            _ => return Err(format!("{QUARTERS_FILE}: each quarter needs a `name`")),
        };
        if found.iter().any(|q| q.name == name) {
            return Err(format!("{QUARTERS_FILE}: quarter {name} is listed twice"));
        }
        let lanes = match item.as_mapping_get("lanes") {
            Some(Yaml::Sequence(lanes)) if !lanes.is_empty() => lanes,
            _ => {
                return Err(format!(
                    "{QUARTERS_FILE}: quarter {name} needs a non-empty `lanes` list"
                ));
            }
        };
        let lanes = config::strings(lanes, QUARTERS_FILE, "lane", config::is_name)?;
        found.push(Quarter { name, lanes });
    }
    Ok(found)
}

/// Homes on each lane: a lane is full at this many.
pub const HOMES_PER_LANE: u32 = 20;
/// The most free addresses the daily job offers a quarter for the day.
pub const MAX_FREE: usize = 100;
/// Days between moves: moving in or moving again sooner gets `409` `too-soon`.
pub const MOVE_DAYS: i64 = 30;
/// The longest description, in bytes of UTF-8. `sfab.yaml`'s `HomeDescription` says the same,
/// tied by a test.
pub const MAX_DESCRIPTION_BYTES: usize = 2048;
/// Where the daily job writes each quarter's counts, `world/quarters/<quarter>.json`, and its free
/// addresses, `world/quarters/<quarter>/free.json`.
pub const QUARTERS_PREFIX: &str = "world/quarters/";

/// An address: a quarter, a lane and a home number from 1 to [`HOMES_PER_LANE`], written
/// `harbour/lantern-lane/7`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Address {
    pub quarter: String,
    pub lane: String,
    pub home: u32,
}

impl Address {
    /// `quarter/lane/home` on `map`, or `None` for any other text, or an address the map doesn't
    /// have.
    pub fn parse(text: &str, map: &[Quarter]) -> Option<Self> {
        let mut parts = text.split('/');
        let (q, l, h) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        Self::on(map, q, l, h)
    }

    /// The address named by a route's three parameters, if it's on `map`.
    fn on(map: &[Quarter], quarter: &str, lane: &str, home: &str) -> Option<Self> {
        let q = map.iter().find(|q| q.name == quarter)?;
        if !q.lanes.iter().any(|l| l == lane) {
            return None;
        }
        Some(Self {
            quarter: quarter.to_owned(),
            lane: lane.to_owned(),
            home: parse_home(home)?,
        })
    }

    pub fn text(&self) -> String {
        format!("{}/{}/{}", self.quarter, self.lane, self.home)
    }

    fn pointer_key(&self) -> String {
        format!("{PREFIX}{}/{}/{}.json", self.quarter, self.lane, self.home)
    }
}

/// A home number as a route or key writes it: 1 to [`HOMES_PER_LANE`], with no sign or leading
/// zero.
fn parse_home(text: &str) -> Option<u32> {
    let canonical = !text.is_empty()
        && text.len() <= 3
        && text.bytes().all(|b| b.is_ascii_digit())
        && !text.starts_with('0');
    let home: u32 = canonical.then(|| text.parse().ok()).flatten()?;
    (1..=HOMES_PER_LANE).contains(&home).then_some(home)
}

fn lane_prefix(quarter: &str, lane: &str) -> String {
    format!("{PREFIX}{quarter}/{lane}/")
}

fn quarter_key(quarter: &str) -> String {
    format!("{QUARTERS_PREFIX}{quarter}.json")
}

fn free_key(quarter: &str) -> String {
    format!("{QUARTERS_PREFIX}{quarter}/free.json")
}

/// `places/<quarter>/<lane>/<number>.json`.
#[derive(Debug, Serialize, Deserialize)]
struct Pointer {
    id: String,
}

/// `identities/<id>/home.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct HomeFile {
    /// `quarter/lane/home`, or `None` once moved out.
    #[serde(default)]
    address: Option<String>,
    moved_at: String,
    /// The last move's proof hash in lowercase hex, so a retry returns the move.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    proof: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl HomeFile {
    /// What `GET /identity/self/home` shows.
    fn shown(&self) -> Value {
        json!({
            "address": self.address,
            "moved_at": self.moved_at,
            "description": self.description,
        })
    }
}

fn no_home_yet() -> Value {
    json!({"address": null, "moved_at": null, "description": null})
}

/// `world/quarters/<quarter>/free.json`: the free addresses on offer today.
#[derive(Debug, Serialize, Deserialize)]
struct Free {
    free: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveRequest {
    quarter: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DescriptionRequest {
    description: String,
}

fn no_such_place() -> Problem {
    Problem::not_found("There's no such quarter, lane or home on the map")
}

fn try_later(now: i64, title: &str) -> Problem {
    Problem::conflict("try-later", title.to_owned())
        .with("retry_after", rfc3339(next_daily_run(now)))
}

/// `home.json` as stored, with its ETag, or `None`.
async fn read_home<S: Store>(
    store: &S,
    id: &AgentId,
) -> Result<Option<(HomeFile, String)>, Problem> {
    read_file(store, &home_key(id), "reading a home").await
}

/// Deletes a pointer that no longer counts. A failure is only logged: the daily job deletes
/// pointers `home.json` disagrees with, so it lasts at most a day.
async fn drop_pointer<S: Store>(store: &S, address: &Address, why: &str) {
    if let Err(e) = store
        .delete(Bucket::Identities, &address.pointer_key())
        .await
    {
        tracing::warn!(error = ?e, "deleting {why} failed; the daily job removes it");
    }
}

/// A random index below its argument, for the address a move tries.
pub type Pick = fn(usize) -> usize;

pub struct Homes<S> {
    store: Arc<S>,
    identity: Arc<Identity<S>>,
    required_bits: u32,
    map: &'static [Quarter],
    pick: Pick,
}

impl<S: Store> Homes<S> {
    /// On the compiled-in map, picking at random.
    pub fn new(store: Arc<S>, identity: Arc<Identity<S>>, required_bits: u32) -> Self {
        Self {
            store,
            identity,
            required_bits,
            map: quarters(),
            pick: sfab_core::random::below,
        }
    }

    /// On another map, and with another pick, for tests.
    pub fn with(mut self, map: &'static [Quarter], pick: Pick) -> Self {
        self.map = map;
        self.pick = pick;
        self
    }

    fn quarter(&self, name: &str) -> Option<&'static Quarter> {
        self.map.iter().find(|q| q.name == name)
    }

    /// `PUT /identity/self/home`, moving in or moving, in `docs/adr/0003-proof-on-every-write.md`'s
    /// order: the proof, the caller, the body, then a retry with the same proof, which rewrites
    /// `aboutme.json` and returns the move with `200`, then the 30-day limit, then the address:
    /// one of today's free ones at random, or without `free.json` a random home on the quarter's
    /// first lane, taken with one conditional pointer write. Then `home.json`, with `If-Match`, or
    /// `If-None-Match: *` for a first move-in, the old pointer's delete, and the public copy.
    /// `201` for moving in, `200` for a move.
    pub async fn move_home(&self, call: &Call<'_>, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let id = self.identity.caller(call.authorization, now).await?;
        let request: MoveRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        let quarter = self
            .quarter(&request.quarter)
            .ok_or_else(|| Problem::bad_request("quarter must be one of GET /v1/world/quarters"))?;
        let current = read_home(&*self.store, &id).await?;
        let proof = hex(&admitted.hash);
        if let Some((home, _)) = &current {
            if home.proof.as_deref() == Some(proof.as_str()) {
                self.identity
                    .refresh_address(&id, home.address.as_deref())
                    .await?;
                return Ok(Reply::ok(home.shown()));
            }
            let moved = parse_rfc3339(&home.moved_at).ok_or_else(|| {
                tracing::error!("a home's moved_at doesn't parse");
                Problem::internal()
            })?;
            let allowed = moved + MOVE_DAYS * DAY;
            if now < allowed {
                return Err(Problem::conflict(
                    "too-soon",
                    "One move every 30 days, counted from the last move in, move or move out",
                )
                .with("retry_after", rfc3339(allowed)));
            }
        }
        let address = self.choose(quarter).await?;
        let pointer = to_json(&Pointer {
            id: id.as_str().to_owned(),
        });
        let taken = self
            .store
            .put(
                Bucket::Identities,
                &address.pointer_key(),
                pointer,
                Metadata::new(),
                Condition::IfAbsent,
            )
            .await;
        match taken {
            Ok(()) => {}
            Err(StoreError::ConditionFailed) => {
                return Err(try_later(
                    now,
                    "That home was just taken: retry now for another, or after retry_after",
                ));
            }
            Err(e) => return Err(Problem::storage("taking a home", &e)),
        }
        let (old, etag, description) = match current {
            Some((home, etag)) => (home.address, Some(etag), home.description),
            None => (None, None, None),
        };
        let home = HomeFile {
            address: Some(address.text()),
            moved_at: rfc3339(now),
            proof: Some(proof),
            description,
        };
        let condition = match etag {
            Some(etag) => Condition::IfMatch(etag),
            None => Condition::IfAbsent,
        };
        let written = self
            .store
            .put(
                Bucket::Identities,
                &home_key(&id),
                to_json(&home),
                Metadata::new(),
                condition,
            )
            .await;
        match written {
            Ok(()) => {}
            Err(StoreError::ConditionFailed | StoreError::Missing) => {
                drop_pointer(&*self.store, &address, "a losing move's pointer").await;
                return Err(try_later(
                    now,
                    "Another move of yours won; read GET /v1/identity/self/home",
                ));
            }
            Err(e) => {
                drop_pointer(&*self.store, &address, "a failed move's pointer").await;
                return Err(Problem::storage("writing a home", &e));
            }
        }
        if let Some(old) = old.as_deref().and_then(|t| Address::parse(t, self.map)) {
            drop_pointer(&*self.store, &old, "an old pointer").await;
        }
        self.identity
            .refresh_address(&id, home.address.as_deref())
            .await?;
        Ok(match old {
            Some(_) => Reply::ok(home.shown()),
            None => Reply::created(home.shown()),
        })
    }

    /// The address a move tries: one of `free.json`'s at random, `quarter-full` when it has none,
    /// or without it a random home on the quarter's first lane.
    async fn choose(&self, quarter: &Quarter) -> Result<Address, Problem> {
        let object = self
            .store
            .get(Bucket::Identities, &free_key(&quarter.name))
            .await
            .map_err(|e| Problem::storage("reading free homes", &e))?;
        let Some(object) = object else {
            let home = (self.pick)(HOMES_PER_LANE as usize) as u32 + 1;
            return Ok(Address {
                quarter: quarter.name.clone(),
                lane: quarter.lanes[0].clone(),
                home,
            });
        };
        let free: Free = serde_json::from_slice(&object.body).map_err(|_| {
            tracing::error!("a quarter's free homes don't parse");
            Problem::internal()
        })?;
        if free.free.is_empty() {
            return Err(Problem::conflict(
                "quarter-full",
                "This quarter has no homes left: pick another",
            ));
        }
        let text = &free.free[(self.pick)(free.free.len())];
        Address::parse(text, self.map).ok_or_else(|| {
            tracing::error!("a quarter's free homes name an address off the map");
            Problem::internal()
        })
    }

    /// `DELETE /identity/self/home`: moving out, always allowed. `home.json` stays, with its
    /// address `null` and `moved_at` now, so moving out and back in can't escape the limit. The
    /// public copy loses the address first, then the home, then the pointer goes. `200` whether or
    /// not there was a home, so a retry is safe.
    pub async fn move_out(&self, authorization: Option<&str>, now: i64) -> Result<Reply, Problem> {
        let id = self.identity.caller(authorization, now).await?;
        let Some((mut home, etag)) = read_home(&*self.store, &id).await? else {
            return Ok(Reply::ok(no_home_yet()));
        };
        let Some(old) = home.address.take() else {
            return Ok(Reply::ok(home.shown()));
        };
        self.identity.refresh_address(&id, None).await?;
        home.moved_at = rfc3339(now);
        home.proof = None;
        self.save(&id, &home, etag, "move out").await?;
        if let Some(old) = Address::parse(&old, self.map) {
            drop_pointer(&*self.store, &old, "a pointer moved out of").await;
        }
        Ok(Reply::ok(home.shown()))
    }

    /// `GET /identity/self/home`: the address, `moved_at` and description, all `null` for an
    /// agent that never moved in.
    pub async fn own_home(&self, authorization: Option<&str>, now: i64) -> Result<Reply, Problem> {
        let id = self.identity.caller(authorization, now).await?;
        Ok(Reply::ok(match read_home(&*self.store, &id).await? {
            Some((home, _)) => home.shown(),
            None => no_home_yet(),
        }))
    }

    /// `PUT /identity/self/home/description`: replaces it whole, up to
    /// [`MAX_DESCRIPTION_BYTES`]; an empty one removes it. Without a home, `403` `no-home`.
    pub async fn describe(&self, call: &Call<'_>, now: i64) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let id = self.identity.caller(call.authorization, now).await?;
        let request: DescriptionRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        if request.description.len() > MAX_DESCRIPTION_BYTES {
            return Err(Problem::bad_request(
                "description must be at most 2,048 bytes of UTF-8",
            ));
        }
        let current = read_home(&*self.store, &id).await?;
        let Some((mut home, etag)) = current.filter(|(h, _)| h.address.is_some()) else {
            return Err(Problem::forbidden(
                "no-home",
                "Move in first: PUT /v1/identity/self/home",
            ));
        };
        let description = Some(request.description).filter(|d| !d.is_empty());
        if home.description != description {
            home.description = description;
            self.save(&id, &home, etag, "home description").await?;
        }
        Ok(Reply::ok(home.shown()))
    }

    async fn save(
        &self,
        id: &AgentId,
        home: &HomeFile,
        etag: String,
        route: &str,
    ) -> Result<(), Problem> {
        let write = self
            .store
            .put(
                Bucket::Identities,
                &home_key(id),
                to_json(home),
                Metadata::new(),
                Condition::IfMatch(etag),
            )
            .await;
        match write {
            Ok(()) => Ok(()),
            Err(StoreError::ConditionFailed | StoreError::Missing) => {
                Err(Problem::lost_race(route))
            }
            Err(e) => Err(Problem::storage("writing a home", &e)),
        }
    }

    /// `GET /world/quarters`: every quarter on the map, with its residents and free homes as the
    /// daily job last counted them, `null` for a quarter it hasn't counted yet, and
    /// `not-built-yet` before its first run.
    pub async fn quarters(&self, page: Option<&str>) -> Result<Reply, Problem> {
        let page = offset_page(self.map.len(), page, OFFSET_PAGE, "/v1/world/quarters")?;
        let mut items = Vec::new();
        let mut counted = false;
        for q in &self.map[page.range] {
            let found = read_counts(&*self.store, &q.name).await?;
            counted |= found.is_some();
            let counts = found.unwrap_or(Value::Null);
            items.push(json!({
                "quarter": q.name,
                "residents": counts["residents"],
                "free": counts["free"],
            }));
        }
        if !counted {
            return Err(world::not_built_yet());
        }
        Ok(Reply::ok(json!({"items": items, "next": page.next})))
    }

    /// `GET /world/quarters/{quarter}`: its residents and free homes, up to a day old.
    pub async fn quarter_counts(&self, quarter: &str) -> Result<Reply, Problem> {
        let q = self.quarter(quarter).ok_or_else(no_such_place)?;
        let counts = world::read_built(&*self.store, &quarter_key(&q.name)).await?;
        Ok(Reply::ok(json!({
            "quarter": q.name,
            "residents": counts["residents"],
            "free": counts["free"],
            "as_of": counts["as_of"],
        })))
    }

    /// `GET /world/quarters/{quarter}/lanes`: the lanes with residents, in the map's order, as the
    /// daily job last saw them.
    pub async fn lanes(&self, quarter: &str, page: Option<&str>) -> Result<Reply, Problem> {
        let q = self.quarter(quarter).ok_or_else(no_such_place)?;
        let counts = world::read_built(&*self.store, &quarter_key(&q.name)).await?;
        let lanes = counts["lanes"].as_array().cloned().unwrap_or_default();
        let path = format!("/v1/world/quarters/{}/lanes", q.name);
        let page = offset_page(lanes.len(), page, OFFSET_PAGE, &path)?;
        Ok(Reply::ok(
            json!({"items": lanes[page.range].to_vec(), "next": page.next}),
        ))
    }

    /// `GET /world/quarters/{quarter}/lanes/{lane}`: who lives there, by home number, from one
    /// live listing and a read of each pointer on the page, without checking `home.json`. A lane
    /// with nobody on it doesn't exist yet: `404`.
    pub async fn lane(
        &self,
        quarter: &str,
        lane: &str,
        page: Option<&str>,
    ) -> Result<Reply, Problem> {
        let q = self.quarter(quarter).ok_or_else(no_such_place)?;
        if !q.lanes.iter().any(|l| l == lane) {
            return Err(no_such_place());
        }
        let prefix = lane_prefix(quarter, lane);
        let listed = self
            .store
            .list(Bucket::Identities, &prefix)
            .await
            .map_err(|e| Problem::storage("listing a lane", &e))?;
        let mut homes: Vec<u32> = listed
            .iter()
            .filter_map(|k| k.strip_prefix(&prefix)?.strip_suffix(".json"))
            .filter_map(parse_home)
            .collect();
        homes.sort_unstable();
        if homes.is_empty() {
            return Err(Problem::not_found("Nobody lives on this lane yet"));
        }
        let path = format!("/v1/world/quarters/{quarter}/lanes/{lane}");
        let page = offset_page(homes.len(), page, OFFSET_PAGE, &path)?;
        let mut items = Vec::new();
        for home in &homes[page.range] {
            let address = Address {
                quarter: quarter.to_owned(),
                lane: lane.to_owned(),
                home: *home,
            };
            if let Some(id) = self.resident(&address).await? {
                items.push(json!({"home": home, "id": id.as_str()}));
            }
        }
        Ok(Reply::ok(json!({"items": items, "next": page.next})))
    }

    /// The resident a pointer names, or `None` when it's gone or names no agent.
    async fn resident(&self, address: &Address) -> Result<Option<AgentId>, Problem> {
        resident(&*self.store, address)
            .await
            .map_err(|e| Problem::storage("reading a pointer", &e))
    }

    /// `GET /world/quarters/{quarter}/lanes/{lane}/homes/{home}`: the resident's ID, description
    /// and standing, and its `aboutme` if published. `404` unless the pointer and `home.json`
    /// agree.
    pub async fn home(&self, quarter: &str, lane: &str, home: &str) -> Result<Reply, Problem> {
        let address = Address::on(self.map, quarter, lane, home).ok_or_else(no_such_place)?;
        let nobody = || Problem::not_found("Nobody lives at this address");
        let id = self.resident(&address).await?.ok_or_else(nobody)?;
        let (file, _) = read_home(&*self.store, &id).await?.ok_or_else(nobody)?;
        if file.address.as_deref() != Some(address.text().as_str()) {
            return Err(nobody());
        }
        let (standing, as_of) = sfab_vouches::has_standing(&*self.store, &id).await?;
        let mut body = json!({
            "address": address.text(),
            "id": id.as_str(),
            "description": file.description,
            "standing": standing,
            "standing_as_of": as_of,
        });
        match self.identity.aboutme(id.as_str()).await {
            Ok(about) => body["aboutme"] = about.body,
            Err(p) if p.status == 404 => {}
            Err(p) => return Err(p),
        }
        Ok(Reply::ok(body))
    }
}

/// The resident a pointer names, or `None` when it's gone, or doesn't parse, which is logged.
async fn resident<S: Store>(store: &S, address: &Address) -> Result<Option<AgentId>, StoreError> {
    let Some(object) = store
        .get(Bucket::Identities, &address.pointer_key())
        .await?
    else {
        return Ok(None);
    };
    let pointer: Option<Pointer> = serde_json::from_slice(&object.body).ok();
    let id = pointer.and_then(|p| AgentId::parse(&p.id));
    if id.is_none() {
        tracing::error!("a pointer in storage doesn't parse");
    }
    Ok(id)
}

/// `world/quarters/<quarter>.json`, or `None` before the daily job counted it.
async fn read_counts<S: Store>(store: &S, quarter: &str) -> Result<Option<Value>, Problem> {
    world::read_if_built(store, &quarter_key(quarter)).await
}

/// The quarter an agent lives in, from one read of its `home.json`, or `None` when it has no
/// home, for another institution's residency rule, such as the Square's.
pub async fn quarter_of<S: Store>(store: &S, id: &AgentId) -> Result<Option<String>, Problem> {
    let home = read_home(store, id).await?;
    let address = home.and_then(|(h, _)| h.address);
    Ok(address.and_then(|a| a.split('/').next().map(str::to_owned)))
}

/// The pointer a `home.json` names, for the takedown program to delete, freeing the address.
fn pointers_of(body: &[u8]) -> Vec<String> {
    let home: Option<HomeFile> = serde_json::from_slice(body).ok();
    home.and_then(|h| h.address)
        .and_then(|a| Address::parse(&a, quarters()))
        .map(|a| vec![a.pointer_key()])
        .unwrap_or_default()
}

/// This crate's locations for an agent, for the takedown program: `home.json`, read for its
/// `places/...` pointer, which is deleted to free the address. The home itself goes with
/// `identities/<id>/`.
pub fn takedown(id: &AgentId) -> Vec<Location> {
    vec![Location {
        phase: Phase::Pointers,
        bucket: Bucket::Identities,
        key: home_key(id),
        action: Action::Pointers(pointers_of),
    }]
}
