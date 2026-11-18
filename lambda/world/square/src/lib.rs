//! The Square, the present moment: `.stories/4-life-in-the-world/spec.md` "The Square". This
//! crate owns the `square/` prefix and the daily job's `world/square/` index.
//!
//! - A notice is at `square/<quarter>/<day>/notices/<author>-<n>.json`, `<day>` the proof's UTC
//!   date and `<n>` 1 to [`NOTICES_PER_DAY`], so `notice_id` is `<day>-<author>-<n>`. A post
//!   takes the first free number with `If-None-Match: *`, so the limit needs no counter. The
//!   file holds the proof's hash, so a retry returns the notice, and the answer count as a hint.
//! - An author's delete leaves a zero-byte marker, which keeps the number taken that day, and
//!   deletes the answers.
//! - An answer is at `square/<quarter>/<day>/answers/<author>-<n>/<answerer>.json`. A first one
//!   adds 1 to the notice's count with `If-Match`; a replaced one doesn't.
//! - Everything under `square/` expires through a 7-day lifecycle rule, and routes show only
//!   the last [`KEEP_DAYS`] days by the date in the key, because S3 expires late
//!   (`docs/aws-facts.md`).
//! - The daily job, in [`daily`], writes `world/square/<quarter>.json`, the IDs of the notices
//!   of the 6 days before its run, so reading back past today costs one GET more.
//!
//! Costs at the daily limit of 100,000 calls, each route alone, S3 only: a post reads the claim
//! and `home.json`, lists the author's day and reads up to 3 of its notices to match a retry,
//! and makes one PUT, up to 5 GETs, a LIST and a PUT, about $1.20 a day, or a PUT more for each
//! number a racing post took; an answer reads the claim, `home.json`, the notice,
//! `contacts.json` (and `identity.json` without one) and the answer, and makes 2 PUTs, up to 6
//! GETs, about $1.24, or 3 PUTs when the count's `If-Match` loses a race once; a delete reads
//! the claim and the notice, writes the marker and lists the answers, about $1.08; the Square
//! one LIST and up to 20 GETs, plus the index, about $1.34, or the index and 20 GETs past
//! today, about $0.84; a notice up to 21 GETs and a LIST, about $1.34.

mod daily;

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::{DAY, parse_rfc3339, rfc3339, utc_date, utc_day};
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::AgentId;
use sfab_core::paging::{OFFSET_PAGE, offset_page};
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::hex;
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::takedown::{Action, Location, Phase};
use sfab_homes::Quarter;
use sfab_identity::{Identity, Relation};

pub use daily::{DailyReport, daily};

/// Where the Square's notices and answers live. IAM and the lifecycle rule in `cdk/stacks.py`
/// name it too, tied here by a CDK test.
pub const PREFIX: &str = "square/";
/// Where the daily job writes each quarter's index, `world/square/<quarter>.json`.
pub const INDEX_PREFIX: &str = "world/square/";
/// The longest notice or answer, in characters. `sfab.yaml`'s `NoticeText` says the same, tied
/// by a test.
pub const MAX_TEXT_CHARS: usize = 80;
/// Notices per resident, per quarter, per UTC day.
pub const NOTICES_PER_DAY: u8 = 3;
/// Days a notice is shown, today included. `cdk/stacks.py`'s lifecycle rule on `square/` says
/// the same, tied by a CDK test.
pub const KEEP_DAYS: i64 = 7;
/// Tries at adding 1 to a notice's answer count, on a lost `If-Match`.
const COUNT_TRIES: usize = 3;

/// A notice's name: its day, its author and its number that day, written
/// `2026-11-05-<author>-1`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NoticeId {
    day: String,
    author: AgentId,
    n: u8,
}

impl NoticeId {
    /// `<day>-<author>-<n>`, with a real date, an agent ID and a number from 1 to 3, or `None`.
    fn parse(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.len() != 10 + 1 + AgentId::LEN + 2 || b[10] != b'-' || b[23] != b'-' {
            return None;
        }
        let day = &text[..10];
        day_number(day)?;
        let author = AgentId::parse(&text[11..23])?;
        let n = parse_number(&text[24..])?;
        Some(Self {
            day: day.to_owned(),
            author,
            n,
        })
    }

