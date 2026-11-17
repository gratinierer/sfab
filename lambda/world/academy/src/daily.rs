//! The daily job's part for the Academy's exams, and the routes that read what it built:
//! `.stories/4-life-in-the-world/spec.md` "The Academy" ("Lineage", "What's public") and
//! "Foundations" ("What the daily job writes", "Before the first daily run", "Unknown IDs",
//! "Paging").
//!
//! One listing of `academy/courses/` gives every course and enrolment by key, and one of
//! `academy/certificates/` every certificate, each of which is read once, so the job never asks
//! for a key it didn't list, which S3 answers with `403` under its conditioned listing
//! (`docs/aws-facts.md`); attempts are never read. Only residents count, so a taken-down agent
//! leaves every list at the next run. Through
//! `sfab_core::world::rebuild` it writes, under `world/academy/`, the catalogue, each course's
//! `enrolled` and `graduates` pages, each teacher's or marker's page with its `graduates`
//! pages, and a file for each certificate that counts; under `world/agents/<id>/`, each agent's
//! `enrolments` and `certificates` pages, beside the takedown index, which isn't this crate's;
//! and last `world/academy.json`, which says the job has run.

use std::collections::{BTreeMap, BTreeSet};

use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use sfab_core::daily::rfc3339;
use sfab_core::id::AgentId;
use sfab_core::paging::OFFSET_PAGE;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::world::{self, PARALLEL, Rebuilt};

use crate::certificates::{self, CERTIFICATES_PREFIX, CertificateFile};
use crate::{Academy, COURSES_PREFIX, CourseFile, CourseId, parse_course, read_file, to_json};

/// Everything this step writes under `world/academy/`.
const WORLD_PREFIX: &str = "world/academy/";
/// Written last by each run, so it says the job has run, with `as_of`.
const BUILT_KEY: &str = "world/academy.json";
const CATALOGUE_PREFIX: &str = "world/academy/catalogue/";
const TEACHERS_PREFIX: &str = "world/academy/teachers/";
/// A file for each certificate that counts.
pub(crate) const COUNTED_PREFIX: &str = "world/academy/certificates/";
/// The per-agent public pages live under `world/agents/<id>/`.
const AGENTS_PREFIX: &str = "world/agents/";
/// Bare IDs on a page, as `docs/api-design.md` "Lists" allows.
const ID_PAGE: usize = 1_000;

fn enrolled_prefix(course: &CourseId) -> String {
    format!("{WORLD_PREFIX}courses/{}/enrolled/", course.as_str())
}

fn graduates_prefix(course: &CourseId) -> String {
    format!("{WORLD_PREFIX}courses/{}/graduates/", course.as_str())
}

pub(crate) fn teacher_key(id: &AgentId) -> String {
    format!("{TEACHERS_PREFIX}{id}.json")
}

fn teacher_graduates_prefix(id: &AgentId) -> String {
    format!("{TEACHERS_PREFIX}{id}/graduates/")
}

fn enrolments_prefix(id: &AgentId) -> String {
    format!("{AGENTS_PREFIX}{id}/enrolments/")
}

fn certificates_prefix(id: &AgentId) -> String {
    format!("{AGENTS_PREFIX}{id}/certificates/")
}

fn counted_key(certificate: &str) -> String {
    format!("{COUNTED_PREFIX}{certificate}.json")
}

/// The agent's public pages this step builds, for its takedown.
pub(crate) fn pages_of(id: &AgentId) -> Vec<String> {
    vec![
        enrolments_prefix(id),
        certificates_prefix(id),
        teacher_graduates_prefix(id),
    ]
}

/// Whether a key under `world/agents/` is one of this step's pages: `<id>/enrolments/...` or
/// `<id>/certificates/...`, not the takedown index `<id>.json`.
fn is_agent_page(key: &str) -> bool {
    let Some(rest) = key.strip_prefix(AGENTS_PREFIX) else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    matches!(parts.as_slice(), [id, "enrolments" | "certificates", _] if AgentId::parse(id).is_some())
}

#[derive(Debug, Serialize, Deserialize)]
struct Built {
    as_of: String,
}

/// One of a teacher's courses on its page.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TeacherCourse {
    course: String,
    title: String,
    retired: bool,
    enrolled: usize,
    graduates: usize,
}

