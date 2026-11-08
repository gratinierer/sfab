//! The owner's genesis-codes tool: `.stories/17-genesis-codes/spec.md` "Two sources" (the owner)
//! and "Tracking". It reaches `genesis/` only through the `genesis` crate, so no key format lives
//! here (`docs/adr/0008-genesis-codes-get-a-crate-and-an-owner-tool.md`).

mod args;

pub use args::{Command, USAGE, parse_args};

use std::fmt::Write as _;

use sfab_core::daily::rfc3339;
use sfab_core::storage::{Store, StoreError};
use sfab_genesis::{Issue, Written, issue_new, read_summary};

/// The daily summary as text to print, with when the daily job wrote it, since its counts are as
/// of that run.
pub async fn status<S: Store>(store: &S) -> Result<String, StoreError> {
    let Some(Written { summary, at }) = read_summary(store).await? else {
        return Ok(
            "No summary yet: the daily job hasn't run since genesis codes arrived.\n".to_owned(),
        );
    };
    let mut text = format!(
        "Written by the daily job at {}; a batch issued since shows after its next run.\n",
        rfc3339(at)
    );
    let mut table = |title: &str, columns: [&str; 2], rows: Vec<(&String, usize, usize)>| {
        // The name column, two spaces of indent included, widens for a long label.
        let width = rows
            .iter()
            .map(|(name, ..)| name.len() + 3)
            .fold(16, usize::max);
        let [a, b] = columns;
        let _ = writeln!(text, "\n{title:<width$}{a:>8}{b:>7}");
        if rows.is_empty() {
            text.push_str("  none\n");
        }
        let name_width = width - 2;
        for (name, a, b) in rows {
            let _ = writeln!(text, "  {name:<name_width$}{a:>8}{b:>7}");
        }
    };
    table(
        "Owner batches",
        ["uploaded", "left"],
        summary
            .labels
            .iter()
            .map(|(label, b)| (label, b.uploaded, b.left))
            .collect(),
    );
    table(
        "World days",
        ["issued", "used"],
        summary
            .world
            .iter()
            .map(|(date, d)| (date, d.issued, d.used))
            .collect(),
    );
    Ok(text)
}

/// Makes `count` new codes and uploads each as `how` says, adding them to the batch `how.label`,
/// whether it exists or not. On an error, the codes before it are uploaded and work, but aren't
/// returned, so the owner issues the batch's rest again.
pub async fn issue_codes<S: Store>(
    store: &S,
    count: usize,
    how: &Issue,
) -> Result<Vec<String>, StoreError> {
    let mut codes = Vec::with_capacity(count);
    for _ in 0..count {
        codes.push(issue_new(store, how).await?.into());
    }
    Ok(codes)
}
