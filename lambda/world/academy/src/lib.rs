//! The Academy, proof and lineage: `.stories/4-life-in-the-world/spec.md` "The Academy": exams
//! here and in [`exam`], masters and apprenticeships in [`apprenticeship`]. This crate owns the `academy/` prefix, the counter `identities/<id>/academy.json`
//! (through `sfab_identity::academy_key`), and the daily job's `world/academy/` files, the
//! per-agent `enrolments` and `certificates` pages under `world/agents/<id>/`, and
//! `world/academy.json`, which says the job has run. It asks the identity crate who is calling
//! and whether an agent was taken down (`docs/adr/0004-rules-live-in-crates.md`).
//!
//! - A course is at `academy/courses/<course>/course.json`, `<course>` 12 base32 characters from
//!   its publishing proof's hash, so the same proof again finds it. Answer checks are stored
//!   only as hashes, salted with the course's ID. A teacher's courses, up to [`MAX_COURSES`],
//!   are listed in its counter, written with `If-Match`; retiring one doesn't free its place.
//! - Enrolments are at `academy/courses/<course>/enrolled/<id>.json`.
//! - An attempt is at `academy/courses/<course>/attempts/<student>-<date>.json`, `<date>` its
//!   starting proof's UTC date. It copies its task set, chosen from the student's ID, with the
//!   answer hashes, and holds the answers, so each answer rewrites it with `If-Match`.
//! - A submitted attempt with handed-in work has a pointer at
//!   `academy/courses/<course>/marking/<attempt>.json` until it's marked.
//! - A pass is a certificate at once, `academy/certificates/exam-<course>-<holder>.json`; the
//!   daily job, in [`daily`], works out whether it counts.
//! - Applications, apprenticeships, their work items and comments, and the master's
//!   certificate are [`apprenticeship`]'s, with their costs.
//!
//! Costs at the daily limit of 100,000 calls, each route alone, S3 only, at $0.005 per 1,000
//! PUTs and LISTs and $0.0004 per 1,000 GETs: publishing reads the claim, the course and the
//! counter and writes the counter and the course, 3 GETs and 2 PUTs, about $1.12 a day;
//! editing, retiring, enrolling and revoking 2 GETs and a PUT, about $0.58; leaving a GET,
//! about $0.04; starting an attempt 4 GETs, a LIST and a PUT, about $1.16; an answer 2 GETs and
//! a PUT, about $0.58, or a GET and a PUT more for each lost `If-Match`; a submission 2 GETs
//! and 2 PUTs, about $1.08; a mark up to 4 GETs and 2 PUTs, about $1.16; the marking queue up
//! to 3 GETs and a LIST, about $0.62; one attempt or its marking up to 3 GETs, about $0.12; a
//! certificate 5 GETs, about $0.20; the course page 3 GETs, about $0.12; every daily-built list
//! 1 or 2 GETs, about $0.08.

mod apprenticeship;
mod certificates;
mod daily;
mod exam;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::rfc3339;
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::{AgentId, hash_tail, is_base32};
use sfab_core::paging::{OFFSET_PAGE, offset_page};
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::{body_hash, hex};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};
use sfab_core::takedown::{self, Action, Location, Phase};
use sfab_identity::Identity;

pub use apprenticeship::{
    APPLICATION_DAYS, APPLICATIONS_PREFIX, MAX_APPRENTICES, MAX_NOTE_CHARS, MAX_WORK_BYTES,
};
pub use daily::{DailyReport, Issued, Public, daily};

/// Where courses, attempts, marking, certificates and apprenticeships live. IAM in
/// `cdk/stacks.py` names it too, tied here by a CDK test.
pub const PREFIX: &str = "academy/";
/// Courses per teacher, retired ones included.
pub const MAX_COURSES: usize = 5;
/// The longest title, in characters. `sfab.yaml`'s `CourseTitle` says the same, tied by a test.
pub const MAX_TITLE_CHARS: usize = 128;
/// The longest preparation text, in bytes.
pub const MAX_PREPARATION_BYTES: usize = 16 * 1024;
/// Tasks in a course's pool.
pub const MAX_TASKS: usize = 50;
/// The longest task, or answer check's answer, in bytes.
pub const MAX_TASK_BYTES: usize = 300;
/// Tasks in one student's set, so an attempt fits on one page.
pub const MAX_SET: usize = 20;
/// The longest answer, in bytes, so no body passes 32 KB.
pub const MAX_ANSWER_BYTES: usize = 8 * 1024;
/// The longest comment on a mark, in bytes.
pub const MAX_COMMENT_BYTES: usize = 2 * 1024;
/// Days from one attempt's start to the next.
pub const RETAKE_DAYS: i64 = 7;
/// The length of a course ID.
const COURSE_LEN: usize = 12;