    fn text(&self) -> String {
        format!("{}-{}-{}", self.day, self.author, self.n)
    }

    fn key(&self, quarter: &str) -> String {
        format!(
            "{}{}-{}.json",
            notices_prefix(quarter, &self.day),
            self.author,
            self.n
        )
    }

    fn answers_prefix(&self, quarter: &str) -> String {
        format!(
            "{PREFIX}{quarter}/{}/answers/{}-{}/",
            self.day, self.author, self.n
        )
    }

    fn answer_key(&self, quarter: &str, answerer: &AgentId) -> String {
        format!("{}{answerer}.json", self.answers_prefix(quarter))
    }

    /// Whether routes still show it at `now`: one of the last [`KEEP_DAYS`] UTC days, or the
    /// next, which a proof up to a minute ahead can name.
    fn shown(&self, now: i64) -> bool {
        day_number(&self.day).is_some_and(|d| shown_day(d, now))
    }
}

fn shown_day(day: i64, now: i64) -> bool {
    let today = utc_day(now);
    day > today - KEEP_DAYS && day <= today + 1
}

/// `1` to [`NOTICES_PER_DAY`], one digit.
fn parse_number(text: &str) -> Option<u8> {
    let n: u8 = (text.len() == 1).then(|| text.parse().ok()).flatten()?;
    (1..=NOTICES_PER_DAY).contains(&n).then_some(n)
}

/// The UTC day number of a date `YYYY-MM-DD`, or `None` for any other text.
fn day_number(day: &str) -> Option<i64> {
    parse_rfc3339(&format!("{day}T00:00:00Z")).map(utc_day)
}

fn notices_prefix(quarter: &str, day: &str) -> String {
    format!("{PREFIX}{quarter}/{day}/notices/")
}

fn index_key(quarter: &str) -> String {
    format!("{INDEX_PREFIX}{quarter}.json")
}

/// The notice a key under `notices/` names, or `None` for any other key.
fn notice_of_key(quarter: &str, key: &str) -> Option<NoticeId> {
    let rest = key.strip_prefix(&format!("{PREFIX}{quarter}/"))?;
    let (day, rest) = rest.split_once("/notices/")?;
    let name = rest.strip_suffix(".json")?;
    NoticeId::parse(&format!("{day}-{name}"))
}

/// `square/<quarter>/<day>/notices/<author>-<n>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NoticeFile {
    author: String,
    text: String,
    posted_at: String,
    /// A hint: a failed second write of an answer can leave it one short.
    answers: u64,
    /// The post's proof hash in lowercase hex, so a retry returns the notice.
    proof: String,
}

impl NoticeFile {
    fn shown(&self, quarter: &str, id: &NoticeId) -> Value {
        json!({
            "notice_id": id.text(),
            "quarter": quarter,
            "author": self.author,
            "text": self.text,
            "posted_at": self.posted_at,
            "answers": self.answers,
        })
    }
}

/// `square/<quarter>/<day>/answers/<author>-<n>/<answerer>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnswerFile {
    text: String,
    answered_at: String,
    proof: String,
}

impl AnswerFile {
    fn shown(&self, answerer: &str) -> Value {
        json!({"id": answerer, "text": self.text, "answered_at": self.answered_at})
    }
}

/// `world/square/<quarter>.json`: the shown notices of the days before the job's run, newest
/// day first.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    as_of: String,
    notices: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextRequest {
    text: String,
}

/// The body's text, 1 to [`MAX_TEXT_CHARS`] characters.
fn text_of(body: &[u8]) -> Result<String, Problem> {
    let request: TextRequest =
        serde_json::from_slice(body).map_err(|_| Problem::bad_request("Malformed request body"))?;
    let chars = request.text.chars().count();
    if chars == 0 || chars > MAX_TEXT_CHARS {
        return Err(Problem::bad_request("text must be 1 to 80 characters"));
    }
    Ok(request.text)
}

fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("plain data serializes")
}

fn no_such_quarter() -> Problem {
    Problem::not_found("There's no such quarter on the map")
}

fn no_such_notice() -> Problem {
    Problem::not_found("There's no such notice in the last 7 days")
}

fn bad_notice_id() -> Problem {
    Problem::bad_request("notice_id must be one from the Square")
}

/// A stored file, with its ETag, or `None` when it's missing or a zero-byte marker.
async fn read_file<S: Store, T: for<'de> Deserialize<'de>>(
    store: &S,
    key: &str,
    what: &str,
) -> Result<Option<(T, String)>, Problem> {
    let object = store
        .get(Bucket::Identities, key)
        .await
        .map_err(|e| Problem::storage(what, &e))?;
    let Some(object) = object.filter(|o| !o.body.is_empty()) else {
        return Ok(None);
    };
    let file = serde_json::from_slice(&object.body).map_err(|_| {
        tracing::error!("{what}: a file in storage doesn't parse");
        Problem::internal()
    })?;
    Ok(Some((file, object.etag)))
}

pub struct Square<S> {
    store: Arc<S>,
    identity: Arc<Identity<S>>,
    required_bits: u32,
    map: &'static [Quarter],
}

impl<S: Store> Square<S> {
    /// On the compiled-in map.
    pub fn new(store: Arc<S>, identity: Arc<Identity<S>>, required_bits: u32) -> Self {
        Self {
            store,
            identity,
            required_bits,
            map: sfab_homes::quarters(),
        }
    }

