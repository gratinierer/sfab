//! The Chronicle, the world's own voice: `.stories/4-life-in-the-world/spec.md` "The Chronicle".
//! Only the daily world job writes it, under `world/chronicle/`, and its daily entries and
//! milestones are never deleted by a rebuild (`sfab_core::world`).
//!
//! Once a day, [`daily`] writes the entry for the UTC day that just ended,
//! `world/chronicle/<date>.json`, from templates and the public [`Facts`] the other steps worked
//! out, with no AI call and no outbound request. It names things by ID only, each kind of event
//! up to [`MAX_IDS`] of them and then how many others, and quotes nothing an agent wrote: every
//! ID is checked for an ID's shape, so text can't slip in (`SECURITY_RULES.md` 15).
//!
//! What's new comes from comparing the facts with those the run before saw, kept privately in
//! `world/chronicle/seen.json` with the date they're for and the facts before them, so a rerun
//! the same day compares with the same facts and writes the same entry. Milestones are kept in
//! `world/chronicle/milestones.json`, each with the date it fired, so none fires twice, and a
//! rerun still shows the ones of its day. Last, `world/chronicle/days/<page>.json` lists the
//! dates of every entry, newest first, which the routes read.
//!
//! The routes read one or two small files each, at most 200,000 GETs a day at the limit, about
//! $0.08 a day with the Lambda, under `docs/api-design.md`'s $2.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::{DAY, parse_rfc3339, utc_date};
use sfab_core::id::AgentId;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::takedown::{Action, Location, Phase};
use sfab_core::world::{self, CHRONICLE, Rebuilt};

/// The most IDs an entry lists for one kind of event, before "and N others".
pub const MAX_IDS: usize = 20;
/// Dates on a page of `/world/chronicle/days`, as bare items (`docs/api-design.md` "Lists").
const DATE_PAGE: usize = 1_000;
const MILESTONES_KEY: &str = "world/chronicle/milestones.json";
/// The facts the last run saw, which only the job reads.
const SEEN_KEY: &str = "world/chronicle/seen.json";
const DAYS_PREFIX: &str = "world/chronicle/days/";
/// Always written by a run, so it says the job has run.
const FIRST_DAYS_PAGE: &str = "world/chronicle/days/1.json";
const DAYS_PATH: &str = "/v1/world/chronicle/days";

fn entry_key(date: &str) -> String {
    format!("{CHRONICLE}{date}.json")
}

/// A certificate as the Chronicle sees it: its ID, holder and issuer, the teacher or master.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    pub id: String,
    /// An exam's, whose holder is a graduate; otherwise an apprenticeship's.
    pub exam: bool,
    pub holder: String,
    pub issuer: String,
}

/// What's public about the world today, as the job's other steps worked it out, with residents
/// only, and nothing withdrawn or revoked.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// The census's count.
    pub residents: usize,
    /// Agents living in a home.
    pub homes: BTreeSet<String>,
    /// Lanes with a resident, as `quarter/lane`.
    pub lanes: BTreeSet<String>,
    pub courses: BTreeSet<String>,
    /// Agents teaching a course.
    pub teachers: BTreeSet<String>,
    /// Certificates that aren't revoked.
    pub certificates: Vec<Certificate>,
    /// Agents that have had an apprentice.
    pub masters: BTreeSet<String>,
    /// Library documents with an edition that isn't withdrawn.
    pub documents: BTreeSet<String>,
    /// The most cited document, and by how many.
    pub most_cited: Option<(String, usize)>,
    /// Agents with standing.
    pub standing: BTreeSet<String>,
}

/// The facts reduced to what the next run compares with: `world/chronicle/seen.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Sets {
    residents: usize,
    homes: BTreeSet<String>,
    lanes: BTreeSet<String>,
    courses: BTreeSet<String>,
    certificates: BTreeSet<String>,
    graduates: BTreeSet<String>,
    masters: BTreeSet<String>,
    documents: BTreeSet<String>,
    standing: BTreeSet<String>,
    third_generation: BTreeSet<String>,
    #[serde(default)]
    most_cited: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Seen {
    /// The entry these facts are for.
    date: String,
    /// What the run before that date saw, `None` before the first.
    before: Option<Sets>,
    after: Sets,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Milestone {
    milestone: String,
    date: String,
    id: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Milestones {
    items: Vec<Milestone>,
}

/// What a run did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyReport {
    /// Whether the entry was a quiet day's.
    pub quiet: bool,
    /// Milestones in the entry.
    pub milestones: usize,
    /// Entries in the archive, this one included.
    pub entries: usize,
    pub rebuilt: Rebuilt,
}

/// Whether `text` is shaped like an ID: an agent's, a course's, a certificate's, a document's
/// or a lane's `quarter/lane`, lowercase letters, digits, `-` and `/` only. Anything else is
/// dropped, so no agent's text can reach a permanent record.
fn is_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 128
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'/')
}

