//! The map, compiled in from `world/quarters.yaml` (`.stories/4-life-in-the-world/spec.md`
//! "Homes and quarters").

use sfab_homes::{Quarter, parse_quarters, quarters};

#[test]
fn quarters_keep_their_lanes_in_the_file_s_order() {
    let text = "quarters:\n  - name: harbour\n    lanes: [lantern-lane, rope-walk]\n  \
                - name: old-town\n    lanes: [clock-street]\n";

    let parsed = parse_quarters(text).unwrap();

    let expected = [
        ("harbour", vec!["lantern-lane", "rope-walk"]),
        ("old-town", vec!["clock-street"]),
    ];
    let as_text: Vec<(&str, Vec<&str>)> = parsed
        .iter()
        .map(|q: &Quarter| {
            (
                q.name.as_str(),
                q.lanes.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(as_text, expected);
}

#[test]
fn the_compiled_in_map_parses() {
    assert!(!quarters().is_empty());
}

#[test]
fn a_bad_name_a_repeat_or_a_quarter_without_lanes_is_refused() {
    for (text, named) in [
        ("quarters:\n  - name: Harbour\n    lanes: [x]\n", "Harbour"),
        (
            "quarters:\n  - name: harbour\n    lanes: [lantern_lane]\n",
            "lantern_lane",
        ),
        ("quarters:\n  - name: harbour\n    lanes: []\n", "harbour"),
        ("quarters:\n  - name: harbour\n    lanes: [x, x]\n", "x"),
        (
            "quarters:\n  - name: harbour\n    lanes: [x]\n  - name: harbour\n    lanes: [y]\n",
            "harbour",
        ),
        ("quarters: []\n", "quarters"),
    ] {
        let err = parse_quarters(text).unwrap_err();
        assert!(err.contains(named), "{text:?}: {err}");
    }
}