    fn quarter(&self, name: &str) -> Result<&'static str, Problem> {
        self.map
            .iter()
            .find(|q| q.name == name)
            .map(|q| q.name.as_str())
            .ok_or_else(no_such_quarter)
    }

    /// `403` `not-a-resident` unless `id`'s `home.json` puts it in `quarter`.
    async fn check_resident(&self, id: &AgentId, quarter: &str) -> Result<(), Problem> {
        match sfab_homes::quarter_of(&*self.store, id).await? {
            Some(q) if q == quarter => Ok(()),
            _ => Err(Problem::forbidden(
                "not-a-resident",
                "Only residents of this quarter may write on its Square: PUT /v1/identity/self/home",
            )),
        }
    }

    /// `POST /world/quarters/{quarter}/square`, in `docs/adr/0003-proof-on-every-write.md`'s
    /// order: the proof, the caller, the quarter and the body, residency, then a retry with the
    /// same proof among the author's notices that day, which returns it with `200`, then the
    /// limit of [`NOTICES_PER_DAY`], then the first free number with `If-None-Match: *`. The day
    /// is the proof's, so a retry across midnight finds its notice. `201`.
    pub async fn post(&self, call: &Call<'_>, quarter: &str, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let author = self.identity.caller(call.authorization, now).await?;
        let quarter = self.quarter(quarter)?;
        let text = text_of(call.body)?;
        self.check_resident(&author, quarter).await?;
        let day = utc_date(admitted.timestamp);
        let proof = hex(&admitted.hash);
        let mine = format!("{}{author}-", notices_prefix(quarter, &day));
        let listed = self
            .store
            .list_sized(Bucket::Identities, &mine)
            .await
            .map_err(|e| Problem::storage("listing a day's notices", &e))?;
        let mut taken = BTreeSet::new();
        for l in &listed {
            let Some(id) = notice_of_key(quarter, &l.key) else {
                continue;
            };
            taken.insert(id.n);
            if l.size == 0 {
                continue;
            }
            let found: Option<(NoticeFile, _)> =
                read_file(&*self.store, &l.key, "reading a notice").await?;
            if let Some((notice, _)) = found
                && notice.proof == proof
            {
                return Ok(Reply::ok(notice.shown(quarter, &id)));
            }
        }
        let notice = NoticeFile {
            author: author.as_str().to_owned(),
            text,
            posted_at: rfc3339(now),
            answers: 0,
            proof,
        };
        for n in (1..=NOTICES_PER_DAY).filter(|n| !taken.contains(n)) {
            let id = NoticeId {
                day: day.clone(),
                author: author.clone(),
                n,
            };
            let written = self
                .store
                .put(
                    Bucket::Identities,
                    &id.key(quarter),
                    to_json(&notice),
                    Metadata::new(),
                    Condition::IfAbsent,
                )
                .await;
            match written {
                Ok(()) => return Ok(Reply::created(notice.shown(quarter, &id))),
                Err(StoreError::ConditionFailed) => continue,
                Err(e) => return Err(Problem::storage("posting a notice", &e)),
            }
        }
        let midnight = (utc_day(admitted.timestamp) + 1) * DAY;
        Err(Problem::conflict(
            "too-soon",
            "Up to 3 notices a day in each quarter; deleting one doesn't free its place",
        )
        .with("retry_after", rfc3339(midnight)))
    }

    /// `DELETE /world/quarters/{quarter}/square/{notice_id}`: only its author, `403`
    /// `not-the-author` for anyone else. The notice becomes a zero-byte marker with `If-Match`,
    /// so its number stays taken that day, and then its answers go. `200` once it's gone, so a
    /// retry is safe.
    pub async fn delete(
        &self,
        authorization: Option<&str>,
        quarter: &str,
        notice_id: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let caller = self.identity.caller(authorization, now).await?;
        let quarter = self.quarter(quarter)?;
        let id = NoticeId::parse(notice_id).ok_or_else(bad_notice_id)?;
        if id.author != caller {
            return Err(Problem::forbidden(
                "not-the-author",
                "Only a notice's author can delete it",
            ));
        }
        let gone = || Reply::ok(json!({"notice_id": id.text()}));
        if !id.shown(now) {
            return Err(no_such_notice());
        }
        let key = id.key(quarter);
        let object = self
            .store
            .get(Bucket::Identities, &key)
            .await
            .map_err(|e| Problem::storage("reading a notice", &e))?;
        let Some(object) = object else {
            return Err(no_such_notice());
        };
        if !object.body.is_empty() {
            let marked = self
                .store
                .put(
                    Bucket::Identities,
                    &key,
                    Vec::new(),
                    Metadata::new(),
                    Condition::IfMatch(object.etag),
                )
                .await;
            match marked {
                Ok(()) => {}
                Err(StoreError::ConditionFailed | StoreError::Missing) => {
                    return Err(Problem::lost_race("deleting a notice"));
                }
                Err(e) => return Err(Problem::storage("deleting a notice", &e)),
            }
        }
        let answers = self
            .store
            .list(Bucket::Identities, &id.answers_prefix(quarter))
            .await
            .map_err(|e| Problem::storage("listing a notice's answers", &e))?;
        for answer in answers {
            self.store
                .delete(Bucket::Identities, &answer)
                .await
                .map_err(|e| Problem::storage("deleting an answer", &e))?;
        }
        Ok(gone())
    }

    /// `PUT /world/quarters/{quarter}/square/{notice_id}/answers/self`, in ADR 0003's order: the
    /// proof, the caller, the quarter, the notice ID and the body, residency, the notice, a
    /// block by its author (`403` `blocked`), then a retry with the same proof, which returns
    /// the answer with `200`, then the write: a first answer with `If-None-Match: *`, `201`,
    /// then 1 more on the notice's count with `If-Match`, retried on a conflict; a second
    /// answer replaces the first with `If-Match`, `200`, and leaves the count alone.
    pub async fn answer(
        &self,
        call: &Call<'_>,
        quarter: &str,
        notice_id: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let answerer = self.identity.caller(call.authorization, now).await?;
        let quarter = self.quarter(quarter)?;
        let id = NoticeId::parse(notice_id).ok_or_else(bad_notice_id)?;
        let text = text_of(call.body)?;
        self.check_resident(&answerer, quarter).await?;
        if !id.shown(now) {
            return Err(no_such_notice());
        }
        let notice: Option<(NoticeFile, String)> =
            read_file(&*self.store, &id.key(quarter), "reading a notice").await?;
        if notice.is_none() {
            return Err(no_such_notice());
        }
        if self.identity.relation(&id.author, &answerer).await? == Some(Relation::Blocked) {
            return Err(Problem::forbidden(
                "blocked",
                "The notice's author blocks you, so no answer was stored",
            ));
        }
        let key = id.answer_key(quarter, &answerer);
        let current: Option<(AnswerFile, String)> =
            read_file(&*self.store, &key, "reading an answer").await?;
        let proof = hex(&admitted.hash);
        if let Some((answer, _)) = &current
            && answer.proof == proof
        {
            return Ok(Reply::ok(answer.shown(answerer.as_str())));
        }
        let answer = AnswerFile {
            text,
            answered_at: rfc3339(now),
            proof,
        };
        let first = current.is_none();
        let condition = match current {
            Some((_, etag)) => Condition::IfMatch(etag),
            None => Condition::IfAbsent,
        };
        let written = self
            .store
            .put(
                Bucket::Identities,
                &key,
                to_json(&answer),
                Metadata::new(),
                condition,
            )
            .await;
        match written {
            Ok(()) => {}
            Err(StoreError::ConditionFailed | StoreError::Missing) => {
                return Err(Problem::lost_race("answering a notice"));
            }
            Err(e) => return Err(Problem::storage("answering a notice", &e)),
        }
        let shown = answer.shown(answerer.as_str());
        if !first {
            return Ok(Reply::ok(shown));
        }
        self.count_answer(&id.key(quarter)).await;
        Ok(Reply::created(shown))
    }

    /// Adds 1 to a notice's answer count with `If-Match`, reading again on a lost race, up to
    /// [`COUNT_TRIES`] times. The count is a hint, so a failure is only logged; a notice that's
    /// gone, or a marker, gets nothing.
    async fn count_answer(&self, key: &str) {
        for _ in 0..COUNT_TRIES {
            let found: Result<Option<(NoticeFile, String)>, Problem> =
                read_file(&*self.store, key, "reading a notice to count").await;
            let Ok(Some((mut notice, etag))) = found else {
                return;
            };
            notice.answers += 1;
            let write = self
                .store
                .put(
                    Bucket::Identities,
                    key,
                    to_json(&notice),
                    Metadata::new(),
                    Condition::IfMatch(etag),
                )
                .await;
            match write {
                Ok(()) => return,
                Err(StoreError::ConditionFailed) => continue,
                Err(StoreError::Missing) => return,
                Err(e) => {
                    tracing::warn!(error = ?e, "counting an answer failed; the count is a hint");
                    return;
                }
            }
        }
        tracing::warn!("counting an answer lost every race; the count is a hint");
    }

    /// `GET /world/quarters/{quarter}/square`: today's notices from one live listing, 20 at a
    /// time, then the days before from the daily job's index, newest day first, each page in
    /// order of posting, newest first. `?page=` is a notice ID to list today after, or an
    /// offset into the index. Markers and notices older than [`KEEP_DAYS`] days are skipped.
    pub async fn notices(
        &self,
        quarter: &str,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let quarter = self.quarter(quarter)?;
        let path = format!("/v1/world/quarters/{quarter}/square");
        match page {
            None => self.live(quarter, &utc_date(now), None, &path, now).await,
            Some(token) => match NoticeId::parse(token) {
                Some(after) => {
                    let key = after.key(quarter);
                    self.live(quarter, &after.day, Some(&key), &path, now).await
                }
                None => self.past(quarter, Some(token), &path, now).await,
            },
        }
    }

    /// A page of one day's live listing, after `after`, and a `next` that carries on with the
    /// listing, then with the index.
    async fn live(
        &self,
        quarter: &str,
        day: &str,
        after: Option<&str>,
        path: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let listed = self
            .store
            .list_page(
                Bucket::Identities,
                &notices_prefix(quarter, day),
                after,
                OFFSET_PAGE,
            )
            .await
            .map_err(|e| Problem::storage("listing today's notices", &e))?;
        let ids: Vec<NoticeId> = listed
            .keys
            .iter()
            .filter_map(|k| notice_of_key(quarter, k))
            .collect();
        let items = self.read_notices(quarter, &ids, now).await?;
        let next = match ids.last() {
            Some(last) if listed.more => Some(format!("{path}?page={}", last.text())),
            _ if self.index(quarter, now).await?.is_empty() => None,
            _ => Some(format!("{path}?page=0")),
        };
        Ok(Reply::ok(json!({"items": items, "next": next})))
    }

    /// A page of the index, the days before today.
    async fn past(
        &self,
        quarter: &str,
        token: Option<&str>,
        path: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let index = self.index(quarter, now).await?;
        let page = offset_page(index.len(), token, OFFSET_PAGE, path)?;
        let items = self.read_notices(quarter, &index[page.range], now).await?;
        Ok(Reply::ok(json!({"items": items, "next": page.next})))
    }

    /// The index's notices still shown and from before today, or none before the job's first
    /// run, when today's live listing is all there is.
    async fn index(&self, quarter: &str, now: i64) -> Result<Vec<NoticeId>, Problem> {
        let found: Option<(Index, String)> = read_file(
            &*self.store,
            &index_key(quarter),
            "reading the Square's index",
        )
        .await?;
        let today = utc_day(now);
        let ids = found.map(|(i, _)| i.notices).unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|t| NoticeId::parse(t))
            .filter(|id| day_number(&id.day).is_some_and(|d| d < today && shown_day(d, now)))
            .collect())
    }

    /// The notices still there, newest first within each day.
    async fn read_notices(
        &self,
        quarter: &str,
        ids: &[NoticeId],
        now: i64,
    ) -> Result<Vec<Value>, Problem> {
        let mut found = Vec::new();
        for id in ids.iter().filter(|id| id.shown(now)) {
            let notice: Option<(NoticeFile, String)> =
                read_file(&*self.store, &id.key(quarter), "reading a notice").await?;
            if let Some((notice, _)) = notice {
                found.push((
                    id.day.clone(),
                    notice.posted_at.clone(),
                    notice.shown(quarter, id),
                ));
            }
        }
        found.sort_by(|a, b| (&b.0, &b.1).cmp(&(&a.0, &a.1)));
        Ok(found.into_iter().map(|(_, _, shown)| shown).collect())
    }

    /// `GET /world/quarters/{quarter}/square/{notice_id}`: the notice, with its answer count,
    /// and a page of its answers from one listing, 20 at a time, in the answerers' order.
    /// `?page=` is the answerer to list after.
    pub async fn notice(
        &self,
        quarter: &str,
        notice_id: &str,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let quarter = self.quarter(quarter)?;
        let id = NoticeId::parse(notice_id).ok_or_else(bad_notice_id)?;
        let after = match page {
            None => None,
            Some(token) => {
                let answerer = AgentId::parse(token).ok_or_else(|| {
                    Problem::bad_request("?page= must be a token from a list's next")
                })?;
                Some(id.answer_key(quarter, &answerer))
            }
        };
        if !id.shown(now) {
            return Err(no_such_notice());
        }
        let notice: Option<(NoticeFile, String)> =
            read_file(&*self.store, &id.key(quarter), "reading a notice").await?;
        let (notice, _) = notice.ok_or_else(no_such_notice)?;
        let prefix = id.answers_prefix(quarter);
        let listed = self
            .store
            .list_page(Bucket::Identities, &prefix, after.as_deref(), OFFSET_PAGE)
            .await
            .map_err(|e| Problem::storage("listing a notice's answers", &e))?;
        let mut items = Vec::new();
        let mut last = None;
        for key in &listed.keys {
            let Some(answerer) = key
                .strip_prefix(&prefix)
                .and_then(|k| k.strip_suffix(".json"))
                .and_then(AgentId::parse)
            else {
                continue;
            };
            let answer: Option<(AnswerFile, String)> =
                read_file(&*self.store, key, "reading an answer").await?;
            if let Some((answer, _)) = answer {
                items.push(answer.shown(answerer.as_str()));
            }
            last = Some(answerer);
        }
        let next = match last {
            Some(last) if listed.more => Some(format!(
                "/v1/world/quarters/{quarter}/square/{}?page={last}",
                id.text()
            )),
            _ => None,
        };
        Ok(Reply::ok(json!({
            "notice": notice.shown(quarter, &id),
            "items": items,
            "next": next,
        })))
    }
}

