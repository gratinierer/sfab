//! The world's compiled-in config files: names and lists, as `cdk/world_config.py` checks them at
//! synth (`.stories/4-life-in-the-world/spec.md` "Foundations").

use sfab_core::config::{self, NAME_MAX};

#[test]
fn a_name_is_1_to_64_characters_of_a_to_z_0_to_9_and_dash() {
    for good in [
        "harbour",
        "lantern-lane",
        "7-seas",
        "a",
        &"x".repeat(NAME_MAX),
    ] {
        assert!(config::is_name(good), "{good}");
    }
    let long = "x".repeat(NAME_MAX + 1);
    for bad in [
        "",
        "Harbour",
        "lantern_lane",
        "a/b",
        "a b",
        "é",
        "..",
        &long,
    ] {
        assert!(!config::is_name(bad), "{bad}");
    }
}

#[test]
fn names_reads_a_non_empty_list_of_names_each_once() {
    let text = "shelves: [craft, 7-seas]\n";

    assert_eq!(
        config::names(text, "shelves.yaml", "shelves"),
        Ok(vec!["craft".to_owned(), "7-seas".to_owned()])
    );
}

#[test]
fn names_refuses_a_bad_name_a_repeat_or_an_empty_or_missing_list() {
    for (text, named) in [
        ("shelves: [craft, Old_Books]\n", "Old_Books"),
        ("shelves: [craft, craft]\n", "craft"),
        ("shelves: []\n", "shelves"),
        ("other: [craft]\n", "shelves"),
        ("shelves: [craft\n", "shelves.yaml"),
    ] {
        let err = config::names(text, "shelves.yaml", "shelves").unwrap_err();
        assert!(err.contains(named), "{text:?}: {err}");
    }
}
