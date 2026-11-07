//! Nothing that handles a request calls a server outside AWS (`.stories/2-identity-api/spec.md`
//! "Federation" ("Outbound calls")): the API Lambda and the crates its routes use never depend on
//! the daily world job, which holds the one outbound check, or on an HTTP client of their own.
//! The AWS SDK's own client reaches only AWS.

/// The `[dependencies]` names of a crate's manifest, relative to this crate.
fn dependencies(manifest: &str) -> Vec<String> {
    let path = format!("{}/{manifest}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut in_deps = false;
    let mut names = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]";
        } else if in_deps && let Some((name, _)) = line.split_once('=') {
            names.push(name.trim().to_owned());
        }
    }
    names
}

const CALLS_OUT: &[&str] = &[
    "sfab-job",
    "hyper",
    "hyper-util",
    "reqwest",
    "rustls",
    "tokio-rustls",
    "ureq",
];

#[test]
fn the_api_and_its_route_crates_bring_no_outbound_client() {
    for manifest in [
        "Cargo.toml",
        "../core/Cargo.toml",
        "../identity/Cargo.toml",
        "../world/census/Cargo.toml",
        "../world/constitution/Cargo.toml",
        "../world/federation/Cargo.toml",
    ] {
        let deps = dependencies(manifest);
        assert!(!deps.is_empty(), "{manifest} has dependencies");
        for name in CALLS_OUT {
            assert!(
                !deps.iter().any(|d| d == name),
                "{manifest} depends on {name}"
            );
        }
    }
}
