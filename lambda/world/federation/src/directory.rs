//! The compiled-in directory: this world's name from `federation/world.yaml`, and the approved
//! worlds from `federation/worlds.yaml`, which the owner edits, commits and deploys to approve a
//! world. There's no approval route to attack.

mod files;

use std::sync::LazyLock;

use serde_json::json;

use sfab_core::paging::{OFFSET_PAGE, offset_page};
use sfab_core::problem::{Problem, Reply};

pub use files::{Approved, parse_world, parse_worlds};

/// The route's path, which `next` repeats with `?page=`.
pub const WORLDS_PATH: &str = "/v1/federation/worlds";

const WORLD_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../federation/world.yaml"
));
const WORLDS_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../federation/worlds.yaml"
));

// build.rs refuses to build with a file that doesn't parse, so these never panic in a build.
static NAME: LazyLock<String> = LazyLock::new(|| {
    parse_world(WORLD_YAML).expect("federation/world.yaml, checked by build.rs and its test")
});
static APPROVED: LazyLock<Vec<Approved>> = LazyLock::new(|| {
    parse_worlds(WORLDS_YAML).expect("federation/worlds.yaml, checked by build.rs and its test")
});

/// This world's name, from `federation/world.yaml`.
pub fn world_name() -> &'static str {
    &NAME
}

/// The approved worlds, in the file's order.
pub fn approved() -> &'static [Approved] {
    &APPROVED
}

/// One page of approved worlds.
pub fn page(worlds: &[Approved], token: Option<&str>) -> Result<Reply, Problem> {
    let page = offset_page(worlds.len(), token, OFFSET_PAGE, WORLDS_PATH)?;
    let items: Vec<_> = worlds[page.range]
        .iter()
        .map(|a| json!({"domain": a.domain}))
        .collect();
    Ok(Reply::ok(json!({"items": items, "next": page.next})))
}
