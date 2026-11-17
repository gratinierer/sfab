//! Certificates: `.stories/4-life-in-the-world/spec.md` "The Academy" ("Who marks", "When
//! things go away"). A certificate is `exam-<course>-<holder>`, issued at once on a pass, or,
//! from ticket 06, `apprenticeship-<apprenticeship>`, so its kind is in its ID. It names its
//! issuer, the course's teacher or the master, and for a marked exam its marker. Whether it
//! counts is the daily job's, from the marker's or else the issuer's standing. Its page shows
//! "removed" for an issuer or marker with a takedown marker, so nothing is rewritten.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sfab_core::daily::rfc3339;
use sfab_core::gate::{Call, admit_write};
use sfab_core::id::AgentId;
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Condition, Store};

use crate::{Academy, CourseId, daily, empty_body, put_file, read_file};

/// Where certificates live, which the daily job lists.
pub(crate) const CERTIFICATES_PREFIX: &str = "academy/certificates/";

/// `academy/certificates/<certificate>.json`, shared by both kinds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CertificateFile {
    /// `exam` or `apprenticeship`.
    pub(crate) kind: String,
    pub(crate) holder: String,
    /// The course's teacher, or the master.
    pub(crate) issuer: String,
    /// Who marked an exam with handed-in work; `None` for an answer-check exam.
    #[serde(default)]
    pub(crate) marker: Option<String>,
    #[serde(default)]
    pub(crate) course: Option<String>,
    pub(crate) issued_at: String,
    #[serde(default)]
    pub(crate) revoked_at: Option<String>,
}

impl CertificateFile {
    pub(crate) fn exam(
        course: &str,
        holder: &str,
        teacher: &str,
        marker: Option<String>,
        issued_at: String,
    ) -> Self {
        Self {
            kind: "exam".to_owned(),
            holder: holder.to_owned(),
            issuer: teacher.to_owned(),
            marker,
            course: Some(course.to_owned()),
            issued_at,
            revoked_at: None,
        }
    }

    /// Whose standing makes it count: the marker's, or else the issuer's.
    pub(crate) fn vouched_by(&self) -> &str {
        self.marker.as_deref().unwrap_or(&self.issuer)
    }
}

pub(crate) fn exam_key(course: &CourseId, holder: &AgentId) -> String {
    format!(
        "{CERTIFICATES_PREFIX}exam-{}-{holder}.json",
        course.as_str()
    )
}

fn key_of(certificate: &str) -> String {
    format!("{CERTIFICATES_PREFIX}{certificate}.json")
}

/// A certificate ID from a key under [`prefix`], or `None` for any other key.
pub(crate) fn id_of_key(key: &str) -> Option<String> {
    let name = key
        .strip_prefix(CERTIFICATES_PREFIX)?
        .strip_suffix(".json")?;
    valid(name).then(|| name.to_owned())
}

/// `exam-<course>-<holder>`, or `apprenticeship-<master>-<apprentice>-<date>`.
pub(crate) fn valid(id: &str) -> bool {
    if let Some(rest) = id.strip_prefix("exam-") {
        return exam_parts(rest).is_some();
    }
    let Some(rest) = id.strip_prefix("apprenticeship-") else {
        return false;
    };
    let b = rest.as_bytes();
    b.len() == 12 + 1 + 12 + 1 + 10
        && b[12] == b'-'
        && b[25] == b'-'
        && AgentId::parse(&rest[..12]).is_some()
        && AgentId::parse(&rest[13..25]).is_some()
        && sfab_core::daily::parse_rfc3339(&format!("{}T00:00:00Z", &rest[26..])).is_some()
}

fn exam_parts(rest: &str) -> Option<(CourseId, AgentId)> {
    let (course, holder) = rest.split_once('-')?;
    Some((CourseId::parse(course)?, AgentId::parse(holder)?))
}