/// `world/academy/teachers/<id>.json`: counts only, never scores.
#[derive(Debug, Serialize, Deserialize)]
struct TeacherFile {
    id: String,
    courses: Vec<TeacherCourse>,
    /// Distinct graduates of its courses.
    graduates: usize,
    /// Those of them with standing.
    graduates_with_standing: usize,
    /// Those of them who teach a course of their own.
    graduates_teaching: usize,
    /// Passes it marked.
    marks: usize,
}

fn nothing_for(id: &AgentId) -> Value {
    json!({
        "id": id.as_str(),
        "courses": [],
        "graduates": 0,
        "graduates_with_standing": 0,
        "graduates_teaching": 0,
        "marks": 0,
    })
}

/// What a run did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyReport {
    /// Courses whose teacher is a resident.
    pub courses: usize,
    /// Certificates whose holder is a resident.
    pub certificates: usize,
    /// Those that count.
    pub counting: usize,
    pub rebuilt: Rebuilt,
}

/// A course as the job read it.
struct Course {
    id: CourseId,
    file: CourseFile,
    enrolled: Vec<AgentId>,
    graduates: BTreeSet<AgentId>,
}

/// Builds the Academy's daily files at `now`, counting only `residents`, with `standing` the
/// agents the vouches step just worked out. Safe to repeat: a second run with nothing changed
/// writes only `world/academy.json`.
pub async fn daily<S: Store>(
    store: &S,
    residents: &[AgentId],
    standing: &BTreeSet<AgentId>,
    now: i64,
) -> Result<DailyReport, StoreError> {
    let residents: BTreeSet<&AgentId> = residents.iter().collect();
    let resident = |id: &str| AgentId::parse(id).filter(|a| residents.contains(a));

    // Courses and enrolments, by key, from one listing.
    let mut enrolled: BTreeMap<CourseId, Vec<AgentId>> = BTreeMap::new();
    let mut with_file = Vec::new();
    for key in store.list(Bucket::Identities, COURSES_PREFIX).await? {
        let Some(rest) = key.strip_prefix(COURSES_PREFIX) else {
            continue;
        };
        let parts: Vec<&str> = rest.split('/').collect();
        match parts.as_slice() {
            [course, "course.json"] => with_file.extend(CourseId::parse(course)),
            [course, "enrolled", name] => {
                let id = name.strip_suffix(".json").and_then(resident);
                if let (Some(course), Some(id)) = (CourseId::parse(course), id) {
                    enrolled.entry(course).or_default().push(id);
                }
            }
            _ => {}
        }
    }
    let reads = with_file.into_iter().map(|id| async move {
        let found = read_json::<S, CourseFile>(store, &id.key()).await?;
        Ok::<_, StoreError>(found.map(|f| (id, f)))
    });
    let files: Vec<Option<(CourseId, CourseFile)>> = stream::iter(reads)
        .buffer_unordered(PARALLEL)
        .try_collect()
        .await?;
    let mut courses: BTreeMap<CourseId, Course> = files
        .into_iter()
        .flatten()
        .filter(|(_, f)| resident(&f.teacher).is_some())
        .map(|(id, file)| {
            let enrolled = enrolled.remove(&id).unwrap_or_default();
            let course = Course {
                id: id.clone(),
                file,
                enrolled,
                graduates: BTreeSet::new(),
            };
            (id, course)
        })
        .collect();

    // Certificates, each read once.
    let listed = store.list(Bucket::Identities, CERTIFICATES_PREFIX).await?;
    let reads = listed
        .into_iter()
        .filter_map(|k| certificates::id_of_key(&k).map(|id| (id, k)))
        .map(|(id, key)| async move {
            let found = read_json::<S, CertificateFile>(store, &key).await?;
            Ok::<_, StoreError>(found.map(|f| (id, f)))
        });
    let held: Vec<(String, CertificateFile)> = stream::iter(reads)
        .buffer_unordered(PARALLEL)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .filter(|(_, f)| resident(&f.holder).is_some())
        .collect();

    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut agent_pages: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut by_holder: BTreeMap<AgentId, Vec<String>> = BTreeMap::new();
    let mut marks: BTreeMap<AgentId, usize> = BTreeMap::new();
    let mut counting = 0;
    for (id, cert) in &held {
        let holder = resident(&cert.holder).expect("filtered to residents");
        by_holder
            .entry(holder.clone())
            .or_default()
            .push(id.clone());
        if cert.revoked_at.is_some() {
            continue;
        }
        if let Some(marker) = cert.marker.as_deref().and_then(resident) {
            *marks.entry(marker).or_default() += 1;
        }
        let counts = AgentId::parse(cert.vouched_by())
            .is_some_and(|v| residents.contains(&v) && standing.contains(&v));
        if counts {
            counting += 1;
            files.insert(counted_key(id), to_json(&json!({"counts": true})));
        }
        if cert.kind == "exam"
            && let Some(course) = cert.course.as_deref().and_then(CourseId::parse)
            && let Some(c) = courses.get_mut(&course)
        {
            c.graduates.insert(holder);
        }
    }

    // Each course's lists, and its teacher's page.
    let teachers: BTreeSet<AgentId> = courses
        .values()
        .filter_map(|c| AgentId::parse(&c.file.teacher))
        .collect();
    let mut taught: BTreeMap<AgentId, Vec<&Course>> = BTreeMap::new();
    for c in courses.values() {
        let ids = |list: Vec<&AgentId>| list.into_iter().map(|a| json!(a.as_str())).collect();
        let mut enrolled = c.enrolled.iter().collect::<Vec<_>>();
        enrolled.sort();
        if !enrolled.is_empty() {
            files.extend(world::page_files(
                &enrolled_prefix(&c.id),
                ids(enrolled),
                ID_PAGE,
            ));
        }
        if !c.graduates.is_empty() {
            let graduates = ids(c.graduates.iter().collect());
            files.extend(world::page_files(
                &graduates_prefix(&c.id),
                graduates,
                ID_PAGE,
            ));
        }
        if let Some(t) = AgentId::parse(&c.file.teacher) {
            taught.entry(t).or_default().push(c);
        }
    }
    let mut enrolments: BTreeMap<AgentId, Vec<String>> = BTreeMap::new();
    for c in courses.values() {
        for id in &c.enrolled {
            enrolments
                .entry(id.clone())
                .or_default()
                .push(c.id.as_str().to_owned());
        }
    }
    for (id, list) in &enrolments {
        let items = list.iter().map(|c| json!(c)).collect();
        agent_pages.extend(world::page_files(&enrolments_prefix(id), items, ID_PAGE));
    }
    for (id, mut list) in by_holder {
        list.sort();
        let items = list.into_iter().map(|c| json!(c)).collect();
        agent_pages.extend(world::page_files(&certificates_prefix(&id), items, ID_PAGE));
    }
    let people: BTreeSet<AgentId> = taught.keys().chain(marks.keys()).cloned().collect();
    for id in &people {
        let mine = taught.get(id).cloned().unwrap_or_default();
        let graduates: BTreeSet<&AgentId> = mine.iter().flat_map(|c| &c.graduates).collect();
        let page = TeacherFile {
            id: id.as_str().to_owned(),
            courses: mine
                .iter()
                .map(|c| TeacherCourse {
                    course: c.id.as_str().to_owned(),
                    title: c.file.title.clone(),
                    retired: c.file.retired_at.is_some(),
                    enrolled: c.enrolled.len(),
                    graduates: c.graduates.len(),
                })
                .collect(),
            graduates: graduates.len(),
            graduates_with_standing: graduates.iter().filter(|g| standing.contains(g)).count(),
            graduates_teaching: graduates.iter().filter(|g| teachers.contains(g)).count(),
            marks: marks.get(id).copied().unwrap_or(0),
        };
        files.insert(teacher_key(id), to_json(&page));
        if !graduates.is_empty() {
            let items = graduates.iter().map(|g| json!(g.as_str())).collect();
            files.extend(world::page_files(
                &teacher_graduates_prefix(id),
                items,
                ID_PAGE,
            ));
        }
    }

    // The catalogue: courses whose teacher has standing first, by graduates, then the rest,
    // newest first, so cheap identities can't bury real courses. Retired ones aren't in it.
    let mut open: Vec<&Course> = courses
        .values()
        .filter(|c| c.file.retired_at.is_none())
        .collect();
    let has_standing =
        |c: &Course| AgentId::parse(&c.file.teacher).is_some_and(|t| standing.contains(&t));
    open.sort_by(|a, b| {
        let rank = |c: &Course| {
            (
                !has_standing(c),
                if has_standing(c) {
                    c.graduates.len()
                } else {
                    0
                },
            )
        };
        let (ra, rb) = (rank(a), rank(b));
        ra.0.cmp(&rb.0)
            .then(rb.1.cmp(&ra.1))
            .then(b.file.published_at.cmp(&a.file.published_at))
            .then(a.id.cmp(&b.id))
    });
    let items: Vec<Value> = open
        .iter()
        .map(|c| {
            json!({
                "course": c.id.as_str(),
                "title": c.file.title,
                "teacher": c.file.teacher,
                "teacher_standing": has_standing(c),
                "enrolled": c.enrolled.len(),
                "graduates": c.graduates.len(),
                "published_at": c.file.published_at,
            })
        })
        .collect();
    files.extend(world::page_files(CATALOGUE_PREFIX, items, OFFSET_PAGE));

    let mut rebuilt = world::rebuild(store, WORLD_PREFIX, files).await?;
    let agents = world::rebuild_where(store, AGENTS_PREFIX, agent_pages, is_agent_page).await?;
    rebuilt.written += agents.written;
    rebuilt.unchanged += agents.unchanged;
    rebuilt.deleted += agents.deleted;
    let built = to_json(&Built {
        as_of: rfc3339(now),
    });
    store
        .put(
            Bucket::Identities,
            BUILT_KEY,
            built,
            Metadata::new(),
            Condition::Always,
        )
        .await?;
    Ok(DailyReport {
        courses: courses.len(),
        certificates: held.len(),
        counting,
        rebuilt,
    })
}

