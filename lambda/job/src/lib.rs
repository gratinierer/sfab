//! The daily world job's steps, in order: `.stories/2-identity-api/spec.md` "Daily world job".
//! It stays thin, as `docs/adr/0004-rules-live-in-crates.md` says: each step's rules live in its
//! crate. Issue 15 adds the federation checks, last, and story 4 adds its daily work.
//!
//! Every step is safe to repeat, because Lambda never retries a failed run and the owner may
//! rerun it by hand.

use sfab_census::Report;
use sfab_core::daily::rfc3339;
use sfab_core::storage::{Store, StoreError};

/// One run at `now`, a Unix time.
pub async fn run<S: Store>(store: &S, now: i64) -> Result<Report, StoreError> {
    let residents = sfab_identity::residents(store).await?;
    sfab_census::snapshot(store, &residents, &rfc3339(now)).await
}