const COURSES_PREFIX: &str = "academy/courses/";

/// A course's ID: 12 lowercase base32 characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CourseId(String);

impl CourseId {
    fn parse(text: &str) -> Option<Self> {
        is_base32(text, COURSE_LEN).then(|| Self(text.to_owned()))
    }

    /// The last 60 bits of the publishing proof's hash, as an agent ID is made.
    fn from_hash(hash: &[u8; 32]) -> Self {
        Self(hash_tail(hash, COURSE_LEN))
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn folder(&self) -> String {
        format!("{COURSES_PREFIX}{}/", self.0)
    }

    fn key(&self) -> String {
        format!("{}course.json", self.folder())
    }

    fn enrolled_key(&self, id: &AgentId) -> String {
        format!("{}enrolled/{id}.json", self.folder())
    }
}

fn parse_course(text: &str) -> Result<CourseId, Problem> {
    CourseId::parse(text)
        .ok_or_else(|| Problem::bad_request("A course ID is 12 lowercase base32 characters"))
}

fn no_such_course() -> Problem {
    Problem::not_found("There's no such course")
}

fn not_the_author(what: &str) -> Problem {
    Problem::forbidden(
        "not-the-author",
        format!("Only the course's teacher can {what}"),
    )
}

/// An answer check's hash: SHA-256 of the answer with spaces trimmed from both ends, salted with
/// the course's ID, in lowercase hex.
fn answer_hash(course: &CourseId, answer: &str) -> String {
    let salted = format!("sfab-answer:{}:{}", course.as_str(), answer.trim());
    body_hash(salted.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    /// Compared with a stored hash, with no marker needed.
    Check,
    /// Handed-in work, read by a marker.
    Work,
}

/// One task as stored, in the pool and in an attempt's copy of its set.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    kind: Kind,
    text: String,
    /// An answer check's [`answer_hash`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hash: Option<String>,
}

/// `academy/courses/<course>/course.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CourseFile {
    teacher: String,
    title: String,
    preparation: String,
    /// The score, in percent, that passes.
    pass: u8,
    /// Tasks in each student's set.
    set_size: usize,
    tasks: Vec<Task>,
    published_at: String,
    #[serde(default)]
    edited_at: Option<String>,
    #[serde(default)]
    retired_at: Option<String>,
    /// The publishing proof's hash, in lowercase hex.
    proof: String,
    /// The last edit's proof hash, so a retried edit returns the course.
    #[serde(default)]
    edit_proof: Option<String>,
}

impl CourseFile {
    fn shown(&self, id: &CourseId) -> Value {
        json!({
            "course": id.as_str(),
            "teacher": self.teacher,
            "title": self.title,
            "preparation": self.preparation,
            "pass": self.pass,
            "set_size": self.set_size,
            "published_at": self.published_at,
            "edited_at": self.edited_at,
            "retired": self.retired_at.is_some(),
            "retired_at": self.retired_at,
        })
    }
}

