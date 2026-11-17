//! Sitting an exam and marking it: `.stories/4-life-in-the-world/spec.md` "The Academy"
//! ("Exams", "Who marks"). An attempt's file holds its copied task set, with the answer hashes,
//! and every answer, so submitting and marking read one file. Each write to it is `If-Match`, read
//! again on a lost race (`docs/aws-facts.md`, `If-Match`).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::{DAY, parse_rfc3339, rfc3339, utc_date, utc_day};
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::AgentId;
use sfab_core::paging::OFFSET_PAGE;
use sfab_core::problem::{Problem, Reply};
use sfab_core::proof::{body_hash, hex};
use sfab_core::storage::{Bucket, Condition, Store};

use crate::certificates::{self, CertificateFile};
use crate::{
    Academy, CourseFile, CourseId, Kind, MAX_ANSWER_BYTES, MAX_COMMENT_BYTES, MAX_SET, RETAKE_DAYS,
    Task, answer_hash, empty_body, no_such_course, parse_course, put_file, read_file,
};

/// Tries at rewriting an attempt on a lost `If-Match`, so two answers handed in at once are
/// both kept.
const TRIES: usize = 3;

/// An attempt's name: its student and the UTC date of its starting proof, `<student>-<date>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptId {
    pub(crate) student: AgentId,
    date: String,
}

impl AttemptId {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.len() != AgentId::LEN + 1 + 10 || b[AgentId::LEN] != b'-' {
            return None;
        }
        let student = AgentId::parse(&text[..AgentId::LEN])?;
        let date = &text[AgentId::LEN + 1..];
        day_number(date)?;
        Some(Self {
            student,
            date: date.to_owned(),
        })
    }

    fn text(&self) -> String {
        format!("{}-{}", self.student, self.date)
    }

    fn key(&self, course: &CourseId) -> String {
        format!("{}attempts/{}.json", course.folder(), self.text())
    }

    fn pointer_key(&self, course: &CourseId) -> String {
        format!("{}marking/{}.json", course.folder(), self.text())
    }
}

/// The UTC day number of a date `YYYY-MM-DD`, or `None` for any other text.
fn day_number(day: &str) -> Option<i64> {
    parse_rfc3339(&format!("{day}T00:00:00Z")).map(utc_day)
}

fn bad_attempt() -> Problem {
    Problem::bad_request("An attempt ID is <student>-<YYYY-MM-DD>")
}

fn no_such_attempt() -> Problem {
    Problem::not_found("There's no such attempt")
}

fn not_a_party() -> Problem {
    Problem::forbidden("not-a-party", "This attempt belongs to another agent")
}

fn parse_attempt(text: &str) -> Result<AttemptId, Problem> {
    AttemptId::parse(text).ok_or_else(bad_attempt)
}

/// The tasks of `pool_len` that `student` sits, `size` of them, in the pool's order: those
/// whose salted hash with the student's ID ranks first, so the set is fixed by the ID and two
/// students usually see different tasks.
fn set_for(course: &CourseId, student: &AgentId, pool_len: usize, size: usize) -> Vec<usize> {
    let mut ranked: Vec<(String, usize)> = (0..pool_len)
        .map(|i| {
            let seed = format!("sfab-set:{}:{student}:{i}", course.as_str());
            (body_hash(seed.as_bytes()), i)
        })
        .collect();
    ranked.sort();
    let mut chosen: Vec<usize> = ranked.into_iter().take(size).map(|(_, i)| i).collect();
    chosen.sort_unstable();
    chosen
}

/// One handed-in answer: an answer check's hash only, or handed-in work's text.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Answer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hash: Option<String>,
    answered_at: String,
    proof: String,
}

/// A marker's mark: the handed-in tasks it accepts, by position.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Mark {
    marker: String,
    accepted: Vec<usize>,
    #[serde(default)]
    comment: Option<String>,
    marked_at: String,
    proof: String,
}

/// `academy/courses/<course>/attempts/<attempt>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AttemptFile {
    student: String,
    /// The course's teacher and pass mark when it started.
    teacher: String,
    pass: u8,
    started_at: String,
    proof: String,
    tasks: Vec<Task>,
    answers: Vec<Option<Answer>>,
    #[serde(default)]
    submitted_at: Option<String>,
    #[serde(default)]
    score: Option<u32>,
    #[serde(default)]
    passed: Option<bool>,
    #[serde(default)]
    mark: Option<Mark>,
}

