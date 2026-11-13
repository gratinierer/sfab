//! The tool's command line: `issue` and `status`, from `.stories/17-genesis-codes/spec.md` "Two
//! sources" (the owner).

use sfab_genesis::Issue;
use sfab_genesis_codes::{Command, parse_args};

fn parse(line: &str) -> Result<Command, String> {
    let args: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
    parse_args(&args)
}

#[test]
fn a_batch_never_expires_and_keeps_the_ip_rule_by_default() {
    assert_eq!(
        parse("issue --label forum-y --count 10").unwrap(),
        Command::Issue {
            count: 10,
            how: Issue {
                label: "forum-y".to_owned(),
                expires: None,
                no_ip_limit: false,
            },
        }
    );
}

#[test]
fn no_ip_limit_and_expires_reach_the_issue() {
    let Command::Issue { how, .. } =
        parse("issue --no-ip-limit --expires 2026-11-05 --count 1 --label smoke").unwrap()
    else {
        panic!("an issue");
    };
    assert!(how.no_ip_limit);
    // Works through 2026-11-05, stops at 2026-11-06T00:00:00Z.
    assert_eq!(how.expires, Some(1_793_923_200));
}

#[test]
fn an_expiry_on_the_last_of_february_in_a_leap_year() {
    let Command::Issue { how, .. } =
        parse("issue --label x --count 1 --expires 2028-02-29").unwrap()
    else {
        panic!("an issue");
    };
    // 2028-03-01T00:00:00Z.
    assert_eq!(how.expires, Some(1_835_481_600));
}

#[test]
fn status_takes_nothing() {
    assert_eq!(parse("status").unwrap(), Command::Status);
    assert!(parse("status --label x").is_err());
}

#[test]
fn bad_labels_are_refused() {
    for label in ["Forum", "forum_y", "forum.y", "forum/y", "über"] {
        let line = format!("issue --count 1 --label {label}");
        assert!(parse(&line).is_err(), "{label:?} should be refused");
    }
    let empty = vec!["issue", "--count", "1", "--label", ""];
    assert!(parse_args(&empty.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
}

#[test]
fn the_world_s_labels_are_refused() {
    let refused = parse("issue --label world-2026-11-05 --count 1").unwrap_err();
    assert!(refused.contains("world-"), "{refused}");
    assert!(parse("issue --label world-x --count 1").is_err());
    assert!(parse("issue --label myworld-x --count 1").is_ok());
}

#[test]
fn bad_lines_are_refused() {
    for line in [
        "",
        "issue",
        "issue --label x",
        "issue --count 3",
        "issue --label x --count 0",
        "issue --label x --count -1",
        "issue --label x --count many",
        "issue --label x --count",
        "issue --label x --count 1 --expires",
        "issue --label x --count 1 --expires 2026-02-30",
        "issue --label x --count 1 --expires 2026-11-05T00:00:00Z",
        "issue --label x --count 1 --expires 26-11-05",
        "issue --label x --count 1 --force",
        "issue --label x --count 1 extra",
        "takedown",
    ] {
        assert!(parse(line).is_err(), "{line:?} should be refused");
    }
}