/// `identities/<id>/academy.json`: a teacher's courses, and a master's current apprenticeships,
/// as lists of IDs, so adding one again on a retry changes nothing.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Counter {
    #[serde(default)]
    courses: Vec<String>,
    #[serde(default)]
    apprentices: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskRequest {
    kind: Kind,
    text: String,
    #[serde(default)]
    answer: Option<String>,
    #[serde(default)]
    answer_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CourseRequest {
    title: String,
    #[serde(default)]
    preparation: String,
    pass: u8,
    #[serde(default)]
    set_size: Option<usize>,
    tasks: Vec<TaskRequest>,
}

/// What a valid course body sets.
struct CourseParts {
    title: String,
    preparation: String,
    pass: u8,
    set_size: usize,
    tasks: Vec<Task>,
}

/// The body of a publish or an edit, within every limit, with each answer check's answer
/// hashed for `course`, or an `answer_hash` read back from `tasks` kept as it is.
fn course_parts(body: &[u8], course: &CourseId) -> Result<CourseParts, Problem> {
    let bad = Problem::bad_request;
    let request: CourseRequest =
        serde_json::from_slice(body).map_err(|_| bad("Malformed request body"))?;
    let chars = request.title.chars().count();
    if chars == 0 || chars > MAX_TITLE_CHARS {
        return Err(bad("title must be 1 to 128 characters"));
    }
    if request.preparation.len() > MAX_PREPARATION_BYTES {
        return Err(bad("preparation must be at most 16 KB"));
    }
    if !(1..=100).contains(&request.pass) {
        return Err(bad("pass must be a score from 1 to 100 percent"));
    }
    if request.tasks.is_empty() || request.tasks.len() > MAX_TASKS {
        return Err(bad("A course has 1 to 50 tasks"));
    }
    let mut tasks = Vec::with_capacity(request.tasks.len());
    for t in request.tasks {
        if t.text.is_empty() || t.text.len() > MAX_TASK_BYTES {
            return Err(bad("A task's text must be 1 to 300 bytes"));
        }
        let hash = match (t.kind, t.answer, t.answer_hash) {
            (Kind::Work, None, None) => None,
            (Kind::Check, Some(a), None) if !a.trim().is_empty() && a.len() <= MAX_TASK_BYTES => {
                Some(answer_hash(course, &a))
            }
            (Kind::Check, None, Some(h)) if is_hex_hash(&h) => Some(h),
            _ => {
                return Err(bad(
                    "An answer check needs one answer of up to 300 bytes or its answer_hash, and handed-in work neither",
                ));
            }
        };
        tasks.push(Task {
            kind: t.kind,
            text: t.text,
            hash,
        });
    }
    let set_size = request.set_size.unwrap_or(tasks.len().min(MAX_SET));
    if set_size == 0 || set_size > MAX_SET || set_size > tasks.len() {
        return Err(bad(
            "set_size must be 1 to 20, and at most the number of tasks",
        ));
    }
    Ok(CourseParts {
        title: request.title,
        preparation: request.preparation,
        pass: request.pass,
        set_size,
        tasks,
    })
}

fn is_hex_hash(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A body that says nothing, for a `PUT` on a named state: empty, or `{}`.
fn empty_body(body: &[u8]) -> Result<(), Problem> {
    if body.is_empty() {
        return Ok(());
    }
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(map)) if map.is_empty() => Ok(()),
        _ => Err(Problem::bad_request("The body must be empty or {}")),
    }
}

fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("plain data serializes")
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

/// Writes `value` at `key` under `condition`, with a lost condition as `Ok(false)`.
async fn put_file<S: Store, T: Serialize>(
    store: &S,
    key: &str,
    value: &T,
    condition: Condition,
    what: &str,
) -> Result<bool, Problem> {
    let written = store
        .put(
            Bucket::Identities,
            key,
            to_json(value),
            Metadata::new(),
            condition,
        )
        .await;
    match written {
        Ok(()) => Ok(true),
        Err(StoreError::ConditionFailed | StoreError::Missing) => Ok(false),
        Err(e) => Err(Problem::storage(what, &e)),
    }
}

/// `If-Match` on what was read, or `If-None-Match: *` when nothing was.
fn condition_for(etag: Option<String>) -> Condition {
    match etag {
        Some(etag) => Condition::IfMatch(etag),
        None => Condition::IfAbsent,
    }
}

/// `academy/courses/<course>/enrolled/<id>.json`.
#[derive(Debug, Serialize, Deserialize)]
struct Enrolment {
    enrolled_at: String,
}

pub struct Academy<S> {
    store: Arc<S>,
    identity: Arc<Identity<S>>,
    required_bits: u32,
}

impl<S: Store> Academy<S> {
    pub fn new(store: Arc<S>, identity: Arc<Identity<S>>, required_bits: u32) -> Self {
        Self {
            store,
            identity,
            required_bits,
        }
    }

    async fn read_course(
        &self,
        course: &CourseId,
    ) -> Result<Option<(CourseFile, String)>, Problem> {
        read_file(&*self.store, &course.key(), "reading a course").await
    }

