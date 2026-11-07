//! The daily world job's federation step.

use std::future::Future;

use sfab_core::storage::{Store, StoreError};

pub trait Fetch: Sync {
    fn fetch(&self, domain: &str, path: &str) -> impl Future<Output = Option<Vec<u8>>> + Send;
}

pub trait Notify: Sync {
    fn notify(&self, subject: &str, message: &str) -> impl Future<Output = Result<(), String>> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyReport {}

pub fn matches(_body: &[u8], _token: &str) -> bool {
    false
}

pub async fn daily<S: Store, F: Fetch, N: Notify>(
    _store: &S,
    _fetch: &F,
    _notify: &N,
    _now: i64,
) -> Result<DailyReport, StoreError> {
    Ok(DailyReport {})
}
