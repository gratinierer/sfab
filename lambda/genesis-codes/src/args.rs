//! The command line.

use sfab_core::daily::{DAY, utc_date};
use sfab_genesis::{Issue, WORLD_LABEL_PREFIX};

pub const USAGE: &str = "\
usage: genesis-codes issue --label <LABEL> --count <N> [--expires <DATE>] [--no-ip-limit]
       genesis-codes status

issue makes N genesis codes in the batch LABEL, uploads them and prints them, one a line. A label
is a-z, 0-9 and -, and doesn't start with world-, which the world's days use; a label that exists
already adds to its batch. Codes never expire unless --expires gives a UTC date, such as
2026-12-31, the last day they work. --no-ip-limit makes codes that skip the IP rule, for the smoke
script and test births.

status prints the daily summary: codes uploaded and left per label, and the world's codes issued
and used per day, as of the daily job's last run.

Needs IDENTITIES_BUCKET, CLAIMS_BUCKET and the owner's AWS credentials.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Make `count` codes, issued as `how` says.
    Issue {
        count: usize,
        how: Issue,
    },
    Status,
}

/// The arguments after the program's name.
pub fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut rest = args.iter().map(String::as_str);
    match rest.next() {
        Some("issue") => parse_issue(rest),
        Some("status") => match rest.next() {
            None => Ok(Command::Status),
            Some(extra) => Err(format!("status takes nothing, not {extra}")),
        },
        _ => Err(USAGE.to_owned()),
    }
}

fn parse_issue<'a>(mut rest: impl Iterator<Item = &'a str>) -> Result<Command, String> {
    let mut label = None;
    let mut count = None;
    let mut expires = None;
    let mut no_ip_limit = false;
    while let Some(arg) = rest.next() {
        match arg {
            "--label" => label = Some(check_label(rest.next().ok_or("--label takes a label")?)?),
            "--count" => {
                count = Some(
                    rest.next()
                        .and_then(|n| n.parse::<usize>().ok())
                        .filter(|n| *n > 0)
                        .ok_or("--count takes a whole number above 0")?,
                );
            }
            "--expires" => {
                let date = rest.next().ok_or("--expires takes a date")?;
                expires = Some(day_after(date).ok_or("--expires takes a date like 2026-12-31")?);
            }
            "--no-ip-limit" => no_ip_limit = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Command::Issue {
        count: count.ok_or("issue needs --count")?,
        how: Issue {
            label: label.ok_or("issue needs --label")?,
            expires,
            no_ip_limit,
        },
    })
}

fn check_label(label: &str) -> Result<String, String> {
    if label.is_empty()
        || !label
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err("a label is a-z, 0-9 and -".to_owned());
    }
    if label.starts_with(WORLD_LABEL_PREFIX) {
        return Err(format!(
            "labels starting {WORLD_LABEL_PREFIX} are the world's days"
        ));
    }
    Ok(label.to_owned())
}

/// 00:00 UTC after the date `YYYY-MM-DD`, as a Unix time: the first moment a code that works
/// through that day no longer does. `None` for anything that isn't a real date.
fn day_after(date: &str) -> Option<i64> {
    let b = date.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = &date[range];
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    // Civil date to days: Howard Hinnant's `days_from_civil`, the inverse of `rfc3339`'s.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let start = (era * 146_097 + doe - 719_468) * DAY;
    // Out-of-range months and days, such as 2026-02-30, come back as another date.
    (utc_date(start) == date).then_some(start + DAY)
}
