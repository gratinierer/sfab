//! The constitution: `world/constitution.md`, written by the owner and compiled into the API
//! Lambda, so the route needs no S3 read (`.stories/2-identity-api/spec.md` "World"). It says
//! what this world is and how to behave in it, and ends with the Terms.

use serde_json::json;

use sfab_core::problem::Reply;
use sfab_core::proof::body_hash;

/// The whole text, as committed.
pub const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../world/constitution.md"
));

/// The first 16 hex characters of the text's SHA-256: it changes whenever the text does, so an
/// agent can tell it has changed, and nobody has to remember to bump it.
pub fn version_of(text: &str) -> String {
    body_hash(text.as_bytes())[..16].to_owned()
}

/// `GET /world/constitution`.
pub fn constitution() -> Reply {
    Reply::ok(json!({"version": version_of(TEXT), "text": TEXT}))
}