impl AttemptFile {
    fn status(&self) -> &'static str {
        match (&self.submitted_at, self.passed) {
            (None, _) => "open",
            (Some(_), None) => "awaiting-mark",
            (Some(_), Some(true)) => "passed",
            (Some(_), Some(false)) => "failed",
        }
    }

    fn has_work(&self) -> bool {
        self.tasks.iter().any(|t| t.kind == Kind::Work)
    }

    /// The answer checks answered right.
    fn correct(&self) -> usize {
        self.tasks
            .iter()
            .zip(&self.answers)
            .filter(|(t, a)| {
                t.kind == Kind::Check
                    && t.hash.is_some()
                    && a.as_ref().and_then(|a| a.hash.as_ref()) == t.hash.as_ref()
            })
            .count()
    }

    /// The score, in whole percent, of `right` tasks out of the set.
    fn score_of(&self, right: usize) -> u32 {
        (right * 100 / self.tasks.len().max(1)) as u32
    }

    fn mark_shown(&self) -> Value {
        match &self.mark {
            None => Value::Null,
            Some(m) => json!({
                "marker": m.marker,
                "accepted": m.accepted,
                "comment": m.comment,
                "marked_at": m.marked_at,
            }),
        }
    }

    /// What its student sees: never the answer hashes.
    fn shown(&self, course: &CourseId, id: &AttemptId) -> Value {
        let tasks: Vec<Value> = self
            .tasks
            .iter()
            .zip(&self.answers)
            .enumerate()
            .map(|(i, (t, a))| {
                json!({
                    "n": i + 1,
                    "kind": t.kind,
                    "text": t.text,
                    "answered": a.is_some(),
                    "answer": a.as_ref().and_then(|a| a.text.clone()),
                })
            })
            .collect();
        json!({
            "attempt": id.text(),
            "course": course.as_str(),
            "student": self.student,
            "started_at": self.started_at,
            "status": self.status(),
            "pass": self.pass,
            "tasks": tasks,
            "submitted_at": self.submitted_at,
            "score": self.score,
            "passed": self.passed,
            "mark": self.mark_shown(),
        })
    }

    /// What a marker sees: the handed-in work, and whether each answer check was right, never
    /// its hash.
    fn shown_to_marker(&self, course: &CourseId, id: &AttemptId) -> Value {
        let tasks: Vec<Value> = self
            .tasks
            .iter()
            .zip(&self.answers)
            .enumerate()
            .map(|(i, (t, a))| {
                let correct = (t.kind == Kind::Check).then(|| {
                    a.as_ref().and_then(|a| a.hash.as_ref()) == t.hash.as_ref() && a.is_some()
                });
                json!({
                    "n": i + 1,
                    "kind": t.kind,
                    "text": t.text,
                    "answer": a.as_ref().and_then(|a| a.text.clone()),
                    "correct": correct,
                })
            })
            .collect();
        json!({
            "attempt": id.text(),
            "course": course.as_str(),
            "student": self.student,
            "submitted_at": self.submitted_at,
            "status": self.status(),
            "pass": self.pass,
            "score": self.score,
            "tasks": tasks,
            "mark": self.mark_shown(),
        })
    }
}

