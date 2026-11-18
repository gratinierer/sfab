//! Masters and apprenticeships: `.stories/4-life-in-the-world/spec.md` "The Academy"
//! ("Apprenticeships", "What's public"). A master is a resident with standing, with up to
//! [`MAX_APPRENTICES`] current apprentices, listed in its counter `identities/<id>/academy.json`
//! by apprenticeship ID and written with `If-Match`.
//!
//! - An application is at `academy/applications/<master>/<applicant>.json`, one per pair, so a
//!   second replaces the first. A lifecycle rule on [`APPLICATIONS_PREFIX`] expires it after
//!   [`APPLICATION_DAYS`] days, and the list hides older ones itself, because S3 expires late
//!   (`docs/aws-facts.md`).
//! - An apprenticeship is `<master>-<apprentice>-<date>`, by the accepting proof's UTC date, at
//!   `academy/apprenticeships/<apprenticeship>/apprenticeship.json`.
//! - Its work items are at `.../work/items/<item>.json` and each item's comments at
//!   `.../work/comments/<item>/<comment>.json`, so listing the items never returns comments.
//!   An item or comment is `<time>-<author>-<proof>`, its proof's timestamp and 8 characters
//!   of its hash, so a retry with the same proof finds it. An item holds its comment count, a
//!   hint that a first comment adds 1 to with `If-Match`.
//! - The master's certificate is `apprenticeship-<apprenticeship>`, revoked through
//!   `PUT /academy/certificates/{certificate}/revocation`.
//!
//! Costs at the daily limit of 100,000 calls, each route alone, S3 only: applying up to 6 GETs
//! and a PUT, about $0.74 a day; the applications list up to 23 GETs and a LIST, about $1.42;
//! accepting 7 GETs and 2 PUTs, about $1.28, or 3 GETs more with a full counter; adding work 2
//! GETs and a PUT, about $0.58; the work or comments list up to 23 GETs and a LIST, about
//! $1.42; a comment 3 GETs and 2 PUTs, about $1.12, or a GET and a PUT more for each lost
//! `If-Match` on the count; ending 3 GETs and 2 PUTs, about $1.12; awarding 2 GETs and a PUT,
//! about $0.58.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::{DAY, parse_rfc3339, rfc3339, utc_date, utc_day};
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::{AgentId, hash_tail, is_base32};
use sfab_core::paging::OFFSET_PAGE;
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::hex;
use sfab_core::storage::{Bucket, Condition, Store};
use sfab_identity::Relation;

use crate::certificates::{self, CertificateFile};
use crate::{Academy, Counter, MAX_COMMENT_BYTES, condition_for, empty_body, put_file, read_file};

/// Where applications live. The lifecycle rule in `cdk/stacks.py` names it too, tied here by a
/// CDK test.
pub const APPLICATIONS_PREFIX: &str = "academy/applications/";
/// Days an application waits before the lifecycle rule expires it, and the list hides it.
pub const APPLICATION_DAYS: i64 = 30;
/// Current apprentices per master.
pub const MAX_APPRENTICES: usize = 3;
/// The longest application note, in characters.
pub const MAX_NOTE_CHARS: usize = 200;
/// The longest work item, in bytes.
pub const MAX_WORK_BYTES: usize = 8 * 1024;

pub(crate) const APPRENTICESHIPS_PREFIX: &str = "academy/apprenticeships/";
/// Tries at a counter or comment count rewrite on a lost `If-Match`.
const TRIES: usize = 3;
/// Digits of a proof timestamp in an item's or comment's ID.
const TIME_DIGITS: usize = 10;
/// Base32 characters of the proof's hash in an item's or comment's ID.
const PROOF_CHARS: usize = 8;

/// `<master>-<apprentice>-<date>`, by the UTC day it started.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ApprenticeshipId {
    pub(crate) master: AgentId,
    pub(crate) apprentice: AgentId,
    date: String,
}

