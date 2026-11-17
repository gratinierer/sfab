//! Reading the world's compiled-in config files, `world/founders.yaml`, `world/quarters.yaml`
//! and `world/shelves.yaml`, in the crates that own them (`.stories/4-life-in-the-world/spec.md`
//! "Foundations"). `cdk/world_config.py` checks the same rules at synth, so a broken file never
//! deploys; these parse what synth passed.

use saphyr::{LoadableYamlNode, Yaml};

/// The longest quarter, lane or shelf name. `cdk/world_config.py`'s `NAME` has the same limit,
/// which a CDK test ties to this one.
pub const NAME_MAX: usize = 64;

/// A quarter, lane or shelf name: 1 to [`NAME_MAX`] characters of `a-z`, `0-9` and `-`, so it's
/// safe as a storage key segment and a path parameter.
pub fn is_name(text: &str) -> bool {
    (1..=NAME_MAX).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The one YAML document in `text`.
pub fn document<'a>(text: &'a str, file: &str) -> Result<Yaml<'a>, String> {
    let mut docs = Yaml::load_from_str(text).map_err(|e| format!("{file}: {e}"))?;
    if docs.len() != 1 {
        return Err(format!("{file}: expected one YAML document"));
    }
    Ok(docs.remove(0))
}

/// The non-empty list under `key` of a mapping.
pub fn list<'a>(doc: &'a Yaml<'a>, file: &str, key: &str) -> Result<&'a [Yaml<'a>], String> {
    match doc.as_mapping_get(key) {
        Some(Yaml::Sequence(items)) if !items.is_empty() => Ok(items),
        _ => Err(format!("{file}: needs a non-empty `{key}` list")),
    }
}

/// Each item as a string that `valid` accepts, each listed once.
pub fn strings(
    items: &[Yaml<'_>],
    file: &str,
    kind: &str,
    valid: fn(&str) -> bool,
) -> Result<Vec<String>, String> {
    let mut found: Vec<String> = Vec::with_capacity(items.len());
    for item in items {
        let text = match item.as_str() {
            Some(text) if valid(text) => text,
            Some(text) => return Err(format!("{file}: {kind} {text:?} isn't valid")),
            None => return Err(format!("{file}: each {kind} must be a string")),
        };
        if found.iter().any(|f| f == text) {
            return Err(format!("{file}: {kind} {text} is listed twice"));
        }
        found.push(text.to_owned());
    }
    Ok(found)
}

/// The non-empty list of [`is_name`] names under `key` in `text`, each listed once.
pub fn names(text: &str, file: &str, key: &str) -> Result<Vec<String>, String> {
    let doc = document(text, file)?;
    strings(list(&doc, file, key)?, file, key, is_name)
}