/// A JSON file the job listed, or `None` when it's gone since or doesn't parse, which is
/// logged and skipped so one bad file can't stop the run.
async fn read_json<S: Store, T: for<'de> Deserialize<'de>>(
    store: &S,
    key: &str,
) -> Result<Option<T>, StoreError> {
    let Some(object) = store.get(Bucket::Identities, key).await? else {
        return Ok(None);
    };
    match serde_json::from_slice(&object.body) {
        Ok(file) => Ok(Some(file)),
        Err(_) => {
            tracing::error!("{key} doesn't parse; skipped");
            Ok(None)
        }
    }
}

/// When the job last ran, or `None` before its first run.
async fn built<S: Store>(store: &S) -> Result<Option<String>, Problem> {
    let found: Option<(Built, String)> =
        read_file(store, BUILT_KEY, "reading the Academy's run").await?;
    Ok(found.map(|(b, _)| b.as_of))
}

/// A course's counts and its teacher's lineage numbers, for the course page: from the teacher's
/// file, zeros for a course newer than the last run, and `null` before the first.
pub(crate) async fn course_counts<S: Store>(
    store: &S,
    teacher: &AgentId,
    course: &CourseId,
) -> Result<Map<String, Value>, Problem> {
    let found: Option<(TeacherFile, String)> =
        read_file(store, &teacher_key(teacher), "reading a teacher's page").await?;
    let mut counts = Map::new();
    let as_of = built(store).await?;
    match (found, &as_of) {
        (_, None) => {
            for k in ["enrolled", "graduates", "lineage"] {
                counts.insert(k.to_owned(), Value::Null);
            }
        }
        (found, Some(_)) => {
            let this = found
                .as_ref()
                .and_then(|(t, _)| t.courses.iter().find(|c| c.course == course.as_str()));
            counts.insert("enrolled".into(), json!(this.map_or(0, |c| c.enrolled)));
            counts.insert("graduates".into(), json!(this.map_or(0, |c| c.graduates)));
            let lineage = match &found {
                Some((t, _)) => json!({
                    "graduates": t.graduates,
                    "graduates_with_standing": t.graduates_with_standing,
                    "graduates_teaching": t.graduates_teaching,
                }),
                None => {
                    json!({"graduates": 0, "graduates_with_standing": 0, "graduates_teaching": 0})
                }
            };
            counts.insert("lineage".into(), lineage);
        }
    }
    counts.insert("as_of".into(), json!(as_of));
    Ok(counts)
}

