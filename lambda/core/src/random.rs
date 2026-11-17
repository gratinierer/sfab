//! Random picks from the operating system's generator, such as a genesis code from the pool or a
//! free home (`.stories/4-life-in-the-world/spec.md` "Homes and quarters").

/// A random index below `n`, which must not be 0.
pub fn below(n: usize) -> usize {
    assert!(n > 0, "a pick from nothing");
    let r = getrandom::u64().expect("the system's random source");
    (r % n as u64) as usize
}
