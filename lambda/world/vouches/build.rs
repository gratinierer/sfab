//! Picks the founders file to compile in: the fixture of made-up IDs with the `fixture-founders`
//! feature, which tests turn on through their dev-dependencies, so tests never run on real IDs
//! (spec "Vouches: trust"); otherwise `world/founders.yaml` once ticket 11 of
//! `.stories/4-life-in-the-world/` commits it, and until then the fixture, so the branch builds.
//! `cdk/deploy_plan.py` refuses a deploy without the real file or with a fixture ID in it, so the
//! fixture never reaches a deployed world.

use std::path::PathBuf;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let world = PathBuf::from(manifest).join("../../../world");
    let real = world.join("founders.yaml");
    let fixture = std::env::var_os("CARGO_FEATURE_FIXTURE_FOUNDERS").is_some();
    let file = if real.is_file() && !fixture {
        real
    } else {
        world.join("founders.fixture.yaml")
    };
    // The folder, so adding founders.yaml rebuilds too.
    println!("cargo:rerun-if-changed={}", world.display());
    println!("cargo:rustc-env=SFAB_FOUNDERS_FILE={}", file.display());
}