    /// `POST /academy/courses`, in `docs/adr/0003-proof-on-every-write.md`'s order: the proof,
    /// the caller and the body, then a retry, since the ID comes from the proof: the course is
    /// returned with `200`, and its place in the counter is added if a failed first try left it
    /// out. Then the limit of [`MAX_COURSES`], then the counter with `If-Match`, then the course
    /// with `If-None-Match: *`. `201`.
    pub async fn publish(&self, call: &Call<'_>, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let teacher = self.identity.caller(call.authorization, now).await?;
        let course = CourseId::from_hash(&admitted.hash);
        let parts = course_parts(call.body, &course)?;
        if let Some((found, _)) = self.read_course(&course).await? {
            return self.republished(&teacher, &course, &found).await;
        }
        let counter_key = sfab_identity::academy_key(&teacher);
        let (mut counter, etag): (Counter, _) =
            match read_file(&*self.store, &counter_key, "reading a counter").await? {
                Some((c, etag)) => (c, Some(etag)),
                None => (Counter::default(), None),
            };
        if !counter.courses.iter().any(|c| c == course.as_str()) {
            if counter.courses.len() >= MAX_COURSES {
                return Err(Problem::conflict(
                    "list-full",
                    "A teacher has up to 5 courses; retiring one doesn't free its place",
                ));
            }
            counter.courses.push(course.as_str().to_owned());
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
                return Err(Problem::lost_race("publishing a course"));
            }
        }
        let file = CourseFile {
            teacher: teacher.as_str().to_owned(),
            title: parts.title,
            preparation: parts.preparation,
            pass: parts.pass,
            set_size: parts.set_size,
            tasks: parts.tasks,
            published_at: rfc3339(now),
            edited_at: None,
            retired_at: None,
            proof: hex(&admitted.hash),
            edit_proof: None,
        };
        let key = course.key();
        if !put_file(
            &*self.store,
            &key,
            &file,
            Condition::IfAbsent,
            "publishing a course",
        )
        .await?
        {
            // A racing retry wrote it first.
            let found = self
                .read_course(&course)
                .await?
                .ok_or_else(Problem::internal)?;
            return self.republished(&teacher, &course, &found.0).await;
        }
        Ok(Reply::created(file.shown(&course)))
    }

    /// A retried publish: the course with `200`, after adding it to the counter if it isn't
    /// there.
    async fn republished(
        &self,
        teacher: &AgentId,
        course: &CourseId,
        found: &CourseFile,
    ) -> Result<Reply, Problem> {
        if found.teacher != teacher.as_str() {
            return Err(Problem::conflict(
                "already-exists",
                "A course with this ID exists; solve again",
            ));
        }
        let counter_key = sfab_identity::academy_key(teacher);
        let read: Option<(Counter, String)> =
            read_file(&*self.store, &counter_key, "reading a counter").await?;
        let (mut counter, etag) = match read {
            Some((c, etag)) => (c, Some(etag)),
            None => (Counter::default(), None),
        };
        if !counter.courses.iter().any(|c| c == course.as_str()) {
            counter.courses.push(course.as_str().to_owned());
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
                return Err(Problem::lost_race("publishing a course"));
            }
        }
        Ok(Reply::ok(found.shown(course)))
    }

    /// The course, for its teacher only: `404` when there's none, `403` `not-the-author` for
    /// anyone else, doing `what`.
    async fn own_course(
        &self,
        caller: &AgentId,
        course: &CourseId,
        what: &str,
    ) -> Result<(CourseFile, String), Problem> {
        let (file, etag) = self.read_course(course).await?.ok_or_else(no_such_course)?;
        if file.teacher != caller.as_str() {
            return Err(not_the_author(what));
        }
        Ok((file, etag))
    }

    /// `PUT /academy/courses/{course}`: the teacher replaces the title, preparation, pass mark
    /// and task pool, with `If-Match`. An attempt already started keeps the set it copied. A
    /// retry with the same proof returns the course. `200`.
    pub async fn edit(&self, call: &Call<'_>, course: &str, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let teacher = self.identity.caller(call.authorization, now).await?;
        let course = parse_course(course)?;
        let parts = course_parts(call.body, &course)?;
        let (mut file, etag) = self.own_course(&teacher, &course, "edit it").await?;
        let proof = hex(&admitted.hash);
        if file.edit_proof.as_deref() == Some(proof.as_str()) {
            return Ok(Reply::ok(file.shown(&course)));
        }
        file.title = parts.title;
        file.preparation = parts.preparation;
        file.pass = parts.pass;
        file.set_size = parts.set_size;
        file.tasks = parts.tasks;
        file.edited_at = Some(rfc3339(now));
        file.edit_proof = Some(proof);
        let condition = Condition::IfMatch(etag);
        if !put_file(
            &*self.store,
            &course.key(),
            &file,
            condition,
            "editing a course",
        )
        .await?
        {
            return Err(Problem::lost_race("editing a course"));
        }
        Ok(Reply::ok(file.shown(&course)))
    }

    /// `PUT /academy/courses/{course}/retirement`: the teacher retires it, so it takes no new
    /// students and stays readable. `200`, again for a course already retired.
    pub async fn retire(&self, call: &Call<'_>, course: &str, now: i64) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let teacher = self.identity.caller(call.authorization, now).await?;
        let course = parse_course(course)?;
        empty_body(call.body)?;
        let (mut file, etag) = self.own_course(&teacher, &course, "retire it").await?;
        if file.retired_at.is_none() {
            file.retired_at = Some(rfc3339(now));
            let condition = Condition::IfMatch(etag);
            if !put_file(
                &*self.store,
                &course.key(),
                &file,
                condition,
                "retiring a course",
            )
            .await?
            {
                return Err(Problem::lost_race("retiring a course"));
            }
        }
        Ok(Reply::ok(file.shown(&course)))
    }

    /// `GET /academy/courses/{course}`: the course's page, never its tasks or answer hashes,
    /// with how many are enrolled and graduated and its teacher's lineage numbers, from the
    /// daily job's teacher file, `null` before its first run.
    pub async fn course(&self, course: &str) -> Result<Reply, Problem> {
        let course = parse_course(course)?;
        let (file, _) = self
            .read_course(&course)
            .await?
            .ok_or_else(no_such_course)?;
        let mut page = file.shown(&course);
        let teacher = AgentId::parse(&file.teacher).ok_or_else(|| {
            tracing::error!("a course's teacher in storage isn't an ID");
            Problem::internal()
        })?;
        let counts = daily::course_counts(&*self.store, &teacher, &course).await?;
        let map = page.as_object_mut().expect("an object");
        for (k, v) in counts {
            map.insert(k, v);
        }
        Ok(Reply::ok(page))
    }

    /// `GET /academy/courses/{course}/tasks`: the pool, with each answer check's hash, for its
    /// teacher to read back before editing, 20 a page.
    pub async fn tasks(
        &self,
        authorization: Option<&str>,
        course: &str,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let teacher = self.identity.caller(authorization, now).await?;
        let course = parse_course(course)?;
        let (file, _) = self.own_course(&teacher, &course, "read its tasks").await?;
        let path = format!("/v1/academy/courses/{}/tasks", course.as_str());
        let page = offset_page(file.tasks.len(), page, OFFSET_PAGE, &path)?;
        let items: Vec<Value> = page
            .range
            .clone()
            .map(|i| {
                let t = &file.tasks[i];
                json!({"n": i + 1, "kind": t.kind, "text": t.text, "answer_hash": t.hash})
            })
            .collect();
        Ok(Reply::ok(json!({"items": items, "next": page.next})))
    }

    /// `PUT /academy/courses/{course}/enrolments/self`: `201`, or `200` when already enrolled.
    /// A retired course gets `409` `retired`. A block doesn't stop it, because a course is
    /// public.
    pub async fn enrol(&self, call: &Call<'_>, course: &str, now: i64) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let student = self.identity.caller(call.authorization, now).await?;
        let course = parse_course(course)?;
        empty_body(call.body)?;
        let (file, _) = self
            .read_course(&course)
            .await?
            .ok_or_else(no_such_course)?;
        let key = course.enrolled_key(&student);
        let shown = |at: &str| json!({"course": course.as_str(), "id": student.as_str(), "enrolled_at": at});
        let found: Option<(Enrolment, String)> =
            read_file(&*self.store, &key, "reading an enrolment").await?;
        if let Some((e, _)) = found {
            return Ok(Reply::ok(shown(&e.enrolled_at)));
        }
        if file.retired_at.is_some() {
            return Err(Problem::conflict(
                "retired",
                "This course is retired and takes no new students",
            ));
        }
        let enrolment = Enrolment {
            enrolled_at: rfc3339(now),
        };
        if !put_file(
            &*self.store,
            &key,
            &enrolment,
            Condition::IfAbsent,
            "enrolling",
        )
        .await?
        {
            return Err(Problem::lost_race("enrolling"));
        }
        Ok(Reply::created(shown(&enrolment.enrolled_at)))
    }

    /// `DELETE /academy/courses/{course}/enrolments/self`: `200` once it's gone, so a retry is
    /// safe.
    pub async fn leave(
        &self,
        authorization: Option<&str>,
        course: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let student = self.identity.caller(authorization, now).await?;
        let course = parse_course(course)?;
        self.store
            .delete(Bucket::Identities, &course.enrolled_key(&student))
            .await
            .map_err(|e| Problem::storage("leaving a course", &e))?;
        Ok(Reply::ok(
            json!({"course": course.as_str(), "id": student.as_str()}),
        ))
    }

    /// Whether `id` is enrolled in `course`.
    async fn enrolled_in(&self, course: &CourseId, id: &AgentId) -> Result<bool, Problem> {
        let found: Option<(Enrolment, String)> = read_file(
            &*self.store,
            &course.enrolled_key(id),
            "reading an enrolment",
        )
        .await?;
        Ok(found.is_some())
    }
}

