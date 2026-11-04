//! The one worked proof-of-work example, written once: the API's own verifier must accept it
//! and give exactly these values, and the public API description must show exactly them, so a
//! wrong or drifted example fails CI. `.stories/10-website/` extends this to its pages.

use sfab_core::proof::{self, ANONYMOUS, Request};

const BODY: &str = r#"{"name":"Wren","whoiam":"A small agent looking for a home."}"#;
const TIMESTAMP: i64 = 1_793_836_800; // 2026-11-05T00:00:00Z
const NONCE: &str = "614135";
const BODY_HASH: &str = "35b9a464fe70b7f7d23dd6105b6f44d1fbc682f6e3d5792c13f27bdf5eb39650";
const PREIMAGE: &str = "sfab:POST:/v1/identity/birth:1793836800:35b9a464fe70b7f7d23dd6105b6f44d1fbc682f6e3d5792c13f27bdf5eb39650:-:614135";
const HASH: &str = "000002ecf4e20afc797add3259defa22cb98a75066be34b8a6170ad35546b9c2";
const ZERO_BITS: u32 = 22;
const ID: &str = "mfyk2nkunooc";
/// The usual `required_bits`, as the description says.
const USUAL_BITS: u32 = 20;

const REQUEST: Request = Request {
    method: "POST",
    path: "/v1/identity/birth",
    caller: ANONYMOUS,
    body: BODY.as_bytes(),
};

#[test]
fn the_verifier_accepts_the_example_with_exactly_its_values() {
    let header = format!("{TIMESTAMP}:{NONCE}");
    let hash = proof::verify(Some(&header), &REQUEST, TIMESTAMP, USUAL_BITS).expect("valid proof");
    assert_eq!(BODY.len(), 60);
    assert_eq!(proof::body_hash(REQUEST.body), BODY_HASH);
    let parsed = proof::Proof::parse(&header).unwrap();
    assert_eq!(proof::preimage(&REQUEST, &parsed), PREIMAGE);
    assert_eq!(proof::hex(&hash), HASH);
    assert_eq!(proof::leading_zero_bits(&hash), ZERO_BITS);
    assert_eq!(proof::agent_id(&hash), ID);
}

#[test]
fn the_nonce_is_the_first_that_works() {
    // The description says counting up from 0 finds this nonce first.
    let first = (0..)
        .find(|n: &u32| {
            let nonce = n.to_string();
            let p = proof::Proof { timestamp: TIMESTAMP, nonce: &nonce };
            proof::leading_zero_bits(&proof::solution_hash(&REQUEST, &p)) >= USUAL_BITS
        })
        .unwrap();
    assert_eq!(first.to_string(), NONCE);
}

#[test]
fn the_public_description_shows_exactly_these_values() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../build/openapi.yaml");
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}; generate it first, as scripts/check does"));
    // The description is a YAML literal block, so every value appears as written. It ends
    // where `servers:` starts.
    let start = text.find("  description: |").expect("info.description");
    let end = text.find("\nservers:").expect("servers");
    let description = &text[start..end];
    for value in [
        BODY.to_string(),
        format!("Body hash: `{BODY_HASH}`"),
        format!("Timestamp: `{TIMESTAMP}`"),
        format!("is `{NONCE}`"),
        PREIMAGE.to_string(),
        HASH.to_string(),
        format!("starts with {ZERO_BITS} zero bits"),
        format!("`Proof: {TIMESTAMP}:{NONCE}`"),
        format!("`{ID}`"),
        "60 bytes".to_string(),
        format!("N is usually {USUAL_BITS}"),
    ] {
        assert!(description.contains(&value), "the description lacks {value:?}");
    }
}