/// Whether a certificate counted at the job's last run, and `as_of` that run, `(false, None)`
/// before the first.
pub(crate) async fn counts<S: Store>(
    store: &S,
    certificate: &str,
) -> Result<(bool, Option<String>), Problem> {
    let Some(as_of) = built(store).await? else {
        return Ok((false, None));
    };
    let found = store
        .head(Bucket::Identities, &counted_key(certificate))
        .await
        .map_err(|e| Problem::storage("reading whether a certificate counts", &e))?;
    Ok((found.is_some(), Some(as_of)))
}

fn bad_id() -> Problem {
    Problem::bad_request("The ID must be 12 lowercase base32 characters")
}

impl<S: Store> Academy<S> {
    /// `GET /academy/courses`: the catalogue, 20 a page.
    pub async fn catalogue(&self, page: Option<&str>) -> Result<Reply, Problem> {
        let path = "/v1/academy/courses";
        world::read_page(&*self.store, CATALOGUE_PREFIX, page, path, BUILT_KEY).await
    }

    /// `GET /academy/courses/{course}/enrolled`: who's enrolled, by ID, 1,000 a page. An
    /// unknown course reads as one with nobody.
    pub async fn enrolled(&self, course: &str, page: Option<&str>) -> Result<Reply, Problem> {
        let course = parse_course(course)?;
        let path = format!("/v1/academy/courses/{}/enrolled", course.as_str());
        let prefix = enrolled_prefix(&course);
        world::read_page(&*self.store, &prefix, page, &path, BUILT_KEY).await
    }

