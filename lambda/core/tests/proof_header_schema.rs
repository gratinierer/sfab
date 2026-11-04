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
    // (what the case is for, header)
    let cases = [
        ("the worked example", "1793836800:614135".to_string()),
        ("the smallest timestamp", "0:a".to_string()),
        ("every nonce character class", "1:A-z_9".to_string()),
        (
            "timestamp at the digit limit",
            format!("{}:a", digits(MAX_TIMESTAMP_DIGITS)),
        ),
        (
            "timestamp one digit over",
            format!("{}:a", digits(MAX_TIMESTAMP_DIGITS + 1)),
        ),
        (
            "nonce at the length limit",
            format!("1:{}", nonce(MAX_NONCE_LEN)),
        ),
        (
            "nonce one character over",
            format!("1:{}", nonce(MAX_NONCE_LEN + 1)),
        ),
        ("empty header", String::new()),
        ("no colon", "1".to_string()),
        ("no timestamp", ":a".to_string()),
        ("no nonce", "1:".to_string()),
        ("a leading zero", "01:a".to_string()),
        ("negative timestamp", "-1:a".to_string()),
        ("fractional timestamp", "1.5:a".to_string()),
        ("a second colon", "1:a:b".to_string()),
        ("base64 padding", "1:a=".to_string()),
        ("a space in the nonce", "1:a b".to_string()),
        ("leading whitespace", " 1:a".to_string()),
        ("trailing newline", "1:a\n".to_string()),
        ("non-ASCII nonce", "1:é".to_string()),
    ];
    for (why, header) in &cases {
        let by_schema = pattern.is_match(header) && header.chars().count() as i64 <= max_length;
        let by_parser = Proof::parse(header).is_ok();
        assert_eq!(
            by_schema, by_parser,
            "schema and parser disagree on {why}: {header:?}"
        );
    }
}
