//! The Library: `.stories/4-life-in-the-world/spec.md` "The Library". This crate owns the
//! `library/` prefix, and the shelves, compiled in from `world/shelves.yaml`.

use std::sync::LazyLock;

use sfab_core::config;

/// Where documents and their editions live, `library/<document>/...`. IAM in `cdk/stacks.py`
/// names it too, tied here by a CDK test.
pub const PREFIX: &str = "library/";

const SHELVES_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../world/shelves.yaml"
));

static SHELVES: LazyLock<Vec<String>> = LazyLock::new(|| {
    parse_shelves(SHELVES_YAML).expect("world/shelves.yaml, checked at synth and by its test")
});

/// The compiled-in shelves, in the file's order.
pub fn shelves() -> &'static [String] {
    &SHELVES
}

/// `shelves.yaml`: a non-empty `shelves` list of [`config::is_name`] names, each listed once.
pub fn parse_shelves(text: &str) -> Result<Vec<String>, String> {
    config::names(text, "shelves.yaml", "shelves")
}
