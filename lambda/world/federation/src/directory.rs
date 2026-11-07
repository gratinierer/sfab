//! The compiled-in directory: this world's name from `federation/world.yaml`, and the approved
//! worlds from `federation/worlds.yaml`, which the owner edits, commits and deploys to approve a
//! world. There's no approval route to attack.

use std::sync::LazyLock;

use saphyr::{LoadableYamlNode, Yaml};
use serde_json::json;

use sfab_core::paging::{OFFSET_PAGE, offset_page};
use sfab_core::problem::{Problem, Reply};

use crate::domain::valid_domain;

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

/// One approved world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    pub domain: String,
}

static NAME: LazyLock<String> =
    LazyLock::new(|| parse_world(WORLD_YAML).expect("federation/world.yaml, checked by its test"));
static APPROVED: LazyLock<Vec<Approved>> = LazyLock::new(|| {
    parse_worlds(WORLDS_YAML).expect("federation/worlds.yaml, checked by its test")
});

/// This world's name, from `federation/world.yaml`.
pub fn world_name() -> &'static str {
    &NAME
}

/// The approved worlds, in the file's order.
pub fn approved() -> &'static [Approved] {
    &APPROVED
}

/// `world.yaml`: a mapping with a non-empty `name` of at most 128 characters.
pub fn parse_world(text: &str) -> Result<String, String> {
    let doc = load(text)?;
    let name = doc
        .as_mapping_get("name")
        .and_then(Yaml::as_str)
        .ok_or("world.yaml needs a `name`")?
        .trim()
        .to_owned();
    if name.is_empty() || name.chars().count() > 128 {
        return Err("world.yaml's `name` must be 1 to 128 characters".into());
    }
    Ok(name)
}

/// `worlds.yaml`: a mapping whose `worlds` is a list of `{domain: <hostname>}`, each domain valid
/// and listed once. An empty list may be written `worlds: []`.
pub fn parse_worlds(text: &str) -> Result<Vec<Approved>, String> {
    let doc = load(text)?;
    let list = match doc.as_mapping_get("worlds") {
        Some(Yaml::Sequence(items)) => items.clone(),
        _ => return Err("worlds.yaml needs a `worlds` list, `worlds: []` when empty".into()),
    };
    let mut found: Vec<Approved> = Vec::new();
    for item in &list {
        let domain = item
            .as_mapping_get("domain")
            .and_then(Yaml::as_str)
            .ok_or("each world in worlds.yaml needs a `domain`")?;
        if !valid_domain(domain) {
            return Err(format!("worlds.yaml: {domain:?} isn't a valid domain"));
        }
        if found.iter().any(|a| a.domain == domain) {
            return Err(format!("worlds.yaml lists {domain} twice"));
        }
        found.push(Approved {
            domain: domain.to_owned(),
        });
    }
    Ok(found)
}

fn load(text: &str) -> Result<Yaml<'_>, String> {
    let mut docs = Yaml::load_from_str(text).map_err(|e| e.to_string())?;
    if docs.len() != 1 {
        return Err("expected one YAML document".into());
    }
    Ok(docs.remove(0))
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
