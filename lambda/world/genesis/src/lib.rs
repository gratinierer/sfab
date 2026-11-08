//! Genesis codes: the single-use code every `birth` needs, the IP rule, and the world's daily pool,
//! from `.stories/17-genesis-codes/spec.md`. This crate owns every `genesis/` key
//! (`docs/adr/0004-rules-live-in-crates.md`, `docs/adr/0008-genesis-codes-get-a-crate-and-an-owner-tool.md`);
//! `identity::birth`, the API router, the daily job and the owner's tool all reach them through it.
//!
//! Everything lives in the identities bucket, which the API Lambda already writes.

mod daily;
mod ip;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;

use sfab_core::daily::{DAY, is_rfc3339_second, next_daily_run, rfc3339, utc_date};
use sfab_core::problem::{Problem, Reply};
use sfab_core::storage::{Bucket, Condition, Metadata, Store, StoreError};

pub use daily::{Batch, Daily, DailyReport, Summary, WorldDay, Written, daily, read_summary};
use ip::IPS_PREFIX;
pub use ip::{IpRule, Window};

/// The bucket every `genesis/` key lives in.
const BUCKET: Bucket = Bucket::Identities;
const POOL_KEY: &str = "genesis/pool.json";
/// The prefix of the labels the daily job gives the world's days, `world-<date>`.
pub const WORLD_LABEL_PREFIX: &str = "world-";
/// Characters in a genesis code.
pub const CODE_CHARS: usize = 16;

const LABEL: &str = "label";
const EXPIRES: &str = "expires";
const NO_IP_LIMIT: &str = "no-ip-limit";
/// `no-ip-limit`'s value on a code that skips the IP rule.
const SET: &str = "true";

const CODES_PREFIX: &str = "genesis/codes/";
const BATCHES_PREFIX: &str = "genesis/batches/";

fn code_key(code: &Code) -> String {
    format!("{CODES_PREFIX}{code}")
}

fn batch_key(label: &str, code: &Code) -> String {
    format!("{BATCHES_PREFIX}{label}/{code}")
}

/// A genesis code: 16 characters of lowercase base32. Only `new_code` and `Code::parse` make
/// one, so every `Code` is safe in a key. It serializes as its text, and deserializes only from
/// text with a code's form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Code(String);

impl Code {
    /// `text` as a code, or `None` when it hasn't a code's form.
    pub fn parse(text: &str) -> Option<Self> {
        is_code(text).then(|| Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for Code {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Code {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text).ok_or_else(|| format!("not a genesis code: {text:?}"))
    }
}

impl From<Code> for String {
    fn from(code: Code) -> Self {
        code.0
    }
}

/// A new genesis code: 80 random bits.
pub fn new_code() -> Code {
    let mut bytes = [0u8; 10];
    getrandom::fill(&mut bytes).expect("the system's random source");
    let bits = bytes
        .iter()
        .fold(0u128, |acc, b| (acc << 8) | u128::from(*b));
    Code(
        (0..CODE_CHARS)
            .rev()
            .map(|i| ALPHABET[((bits >> (5 * i)) & 31) as usize] as char)
            .collect(),
    )
}

const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

/// Whether `text` has a genesis code's form, so it's safe in a key.
pub fn is_code(text: &str) -> bool {
    text.len() == CODE_CHARS && text.bytes().all(|c| ALPHABET.contains(&c))
}

/// How a code is issued: its batch's label, when it stops working, and whether it skips the IP
/// rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// `[a-z0-9-]`; the daily job's are `world-<date>`.
    pub label: String,
    /// The Unix time from which the code no longer works. `None` never expires.
    pub expires: Option<i64>,
    pub no_ip_limit: bool,
}

impl Issue {
    /// A world code made at `now`: labelled `world-<date>`, valid until the UTC day ends.
    pub fn world(now: i64) -> Self {
        Self {
            label: format!("{WORLD_LABEL_PREFIX}{}", utc_date(now)),
            expires: Some((now.div_euclid(DAY) + 1) * DAY),
            no_ip_limit: false,
        }
    }

    fn metadata(&self) -> Metadata {
        let mut metadata = Metadata::from([(LABEL.to_owned(), self.label.clone())]);
        if let Some(expires) = self.expires {
            metadata.insert(EXPIRES.to_owned(), rfc3339(expires));
        }
        if self.no_ip_limit {
            metadata.insert(NO_IP_LIMIT.to_owned(), SET.to_owned());
        }
        metadata
    }
}

/// Uploads one code: `genesis/codes/<code>` with its metadata, and its batch entry
/// `genesis/batches/<label>/<code>`, which is never deleted.
pub async fn issue<S: Store>(store: &S, code: &Code, how: &Issue) -> Result<(), StoreError> {
    store
        .put(
            BUCKET,
            &code_key(code),
            Vec::new(),
            how.metadata(),
            Condition::Always,
        )
        .await?;
    store
        .put(
            BUCKET,
            &batch_key(&how.label, code),
            Vec::new(),
            Metadata::new(),
            Condition::Always,
        )
        .await
}

/// Makes a new code and issues it as `how` says.
pub async fn issue_new<S: Store>(store: &S, how: &Issue) -> Result<Code, StoreError> {
    let code = new_code();
    issue(store, &code, how).await?;
    Ok(code)
}

/// `genesis/pool.json`: today's unused world codes, with the UTC date they're for.
#[derive(Debug, Serialize, Deserialize)]
struct Pool {
    date: String,
    codes: Vec<Code>,
}

/// Writes `genesis/pool.json` with `codes`, for the UTC day of `now`.
pub async fn write_pool<S: Store>(store: &S, now: i64, codes: &[Code]) -> Result<(), StoreError> {
    let pool = Pool {
        date: utc_date(now),
        codes: codes.to_vec(),
    };
    store
        .put(
            BUCKET,
            POOL_KEY,
            to_json(&pool),
            Metadata::new(),
            Condition::Always,
        )
        .await
}

fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("plain data serializes")
}

