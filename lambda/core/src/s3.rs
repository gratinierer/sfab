//! [`Store`] on S3, for the API Lambda and the takedown program.

use std::collections::HashMap;

use aws_sdk_s3::Client;
use aws_sdk_s3::error::{DisplayErrorContext, SdkError};
use aws_sdk_s3::primitives::ByteStream;

use crate::storage::{Bucket, Condition, KeyPage, Listed, Metadata, Object, Store, StoreError};

#[derive(Debug, Clone)]
pub struct S3Store {
    client: Client,
    identities: String,
    claims: String,
}

impl S3Store {
    pub fn new(client: Client, identities: String, claims: String) -> Self {
        Self {
            client,
            identities,
            claims,
        }
    }

    fn name(&self, bucket: Bucket) -> &str {
        match bucket {
            Bucket::Identities => &self.identities,
            Bucket::Claims => &self.claims,
        }
    }
}

/// S3's status for a failed call, when it got that far.
fn status<E>(err: &SdkError<E>) -> Option<u16> {
    err.raw_response().map(|r| r.status().as_u16())
}

/// Any other S3 failure, as text for the logs.
fn store_error<E: std::error::Error>(err: E) -> StoreError {
    StoreError::Other(DisplayErrorContext(err).to_string())
}

impl Store for S3Store {
    async fn get(&self, bucket: Bucket, key: &str) -> Result<Option<Object>, StoreError> {
        let out = match self
            .client
            .get_object()
            .bucket(self.name(bucket))
            .key(key)
            .send()
            .await
        {
            Ok(out) => out,
            Err(e) if e.as_service_error().is_some_and(|s| s.is_no_such_key()) => return Ok(None),
            Err(e) => return Err(store_error(e)),
        };
        let etag = out.e_tag().unwrap_or_default().to_owned();
        let metadata: Metadata = out
            .metadata()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let body = out
            .body
            .collect()
            .await
            .map_err(store_error)?
            .into_bytes()
            .to_vec();
        Ok(Some(Object {
            body,
            etag,
            metadata,
        }))
    }

    /// One `HeadObject`: the metadata, without the body.
    async fn head(&self, bucket: Bucket, key: &str) -> Result<Option<Metadata>, StoreError> {
        let out = match self
            .client
            .head_object()
            .bucket(self.name(bucket))
            .key(key)
            .send()
            .await
        {
            Ok(out) => out,
            Err(e) if e.as_service_error().is_some_and(|s| s.is_not_found()) => return Ok(None),
            Err(e) => return Err(store_error(e)),
        };
        Ok(Some(
            out.metadata()
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        ))
    }

    async fn put(
        &self,
        bucket: Bucket,
        key: &str,
        body: Vec<u8>,
        metadata: Metadata,
        condition: Condition,
    ) -> Result<(), StoreError> {
        let mut request = self
            .client
            .put_object()
            .bucket(self.name(bucket))
            .key(key)
            .body(ByteStream::from(body))
            .set_metadata(
                (!metadata.is_empty()).then(|| metadata.into_iter().collect::<HashMap<_, _>>()),
            );
        request = match condition {
            Condition::Always => request,
            Condition::IfAbsent => request.if_none_match("*"),
            Condition::IfMatch(etag) => request.if_match(etag),
        };
        match request.send().await {
            Ok(_) => Ok(()),
            Err(e) => Err(match status(&e) {
                Some(412 | 409) => StoreError::ConditionFailed,
                Some(404) => StoreError::Missing,
                _ => store_error(e),
            }),
        }
    }

    async fn delete(&self, bucket: Bucket, key: &str) -> Result<(), StoreError> {
        self.client
            .delete_object()
            .bucket(self.name(bucket))
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(store_error)
    }

    async fn list(&self, bucket: Bucket, prefix: &str) -> Result<Vec<String>, StoreError> {
        let mut keys = Vec::new();
        let mut pages = self
            .client
            .list_objects_v2()
            .bucket(self.name(bucket))
            .prefix(prefix)
            .into_paginator()
            .send();
        while let Some(page) = pages.next().await {
            let page = page.map_err(store_error)?;
            keys.extend(
                page.contents()
                    .iter()
                    .filter_map(|o| o.key().map(str::to_owned)),
            );
        }
        Ok(keys)
    }

    async fn list_sized(&self, bucket: Bucket, prefix: &str) -> Result<Vec<Listed>, StoreError> {
        let mut listed = Vec::new();
        let mut pages = self
            .client
            .list_objects_v2()
            .bucket(self.name(bucket))
            .prefix(prefix)
            .into_paginator()
            .send();
        while let Some(page) = pages.next().await {
            let page = page.map_err(store_error)?;
            listed.extend(page.contents().iter().filter_map(|o| {
                Some(Listed {
                    key: o.key()?.to_owned(),
                    size: o.size().unwrap_or_default().max(0) as u64,
                })
            }));
        }
        Ok(listed)
    }

    /// One `ListObjectsV2` call: S3 returns keys after `StartAfter` in order, and `IsTruncated`
    /// says whether more follow.
    async fn list_page(
        &self,
        bucket: Bucket,
        prefix: &str,
        after: Option<&str>,
        max: usize,
    ) -> Result<KeyPage, StoreError> {
        let page = self
            .client
            .list_objects_v2()
            .bucket(self.name(bucket))
            .prefix(prefix)
            .set_start_after(after.map(str::to_owned))
            .max_keys(i32::try_from(max).unwrap_or(1000))
            .send()
            .await
            .map_err(store_error)?;
        let keys = page
            .contents()
            .iter()
            .filter_map(|o| o.key().map(str::to_owned))
            .collect();
        Ok(KeyPage {
            keys,
            more: page.is_truncated().unwrap_or(false),
        })
    }
}
