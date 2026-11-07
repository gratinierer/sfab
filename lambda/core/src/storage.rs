//! Typed access to storage: two S3 buckets, as `.stories/2-identity-api/spec.md` "Storage" says.
//! Institution crates own their keys and reach S3 only through [`Store`], so they're tested
//! against [`MemoryStore`], and the API Lambda plugs in S3.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Bucket {
    /// `identities/`, `vitals/`, `messages/`, `world/` and the other prefixes.
    Identities,
    /// `claims/<id>/claim.json`, kept apart so a claim hash never shares a bucket with content.
    Claims,
}

/// S3 user metadata, such as a takedown marker's `born_at`.
pub type Metadata = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub body: Vec<u8>,
    pub etag: String,
    pub metadata: Metadata,
}

/// A write's condition, as S3's conditional `PutObject` offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    Always,
    /// `If-None-Match: *`: only if nothing is there.
    IfAbsent,
    /// `If-Match: <etag>`: only if the object is unchanged.
    IfMatch(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// A conditional write lost: S3's `412`, or `409` for a concurrent conditional write.
    ConditionFailed,
    /// `If-Match` on an object that's gone: S3's `404`.
    Missing,
    /// Anything else. The text is for logs and must never hold a secret.
    Other(String),
}

pub trait Store: Send + Sync + 'static {
    /// `None` when there's no such key.
    fn get(
        &self,
        bucket: Bucket,
        key: &str,
    ) -> impl Future<Output = Result<Option<Object>, StoreError>> + Send;

    fn put(
        &self,
        bucket: Bucket,
        key: &str,
        body: Vec<u8>,
        metadata: Metadata,
        condition: Condition,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Deleting a key that's already gone succeeds.
    fn delete(
        &self,
        bucket: Bucket,
        key: &str,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Every key under `prefix`, in order.
    fn list(
        &self,
        bucket: Bucket,
        prefix: &str,
    ) -> impl Future<Output = Result<Vec<String>, StoreError>> + Send;

    /// Up to `max` keys under `prefix`, in order, after `after` when given, as S3's
    /// `ListObjectsV2` with `StartAfter` and `MaxKeys` gives them, for a live list's pages. This
    /// default lists everything; S3 asks for one page.
    fn list_page(
        &self,
        bucket: Bucket,
        prefix: &str,
        after: Option<&str>,
        max: usize,
    ) -> impl Future<Output = Result<KeyPage, StoreError>> + Send {
        async move {
            let mut keys: Vec<String> = self
                .list(bucket, prefix)
                .await?
                .into_iter()
                .filter(|k| after.is_none_or(|a| k.as_str() > a))
                .collect();
            let more = keys.len() > max;
            keys.truncate(max);
            Ok(KeyPage { keys, more })
        }
    }
}

/// One page of a listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPage {
    pub keys: Vec<String>,
    /// More keys follow this page.
    pub more: bool,
}

/// The in-memory fake every crate tests against. It can be told to fail writes under a prefix,
/// and counts writes per key.
#[derive(Debug, Default)]
pub struct MemoryStore {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    objects: HashMap<(Bucket, String), Object>,
    next_etag: u64,
    failing: HashSet<(Bucket, String)>,
    writes: HashMap<(Bucket, String), usize>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// From now on, every write and delete under `prefix` fails.
    pub fn fail_writes_under(&self, bucket: Bucket, prefix: &str) {
        self.lock().failing.insert((bucket, prefix.to_owned()));
    }

    /// How many writes reached `key`, failed ones included.
    pub fn writes_to(&self, bucket: Bucket, key: &str) -> usize {
        self.lock()
            .writes
            .get(&(bucket, key.to_owned()))
            .copied()
            .unwrap_or(0)
    }

    pub fn object(&self, bucket: Bucket, key: &str) -> Option<Object> {
        self.lock().objects.get(&(bucket, key.to_owned())).cloned()
    }

    /// Every key in `bucket`, in order.
    pub fn keys(&self, bucket: Bucket) -> Vec<String> {
        let mut keys: Vec<_> = self
            .lock()
            .objects
            .keys()
            .filter(|(b, _)| *b == bucket)
            .map(|(_, k)| k.clone())
            .collect();
        keys.sort();
        keys
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().expect("the fake's lock")
    }
}

impl State {
    fn fails(&self, bucket: Bucket, key: &str) -> bool {
        self.failing
            .iter()
            .any(|(b, prefix)| *b == bucket && key.starts_with(prefix.as_str()))
    }
}

impl Store for MemoryStore {
    async fn get(&self, bucket: Bucket, key: &str) -> Result<Option<Object>, StoreError> {
        Ok(self.object(bucket, key))
    }

    async fn put(
        &self,
        bucket: Bucket,
        key: &str,
        body: Vec<u8>,
        metadata: Metadata,
        condition: Condition,
    ) -> Result<(), StoreError> {
        let mut state = self.lock();
        *state.writes.entry((bucket, key.to_owned())).or_default() += 1;
        if state.fails(bucket, key) {
            return Err(StoreError::Other(format!("injected failure on {key}")));
        }
        let current = state.objects.get(&(bucket, key.to_owned()));
        match (&condition, current) {
            (Condition::IfAbsent, Some(_)) => return Err(StoreError::ConditionFailed),
            (Condition::IfMatch(_), None) => return Err(StoreError::Missing),
            (Condition::IfMatch(etag), Some(obj)) if obj.etag != *etag => {
                return Err(StoreError::ConditionFailed);
            }
            _ => {}
        }
        state.next_etag += 1;
        let etag = format!("\"{}\"", state.next_etag);
        state.objects.insert(
            (bucket, key.to_owned()),
            Object {
                body,
                etag,
                metadata,
            },
        );
        Ok(())
    }

    async fn delete(&self, bucket: Bucket, key: &str) -> Result<(), StoreError> {
        let mut state = self.lock();
        if state.fails(bucket, key) {
            return Err(StoreError::Other(format!("injected failure on {key}")));
        }
        state.objects.remove(&(bucket, key.to_owned()));
        Ok(())
    }

    async fn list(&self, bucket: Bucket, prefix: &str) -> Result<Vec<String>, StoreError> {
        Ok(self
            .keys(bucket)
            .into_iter()
            .filter(|k| k.starts_with(prefix))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_page_of_keys_starts_after_the_last_one_and_says_whether_more_follow() {
        let store = MemoryStore::new();
        for k in ["m/a", "m/b", "m/c", "n/a"] {
            let put = store.put(
                Bucket::Identities,
                k,
                vec![],
                Metadata::new(),
                Condition::Always,
            );
            put.await.unwrap();
        }
        let first = store
            .list_page(Bucket::Identities, "m/", None, 2)
            .await
            .unwrap();
        assert_eq!(
            first,
            KeyPage {
                keys: vec!["m/a".into(), "m/b".into()],
                more: true
            }
        );
        let last = store
            .list_page(Bucket::Identities, "m/", Some("m/b"), 2)
            .await
            .unwrap();
        assert_eq!(
            last,
            KeyPage {
                keys: vec!["m/c".into()],
                more: false
            }
        );
    }
}