/// A code that passed `admit`, for `use_up` after the birth.
#[derive(Debug, Clone)]
pub struct Admission {
    code: Code,
    /// Whether it is a world code, from the daily pool, which `use_up` takes it out of.
    from_pool: bool,
    /// The IP record to write, or `None` for a code that skips the rule.
    ip: Option<String>,
}

fn invalid_code() -> Problem {
    Problem::forbidden("invalid-genesis-code", "This genesis code doesn't work").with(
        "detail",
        "A birth needs an unused, unexpired genesis code. Get one from GET /v1/world/genesis-code, \
         or from the owner's posts, and solve the proof again, since it covers the code.",
    )
}

/// The genesis checks for `birth`, and the pool route.
pub struct Genesis<S> {
    store: Arc<S>,
    rule: IpRule,
}

impl<S: Store> Genesis<S> {
    pub fn new(store: Arc<S>, rule: IpRule) -> Self {
        Self { store, rule }
    }

    /// Steps 2 and 3 of `birth`: one HEAD of the code, then, unless the code skips it, one GET of
    /// the IP record. Every refused code gets the same `403`. `viewer` is CloudFront's
    /// `CloudFront-Viewer-Address`; an ordinary code with none, or one that doesn't parse, gets
    /// `500`, since only a deployment fault or the owner causes it. Writes nothing.
    pub async fn admit(
        &self,
        code: &str,
        viewer: Option<&str>,
        now: i64,
    ) -> Result<Admission, Problem> {
        let code = Code::parse(code).ok_or_else(invalid_code)?;
        let metadata = self
            .store
            .head(BUCKET, &code_key(&code))
            .await
            .map_err(|e| Problem::storage("reading a genesis code", &e))?
            .ok_or_else(invalid_code)?;
        if let Some(expires) = metadata.get(EXPIRES)
            && (!is_rfc3339_second(expires) || rfc3339(now) >= *expires)
        {
            return Err(invalid_code());
        }
        let from_pool = metadata
            .get(LABEL)
            .is_some_and(|l| l.starts_with(WORLD_LABEL_PREFIX));
        let ip = if metadata.get(NO_IP_LIMIT).is_some_and(|v| v == SET) {
            None
        } else {
            let Some(fingerprint) = viewer.and_then(|v| self.rule.fingerprint(v)) else {
                tracing::error!(
                    "birth: no usable viewer address (CloudFront-Viewer-Address), refused"
                );
                return Err(Problem::internal());
            };
            self.check_window(&fingerprint, now).await?;
            Some(fingerprint)
        };
        Ok(Admission {
            code,
            from_pool,
            ip,
        })
    }

    /// `409 too-soon` when the IP record holds a birth inside the window. Judged by the stored
    /// time, never by whether the record exists.
    async fn check_window(&self, fingerprint: &str, now: i64) -> Result<(), Problem> {
        let object = self
            .store
            .get(BUCKET, &ip::record_key(fingerprint))
            .await
            .map_err(|e| Problem::storage("reading an IP record", &e))?;
        let Some(object) = object else {
            return Ok(());
        };
        let Ok(record) = serde_json::from_slice::<ip::Record>(&object.body) else {
            tracing::warn!("an IP record doesn't parse, ignored");
            return Ok(());
        };
        let until = record.last_birth + self.rule.window_seconds();
        if now < until {
            return Err(Problem::conflict(
                "too-soon",
                "One birth per IP address per window: retry after retry_after",
            )
            .with("retry_after", rfc3339(until)));
        }
        Ok(())
    }

    /// Step 5 of `birth`, after it succeeded: delete the code, write the IP record unless the
    /// code skips the rule, and for a world code, take it out of the pool with a conditional PUT.
    /// Every failure is logged and ignored: the birth stands, and a code left behind is the
    /// accepted "used twice".
    pub async fn use_up(&self, admission: &Admission, now: i64) {
        if let Err(e) = self.store.delete(BUCKET, &code_key(&admission.code)).await {
            tracing::warn!(error = ?e, "deleting a used genesis code failed, ignored");
        }
        if let Some(fingerprint) = &admission.ip {
            let record = ip::Record { last_birth: now };
            let key = ip::record_key(fingerprint);
            let write = self.store.put(
                BUCKET,
                &key,
                to_json(&record),
                Metadata::new(),
                Condition::Always,
            );
            if let Err(e) = write.await {
                tracing::warn!(error = ?e, "writing an IP record failed, ignored");
            }
        }
        if admission.from_pool {
            self.leave_pool(&admission.code).await;
        }
    }

