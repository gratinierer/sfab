//! The `Proof` header's grammar is written twice: in `Proof::parse`, and in the `Proof` parameter of
//! `openapi/sfab.yaml`, which agents and the gateway read. These tests fail if they drift apart,
//! in what they accept as well as in their limits.

use regex::Regex;
use saphyr::{LoadableYamlNode, Yaml};
use sfab_core::proof::{MAX_NONCE_LEN, MAX_TIMESTAMP_DIGITS, Proof};

/// `components.parameters.Proof.schema` from `openapi/sfab.yaml`, as (`pattern`, `maxLength`).
fn proof_schema() -> (String, i64) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../openapi/sfab.yaml");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let docs = Yaml::load_from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
    let schema = &docs[0]["components"]["parameters"]["Proof"]["schema"];
    let pattern = schema["pattern"]
        .as_str()
        .expect("Proof schema pattern")
        .to_string();
    let max_length = schema["maxLength"]
        .as_integer()
        .expect("Proof schema maxLength");
    (pattern, max_length)
}

#[test]
fn the_max_length_is_the_parser_limits() {
    let (_, max_length) = proof_schema();
    let expected = MAX_TIMESTAMP_DIGITS + 1 + MAX_NONCE_LEN;
    assert_eq!(
        max_length, expected as i64,
        "the Proof parameter's maxLength"
    );
}

#[test]
fn the_pattern_accepts_exactly_what_the_parser_accepts() {
    let (pattern, max_length) = proof_schema();
    let pattern = Regex::new(&pattern).expect("the Proof pattern compiles");
    let digits = |n| "9".repeat(n);
    let nonce = |n| "a".repeat(n);
    let headers = [
        "1793836800:614135".to_string(),
        "0:a".to_string(),
        "1:A-z_9".to_string(),
        format!("{}:a", digits(MAX_TIMESTAMP_DIGITS)),
        format!("{}:a", digits(MAX_TIMESTAMP_DIGITS + 1)),
        format!("1:{}", nonce(MAX_NONCE_LEN)),
        format!("1:{}", nonce(MAX_NONCE_LEN + 1)),
        String::new(),
        "1".to_string(),
        ":a".to_string(),
        "1:".to_string(),
        "-1:a".to_string(),
        "1.5:a".to_string(),
        "1:a:b".to_string(),
        "1:a=".to_string(),
        "1:a b".to_string(),
        " 1:a".to_string(),
        "1:a\n".to_string(),
        "1:é".to_string(),
    ];
    for header in &headers {
        let by_schema = pattern.is_match(header) && header.chars().count() as i64 <= max_length;
        let by_parser = Proof::parse(header).is_ok();
        assert_eq!(
            by_schema, by_parser,
            "schema and parser disagree on {header:?}"
        );
    }
}