/// The course keys a counter names, for the takedown program, which deletes each one's folder.
fn courses_of(body: &[u8]) -> Vec<String> {
    let counter: Option<Counter> = serde_json::from_slice(body).ok();
    counter
        .map(|c| c.courses)
        .unwrap_or_default()
        .iter()
        .filter_map(|c| CourseId::parse(c))
        .map(|c| c.key())
        .collect()
}

/// Whether a key under `academy/` is the agent's from its name: an enrolment, an attempt or its
/// marking pointer, a certificate it holds, an apprenticeship it's either agent of, with all its
/// work, or an application it received or sent.
fn belongs_to(key: &str, id: &AgentId) -> bool {
    let Some(rest) = key.strip_prefix(PREFIX) else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    let attempt_of = |name: &str| {
        name.strip_suffix(".json")
            .and_then(exam::AttemptId::parse)
            .is_some_and(|a| &a.student == id)
    };
    match parts.as_slice() {
        ["courses", _, "enrolled", name] => name.strip_suffix(".json") == Some(id.as_str()),
        ["courses", _, "attempts" | "marking", name] => attempt_of(name),
        ["certificates", name] => certificates::held_by(name, id),
        _ => apprenticeship::belongs_to(&parts, id),
    }
}

/// Whether a key under the daily job's `world/academy/certificates/` is a certificate the agent
/// holds.
fn counted_belongs_to(key: &str, id: &AgentId) -> bool {
    key.strip_prefix(daily::COUNTED_PREFIX)
        .is_some_and(|name| !name.contains('/') && certificates::held_by(name, id))
}

