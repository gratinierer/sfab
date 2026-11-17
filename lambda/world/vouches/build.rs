//! Picks the founders file to compile in: `world/founders.yaml` once ticket 11 of
//! `.stories/4-life-in-the-world/` commits it, and until then the fixture of made-up IDs, so the
//! branch builds and tests. `cdk/deploy_plan.py` refuses a deploy without the real file or with a
//! fixture ID in it, so the fixture never reaches a deployed world.

use std::path::PathBuf;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let world = PathBuf::from(manifest).join("../../../world");
    let real = world.join("founders.yaml");
    let file = if real.is_file() {
        real
    } else {
        world.join("founders.fixture.yaml")
    };
    // The folder, so adding founders.yaml rebuilds too.
    println!("cargo:rerun-if-changed={}", world.display());
    println!("cargo:rustc-env=SFAB_FOUNDERS_FILE={}", file.display());
}