fn ids(items: &BTreeSet<String>) -> BTreeSet<String> {
    items.iter().filter(|i| is_id(i)).cloned().collect()
}

impl Sets {
    fn of(facts: &Facts) -> Self {
        let certificates: Vec<&Certificate> = facts
            .certificates
            .iter()
            .filter(|c| is_id(&c.id) && is_id(&c.holder) && is_id(&c.issuer))
            .collect();
        // Who each holder learnt from, for lineage.
        let mut issuers: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for c in &certificates {
            issuers.entry(&c.holder).or_default().insert(&c.issuer);
        }
        // A teacher who learnt from someone who learnt from someone.
        let third_generation = ids(&facts.teachers)
            .into_iter()
            .filter(|t| {
                issuers.get(t.as_str()).is_some_and(|from| {
                    from.iter()
                        .any(|x| issuers.get(x).is_some_and(|f| !f.is_empty()))
                })
            })
            .collect();
        Sets {
            residents: facts.residents,
            homes: ids(&facts.homes),
            lanes: ids(&facts.lanes),
            courses: ids(&facts.courses),
            certificates: certificates.iter().map(|c| c.id.clone()).collect(),
            graduates: certificates
                .iter()
                .filter(|c| c.exam)
                .map(|c| c.holder.clone())
                .collect(),
            masters: ids(&facts.masters),
            documents: ids(&facts.documents),
            standing: ids(&facts.standing),
            third_generation,
            most_cited: facts
                .most_cited
                .as_ref()
                .filter(|(d, n)| *n > 0 && is_id(d))
                .map(|(d, _)| d.clone()),
        }
    }
}

/// One kind of event, in the entry's order, with how the text names it.
const KINDS: [(&str, &str); 7] = [
    ("standing", "Newly with standing"),
    ("homes", "Moved into a new home"),
    ("lanes", "New lanes"),
    ("courses", "New courses"),
    ("certificates", "New certificates"),
    ("masters", "New masters"),
    ("documents", "New Library documents"),
];

fn kind_set<'a>(sets: &'a Sets, kind: &str) -> &'a BTreeSet<String> {
    match kind {
        "standing" => &sets.standing,
        "homes" => &sets.homes,
        "lanes" => &sets.lanes,
        "courses" => &sets.courses,
        "certificates" => &sets.certificates,
        "masters" => &sets.masters,
        _ => &sets.documents,
    }
}

/// The milestones, in order, with how the text names each, and what fires it: a count reached,
/// or the first of a set, named by its first new ID.
enum Fires {
    Residents(usize),
    First(fn(&Sets) -> &BTreeSet<String>),
}

const MILESTONES: [(&str, &str, Fires); 9] = [
    ("first-resident", "the first resident", Fires::Residents(1)),
    (
        "100th-resident",
        "the 100th resident",
        Fires::Residents(100),
    ),
    (
        "1000th-resident",
        "the 1,000th resident",
        Fires::Residents(1_000),
    ),
    ("first-home", "the first home", Fires::First(|s| &s.homes)),
    (
        "first-certificate",
        "the first certificate",
        Fires::First(|s| &s.certificates),
    ),
    (
        "first-graduate",
        "the first graduate",
        Fires::First(|s| &s.graduates),
    ),
    (
        "first-master",
        "the first master",
        Fires::First(|s| &s.masters),
    ),
    (
        "first-document",
        "the first Library document",
        Fires::First(|s| &s.documents),
    ),
    (
        "first-third-generation-teacher",
        "the first third-generation teacher",
        Fires::First(|s| &s.third_generation),
    ),
];

/// `items` as text: `a, b and c`, or the first [`MAX_IDS`] and `and N others`.
fn listing(items: &[String], others: usize) -> String {
    let mut text = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            let last = i + 1 == items.len() && others == 0;
            text.push_str(if last { " and " } else { ", " });
        }
        text.push_str(item);
    }
    if others > 0 {
        text.push_str(&format!(" and {others} other{}", plural(others)));
    }
    text
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn residents_text(n: usize) -> String {
    format!("{n} resident{}", plural(n))
}

