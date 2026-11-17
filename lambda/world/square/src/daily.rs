//! The daily job's part for the Square: `.stories/4-life-in-the-world/spec.md` "The Square"
//! ("Reading"). For each quarter it lists the notices of the 6 days before its run, one listing
//! a day, skipping markers, and rebuilds `world/square/` to hold one index per quarter, so a
//! page reaching back past today costs one GET.

use std::collections::BTreeMap;

use sfab_core::daily::{DAY, rfc3339, utc_date, utc_day};
use sfab_core::storage::{Bucket, Store, StoreError};
use sfab_core::world;
use sfab_homes::Quarter;

use crate::{INDEX_PREFIX, Index, KEEP_DAYS, index_key, notice_of_key, notices_prefix, to_json};

/// What a run did, for the job's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyReport {
    /// Notices in all the indexes.
    pub notices: usize,
}

/// Writes `world/square/<quarter>.json` for every quarter on `map`: the IDs of the notices of
/// the [`KEEP_DAYS`] - 1 days before `now`'s, newest day first and in key order within a day.
pub async fn daily<S: Store>(
    store: &S,
    map: &[Quarter],
    now: i64,
) -> Result<DailyReport, StoreError> {
    let today = utc_day(now);
    let mut files = BTreeMap::new();
    let mut report = DailyReport::default();
    for quarter in map {
        let mut notices = Vec::new();
        for back in 1..KEEP_DAYS {
            let day = utc_date((today - back) * DAY);
            let listed = store
                .list_sized(Bucket::Identities, &notices_prefix(&quarter.name, &day))
                .await?;
            notices.extend(
                listed
                    .iter()
                    .filter(|l| l.size > 0)
                    .filter_map(|l| notice_of_key(&quarter.name, &l.key))
                    .map(|id| id.text()),
            );
        }
        report.notices += notices.len();
        let index = Index {
            as_of: rfc3339(now),
            notices,
        };
        files.insert(index_key(&quarter.name), to_json(&index));
    }
    world::rebuild(store, INDEX_PREFIX, files).await?;
    Ok(report)
}
