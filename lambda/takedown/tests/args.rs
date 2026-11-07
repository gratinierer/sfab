//! The program's command line: two modes, a dry run until `--execute`.

use sfab_takedown::{Args, Mode, parse_args};

fn parse(line: &str) -> Result<Args, String> {
    let args: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
    parse_args(&args)
}

#[test]
fn one_id_is_a_dry_run_with_the_default_wait() {
    let args = parse("id abcdefghijkl").unwrap();
    assert!(matches!(&args.mode, Mode::One(id) if id.as_str() == "abcdefghijkl"));
    assert!(!args.execute);
    assert_eq!(args.wait_seconds, 60);
}

#[test]
fn a_window_with_execute_and_a_wait() {
    let args =
        parse("window 2026-11-05T12:00:00Z 2026-11-05T14:00:00Z --execute --wait-seconds 90")
            .unwrap();
    assert!(matches!(
        &args.mode,
        Mode::Window { from, to } if from == "2026-11-05T12:00:00Z" && to == "2026-11-05T14:00:00Z"
    ));
    assert!(args.execute);
    assert_eq!(args.wait_seconds, 90);
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
        "id abcdefghijkl --force",
    ] {
        assert!(parse(line).is_err(), "{line:?} should be refused");
    }
}
