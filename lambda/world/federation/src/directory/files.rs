//! The rules of `federation/world.yaml` and `federation/worlds.yaml`. The crate's `build.rs`
//! includes this file too, so building the Lambdas fails on a committed file that breaks them,
//! instead of the statics in `directory.rs` panicking on the first request
//! (`.stories/19-deploy-build-cache/spec.md` Goal 2). It uses only `saphyr` and `crate::domain`,
//! which the build script includes as well.

use saphyr::{LoadableYamlNode, Yaml};

use crate::domain::valid_domain;

/// One approved world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    pub domain: String,
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
