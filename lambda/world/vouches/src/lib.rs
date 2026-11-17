//! Vouches and standing: `.stories/4-life-in-the-world/spec.md` "Vouches: trust" and
//! `docs/adr/0002-life-in-the-world.md`. This crate owns the `vouches/` prefix, and the founders,
//! from whom all standing flows: compiled in from `world/founders.yaml`, or until ticket 11
//! commits it, from `world/founders.fixture.yaml` (see `build.rs`).

use std::sync::LazyLock;

use sfab_core::config;
use sfab_core::id::AgentId;

/// Where each agent's given vouches live, `vouches/<id>.json`. IAM in `cdk/stacks.py` names it
/// too, tied here by a CDK test.
pub const PREFIX: &str = "vouches/";

/// The fewest founders a world may have: with fewer, nobody else could ever gain standing.
/// `cdk/world_config.py` checks the same at synth.
pub const MIN_FOUNDERS: usize = 3;

const FOUNDERS_YAML: &str = include_str!(env!("SFAB_FOUNDERS_FILE"));
const FOUNDERS_FILE: &str = "founders.yaml";

static FOUNDERS: LazyLock<Vec<AgentId>> = LazyLock::new(|| {
    parse_founders(FOUNDERS_YAML).expect("the founders file, checked at synth and by its test")
});

/// The compiled-in founders, in the file's order.
pub fn founders() -> &'static [AgentId] {
    &FOUNDERS
}

/// `founders.yaml`: a `founders` list of at least [`MIN_FOUNDERS`] agent IDs, each listed once.
pub fn parse_founders(text: &str) -> Result<Vec<AgentId>, String> {
    let doc = config::document(text, FOUNDERS_FILE)?;
    let items = config::list(&doc, FOUNDERS_FILE, "founders")?;
    let ids = config::strings(items, FOUNDERS_FILE, "founder", |t| {
        AgentId::parse(t).is_some()
    })?;
    if ids.len() < MIN_FOUNDERS {
        return Err(format!(
            "{FOUNDERS_FILE}: needs at least {MIN_FOUNDERS} founders, found {}",
            ids.len()
        ));
    }
    Ok(ids
        .iter()
        .map(|t| AgentId::parse(t).expect("checked above"))
        .collect())
}