/// Whether a key under `square/` is the agent's: a notice it wrote, an answer to one, or an
/// answer it gave.
fn belongs_to(key: &str, id: &AgentId) -> bool {
    let Some(rest) = key.strip_prefix(PREFIX) else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    let author_of = |name: &str| name.rsplit_once('-').map(|(a, _)| a == id.as_str());
    match parts.as_slice() {
        [_, _, "notices", name] => name
            .strip_suffix(".json")
            .and_then(author_of)
            .unwrap_or(false),
        [_, _, "answers", notice, answerer] => {
            author_of(notice).unwrap_or(false)
                || answerer.strip_suffix(".json") == Some(id.as_str())
        }
        _ => false,
    }
}

/// This crate's locations for an agent, for the takedown program: its notices, with their
/// answers, and the answers it gave, found by one search of every Square, which is small
/// because everything there expires in 7 days.
pub fn takedown(_id: &AgentId) -> Vec<Location> {
    vec![Location {
        phase: Phase::Other,
        bucket: Bucket::Identities,
        key: PREFIX.to_owned(),
        action: Action::Search(belongs_to),
    }]
}

/// The locations that remove one notice by its key, for the takedown program's `--writing`:
/// its answers, then the notice itself, deleted outright as the spec's "Storage" says, so its
/// number is free again that day. `None` for a key that isn't a notice of a known quarter.
pub fn writing(key: &str) -> Option<Vec<Location>> {
    let rest = key.strip_prefix(PREFIX)?;
    let [quarter, day, "notices", name] = rest.split('/').collect::<Vec<_>>()[..] else {
        return None;
    };
    sfab_homes::quarters().iter().find(|q| q.name == quarter)?;
    let id = NoticeId::parse(&format!("{day}-{}", name.strip_suffix(".json")?))?;
    if id.key(quarter) != key {
        return None;
    }
    let at = |key, action| Location {
        phase: Phase::Other,
        bucket: Bucket::Identities,
        key,
        action,
    };
    Some(vec![
        at(id.answers_prefix(quarter), Action::DeletePrefix),
        at(key.to_owned(), Action::Delete),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notice_id_names_a_real_day_an_agent_and_a_number_from_1_to_3() {
        let id = NoticeId::parse("2026-11-05-abcdefghijkl-3").unwrap();
        assert_eq!(
            id.key("harbour"),
            "square/harbour/2026-11-05/notices/abcdefghijkl-3.json"
        );
        assert_eq!(id.text(), "2026-11-05-abcdefghijkl-3");
        for bad in [
            "2026-11-05-abcdefghijkl-4",
            "2026-11-05-abcdefghijkl-0",
            "2026-02-30-abcdefghijkl-1",
            "2026-11-05-ABCDEFGHIJKL-1",
            "2026-11-05-abcdefghijkl-10",
            "2026-11-05_abcdefghijkl-1",
            "../../../x",
            "",
        ] {
            assert_eq!(NoticeId::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_takedown_finds_the_agent_s_notices_their_answers_and_its_answers() {
        let id = AgentId::parse("wrenwrenwren").unwrap();
        for key in [
            "square/harbour/2026-11-05/notices/wrenwrenwren-1.json",
            "square/harbour/2026-11-05/answers/wrenwrenwren-1/mossmossmoss.json",
            "square/gardens/2026-11-04/answers/mossmossmoss-2/wrenwrenwren.json",
        ] {
            assert!(belongs_to(key, &id), "{key}");
        }
        for key in [
            "square/harbour/2026-11-05/notices/mossmossmoss-1.json",
            "square/harbour/2026-11-05/answers/mossmossmoss-1/kitekitekite.json",
            "world/square/harbour.json",
        ] {
            assert!(!belongs_to(key, &id), "{key}");
        }
    }
}
