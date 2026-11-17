//! The founders, compiled in from `world/founders.yaml`, or until ticket 11 commits it, from
//! `world/founders.fixture.yaml` (`.stories/4-life-in-the-world/spec.md` "Vouches").

use sfab_core::id::AgentId;
use sfab_vouches::{MIN_FOUNDERS, founders, parse_founders};

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../world/founders.fixture.yaml"
));

fn ids(list: &[&str]) -> Vec<AgentId> {
    list.iter().map(|t| AgentId::parse(t).unwrap()).collect()
}

#[test]
fn the_fixture_holds_three_made_up_founders() {
    assert_eq!(
        parse_founders(FIXTURE),
        Ok(ids(&["foundera2222", "founderb2222", "founderc2222"]))
    );
}

#[test]
fn the_compiled_in_founders_parse() {
    assert!(founders().len() >= MIN_FOUNDERS);
}

#[test]
fn fewer_than_three_founders_a_bad_id_or_a_repeat_are_refused() {
    for (text, named) in [
        ("founders: [foundera2222, founderb2222]\n", "at least 3"),
        (
            "founders: [foundera2222, founderb2222, Founderc2222]\n",
            "Founderc2222",
        ),
        (
            "founders: [foundera2222, founderb2222, founderb2222]\n",
            "founderb2222",
        ),
        ("founders: foundera2222\n", "founders"),
    ] {
        let err = parse_founders(text).unwrap_err();
        assert!(err.contains(named), "{text:?}: {err}");
    }
}
