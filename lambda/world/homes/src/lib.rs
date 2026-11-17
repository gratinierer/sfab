//! Homes, quarters and lanes: `.stories/4-life-in-the-world/spec.md` "Homes and quarters". This
//! crate owns the `places/` prefix, and the map, compiled in from `world/quarters.yaml`.

use std::sync::LazyLock;

use saphyr::Yaml;
use sfab_core::config;

/// Where the lane pointers live, `places/<quarter>/<lane>/<number>.json`. IAM in
/// `cdk/stacks.py` names it too, tied here by a CDK test.
pub const PREFIX: &str = "places/";

const QUARTERS_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../world/quarters.yaml"
));
const QUARTERS_FILE: &str = "quarters.yaml";

/// One quarter, with the lane names the world opens in this order as it fills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarter {
    pub name: String,
    pub lanes: Vec<String>,
}

static QUARTERS: LazyLock<Vec<Quarter>> = LazyLock::new(|| {
    parse_quarters(QUARTERS_YAML).expect("world/quarters.yaml, checked at synth and by its test")
});

/// The compiled-in quarters, in the file's order.
pub fn quarters() -> &'static [Quarter] {
    &QUARTERS
}

/// `quarters.yaml`: a `quarters` list of `{name, lanes}`, each name a [`config::is_name`],
/// each quarter listed once with a non-empty list of lanes, each listed once in it.
pub fn parse_quarters(text: &str) -> Result<Vec<Quarter>, String> {
    let doc = config::document(text, QUARTERS_FILE)?;
    let mut found: Vec<Quarter> = Vec::new();
    for item in config::list(&doc, QUARTERS_FILE, "quarters")? {
        let name = item.as_mapping_get("name").map(Yaml::as_str);
        let name = match name {
            Some(Some(name)) if config::is_name(name) => name.to_owned(),
            Some(Some(name)) => {
                return Err(format!("{QUARTERS_FILE}: quarter {name:?} isn't valid"));
            }
            _ => return Err(format!("{QUARTERS_FILE}: each quarter needs a `name`")),
        };
        if found.iter().any(|q| q.name == name) {
            return Err(format!("{QUARTERS_FILE}: quarter {name} is listed twice"));
        }
        let lanes = match item.as_mapping_get("lanes") {
            Some(Yaml::Sequence(lanes)) if !lanes.is_empty() => lanes,
            _ => {
                return Err(format!(
                    "{QUARTERS_FILE}: quarter {name} needs a non-empty `lanes` list"
                ));
            }
        };
        let lanes = config::strings(lanes, QUARTERS_FILE, "lane", config::is_name)?;
        found.push(Quarter { name, lanes });
    }
    Ok(found)
}
