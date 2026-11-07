//! The daily world job's steps, in order: `.stories/2-identity-api/spec.md` "Daily world job".
//! It stays thin, as `docs/adr/0004-rules-live-in-crates.md` says: each step's rules live in its
//! crate. The federation checks run last, so slow servers can't hold up the rest, and story 4
//! adds its daily work before them.
//!
//! Every step is safe to repeat, because Lambda never retries a failed run and the owner may
//! rerun it by hand.

pub mod outbound;

use sfab_census::Report as CensusReport;
use sfab_core::daily::rfc3339;
use sfab_core::storage::{Store, StoreError};
use sfab_federation::{DailyReport, Fetch, Notify};

/// What one run did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub census: CensusReport,
    pub federation: DailyReport,
}

/// One run at `now`, a Unix time, fetching tokens through `fetch` and emailing through `notify`.
pub async fn run<S: Store, F: Fetch, N: Notify>(
    store: &S,
    fetch: &F,
    notify: &N,
    now: i64,
) -> Result<Report, StoreError> {
    let residents = sfab_identity::residents(store).await?;
    let census = sfab_census::snapshot(store, &residents, &rfc3339(now)).await?;
    let federation = sfab_federation::daily(store, fetch, notify, now).await?;
    Ok(Report { census, federation })
}