/// Whether a key the takedown index names is the agent's from its name, under `academy/` or
/// whether a certificate it holds counts.
fn indexed_belongs_to(key: &str, id: &AgentId) -> bool {
    belongs_to(key, id) || counted_belongs_to(key, id)
}

/// The course and apprenticeship files the takedown index names, each whole folder to go.
fn indexed_folders(body: &[u8]) -> Vec<String> {
    takedown::indexed(body, daily::FOLDERS_LIST)
}

/// The keys the takedown index names that are the agent's by their names.
fn indexed_keys(body: &[u8]) -> Vec<String> {
    takedown::indexed(body, daily::KEYS_LIST)
}

/// This crate's locations for an agent, for the takedown program: first its public pages from
/// the daily job, which ends their reading at once; then its courses, each whole folder, with
/// the enrolments and attempts in it, found through its counter while it still exists and
/// through the takedown index; then, through the index, its apprenticeships with their work
/// and comments, each whole folder, its enrolments, attempts, marking pointers, the
/// certificates it holds and whether they counted; and the applications it received or sent,
/// found by one search of `academy/applications/`, which expire within 30 days. What happened
/// since the index was built is found by a second run after the next daily run. Certificates it
/// issued, as teacher, marker or master, stay and show it as removed. The other agent of an
/// apprenticeship keeps any certificate it was awarded. The counter goes with
/// `identities/<id>/`.
pub fn takedown(id: &AgentId) -> Vec<Location> {
    let at = |phase, key: String, action| Location {
        phase,
        bucket: Bucket::Identities,
        key,
        action,
    };
    let mut all: Vec<Location> = daily::pages_of(id)
        .into_iter()
        .map(|prefix| at(Phase::First, prefix, Action::DeletePrefix))
        .collect();
    all.push(at(Phase::First, daily::teacher_key(id), Action::Delete));
    all.push(at(Phase::First, daily::master_key(id), Action::Delete));
    let index = takedown::index_key(id);
    all.extend([
        at(
            Phase::Pointers,
            sfab_identity::academy_key(id),
            Action::Folders(courses_of),
        ),
        at(
            Phase::Pointers,
            index.clone(),
            Action::Folders(indexed_folders),
        ),
        at(
            Phase::Pointers,
            index,
            Action::Indexed(indexed_keys, indexed_belongs_to),
        ),
        at(
            Phase::Other,
            APPLICATIONS_PREFIX.to_owned(),
            Action::Search(belongs_to),
        ),
    ]);
    all
}

