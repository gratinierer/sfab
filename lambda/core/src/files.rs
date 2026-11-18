//! One JSON file a route reads or writes, with the ETag that makes its next write conditional:
//! the typed storage access every institution shares (`CODING_STANDARDS.md` "What every route
//! shares lives in `lambda/core`", `docs/adr/0003-proof-on-every-write.md` for the conditional
//! writes).

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::problem::Problem;
use crate::storage::{Bucket, Condition, Metadata, Store, StoreError};

/// `value` as the JSON bytes a file holds.
pub fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("plain data serializes")
}

/// A stored file, with its ETag, or `None` when it's missing or a zero-byte marker. A storage
/// error or a file that doesn't parse is a logged `500`; `what` names the call in the log.
pub async fn read_file<S: Store, T: DeserializeOwned>(
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

/// Writes `value` at `key` under `condition`, with a lost condition as `Ok(false)`
/// (`docs/aws-facts.md`, "`PutObject` with `If-None-Match: *`" and "with `If-Match`").
pub async fn put_file<S: Store, T: Serialize>(
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
pub fn condition_for(etag: Option<String>) -> Condition {
    match etag {
        Some(etag) => Condition::IfMatch(etag),
        None => Condition::IfAbsent,
    }
}

/// A body that says nothing, for a `PUT` on a named state: empty, or `{}`.
pub fn empty_body(body: &[u8]) -> Result<(), Problem> {
    if body.is_empty() {
        return Ok(());
    }
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(map)) if map.is_empty() => Ok(()),
        _ => Err(Problem::bad_request("The body must be empty or {}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStore;

    #[tokio::test]
    async fn a_file_reads_back_with_its_etag_and_a_marker_reads_as_missing() {
        {
            let store = MemoryStore::new();
            assert!(
                put_file(
                    &store,
                    "a.json",
                    &serde_json::json!({"n": 1}),
                    Condition::IfAbsent,
                    "t"
                )
                .await
                .unwrap()
            );
            let (file, etag): (Value, String) =
                read_file(&store, "a.json", "t").await.unwrap().unwrap();
            assert_eq!(file["n"], 1);
            assert!(
                !put_file(&store, "a.json", &file, Condition::IfAbsent, "t")
                    .await
                    .unwrap()
            );
            assert!(
                put_file(&store, "a.json", &file, condition_for(Some(etag)), "t")
                    .await
                    .unwrap()
            );
            store
                .put(
                    Bucket::Identities,
                    "m.json",
                    Vec::new(),
                    Metadata::new(),
                    Condition::Always,
                )
                .await
                .unwrap();
            let marker: Option<(Value, String)> = read_file(&store, "m.json", "t").await.unwrap();
            assert!(marker.is_none());
        }
    }

    #[test]
    fn only_an_empty_body_or_an_empty_object_is_empty() {
        assert!(empty_body(b"").is_ok());
        assert!(empty_body(b"{}").is_ok());
        assert_eq!(empty_body(b"{\"a\":1}").unwrap_err().status, 400);
        assert_eq!(empty_body(b"[]").unwrap_err().status, 400);
    }
}