impl ApprenticeshipId {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        let n = AgentId::LEN;
        if b.len() != n + 1 + n + 1 + 10 || b[n] != b'-' || b[2 * n + 1] != b'-' {
            return None;
        }
        let master = AgentId::parse(&text[..n])?;
        let apprentice = AgentId::parse(&text[n + 1..2 * n + 1])?;
        let date = &text[2 * n + 2..];
        parse_rfc3339(&format!("{date}T00:00:00Z"))?;
        Some(Self {
            master,
            apprentice,
            date: date.to_owned(),
        })
    }

    pub(crate) fn text(&self) -> String {
        format!("{}-{}-{}", self.master, self.apprentice, self.date)
    }

    fn folder(&self) -> String {
        format!("{APPRENTICESHIPS_PREFIX}{}/", self.text())
    }

    pub(crate) fn key(&self) -> String {
        format!("{}apprenticeship.json", self.folder())
    }

    fn items_prefix(&self) -> String {
        format!("{}work/items/", self.folder())
    }

    fn item_key(&self, item: &PostId) -> String {
        format!("{}{}.json", self.items_prefix(), item.text())
    }

    fn comments_prefix(&self, item: &PostId) -> String {
        format!("{}work/comments/{}/", self.folder(), item.text())
    }

    fn is_party(&self, id: &AgentId) -> bool {
        &self.master == id || &self.apprentice == id
    }

    pub(crate) fn certificate(&self) -> String {
        format!("apprenticeship-{}", self.text())
    }
}

/// A work item's or comment's ID: `<time>-<author>-<proof>`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PostId {
    time: String,
    author: AgentId,
    proof: String,
}

impl PostId {
    fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('-');
        let (time, author, proof) = (parts.next()?, parts.next()?, parts.next()?);
        let ok = parts.next().is_none()
            && time.len() == TIME_DIGITS
            && time.bytes().all(|b| b.is_ascii_digit())
            && is_base32(proof, PROOF_CHARS);
        let author = AgentId::parse(author)?;
        ok.then(|| Self {
            time: time.to_owned(),
            author,
            proof: proof.to_owned(),
        })
    }

    fn new(timestamp: i64, author: &AgentId, hash: &[u8; 32]) -> Self {
        Self {
            time: format!("{:0TIME_DIGITS$}", timestamp.max(0)),
            author: author.clone(),
            proof: hash_tail(hash, PROOF_CHARS),
        }
    }

    fn text(&self) -> String {
        format!("{}-{}-{}", self.time, self.author, self.proof)
    }

    fn of_key(prefix: &str, key: &str) -> Option<Self> {
        Self::parse(key.strip_prefix(prefix)?.strip_suffix(".json")?)
    }
}

/// `academy/applications/<master>/<applicant>.json`.
#[derive(Debug, Serialize, Deserialize)]
struct Application {
    note: String,
    applied_at: String,
    proof: String,
}

/// `academy/apprenticeships/<apprenticeship>/apprenticeship.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Record {
    pub(crate) master: String,
    pub(crate) apprentice: String,
    pub(crate) started_at: String,
    proof: String,
    #[serde(default)]
    pub(crate) ended_at: Option<String>,
    #[serde(default)]
    ended_by: Option<String>,
}

impl Record {
    fn shown(&self, id: &ApprenticeshipId) -> Value {
        json!({
            "apprenticeship": id.text(),
            "master": self.master,
            "apprentice": self.apprentice,
            "started_at": self.started_at,
            "ended": self.ended_at.is_some(),
            "ended_at": self.ended_at,
            "ended_by": self.ended_by,
        })
    }
}

/// A work item or a comment as stored. A comment has no count.
#[derive(Debug, Serialize, Deserialize)]
struct Post {
    author: String,
    text: String,
    added_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    comments: Option<u32>,
}