/// Whether a file name `<certificate>.json` is an exam certificate `id` holds.
pub(crate) fn held_by(name: &str, id: &AgentId) -> bool {
    name.strip_suffix(".json")
        .and_then(|n| n.strip_prefix("exam-"))
        .and_then(exam_parts)
        .is_some_and(|(_, holder)| &holder == id)
}

fn bad_certificate() -> Problem {
    Problem::bad_request(
        "A certificate is exam-<course>-<holder> or apprenticeship-<apprenticeship>",
    )
}

fn no_such_certificate() -> Problem {
    Problem::not_found("There's no such certificate")
}

impl<S: Store> Academy<S> {
    /// An agent ID with whether it has a takedown marker, so a certificate shows "removed".
    async fn named(&self, id: &str) -> Result<Value, Problem> {
        let removed = match AgentId::parse(id) {
            Some(agent) => sfab_identity::born(&*self.store, &agent)
                .await
                .map_err(|e| Problem::storage("reading a claim", &e))?
                .is_some_and(|b| b.taken_down),
            None => true,
        };
        Ok(json!({"id": id, "removed": removed}))
    }

    /// `GET /academy/certificates/{certificate}`: its kind, holder, teacher or master, marker
    /// and date, whether it's revoked, and whether it counts, as of the daily job's last run.
    pub async fn certificate(&self, certificate: &str) -> Result<Reply, Problem> {
        if !valid(certificate) {
            return Err(bad_certificate());
        }
        let found: Option<(CertificateFile, String)> =
            read_file(&*self.store, &key_of(certificate), "reading a certificate").await?;
        let (file, _) = found.ok_or_else(no_such_certificate)?;
        let (counts, as_of) = daily::counts(&*self.store, certificate).await?;
        let issuer = self.named(&file.issuer).await?;
        let marker = match &file.marker {
            Some(m) => self.named(m).await?,
            None => Value::Null,
        };
        let role = if file.kind == "exam" {
            "teacher"
        } else {
            "master"
        };
        let mut shown = json!({
            "certificate": certificate,
            "kind": file.kind,
            "holder": file.holder,
            "course": file.course,
            "marker": marker,
            "issued_at": file.issued_at,
            "revoked": file.revoked_at.is_some(),
            "revoked_at": file.revoked_at,
            "counts": counts && file.revoked_at.is_none(),
            "as_of": as_of,
        });
        shown[role] = issuer;
        Ok(Reply::ok(shown))
    }

    /// `PUT /academy/certificates/{certificate}/revocation`: only its issuer, the course's
    /// teacher or the master, revokes it; a graduate who marked it gets `403`
    /// `not-the-author`. It stays visible as revoked. `200`, again for one already revoked.
    pub async fn revoke(
        &self,
        call: &Call<'_>,
        certificate: &str,
        now: i64,
    ) -> Result<Reply, Problem> {
        admit_write(call, now, self.required_bits)?;
        let caller = self.identity.caller(call.authorization, now).await?;
        if !valid(certificate) {
            return Err(bad_certificate());
        }
        empty_body(call.body)?;
        let key = key_of(certificate);
        let found: Option<(CertificateFile, String)> =
            read_file(&*self.store, &key, "reading a certificate").await?;
        let (mut file, etag) = found.ok_or_else(no_such_certificate)?;
        if file.issuer != caller.as_str() {
            return Err(Problem::forbidden(
                "not-the-author",
                "Only the certificate's teacher or master can revoke it",
            ));
        }
        if file.revoked_at.is_none() {
            file.revoked_at = Some(rfc3339(now));
            let condition = Condition::IfMatch(etag);
            if !put_file(&*self.store, &key, &file, condition, "revoking").await? {
                return Err(Problem::lost_race("revoking a certificate"));
            }
        }
        Ok(Reply::ok(json!({
            "certificate": certificate,
            "revoked": true,
            "revoked_at": file.revoked_at,
        })))
    }
}