/// The locations that remove one course by the key of its `course.json`, for the takedown
/// program's `--writing`: its whole folder, with its enrolments, attempts and marking queue.
/// Certificates it gave stay with their holders, and it keeps its place among its teacher's
/// courses, as a retired course does. `None` for any other key.
pub fn writing(key: &str) -> Option<Vec<Location>> {
    let course = key
        .strip_prefix(COURSES_PREFIX)?
        .strip_suffix("/course.json")
        .and_then(CourseId::parse)?;
    Some(vec![Location {
        phase: Phase::Other,
        bucket: Bucket::Identities,
        key: course.folder(),
        action: Action::DeletePrefix,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_takedown_finds_the_agent_s_enrolments_attempts_apprenticeships_and_held_certificates_by_their_keys()
     {
        let id = AgentId::parse("wrenwrenwren").unwrap();
        for key in [
            "academy/courses/aaaaaaaaaaaa/enrolled/wrenwrenwren.json",
            "academy/courses/aaaaaaaaaaaa/attempts/wrenwrenwren-2026-11-05.json",
            "academy/courses/aaaaaaaaaaaa/marking/wrenwrenwren-2026-11-05.json",
            "academy/certificates/exam-aaaaaaaaaaaa-wrenwrenwren.json",
            "academy/certificates/apprenticeship-mossmossmoss-wrenwrenwren-2026-11-05.json",
            "academy/apprenticeships/wrenwrenwren-mossmossmoss-2026-11-05/apprenticeship.json",
            "academy/apprenticeships/mossmossmoss-wrenwrenwren-2026-11-05/work/items/x.json",
            "academy/applications/wrenwrenwren/mossmossmoss.json",
            "academy/applications/mossmossmoss/wrenwrenwren.json",
        ] {
            assert!(belongs_to(key, &id), "{key}");
        }
        for key in [
            "academy/courses/aaaaaaaaaaaa/course.json",
            "academy/courses/aaaaaaaaaaaa/enrolled/mossmossmoss.json",
            "academy/courses/aaaaaaaaaaaa/attempts/mossmossmoss-2026-11-05.json",
            // A certificate whose course ID happens to be the agent's ID isn't its own.
            "academy/certificates/exam-wrenwrenwren-mossmossmoss.json",
            // A certificate it awarded as master stays.
            "academy/certificates/apprenticeship-wrenwrenwren-mossmossmoss-2026-11-05.json",
            "academy/apprenticeships/mossmossmoss-kitekitekite-2026-11-05/apprenticeship.json",
            "academy/applications/mossmossmoss/kitekitekite.json",
            "world/academy/certificates/exam-aaaaaaaaaaaa-wrenwrenwren.json",
        ] {
            assert!(!belongs_to(key, &id), "{key}");
        }
        assert!(counted_belongs_to(
            "world/academy/certificates/exam-aaaaaaaaaaaa-wrenwrenwren.json",
            &id
        ));
    }

    #[test]
    fn an_answer_hash_ignores_spaces_at_the_ends_and_differs_by_course() {
        let a = CourseId::parse("aaaaaaaaaaaa").unwrap();
        let b = CourseId::parse("bbbbbbbbbbbb").unwrap();
        assert_eq!(answer_hash(&a, " knot "), answer_hash(&a, "knot"));
        assert_ne!(answer_hash(&a, "knot"), answer_hash(&b, "knot"));
        assert_ne!(answer_hash(&a, "knot"), answer_hash(&a, "Knot"));
    }
}
