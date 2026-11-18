//! The program's command line: an agent by ID, a window, or one writing, a dry run until
//! `--execute`.

use sfab_takedown::{Args, Command, Mode, parse_args};

fn parse(line: &str) -> Result<Args, String> {
    let args: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
    parse_args(&args)
}

#[test]
fn one_id_is_a_dry_run_with_the_default_wait() {
    let args = parse("id abcdefghijkl").unwrap();
    assert!(matches!(&args.command, Command::Agents(Mode::One(id)) if id.as_str() == "abcdefghijkl"));
    assert!(!args.execute);
    assert_eq!(args.wait_seconds, 60);
}

#[test]
fn a_window_with_execute_and_a_wait() {
    let args =
        parse("window 2026-11-05T12:00:00Z 2026-11-05T14:00:00Z --execute --wait-seconds 90")
            .unwrap();
    assert!(matches!(
        &args.command,
        Command::Agents(Mode::Window { from, to }) if from == "2026-11-05T12:00:00Z" && to == "2026-11-05T14:00:00Z"
    ));
    assert!(args.execute);
    assert_eq!(args.wait_seconds, 90);
}

#[test]
fn the_wait_may_be_exactly_the_lambda_timeout() {
    assert_eq!(
        parse("id abcdefghijkl --wait-seconds 10")
            .unwrap()
            .wait_seconds,
        10
    );
}

#[test]
fn bad_lines_are_refused() {
    for line in [
        "",
        "id",
        "id ABCDEFGHIJKL",
        "id abcdefghijkl extra",
        "window 2026-11-05 2026-11-06",
        "window 2026-11-05T12:00:00+01:00 2026-11-05T14:00:00Z",
        "window 2026-11-05T14:00:00Z 2026-11-05T12:00:00Z",
        "window 2026-11-05T12:00:00Z",
        "id abcdefghijkl --wait-seconds",
        "id abcdefghijkl --wait-seconds soon",
        "id abcdefghijkl --wait-seconds 0",
        "id abcdefghijkl --wait-seconds 9",
        "id abcdefghijkl --force",
        "--writing",
        "id abcdefghijkl --writing library/abcdefghijkl/2.json",
    ] {
        assert!(parse(line).is_err(), "{line:?} should be refused");
    }
}

#[test]
fn a_writing_is_a_notice_an_edition_or_a_course_by_its_key() {
    for key in [
        "square/harbour/2026-11-05/notices/abcdefghijkl-1.json",
        "library/abcdefghijkl/document.json",
        "library/abcdefghijkl/2.json",
        "library/abcdefghijkl/5.json",
        "academy/courses/abcdefghijkl/course.json",
    ] {
        let args = parse(&format!("--writing {key}")).unwrap();
        assert!(matches!(&args.command, Command::Writing(k) if k == key), "{key}");
        assert!(!args.execute);
        assert!(parse(&format!("--writing {key} --execute")).unwrap().execute);
    }
    for key in [
        // An unknown quarter, a day that doesn't exist, and a notice's answer.
        "square/nowhere/2026-11-05/notices/abcdefghijkl-1.json",
        "square/harbour/2026-02-30/notices/abcdefghijkl-1.json",
        "square/harbour/2026-11-05/answers/abcdefghijkl-1/mossmossmoss.json",
        // Edition 1 is in `document.json`, and there are at most 5.
        "library/abcdefghijkl/1.json",
        "library/abcdefghijkl/6.json",
        "library/ABCDEFGHIJKL/2.json",
        "academy/courses/abcdefghijkl/",
        "academy/courses/abcdefghijkl/enrolled/mossmossmoss.json",
        "identities/abcdefghijkl/home.json",
        "../library/abcdefghijkl/2.json",
    ] {
        assert!(parse(&format!("--writing {key}")).is_err(), "{key}");
    }
}
