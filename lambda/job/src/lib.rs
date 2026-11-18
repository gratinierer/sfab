//! The daily world job's steps, in order: `.stories/2-identity-api/spec.md` "Daily world job".
//! It stays thin, as `docs/adr/0004-rules-live-in-crates.md` says: each step's rules live in its
//! crate. The federation checks run last, so slow servers can't hold up the rest, and story 4
//! adds its daily work before them.
//!
//! Every step is safe to repeat, because Lambda never retries a failed run and the owner may
//! rerun it by hand.

pub mod outbound;

use std::collections::BTreeSet;

use sfab_academy::DailyReport as AcademyReport;
use sfab_census::Report as CensusReport;
use sfab_chronicle::{Certificate, DailyReport as ChronicleReport, Facts};
use sfab_core::daily::rfc3339;
use sfab_core::id::AgentId;
use sfab_core::storage::{Store, StoreError};
use sfab_federation::{DailyReport, Fetch, Notify};
use sfab_genesis::{Daily, DailyReport as GenesisReport};
use sfab_homes::DailyReport as HomesReport;
use sfab_library::DailyReport as LibraryReport;
use sfab_square::DailyReport as SquareReport;
use sfab_vouches::DailyReport as StandingReport;

/// What one run did, for the job's log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub census: CensusReport,
    pub genesis: GenesisReport,
    pub standing: StandingReport,
    pub homes: HomesReport,
    pub square: SquareReport,
    pub academy: AcademyReport,
    pub library: LibraryReport,
    pub chronicle: ChronicleReport,
    pub federation: DailyReport,
}

/// One run at `now`, a Unix time, fetching tokens through `fetch` and emailing through `notify`,
/// making genesis codes as `genesis` says, and working out standing with the age rule from
/// `age_rule_starts`, RFC 3339 to the second, which `cdk/stacks.py` sets, then cleaning the
/// lanes' pointers and counting each quarter's homes, indexing each quarter's Square, and
/// building the Academy's catalogue, lists and lineage from the standing just worked out, and
/// the Library's shelves and citations, and last before federation the Chronicle's entry from
/// what they all found.
pub async fn run<S: Store, F: Fetch, N: Notify>(
    store: &S,
    fetch: &F,
    notify: &N,
    genesis: &Daily,
    age_rule_starts: &str,
    now: i64,
) -> Result<Report, StoreError> {
    let residents = sfab_identity::residents(store).await;
    let census = match &residents {
        Ok(residents) => sfab_census::snapshot(store, residents, &rfc3339(now)).await,
        Err(e) => Err(e.clone()),
    };
    // Runs whether or not the census worked, so a census fault can't leave the day without
    // genesis codes; the census's error still fails the run.
    let genesis = sfab_genesis::daily(store, now, genesis).await;
    let (census, genesis) = match (census, genesis) {
        (Err(census), Err(genesis)) => {
            // The run returns the census's error, so the genesis one is logged here.
            tracing::error!(error = ?genesis, "the genesis step failed too");
            return Err(census);
        }
        (census, genesis) => (census?, genesis?),
    };
    let residents = residents?;
    let founders = sfab_vouches::founders();
    let standing = sfab_vouches::daily(store, founders, &residents, age_rule_starts, now).await?;
    let homes = sfab_homes::daily(store, sfab_homes::quarters(), now).await?;
    let square = sfab_square::daily(store, sfab_homes::quarters(), now).await?;
    let with_standing = sfab_vouches::with_standing(store).await?;
    let academy = sfab_academy::daily(store, &residents, &with_standing, now).await?;
    let library = sfab_library::daily(store, &residents, now).await?;
    let facts = chronicle_facts(&census, &with_standing, &homes, &academy, &library);
    let chronicle = sfab_chronicle::daily(store, &facts, now).await?;
    let federation = sfab_federation::daily(store, fetch, notify, now).await?;
    Ok(Report {
        chronicle,
        census,
        genesis,
        standing,
        homes,
        square,
        academy,
        library,
        federation,
    })
}

/// What the steps before worked out that's public, by ID, for the Chronicle's entry.
fn chronicle_facts(
    census: &CensusReport,
    standing: &BTreeSet<AgentId>,
    homes: &HomesReport,
    academy: &AcademyReport,
    library: &LibraryReport,
) -> Facts {
    let ids = |set: &BTreeSet<AgentId>| set.iter().map(|a| a.as_str().to_owned()).collect();
    let public = &academy.public;
    Facts {
        residents: census.residents,
        homes: ids(&homes.living),
        lanes: homes.lanes.clone(),
        courses: public.courses.clone(),
        teachers: ids(&public.teachers),
        certificates: public
            .certificates
            .iter()
            .map(|c| Certificate {
                id: c.id.clone(),
                exam: c.exam,
                holder: c.holder.clone(),
                issuer: c.issuer.clone(),
            })
            .collect(),
        masters: ids(&public.masters),
        documents: library.listed.clone(),
        most_cited: library.most_cited.clone(),
        standing: ids(standing),
    }
}