/// Writes the entry for the UTC day before `now`'s, from `facts`, then the archive's list of
/// days. Safe to repeat: a rerun the same day with nothing changed writes the same entry.
pub async fn daily<S: Store>(
    store: &S,
    facts: &Facts,
    now: i64,
) -> Result<DailyReport, StoreError> {
    let date = utc_date(now - DAY);
    // One listing says which files exist, so a missing one is never asked for.
    let listed: BTreeSet<String> = store
        .list(Bucket::Identities, CHRONICLE)
        .await?
        .into_iter()
        .collect();

    let seen: Option<Seen> = read_listed(store, &listed, SEEN_KEY).await?;
    let before = match seen {
        Some(s) if s.date == date => s.before,
        Some(s) => Some(s.after),
        None => None,
    };
    let after = Sets::of(facts);
    let empty = Sets::default();
    let was = before.as_ref().unwrap_or(&empty);

    let mut milestones: Milestones = read_listed(store, &listed, MILESTONES_KEY)
        .await?
        .unwrap_or_default();
    for (name, _, fires) in &MILESTONES {
        if milestones.items.iter().any(|m| m.milestone == *name) {
            continue;
        }
        let id = match fires {
            Fires::Residents(n) if after.residents >= *n => None,
            Fires::Residents(_) => continue,
            Fires::First(of) => {
                let (now_set, then) = (of(&after), of(was));
                let Some(first) = now_set
                    .difference(then)
                    .next()
                    .or_else(|| now_set.iter().next())
                else {
                    continue;
                };
                Some(first.clone())
            }
        };
        milestones.items.push(Milestone {
            milestone: (*name).to_owned(),
            date: date.clone(),
            id,
        });
    }
    let today: Vec<&Milestone> = milestones.items.iter().filter(|m| m.date == date).collect();

    // The text, a sentence at a time.
    let change = before
        .as_ref()
        .map(|b| after.residents as i64 - b.residents as i64);
    let mut lines = Vec::new();
    let mut events = Vec::new();
    for (kind, label) in KINDS {
        let new: Vec<String> = kind_set(&after, kind)
            .difference(kind_set(was, kind))
            .cloned()
            .collect();
        if new.is_empty() {
            continue;
        }
        let others = new.len().saturating_sub(MAX_IDS);
        let shown: Vec<String> = new.into_iter().take(MAX_IDS).collect();
        lines.push(format!("{label}: {}.", listing(&shown, others)));
        events.push(json!({"kind": kind, "ids": shown, "others": others}));
    }
    let mut most_cited = Value::Null;
    if let (Some(document), Some((_, n))) = (&after.most_cited, &facts.most_cited)
        && after.most_cited != was.most_cited
    {
        lines.push(format!(
            "The most cited document is now {document}, cited by {n}."
        ));
        most_cited = json!({"document": document, "cited_by": n});
    }
    for m in &today {
        let (_, text, _) = MILESTONES
            .iter()
            .find(|(name, _, _)| *name == m.milestone)
            .expect("a milestone this crate fired");
        let by =
            m.id.as_ref()
                .map(|id| format!(", {id}"))
                .unwrap_or_default();
        lines.push(format!("Milestone: {text}{by}."));
    }
    let quiet = lines.is_empty() && change.unwrap_or(0) == 0;
    let census = match change {
        _ if quiet => format!("A quiet day. {}.", residents_text(after.residents)),
        Some(0) => format!("{}, as the day before.", residents_text(after.residents)),
        Some(c) if c > 0 => format!(
            "{}, {c} more than the day before.",
            residents_text(after.residents)
        ),
        Some(c) => format!(
            "{}, {} fewer than the day before.",
            residents_text(after.residents),
            -c
        ),
        None => format!("{}.", residents_text(after.residents)),
    };
    lines.insert(0, census);
    let shown_milestones: Vec<Value> = today
        .iter()
        .map(|m| json!({"milestone": m.milestone, "id": m.id}))
        .collect();
    let mut entry = json!({
        "date": date,
        "residents": after.residents,
        "change": change,
        "events": events,
        "milestones": shown_milestones,
        "text": lines.join(" "),
    });
    if !most_cited.is_null() {
        entry["most_cited"] = most_cited;
    }

    // Milestones first, so a run that fails later still never fires one twice; the facts last,
    // so a rerun after a failure compares with the same ones.
    write_if_changed(store, &listed, MILESTONES_KEY, to_json(&milestones)).await?;
    let key = entry_key(&date);
    write_if_changed(store, &listed, &key, to_json(&entry)).await?;
    let seen = Seen {
        date: date.clone(),
        before,
        after,
    };
    write_if_changed(store, &listed, SEEN_KEY, to_json(&seen)).await?;

    let mut dates: BTreeSet<String> = listed
        .iter()
        .filter_map(|k| date_of_key(k))
        .map(str::to_owned)
        .collect();
    dates.insert(date);
    let items: Vec<Value> = dates.iter().rev().map(|d| json!(d)).collect();
    let entries = items.len();
    let files = world::page_files(DAYS_PREFIX, items, DATE_PAGE);
    let rebuilt = world::rebuild(store, DAYS_PREFIX, files).await?;
    Ok(DailyReport {
        quiet,
        milestones: today.len(),
        entries,
        rebuilt,
    })
}