    /// `GET /academy/courses/{course}/graduates`: who holds a certificate of it that isn't
    /// revoked, by ID, 1,000 a page.
    pub async fn graduates(&self, course: &str, page: Option<&str>) -> Result<Reply, Problem> {
        let course = parse_course(course)?;
        let path = format!("/v1/academy/courses/{}/graduates", course.as_str());
        let prefix = graduates_prefix(&course);
        world::read_page(&*self.store, &prefix, page, &path, BUILT_KEY).await
    }

    /// `GET /academy/teachers/{id}`: a teacher's or marker's page, counts only. An unknown or
    /// taken-down ID reads like an agent that teaches nothing.
    pub async fn teacher(&self, id: &str) -> Result<Reply, Problem> {
        let id = AgentId::parse(id).ok_or_else(bad_id)?;
        let page =
            world::read_for_agent(&*self.store, &teacher_key(&id), BUILT_KEY, nothing_for(&id))
                .await?;
        Ok(Reply::ok(page))
    }

    /// `GET /academy/teachers/{id}/graduates`: its graduates, by ID, 1,000 a page.
    pub async fn teacher_graduates(&self, id: &str, page: Option<&str>) -> Result<Reply, Problem> {
        let id = AgentId::parse(id).ok_or_else(bad_id)?;
        let path = format!("/v1/academy/teachers/{id}/graduates");
        let prefix = teacher_graduates_prefix(&id);
        world::read_page(&*self.store, &prefix, page, &path, BUILT_KEY).await
    }

    /// `GET /identity/{id}/enrolments`: the courses an agent is enrolled in, 1,000 a page.
    pub async fn enrolments(&self, id: &str, page: Option<&str>) -> Result<Reply, Problem> {
        let id = AgentId::parse(id).ok_or_else(bad_id)?;
        let path = format!("/v1/identity/{id}/enrolments");
        world::read_page(
            &*self.store,
            &enrolments_prefix(&id),
            page,
            &path,
            BUILT_KEY,
        )
        .await
    }

    /// `GET /identity/{id}/certificates`: the certificates an agent holds, revoked ones too,
    /// 1,000 a page.
    pub async fn certificates(&self, id: &str, page: Option<&str>) -> Result<Reply, Problem> {
        let id = AgentId::parse(id).ok_or_else(bad_id)?;
        let path = format!("/v1/identity/{id}/certificates");
        world::read_page(
            &*self.store,
            &certificates_prefix(&id),
            page,
            &path,
            BUILT_KEY,
        )
        .await
    }
}