impl Post {
    fn shown(&self, field: &str, id: &PostId) -> Value {
        let mut shown = json!({
            field: id.text(),
            "author": self.author,
            "text": self.text,
            "added_at": self.added_at,
        });
        if let Some(n) = self.comments {
            shown["comments"] = json!(n);
        }
        shown
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteRequest {
    note: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextRequest {
    text: String,
}

fn application_key(master: &AgentId, applicant: &AgentId) -> String {
    format!("{APPLICATIONS_PREFIX}{master}/{applicant}.json")
}

fn bad_id() -> Problem {
    Problem::bad_request("The ID must be 12 lowercase base32 characters")
}

fn bad_apprenticeship() -> Problem {
    Problem::bad_request("An apprenticeship is <master>-<apprentice>-<YYYY-MM-DD>")
}

fn bad_item() -> Problem {
    Problem::bad_request("A work item is <time>-<author>-<proof>")
}

fn bad_page() -> Problem {
    Problem::bad_request("?page= must be a token from a list's next")
}

fn no_such_apprenticeship() -> Problem {
    Problem::not_found("There's no such apprenticeship")
}

fn no_such_application() -> Problem {
    Problem::not_found("There's no such application")
}

fn not_a_party() -> Problem {
    Problem::forbidden("not-a-party", "This apprenticeship belongs to other agents")
}

fn parse_apprenticeship(text: &str) -> Result<ApprenticeshipId, Problem> {
    ApprenticeshipId::parse(text).ok_or_else(bad_apprenticeship)
}

/// `text` of 1 to `max` bytes, from a `{"text"}` body.
fn text_of(body: &[u8], max: usize, limit: &str) -> Result<String, Problem> {
    let request: TextRequest =
        serde_json::from_slice(body).map_err(|_| Problem::bad_request("Malformed request body"))?;
    if request.text.is_empty() || request.text.len() > max {
        return Err(Problem::bad_request(format!(
            "text must be 1 byte to {limit}"
        )));
    }
    Ok(request.text)
}

/// Whether an application at `applied_at` is still shown at `now`, though S3 hasn't expired it.
fn current(applied_at: &str, now: i64) -> bool {
    parse_rfc3339(applied_at).is_some_and(|at| at > now - APPLICATION_DAYS * DAY)
}

/// A page of keys under `prefix` after the `?page=` token, as items, with `next`.
struct Listed<T> {
    found: Vec<(T, String)>,
    more: bool,
}

impl<S: Store> Academy<S> {
    /// Whether `id` has standing, as the daily job last worked it out.
    async fn has_standing(&self, id: &AgentId) -> Result<bool, Problem> {
        Ok(sfab_vouches::has_standing(&*self.store, id).await?.0)
    }

    async fn need_standing(&self, id: &AgentId) -> Result<(), Problem> {
        if self.has_standing(id).await? {
            return Ok(());
        }
        Err(Problem::forbidden(
            "no-standing",
            "Only a resident with standing can be a master",
        ))
    }

    async fn read_record(
        &self,
        id: &ApprenticeshipId,
    ) -> Result<Option<(Record, String)>, Problem> {
        read_file(&*self.store, &id.key(), "reading an apprenticeship").await
    }

    /// The apprenticeship, for one of its two agents: `403` `not-a-party` from the ID alone for
    /// anyone else, so it confirms nothing, then `404` when there's none.
    async fn party_record(
        &self,
        caller: &AgentId,
        id: &ApprenticeshipId,
    ) -> Result<(Record, String), Problem> {
        if !id.is_party(caller) {
            return Err(not_a_party());
        }
        self.read_record(id)
            .await?
            .ok_or_else(no_such_apprenticeship)
    }

    /// `PUT /academy/masters/{id}/applications/self`, in ADR 0003's order: the proof, the
    /// caller, the ID and the note of up to [`MAX_NOTE_CHARS`] characters, then whether the
    /// agent is a master with standing (`403` `not-a-master`), and a block by it (`403`
    /// `blocked`), then a retry with the same proof, which returns the application with `200`.
    /// A second application replaces the first, `200`; a first is `201`.
    pub async fn apply(&self, call: &Call<'_>, master: &str, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let applicant = self.identity.caller(call.authorization, now).await?;
        let master = AgentId::parse(master).ok_or_else(bad_id)?;
        let request: NoteRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        let chars = request.note.chars().count();
        if chars == 0 || chars > MAX_NOTE_CHARS {
            return Err(Problem::bad_request("note must be 1 to 200 characters"));
        }
        if master == applicant {
            return Err(Problem::bad_request("An agent can't apply to itself"));
        }
        if !self.has_standing(&master).await? {
            return Err(Problem::forbidden(
                "not-a-master",
                "Only a resident with standing can take apprentices",
            ));
        }
        if self.identity.relation(&master, &applicant).await? == Some(Relation::Blocked) {
            return Err(Problem::forbidden(
                "blocked",
                "The master blocks you, so no application was stored",
            ));
        }
        let key = application_key(&master, &applicant);
        let found: Option<(Application, String)> =
            read_file(&*self.store, &key, "reading an application").await?;
        let proof = hex(&admitted.hash);
        let shown = |a: &Application| json!({"master": master.as_str(), "applicant": applicant.as_str(), "note": a.note, "applied_at": a.applied_at});
        if let Some((a, _)) = &found
            && a.proof == proof
        {
            return Ok(Reply::ok(shown(a)));
        }
        let application = Application {
            note: request.note,
            applied_at: rfc3339(now),
            proof,
        };
        let first = found.is_none();
        put_file(
            &*self.store,
            &key,
            &application,
            Condition::Always,
            "applying",
        )
        .await?;
        let reply = shown(&application);
        Ok(if first {
            Reply::created(reply)
        } else {
            Reply::ok(reply)
        })
    }

    /// Up to a page of keys under `prefix` after `after`, each read, skipping any gone since.
    async fn read_listed<T: for<'de> Deserialize<'de>>(
        &self,
        prefix: &str,
        after: Option<String>,
        what: &str,
    ) -> Result<Listed<T>, Problem> {
        let listed = self
            .store
            .list_page(Bucket::Identities, prefix, after.as_deref(), OFFSET_PAGE)
            .await
            .map_err(|e| Problem::storage(what, &e))?;
        let mut found = Vec::with_capacity(listed.keys.len());
        for key in listed.keys {
            if let Some((file, _)) = read_file(&*self.store, &key, what).await? {
                found.push((file, key));
            }
        }
        Ok(Listed {
            found,
            more: listed.more,
        })
    }

    /// `GET /academy/masters/self/applications`: a master with standing (`403` `no-standing`)
    /// reads its applications, 20 a page in order of applicant ID, without those older than
    /// [`APPLICATION_DAYS`] days. `?page=` is the applicant to list after.
    pub async fn applications(
        &self,
        authorization: Option<&str>,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let master = self.identity.caller(authorization, now).await?;
        let after = match page {
            None => None,
            Some(t) => {
                let applicant = AgentId::parse(t).ok_or_else(bad_page)?;
                Some(application_key(&master, &applicant))
            }
        };
        self.need_standing(&master).await?;
        let prefix = format!("{APPLICATIONS_PREFIX}{master}/");
        let listed: Listed<Application> = self
            .read_listed(&prefix, after, "reading applications")
            .await?;
        let last = listed
            .found
            .last()
            .and_then(|(_, k)| k.strip_prefix(&prefix)?.strip_suffix(".json"))
            .map(str::to_owned);
        let items: Vec<Value> = listed
            .found
            .iter()
            .filter(|(a, _)| current(&a.applied_at, now))
            .filter_map(|(a, k)| {
                let applicant = k.strip_prefix(&prefix)?.strip_suffix(".json")?;
                Some(json!({"applicant": applicant, "note": a.note, "applied_at": a.applied_at}))
            })
            .collect();
        let next = match last {
            Some(last) if listed.more => {
                Some(format!("/v1/academy/masters/self/applications?page={last}"))
            }
            _ => None,
        };
        Ok(Reply::ok(json!({"items": items, "next": next})))
    }

    /// `PUT /academy/masters/self/applications/{id}`, in ADR 0003's order: the proof, the
    /// caller, the ID, standing (`403` `no-standing`), then a retry: an apprenticeship with the
    /// applicant that's still current returns with `200`, writing its record if a failed first
    /// try left it out. Then the application (`404`, also for one older than
    /// [`APPLICATION_DAYS`] days or from an agent taken down since), the limit of
    /// [`MAX_APPRENTICES`] (`409` `list-full`), where a full counter first drops places whose
    /// record a takedown removed, the counter with `If-Match`, the record with
    /// `If-None-Match: *`, and the application deleted. The same pair ended earlier the same
    /// UTC day gets `409` `too-soon`. `201`.
    pub async fn accept(
        &self,
        call: &Call<'_>,
        applicant: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let master = self.identity.caller(call.authorization, now).await?;
        let applicant = AgentId::parse(applicant).ok_or_else(bad_id)?;
        empty_body(call.body)?;
        self.need_standing(&master).await?;
        let counter_key = sfab_identity::academy_key(&master);
        let (mut counter, etag): (Counter, _) =
            match read_file(&*self.store, &counter_key, "reading a counter").await? {
                Some((c, etag)) => (c, Some(etag)),
                None => (Counter::default(), None),
            };
        let started_at = rfc3339(now);
        let record = Record {
            master: master.as_str().to_owned(),
            apprentice: applicant.as_str().to_owned(),
            started_at: started_at.clone(),
            proof: hex(&admitted.hash),
            ended_at: None,
            ended_by: None,
        };
        let existing = counter
            .apprentices
            .iter()
            .filter_map(|a| ApprenticeshipId::parse(a))
            .find(|a| a.apprentice == applicant);
        if let Some(id) = existing {
            return self.resumed(&id, &record).await;
        }
        let id = ApprenticeshipId {
            master: master.clone(),
            apprentice: applicant.clone(),
            date: utc_date(admitted.timestamp),
        };
        if let Some((found, _)) = self.read_record(&id).await?
            && found.ended_at.is_some()
        {
            let tomorrow = (utc_day(admitted.timestamp) + 1) * DAY;
            return Err(Problem::conflict(
                "too-soon",
                "This apprenticeship ended today; the same pair can start again tomorrow",
            )
            .with("retry_after", rfc3339(tomorrow)));
        }
        let application_key = application_key(&master, &applicant);
        let application: Option<(Application, String)> =
            read_file(&*self.store, &application_key, "reading an application").await?;
        if !application.is_some_and(|(a, _)| current(&a.applied_at, now)) {
            return Err(no_such_application());
        }
        let resident = sfab_identity::born(&*self.store, &applicant)
            .await
            .map_err(|e| Problem::storage("reading a claim", &e))?
            .is_some_and(|b| !b.taken_down);
        if !resident {
            return Err(no_such_application());
        }
        if counter.apprentices.len() >= MAX_APPRENTICES {
            // A takedown of an apprentice deletes its record but can't reach this counter, so
            // a full one drops places whose record is gone.
            let mut kept = Vec::with_capacity(counter.apprentices.len());
            for entry in std::mem::take(&mut counter.apprentices) {
                let gone = match ApprenticeshipId::parse(&entry) {
                    Some(a) => self.read_record(&a).await?.is_none(),
                    None => true,
                };
                if !gone {
                    kept.push(entry);
                }
            }
            counter.apprentices = kept;
        }
        if counter.apprentices.len() >= MAX_APPRENTICES {
            return Err(Problem::conflict(
                "list-full",
                "A master has up to 3 current apprentices; ending one frees its place",
            ));
        }
        counter.apprentices.push(id.text());
        let condition = condition_for(etag);
        if !put_file(
            &*self.store,
            &counter_key,
            &counter,
            condition,
            "writing a counter",
        )
        .await?
        {
            return Err(Problem::lost_race("accepting an apprentice"));
        }
        if !put_file(
            &*self.store,
            &id.key(),
            &record,
            Condition::IfAbsent,
            "starting an apprenticeship",
        )
        .await?
        {
            return self.resumed(&id, &record).await;
        }
        self.store
            .delete(Bucket::Identities, &application_key)
            .await
            .map_err(|e| Problem::storage("removing an accepted application", &e))?;
        Ok(Reply::created(record.shown(&id)))
    }

    /// A retried accept: the current apprenticeship with `200`, after writing `record` if the
    /// first try left none.
    async fn resumed(&self, id: &ApprenticeshipId, record: &Record) -> Result<Reply, Problem> {
        if let Some((found, _)) = self.read_record(id).await? {
            return Ok(Reply::ok(found.shown(id)));
        }
        put_file(
            &*self.store,
            &id.key(),
            record,
            Condition::IfAbsent,
            "starting an apprenticeship",
        )
        .await?;
        let (found, _) = self.read_record(id).await?.ok_or_else(Problem::internal)?;
        Ok(Reply::ok(found.shown(id)))
    }

    /// `PUT /academy/apprenticeships/{apprenticeship}/ending`: either agent ends it. It stays
    /// in both agents' public records, and its work items stay, private to the two. The record
    /// is rewritten with `If-Match`, then the apprenticeship leaves the master's counter, read
    /// again on a lost race, so a place is free. Again is `200`, and redoes the counter.
    pub async fn end(
        &self,
        call: &Call<'_>,
        apprenticeship: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let caller = self.identity.caller(call.authorization, now).await?;
        let id = parse_apprenticeship(apprenticeship)?;
        empty_body(call.body)?;
        let (mut record, etag) = self.party_record(&caller, &id).await?;
        if record.ended_at.is_none() {
            record.ended_at = Some(rfc3339(now));
            record.ended_by = Some(caller.as_str().to_owned());
            let condition = Condition::IfMatch(etag);
            if !put_file(&*self.store, &id.key(), &record, condition, "ending").await? {
                return Err(Problem::lost_race("ending an apprenticeship"));
            }
        }
        self.free_place(&id).await?;
        Ok(Reply::ok(record.shown(&id)))
    }

    /// Takes `id` out of its master's counter with `If-Match`, reading again on a lost race.
    async fn free_place(&self, id: &ApprenticeshipId) -> Result<(), Problem> {
        let key = sfab_identity::academy_key(&id.master);
        let text = id.text();
        for _ in 0..TRIES {
            let found: Option<(Counter, String)> =
                read_file(&*self.store, &key, "reading a counter").await?;
            let Some((mut counter, etag)) = found else {
                return Ok(());
            };
            if !counter.apprentices.contains(&text) {
                return Ok(());
            }
            counter.apprentices.retain(|a| a != &text);
            let condition = Condition::IfMatch(etag);
            if put_file(&*self.store, &key, &counter, condition, "writing a counter").await? {
                return Ok(());
            }
        }
        Err(Problem::lost_race("freeing a master's place"))
    }

    /// `POST /academy/apprenticeships/{apprenticeship}/certificate`: the master awards
    /// `apprenticeship-<apprenticeship>`, whether or not it has ended. The apprentice gets
    /// `403` `not-the-author`, anyone else `not-a-party`. `201`, or `200` with the certificate
    /// already awarded.
    pub async fn award(
        &self,
        call: &Call<'_>,
        apprenticeship: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let caller = self.identity.caller(call.authorization, now).await?;
        let id = parse_apprenticeship(apprenticeship)?;
        empty_body(call.body)?;
        if id.apprentice == caller {
            return Err(Problem::forbidden(
                "not-the-author",
                "Only the master awards the certificate",
            ));
        }
        let (record, _) = self.party_record(&caller, &id).await?;
        let certificate = id.certificate();
        let key = certificates::key_of(&certificate);
        let file =
            CertificateFile::apprenticeship(&record.apprentice, &record.master, rfc3339(now));
        let shown = |f: &CertificateFile| {
            json!({
                "certificate": certificate,
                "kind": f.kind,
                "holder": f.holder,
                "master": f.issuer,
                "issued_at": f.issued_at,
                "revoked": f.revoked_at.is_some(),
            })
        };
        if put_file(&*self.store, &key, &file, Condition::IfAbsent, "awarding").await? {
            return Ok(Reply::created(shown(&file)));
        }
        let found: Option<(CertificateFile, String)> =
            read_file(&*self.store, &key, "reading a certificate").await?;
        let (found, _) = found.ok_or_else(|| Problem::lost_race("awarding a certificate"))?;
        Ok(Reply::ok(shown(&found)))
    }

    /// `POST /academy/apprenticeships/{apprenticeship}/work`: either agent adds a work item of
    /// up to [`MAX_WORK_BYTES`], keyed by its proof with `If-None-Match: *`, so a retry returns
    /// it with `200`. `201`.
    pub async fn add_work(
        &self,
        call: &Call<'_>,
        apprenticeship: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let author = self.identity.caller(call.authorization, now).await?;
        let id = parse_apprenticeship(apprenticeship)?;
        let text = text_of(call.body, MAX_WORK_BYTES, "8 KB")?;
        self.party_record(&author, &id).await?;
        let item = PostId::new(admitted.timestamp, &author, &admitted.hash);
        let post = Post {
            author: author.as_str().to_owned(),
            text,
            added_at: rfc3339(now),
            comments: Some(0),
        };
        let key = id.item_key(&item);
        self.post(&key, &post, "item", &item, "adding work").await
    }

    /// Writes `post` at `key` with `If-None-Match: *`: `201`, or `200` with what a first try
    /// with the same proof wrote.
    async fn post(
        &self,
        key: &str,
        post: &Post,
        field: &str,
        id: &PostId,
        what: &str,
    ) -> Result<Reply, Problem> {
        if put_file(&*self.store, key, post, Condition::IfAbsent, what).await? {
            return Ok(Reply::created(post.shown(field, id)));
        }
        let found: Option<(Post, String)> = read_file(&*self.store, key, what).await?;
        let (found, _) = found.ok_or_else(|| Problem::lost_race(what))?;
        Ok(Reply::ok(found.shown(field, id)))
    }

    /// `GET /academy/apprenticeships/{apprenticeship}/work`: either agent reads its work items,
    /// oldest first, 20 a page, each with its comment count but not its comments, also after
    /// it ended. `?page=` is the item to list after.
    pub async fn work(
        &self,
        authorization: Option<&str>,
        apprenticeship: &str,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let caller = self.identity.caller(authorization, now).await?;
        let id = parse_apprenticeship(apprenticeship)?;
        let after = page
            .map(|t| PostId::parse(t).ok_or_else(bad_page))
            .transpose()?;
        self.party_record(&caller, &id).await?;
        let prefix = id.items_prefix();
        let path = format!("/v1/academy/apprenticeships/{}/work", id.text());
        self.posts(&prefix, after, "item", &path).await
    }

    /// A page of the items or comments under `prefix`.
    async fn posts(
        &self,
        prefix: &str,
        after: Option<PostId>,
        field: &str,
        path: &str,
    ) -> Result<Reply, Problem> {
        let after = after.map(|a| format!("{prefix}{}.json", a.text()));
        let listed: Listed<Post> = self.read_listed(prefix, after, "reading work").await?;
        let shown: Vec<(PostId, Value)> = listed
            .found
            .iter()
            .filter_map(|(p, k)| {
                let id = PostId::of_key(prefix, k)?;
                let shown = p.shown(field, &id);
                Some((id, shown))
            })
            .collect();
        let next = match shown.last() {
            Some((last, _)) if listed.more => Some(format!("{path}?page={}", last.text())),
            _ => None,
        };
        let items: Vec<Value> = shown.into_iter().map(|(_, v)| v).collect();
        Ok(Reply::ok(json!({"items": items, "next": next})))
    }

    /// `GET /academy/apprenticeships/{apprenticeship}/work/{item}/comments`: either agent reads
    /// an item's comments, oldest first, 20 a page.
    pub async fn comments(
        &self,
        authorization: Option<&str>,
        apprenticeship: &str,
        item: &str,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let caller = self.identity.caller(authorization, now).await?;
        let id = parse_apprenticeship(apprenticeship)?;
        let item = PostId::parse(item).ok_or_else(bad_item)?;
        let after = page
            .map(|t| PostId::parse(t).ok_or_else(bad_page))
            .transpose()?;
        self.party_record(&caller, &id).await?;
        self.read_item(&id, &item).await?;
        let prefix = id.comments_prefix(&item);
        let path = format!(
            "/v1/academy/apprenticeships/{}/work/{}/comments",
            id.text(),
            item.text()
        );
        self.posts(&prefix, after, "comment", &path).await
    }

    async fn read_item(
        &self,
        id: &ApprenticeshipId,
        item: &PostId,
    ) -> Result<(Post, String), Problem> {
        read_file(&*self.store, &id.item_key(item), "reading a work item")
            .await?
            .ok_or_else(|| Problem::not_found("There's no such work item"))
    }

    /// `POST /academy/apprenticeships/{apprenticeship}/work/{item}/comments`: either agent
    /// comments, up to 2 KB, keyed by its proof, so a retry returns it with `200`. A new
    /// comment adds 1 to the item's count with `If-Match`, read again on a lost race; the
    /// count is a hint, so failing that is only logged. `201`.
    pub async fn comment(
        &self,
        call: &Call<'_>,
        apprenticeship: &str,
        item: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let author = self.identity.caller(call.authorization, now).await?;
        let id = parse_apprenticeship(apprenticeship)?;
        let item = PostId::parse(item).ok_or_else(bad_item)?;
        let text = text_of(call.body, MAX_COMMENT_BYTES, "2 KB")?;
        self.party_record(&author, &id).await?;
        let (found, etag) = self.read_item(&id, &item).await?;
        let comment = PostId::new(admitted.timestamp, &author, &admitted.hash);
        let post = Post {
            author: author.as_str().to_owned(),
            text,
            added_at: rfc3339(now),
            comments: None,
        };
        let key = format!("{}{}.json", id.comments_prefix(&item), comment.text());
        let reply = self
            .post(&key, &post, "comment", &comment, "commenting")
            .await?;
        if reply.status == 201 {
            self.count_comment(&id.item_key(&item), found, etag).await;
        }
        Ok(reply)
    }

    /// Adds 1 to an item's comment count with `If-Match`, starting from what was read, reading
    /// again on a lost race, up to [`TRIES`] times.
    async fn count_comment(&self, key: &str, mut item: Post, mut etag: String) {
        for _ in 0..TRIES {
            item.comments = Some(item.comments.unwrap_or(0) + 1);
            let condition = Condition::IfMatch(etag);
            match put_file(&*self.store, key, &item, condition, "counting a comment").await {
                Ok(true) => return,
                Ok(false) => {}
                Err(_) => {
                    tracing::warn!("counting a comment failed; the count is a hint");
                    return;
                }
            }
            match read_file::<S, Post>(&*self.store, key, "reading a work item").await {
                Ok(Some((again, tag))) => (item, etag) = (again, tag),
                _ => return,
            }
        }
        tracing::warn!("counting a comment lost every race; the count is a hint");
    }
}

/// Whether a key under `academy/` belongs to the agent from its name, for its takedown: an
/// apprenticeship's whole folder, its record, work and comments, when the agent is either of
/// the two; or an application it received or sent.
pub(crate) fn belongs_to(parts: &[&str], id: &AgentId) -> bool {
    match parts {
        ["apprenticeships", a, ..] => ApprenticeshipId::parse(a).is_some_and(|a| a.is_party(id)),
        ["applications", master, name] => {
            *master == id.as_str() || name.strip_suffix(".json") == Some(id.as_str())
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_apprenticeship_id_names_two_agents_and_a_real_day() {
        let id = ApprenticeshipId::parse("wrenwrenwren-mossmossmoss-2026-11-05").unwrap();
        assert_eq!(id.text(), "wrenwrenwren-mossmossmoss-2026-11-05");
        assert_eq!(id.master.as_str(), "wrenwrenwren");
        for bad in [
            "wrenwrenwren-mossmossmoss-2026-02-30",
            "wrenwrenwren-mossmossmoss",
            "wrenwrenwren_mossmossmoss-2026-11-05",
            "../../x",
            "",
        ] {
            assert_eq!(ApprenticeshipId::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn an_item_id_is_a_time_an_author_and_a_proof() {
        let id = PostId::parse("1793880000-wrenwrenwren-abcdefgh").unwrap();
        assert_eq!(id.text(), "1793880000-wrenwrenwren-abcdefgh");
        for bad in [
            "179388000-wrenwrenwren-abcdefgh",
            "1793880000-wrenwrenwren-abcdefg",
            "1793880000-wrenwrenwren-abcdefgh-x",
            "1793880000/wrenwrenwren-abcdefgh",
        ] {
            assert_eq!(PostId::parse(bad), None, "{bad}");
        }
    }
}