/// `academy/courses/<course>/marking/<attempt>.json`: a pointer, so the queue lists without
/// reading work.
#[derive(Debug, Serialize, Deserialize)]
struct Pointer {
    submitted_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnswerRequest {
    answer: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MarkRequest {
    accepted: Vec<usize>,
    #[serde(default)]
    comment: Option<String>,
}

/// A task's position from 1 to [`MAX_SET`], with no sign or leading zero.
fn parse_position(text: &str) -> Option<usize> {
    let ok = !text.is_empty()
        && text.len() <= 2
        && text.bytes().all(|b| b.is_ascii_digit())
        && !text.starts_with('0');
    let n: usize = ok.then(|| text.parse().ok()).flatten()?;
    (1..=MAX_SET).contains(&n).then_some(n)
}

impl<S: Store> Academy<S> {
    async fn read_attempt(
        &self,
        course: &CourseId,
        id: &AttemptId,
    ) -> Result<Option<(AttemptFile, String)>, Problem> {
        read_file(&*self.store, &id.key(course), "reading an attempt").await
    }

    /// `POST /academy/courses/{course}/attempts`, in ADR 0003's order: the proof, the caller,
    /// the course, enrolment (`403` `not-enrolled`), then the student's latest attempt from one
    /// listing: an open one is returned with `200`, and one started less than
    /// [`RETAKE_DAYS`] days before the proof's date gets `409` `too-soon`. Then the new attempt,
    /// `<student>-<date>` by the proof's date, with its set copied from the pool, with
    /// `If-None-Match: *`. `201`.
    pub async fn start(&self, call: &Call<'_>, course: &str, now: i64) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let student = self.identity.caller(call.authorization, now).await?;
        let course = parse_course(course)?;
        empty_body(call.body)?;
        let (file, _) = self
            .read_course(&course)
            .await?
            .ok_or_else(no_such_course)?;
        if !self.enrolled_in(&course, &student).await? {
            return Err(Problem::forbidden(
                "not-enrolled",
                "Enrol first: PUT /v1/academy/courses/{course}/enrolments/self",
            ));
        }
        let mine = format!("{}attempts/{student}-", course.folder());
        let listed = self
            .store
            .list(Bucket::Identities, &mine)
            .await
            .map_err(|e| Problem::storage("listing attempts", &e))?;
        let latest = listed
            .iter()
            .filter_map(|k| k.strip_prefix(&format!("{}attempts/", course.folder())))
            .filter_map(|k| k.strip_suffix(".json"))
            .filter_map(AttemptId::parse)
            .max_by(|a, b| a.date.cmp(&b.date));
        let day = utc_day(admitted.timestamp);
        if let Some(latest) = latest
            && let Some((found, _)) = self.read_attempt(&course, &latest).await?
        {
            if found.submitted_at.is_none() {
                return Ok(Reply::ok(found.shown(&course, &latest)));
            }
            let next = day_number(&latest.date).unwrap_or(day) + RETAKE_DAYS;
            if day < next {
                return Err(Problem::conflict(
                    "too-soon",
                    "One attempt every 7 days, counted from the last one's start",
                )
                .with("retry_after", rfc3339(next * DAY)));
            }
        }
        let id = AttemptId {
            student: student.clone(),
            date: utc_date(admitted.timestamp),
        };
        let attempt = new_attempt(&file, &course, &student, &hex(&admitted.hash), now);
        if !put_file(
            &*self.store,
            &id.key(&course),
            &attempt,
            Condition::IfAbsent,
            "starting an attempt",
        )
        .await?
        {
            return Err(Problem::lost_race("starting an attempt"));
        }
        Ok(Reply::created(attempt.shown(&course, &id)))
    }

    /// `GET /academy/courses/{course}/attempts/{attempt}`: its student only (`403`
    /// `not-a-party`), with its tasks and answers, never their hashes.
    pub async fn attempt(
        &self,
        authorization: Option<&str>,
        course: &str,
        attempt: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let caller = self.identity.caller(authorization, now).await?;
        let course = parse_course(course)?;
        let id = parse_attempt(attempt)?;
        if id.student != caller {
            return Err(not_a_party());
        }
        let (file, _) = self
            .read_attempt(&course, &id)
            .await?
            .ok_or_else(no_such_attempt)?;
        Ok(Reply::ok(file.shown(&course, &id)))
    }

    /// `PUT /academy/courses/{course}/attempts/{attempt}/answers/{n}`: its student hands in
    /// task `n`, up to [`MAX_ANSWER_BYTES`], an answer check's stored only as its hash. A
    /// submitted attempt gets `409` `submitted`. The attempt file is rewritten with
    /// `If-Match`, read again on a lost race, so two answers at once are both kept. A retry
    /// with the same proof returns the answer. `200`.
    pub async fn answer(
        &self,
        call: &Call<'_>,
        course: &str,
        attempt: &str,
        n: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let caller = self.identity.caller(call.authorization, now).await?;
        let course = parse_course(course)?;
        let id = parse_attempt(attempt)?;
        let n = parse_position(n)
            .ok_or_else(|| Problem::bad_request("A task's position is 1 to 20"))?;
        let request: AnswerRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        if request.answer.is_empty() || request.answer.len() > MAX_ANSWER_BYTES {
            return Err(Problem::bad_request("answer must be 1 byte to 8 KB"));
        }
        if id.student != caller {
            return Err(not_a_party());
        }
        let proof = hex(&admitted.hash);
        for _ in 0..TRIES {
            let (mut file, etag) = self
                .read_attempt(&course, &id)
                .await?
                .ok_or_else(no_such_attempt)?;
            let Some(task) = file.tasks.get(n - 1).cloned() else {
                return Err(Problem::not_found("This attempt has no such task"));
            };
            let shown = |a: &Answer| json!({"attempt": id.text(), "n": n, "kind": task.kind, "answered_at": a.answered_at});
            if file.submitted_at.is_some() {
                return Err(Problem::conflict(
                    "submitted",
                    "This attempt is submitted, so its answers can't change",
                ));
            }
            if let Some(a) = &file.answers[n - 1]
                && a.proof == proof
            {
                return Ok(Reply::ok(shown(a)));
            }
            let given = match task.kind {
                Kind::Check => Answer {
                    text: None,
                    hash: Some(answer_hash(&course, &request.answer)),
                    answered_at: rfc3339(now),
                    proof: proof.clone(),
                },
                Kind::Work => Answer {
                    text: Some(request.answer.clone()),
                    hash: None,
                    answered_at: rfc3339(now),
                    proof: proof.clone(),
                },
            };
            let reply = shown(&given);
            file.answers[n - 1] = Some(given);
            let condition = Condition::IfMatch(etag);
            if put_file(
                &*self.store,
                &id.key(&course),
                &file,
                condition,
                "handing in",
            )
            .await?
            {
                return Ok(Reply::ok(reply));
            }
        }
        Err(Problem::lost_race("handing in an answer"))
    }

    /// `PUT /academy/courses/{course}/attempts/{attempt}/submission`: its student submits.
    /// Answer checks are scored; with no handed-in work, that's the result, and a pass is a
    /// certificate at once. Otherwise it joins the marking queue. Submitting again returns the
    /// first submission, after redoing the follow-up write it may have missed. `200`.
    pub async fn submit(
        &self,
        call: &Call<'_>,
        course: &str,
        attempt: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let caller = self.identity.caller(call.authorization, now).await?;
        let course = parse_course(course)?;
        let id = parse_attempt(attempt)?;
        empty_body(call.body)?;
        if id.student != caller {
            return Err(not_a_party());
        }
        for _ in 0..TRIES {
            let (mut file, etag) = self
                .read_attempt(&course, &id)
                .await?
                .ok_or_else(no_such_attempt)?;
            if file.submitted_at.is_some() {
                self.settle(&course, &id, &file).await?;
                return Ok(Reply::ok(file.shown(&course, &id)));
            }
            file.submitted_at = Some(rfc3339(now));
            if !file.has_work() {
                let score = file.score_of(file.correct());
                file.score = Some(score);
                file.passed = Some(score >= u32::from(file.pass));
            }
            let condition = Condition::IfMatch(etag);
            if put_file(
                &*self.store,
                &id.key(&course),
                &file,
                condition,
                "submitting",
            )
            .await?
            {
                self.settle(&course, &id, &file).await?;
                return Ok(Reply::ok(file.shown(&course, &id)));
            }
        }
        Err(Problem::lost_race("submitting an attempt"))
    }

    /// The writes that follow an attempt's state, each safe to repeat: a pointer in the
    /// marking queue while it awaits a mark, the certificate for a pass, and no pointer once
    /// it's decided.
    async fn settle(
        &self,
        course: &CourseId,
        id: &AttemptId,
        file: &AttemptFile,
    ) -> Result<(), Problem> {
        let pointer = id.pointer_key(course);
        match file.passed {
            None => {
                let at = file.submitted_at.clone().unwrap_or_default();
                let p = Pointer { submitted_at: at };
                put_file(&*self.store, &pointer, &p, Condition::Always, "queueing").await?;
                return Ok(());
            }
            Some(true) => {
                let marker = file.mark.as_ref().map(|m| m.marker.clone());
                let issued_at = file
                    .mark
                    .as_ref()
                    .map(|m| m.marked_at.clone())
                    .or_else(|| file.submitted_at.clone())
                    .unwrap_or_default();
                let certificate = CertificateFile::exam(
                    course.as_str(),
                    &file.student,
                    &file.teacher,
                    marker,
                    issued_at,
                );
                let key = certificates::exam_key(course, &id.student);
                put_file(
                    &*self.store,
                    &key,
                    &certificate,
                    Condition::IfAbsent,
                    "issuing",
                )
                .await?;
            }
            Some(false) => {}
        }
        if file.has_work() {
            self.store
                .delete(Bucket::Identities, &pointer)
                .await
                .map_err(|e| Problem::storage("leaving the marking queue", &e))?;
        }
        Ok(())
    }

    /// The course, if `caller` may mark for it: its teacher, or a graduate holding a
    /// certificate of it that isn't revoked; anyone else gets `403` `not-a-graduate`.
    async fn may_mark(&self, caller: &AgentId, course: &CourseId) -> Result<CourseFile, Problem> {
        let (file, _) = self.read_course(course).await?.ok_or_else(no_such_course)?;
        if file.teacher == caller.as_str() {
            return Ok(file);
        }
        let key = certificates::exam_key(course, caller);
        let held: Option<(CertificateFile, String)> =
            read_file(&*self.store, &key, "reading a certificate").await?;
        match held {
            Some((c, _)) if c.revoked_at.is_none() => Ok(file),
            _ => Err(Problem::forbidden(
                "not-a-graduate",
                "Only the course's teacher or a graduate of it can mark",
            )),
        }
    }

    /// `GET /academy/courses/{course}/marking`: the attempts awaiting a mark, 20 IDs a page
    /// from one listing, with the dates they started, never their work. `?page=` is the
    /// attempt to list after.
    pub async fn queue(
        &self,
        authorization: Option<&str>,
        course: &str,
        page: Option<&str>,
        now: i64,
    ) -> Result<Reply, Problem> {
        let caller = self.identity.caller(authorization, now).await?;
        let course = parse_course(course)?;
        let after = match page {
            None => None,
            Some(t) => Some(
                AttemptId::parse(t)
                    .ok_or_else(|| {
                        Problem::bad_request("?page= must be a token from a list's next")
                    })?
                    .pointer_key(&course),
            ),
        };
        self.may_mark(&caller, &course).await?;
        let prefix = format!("{}marking/", course.folder());
        let listed = self
            .store
            .list_page(Bucket::Identities, &prefix, after.as_deref(), OFFSET_PAGE)
            .await
            .map_err(|e| Problem::storage("listing the marking queue", &e))?;
        let ids: Vec<AttemptId> = listed
            .keys
            .iter()
            .filter_map(|k| k.strip_prefix(&prefix)?.strip_suffix(".json"))
            .filter_map(AttemptId::parse)
            .collect();
        let items: Vec<Value> = ids
            .iter()
            .map(|a| json!({"attempt": a.text(), "date": a.date}))
            .collect();
        let next = match ids.last() {
            Some(last) if listed.more => Some(format!(
                "/v1/academy/courses/{}/marking?page={}",
                course.as_str(),
                last.text()
            )),
            _ => None,
        };
        Ok(Reply::ok(json!({"items": items, "next": next})))
    }

    /// `GET /academy/courses/{course}/marking/{attempt}`: one submitted attempt's handed-in
    /// answers, for its teacher or a graduate.
    pub async fn marking(
        &self,
        authorization: Option<&str>,
        course: &str,
        attempt: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let caller = self.identity.caller(authorization, now).await?;
        let course = parse_course(course)?;
        let id = parse_attempt(attempt)?;
        self.may_mark(&caller, &course).await?;
        let (file, _) = self
            .read_attempt(&course, &id)
            .await?
            .filter(|(f, _)| f.submitted_at.is_some())
            .ok_or_else(no_such_attempt)?;
        Ok(Reply::ok(file.shown_to_marker(&course, &id)))
    }

    /// `PUT /academy/courses/{course}/marking/{attempt}`: the teacher or a graduate, never the
    /// student itself, marks a submitted attempt, accepting handed-in tasks by position, with
    /// an optional comment of up to [`MAX_COMMENT_BYTES`]. The score is the right answer checks
    /// and accepted work out of the set; a pass is a certificate naming the marker at once. A
    /// decided attempt returns its mark, after redoing the follow-up writes. `200`.
    pub async fn mark(
        &self,
        call: &Call<'_>,
        course: &str,
        attempt: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        let admitted = admit_write(call, now, self.required_bits)?;
        let marker = self.identity.caller(call.authorization, now).await?;
        let course = parse_course(course)?;
        let id = parse_attempt(attempt)?;
        let request: MarkRequest = serde_json::from_slice(call.body)
            .map_err(|_| Problem::bad_request("Malformed request body"))?;
        if request.accepted.len() > MAX_SET
            || request
                .comment
                .as_ref()
                .is_some_and(|c| c.len() > MAX_COMMENT_BYTES)
        {
            return Err(Problem::bad_request(
                "accepted holds up to 20 positions, and comment up to 2 KB",
            ));
        }
        self.may_mark(&marker, &course).await?;
        if id.student == marker {
            return Err(Problem::forbidden(
                "not-a-graduate",
                "No one marks its own attempt",
            ));
        }
        for _ in 0..TRIES {
            let (mut file, etag) = self
                .read_attempt(&course, &id)
                .await?
                .filter(|(f, _)| f.submitted_at.is_some())
                .ok_or_else(no_such_attempt)?;
            if file.passed.is_some() {
                self.settle(&course, &id, &file).await?;
                return Ok(Reply::ok(file.shown_to_marker(&course, &id)));
            }
            let mut accepted = request.accepted.clone();
            accepted.sort_unstable();
            accepted.dedup();
            let valid = accepted.len() == request.accepted.len()
                && accepted.iter().all(|n| {
                    file.tasks
                        .get(n.wrapping_sub(1))
                        .is_some_and(|t| t.kind == Kind::Work)
                });
            if !valid {
                return Err(Problem::bad_request(
                    "accepted must list handed-in tasks of this attempt, each once",
                ));
            }
            let score = file.score_of(file.correct() + accepted.len());
            file.score = Some(score);
            file.passed = Some(score >= u32::from(file.pass));
            file.mark = Some(Mark {
                marker: marker.as_str().to_owned(),
                accepted,
                comment: request.comment.clone(),
                marked_at: rfc3339(now),
                proof: hex(&admitted.hash),
            });
            let condition = Condition::IfMatch(etag);
            if put_file(&*self.store, &id.key(&course), &file, condition, "marking").await? {
                self.settle(&course, &id, &file).await?;
                return Ok(Reply::ok(file.shown_to_marker(&course, &id)));
            }
        }
        Err(Problem::lost_race("marking an attempt"))
    }
}

/// A new attempt at `course` for `student`, with its set copied from the pool as it stands.
fn new_attempt(
    file: &CourseFile,
    course: &CourseId,
    student: &AgentId,
    proof: &str,
    now: i64,
) -> AttemptFile {
    let size = file.set_size.min(file.tasks.len());
    let tasks: Vec<Task> = set_for(course, student, file.tasks.len(), size)
        .into_iter()
        .map(|i| file.tasks[i].clone())
        .collect();
    AttemptFile {
        student: student.as_str().to_owned(),
        teacher: file.teacher.clone(),
        pass: file.pass,
        started_at: rfc3339(now),
        proof: proof.to_owned(),
        answers: vec![None; tasks.len()],
        tasks,
        submitted_at: None,
        score: None,
        passed: None,
        mark: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_attempt_id_names_an_agent_and_a_real_day() {
        let id = AttemptId::parse("wrenwrenwren-2026-11-05").unwrap();
        assert_eq!(id.text(), "wrenwrenwren-2026-11-05");
        for bad in [
            "wrenwrenwren-2026-02-30",
            "WRENWRENWREN-2026-11-05",
            "wrenwrenwren_2026-11-05",
            "wrenwrenwren-2026-11-5",
            "../../x",
            "",
        ] {
            assert_eq!(AttemptId::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_task_position_is_1_to_20() {
        assert_eq!(parse_position("1"), Some(1));
        assert_eq!(parse_position("20"), Some(20));
        for bad in ["0", "21", "01", "+1", "x", ""] {
            assert_eq!(parse_position(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_set_is_fixed_by_the_student_s_id_and_differs_between_students() {
        let course = CourseId::parse("aaaaaaaaaaaa").unwrap();
        let wren = AgentId::parse("wrenwrenwren").unwrap();
        let moss = AgentId::parse("mossmossmoss").unwrap();
        let a = set_for(&course, &wren, 50, 20);
        assert_eq!(a.len(), 20);
        assert_eq!(a, set_for(&course, &wren, 50, 20));
        assert!(a.windows(2).all(|w| w[0] < w[1]), "in the pool's order");
        assert_ne!(a, set_for(&course, &moss, 50, 20));
    }
}
