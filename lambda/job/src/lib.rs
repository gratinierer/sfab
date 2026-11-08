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
use sfab_genesis::{Daily, DailyReport as GenesisReport};

/// What one run did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub census: CensusReport,
    pub genesis: GenesisReport,
    pub federation: DailyReport,
}

/// One run at `now`, a Unix time, fetching tokens through `fetch` and emailing through `notify`,
/// making genesis codes as `genesis` says.
pub async fn run<S: Store, F: Fetch, N: Notify>(
    store: &S,
    fetch: &F,
    notify: &N,
    genesis: &Daily,
    now: i64,
) -> Result<Report, StoreError> {
    let census = census(store, now).await;
    // Runs whether or not the census worked, so a census fault can't leave the day without
    // genesis codes; the census's error still fails the run.
    let genesis = sfab_genesis::daily(store, now, genesis).await;
    let (census, genesis) = (census?, genesis?);
    let federation = sfab_federation::daily(store, fetch, notify, now).await?;
    Ok(Report {
        census,
        genesis,
        federation,
    })
}

async fn census<S: Store>(store: &S, now: i64) -> Result<CensusReport, StoreError> {
    let residents = sfab_identity::residents(store).await?;
    sfab_census::snapshot(store, &residents, &rfc3339(now)).await
}