    /// One GET and one conditional PUT, whose lost race is ignored, so the pool is only mostly
    /// fresh.
    async fn leave_pool(&self, code: &Code) {
        let object = match self.store.get(BUCKET, POOL_KEY).await {
            Ok(Some(object)) => object,
            Ok(None) => return,
            Err(e) => return tracing::warn!(error = ?e, "reading the pool failed, ignored"),
        };
        let Ok(mut pool) = serde_json::from_slice::<Pool>(&object.body) else {
            return tracing::error!("the genesis pool doesn't parse");
        };
        let before = pool.codes.len();
        pool.codes.retain(|c| c != code);
        if pool.codes.len() == before {
            return;
        }
        let write = self.store.put(
            BUCKET,
            POOL_KEY,
            to_json(&pool),
            Metadata::new(),
            Condition::IfMatch(object.etag),
        );
        if let Err(e) = write.await {
            tracing::info!(error = ?e, "updating the pool lost a race, ignored");
        }
    }

    /// `GET /world/genesis-code`: one random code from today's pool, with one GET. `404
    /// not-built-yet` before the daily job's first pool, and `409 try-later` until its next run
    /// when the pool is empty or from an earlier UTC day.
    pub async fn pool_code(&self, now: i64) -> Result<Reply, Problem> {
        let object = self
            .store
            .get(BUCKET, POOL_KEY)
            .await
            .map_err(|e| Problem::storage("reading the genesis pool", &e))?
            .ok_or_else(|| {
                Problem::not_built_yet("No genesis codes yet: the daily job hasn't run once")
            })?;
        let pool: Pool = serde_json::from_slice(&object.body).map_err(|_| {
            tracing::error!("the genesis pool doesn't parse");
            Problem::internal()
        })?;
        if pool.date != utc_date(now) || pool.codes.is_empty() {
            return Err(Problem::conflict(
                "try-later",
                "Today's genesis codes are taken: retry after retry_after",
            )
            .with("retry_after", rfc3339(next_daily_run(now))));
        }
        let pick = random_below(pool.codes.len());
        Ok(Reply::ok(json!({ "genesis_code": pool.codes[pick] })))
    }
}

/// A random index below `n`; `n` is never 0.
fn random_below(n: usize) -> usize {
    let r = getrandom::u64().expect("the system's random source");
    (r % n as u64) as usize
}

/// Helpers for other crates' tests, which birth through a seeded code.
#[cfg(feature = "test-support")]
pub mod testing {
    use super::*;

    /// The IP rule tests use: a fixed key, and the 4-hour window `spec.md` starts with.
    pub fn ip_rule() -> IpRule {
        IpRule::new("testkeytestkeytestkeytestkeytestkey", Window::hours(4))
    }

    /// The genesis checks over `store`, under `ip_rule`, for an `Identity`.
    pub fn genesis<S: Store>(store: &Arc<S>) -> Arc<Genesis<S>> {
        Arc::new(Genesis::new(store.clone(), ip_rule()))
    }

    /// A fresh `--no-ip-limit` code that never expires, so a test can birth many times from one
    /// address, or from none.
    pub async fn seed_exempt_code<S: Store>(store: &S) -> String {
        seed(store, true).await
    }

    /// A fresh ordinary code that never expires, under the IP rule.
    pub async fn seed_code<S: Store>(store: &S) -> String {
        seed(store, false).await
    }

    /// Puts back a used code, as a replay test needs.
    pub async fn reseed_exempt_code<S: Store>(store: &S, code: &str) {
        let how = owner_batch(true);
        let code = Code::parse(code).expect("a code");
        issue(store, &code, &how).await.expect("seeding a code");
    }

    fn owner_batch(no_ip_limit: bool) -> Issue {
        Issue {
            label: "test".into(),
            expires: None,
            no_ip_limit,
        }
    }

    async fn seed<S: Store>(store: &S, no_ip_limit: bool) -> String {
        issue_new(store, &owner_batch(no_ip_limit))
            .await
            .expect("seeding a code")
            .into()
    }

    /// `n` world codes for the UTC day of `now`, issued and in that day's pool.
    pub async fn seed_world_codes<S: Store>(store: &S, now: i64, n: usize) -> Vec<String> {
        let how = Issue::world(now);
        let mut codes = Vec::with_capacity(n);
        for _ in 0..n {
            codes.push(issue_new(store, &how).await.expect("seeding a code"));
        }
        write_pool(store, now, &codes)
            .await
            .expect("seeding a pool");
        codes.into_iter().map(String::from).collect()
    }
}