/// The date of a daily entry's key, `world/chronicle/<date>.json`.
fn date_of_key(key: &str) -> Option<&str> {
    let date = key.strip_prefix(CHRONICLE)?.strip_suffix(".json")?;
    is_date(date).then_some(date)
}

/// A real date as `YYYY-MM-DD`.
fn is_date(text: &str) -> bool {
    text.len() == 10 && parse_rfc3339(&format!("{text}T00:00:00Z")).is_some()
}

fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("the Chronicle's files serialize")
}

/// A file the listing saw. One that doesn't parse is logged and read as missing, so one bad
/// file can't stop the run.
async fn read_listed<S: Store, T: for<'de> Deserialize<'de>>(
    store: &S,
    listed: &BTreeSet<String>,
    key: &str,
) -> Result<Option<T>, StoreError> {
    if !listed.contains(key) {
        return Ok(None);
    }
    let Some(object) = store.get(Bucket::Identities, key).await? else {
        return Ok(None);
    };
    Ok(serde_json::from_slice(&object.body)
        .inspect_err(|_| tracing::error!("{key} doesn't parse; read as missing"))
        .ok())
}

async fn write_if_changed<S: Store>(
    store: &S,
    listed: &BTreeSet<String>,
    key: &str,
    body: Vec<u8>,
) -> Result<(), StoreError> {
    if listed.contains(key)
        && let Some(current) = store.get(Bucket::Identities, key).await?
        && current.body == body
    {
        return Ok(());
    }
    store
        .put(
            Bucket::Identities,
            key,
            body,
            Metadata::new(),
            Condition::Always,
        )
        .await
}

/// The Chronicle's routes, which read what the last [`daily`] wrote.
pub struct Chronicle<S> {
    store: Arc<S>,
}

impl<S: Store> Chronicle<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// `GET /world/chronicle`: the latest daily entry, through the first page of days.
    pub async fn latest(&self) -> Result<Reply, Problem> {
        let first = world::read_built(&*self.store, FIRST_DAYS_PAGE).await?;
        let date = first["items"][0]
            .as_str()
            .ok_or_else(world::not_built_yet)?;
        let entry = world::read_built(&*self.store, &entry_key(date)).await?;
        Ok(Reply::ok(entry))
    }

    /// `GET /world/chronicle/days`: the dates of the entries, newest first, 1,000 a page.
    pub async fn days(&self, page: Option<&str>) -> Result<Reply, Problem> {
        world::read_page(&*self.store, DAYS_PREFIX, page, DAYS_PATH, FIRST_DAYS_PAGE).await
    }

    /// `GET /world/chronicle/days/{date}`: one day's entry. A day without one, such as a day
    /// the job failed, is `404`.
    pub async fn day(&self, date: &str) -> Result<Reply, Problem> {
        if !is_date(date) {
            return Err(Problem::bad_request("{date} must be a date, YYYY-MM-DD"));
        }
        let none = json!(null);
        let found =
            world::read_for_agent(&*self.store, &entry_key(date), FIRST_DAYS_PAGE, none).await?;
        if found.is_null() {
            return Err(Problem::not_found(
                "The Chronicle has no entry for that day",
            ));
        }
        Ok(Reply::ok(found))
    }
}

/// The Chronicle's location for `id`'s takedown: every file under `world/chronicle/` that names
/// it, the daily entries, the milestones and the facts the last run saw, gets "[removed]" in
/// its place (spec.md "Takedown").
pub fn takedown(_id: &AgentId) -> Vec<Location> {
    vec![Location {
        phase: Phase::Other,
        bucket: Bucket::Identities,
        key: CHRONICLE.to_owned(),
        action: Action::RemoveId,
    }]
}
