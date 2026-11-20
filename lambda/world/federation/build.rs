//! Refuses to build with a `federation/world.yaml` or `federation/worlds.yaml` that doesn't parse,
//! with the rules the crate itself applies (`src/directory/files.rs`, included here rather than
//! copied). The deploy runs no `cargo test`, only `scripts/build`, whose Lambda build compiles
//! this crate, so a bad edit fails the deploy instead of panicking on the first request
//! (`.stories/19-deploy-build-cache/spec.md` Goal 2).

#[path = "src/domain.rs"]
mod domain;
#[path = "src/directory/files.rs"]
#[allow(dead_code)]
mod files;

use std::path::{Path, PathBuf};

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let federation = PathBuf::from(manifest).join("../../../federation");
    for rules in ["src/domain.rs", "src/directory/files.rs"] {
        println!("cargo:rerun-if-changed={rules}");
    }
    check(&federation, "world.yaml", files::parse_world);
    check(&federation, "worlds.yaml", files::parse_worlds);
}

/// Panics, failing the build, when `federation/<name>` can't be read or `parse` refuses it.
fn check<T>(federation: &Path, name: &str, parse: fn(&str) -> Result<T, String>) {
    let path = federation.join(name);
    println!("cargo:rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()));
    if let Err(problem) = parse(&text) {
        panic!("federation/{name} doesn't parse: {problem}");
    }
}
