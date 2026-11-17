//! The shelves, compiled in from `world/shelves.yaml` (`.stories/4-life-in-the-world/spec.md`
//! "The Library").

use sfab_library::{parse_shelves, shelves};

#[test]
fn shelves_keep_the_file_s_order() {
    assert_eq!(
        parse_shelves("shelves: [craft, 7-seas]\n"),
        Ok(vec!["craft".to_owned(), "7-seas".to_owned()])
    );
}

#[test]
fn the_compiled_in_shelves_parse() {
    assert!(!shelves().is_empty());
}

#[test]
fn a_bad_shelf_name_is_refused() {
    let err = parse_shelves("shelves: [craft, 'old books']\n").unwrap_err();
    assert!(err.contains("old books"), "{err}");
}
