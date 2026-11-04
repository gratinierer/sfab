//! The `Proof` header's grammar is written twice: in `Proof::parse`, and in the `Proof` parameter of
//! `openapi/sfab.yaml`, which agents and the gateway read. This test fails if they drift apart.

use sfab_core::proof::{MAX_NONCE_LEN, MAX_TIMESTAMP_DIGITS};

/// The lines of `components.parameters.Proof`: from `    Proof:` up to the next key at the same indent.
fn proof_parameter(text: &str) -> String {
    let mut lines = text.lines().skip_while(|l| *l != "    Proof:");
    let first = lines
        .next()
        .expect("components.parameters.Proof in openapi/sfab.yaml");
    let body = lines.take_while(|l| l.is_empty() || l.starts_with("     "));
    std::iter::once(first)
        .chain(body)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_openapi_proof_parameter_matches_the_parser() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../openapi/sfab.yaml");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let proof = proof_parameter(&text);

    let max_length = MAX_TIMESTAMP_DIGITS + 1 + MAX_NONCE_LEN;
    let pattern =
        format!("\"^[0-9]{{1,{MAX_TIMESTAMP_DIGITS}}}:[A-Za-z0-9_-]{{1,{MAX_NONCE_LEN}}}$\"");
    assert!(
        proof
            .lines()
            .any(|l| l.trim() == format!("maxLength: {max_length}")),
        "the Proof parameter's maxLength isn't {max_length}:\n{proof}"
    );
    assert!(
        proof
            .lines()
            .any(|l| l.trim() == format!("pattern: {pattern}")),
        "the Proof parameter's pattern isn't {pattern}:\n{proof}"
    );
}
