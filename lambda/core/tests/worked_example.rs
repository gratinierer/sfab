//! The one worked proof-of-work example, written once: the API's own verifier must accept it
//! and give exactly these values, and the public API description must show exactly them, so a
//! wrong or drifted example fails CI. `.stories/10-website/` issue 02 extends this to both
//! website pages.

use sfab_core::id::Caller;
use sfab_core::proof::{self, Request};

const BODY: &str = r#"{"name":"Wren","whoiam":"A small agent looking for a home.","genesis_code":"k7m2xq4tnb5w3hpa"}"#;
const TIMESTAMP: i64 = 1_793_836_800; // 2026-11-05T00:00:00Z
const NONCE: &str = "696785";
const BODY_HASH: &str = "8629a64dbb5cd4281e5c294d44d0cb89ec25edd32952d3e12ebf917c37474ac0";
const PREIMAGE: &str = "sfab:POST:/v1/identity/birth:1793836800:8629a64dbb5cd4281e5c294d44d0cb89ec25edd32952d3e12ebf917c37474ac0:-:696785";
const HASH: &str = "00000a95c3622796c2c8f0e1f1583e874e50aefe89cbffac6cbecb0958526fe4";
const ZERO_BITS: u32 = 20;
const ID: &str = "zpwlbfmfe37e";
/// The usual `required_bits`, as the description says.
const USUAL_BITS: u32 = 20;

const REQUEST: Request = Request {
    method: "POST",
    path: "/v1/identity/birth",
    caller: &Caller::Anonymous,
    body: BODY.as_bytes(),
};

#[test]
fn the_verifier_accepts_the_example_with_exactly_its_values() {
    let header = format!("{TIMESTAMP}:{NONCE}");
    let checked = proof::check_header(Some(&header), TIMESTAMP).expect("a sound header");
    let hash = proof::check_hash(&REQUEST, &checked, USUAL_BITS).expect("valid proof");
    assert_eq!(BODY.len(), 94);
    assert_eq!(proof::body_hash(REQUEST.body), BODY_HASH);
    let parsed = proof::Proof::parse(&header).unwrap();
    assert_eq!(proof::preimage(&REQUEST, &parsed), PREIMAGE);
    assert_eq!(proof::hex(&hash), HASH);
    assert_eq!(proof::leading_zero_bits(&hash), ZERO_BITS);
    assert_eq!(proof::agent_id(&hash).as_str(), ID);
}

#[test]
fn the_nonce_is_the_first_that_works() {
    // The description says counting up from 0 finds this nonce first.
    let first = (0..)
        .find(|n: &u32| {
            let nonce = n.to_string();
            let p = proof::Proof {
                timestamp: TIMESTAMP,
                nonce: &nonce,
            };
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
        "94 bytes".to_string(),
        format!("N is usually {USUAL_BITS}"),
    ] {
        assert!(
            description.contains(&value),
            "the description lacks {value:?}"
        );
    }
}

#[test]
fn both_website_pages_show_exactly_these_values() {
    // The committed pages, not the generated ones: the example holds no placeholder.
    for page in ["index.html", "agents/index.html"] {
        let path = format!("{}/../../website/{page}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        // The example sits in the proof-of-work section's <details> block.
        // The Python page tests parse the section; here plain slicing is enough, so long as the
        // section holds exactly one <details>, which this checks, so the slice can't take another.
        let start = text.find(r#"<section id="proof-of-work""#).expect("the section");
        let section = &text[start..];
        let section = &section[..section.find("</section>").expect("</section>")];
        assert_eq!(section.matches("<details").count(), 1, "{page}: one <details> in the section");
        let details = &section[section.find("<details>").expect("<details>")..];
        let details = &details[..details.find("</details>").expect("</details>")];
        for value in [
            format!("<code>{BODY}</code>"),
            format!("<code>{BODY_HASH}</code>"),
            format!("<code>{TIMESTAMP}</code>"),
            format!("<code>{NONCE}</code>"),
            format!("<code>{PREIMAGE}</code>"),
            format!("<code>{HASH}</code>"),
            format!("{ZERO_BITS} zero bits"),
            format!("<code>Proof: {TIMESTAMP}:{NONCE}</code>"),
            format!("<code>{ID}</code>"),
            "94 bytes".to_string(),
        ] {
            assert!(details.contains(&value), "{page} lacks {value:?}");
        }
    }
}
