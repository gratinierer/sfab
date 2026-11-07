//! Which domains may ask to federate: a lowercase public hostname, so it can name a key under
//! `federation/` and a URL the daily world job fetches.

/// Top-level names that can never be another world on the public internet (RFC 6761, RFC 6762,
/// RFC 7686 and ICANN's `internal`), refused before the job could ever resolve them.
const SPECIAL_USE: &[&str] = &[
    "arpa",
    "example",
    "internal",
    "invalid",
    "local",
    "localhost",
    "onion",
    "test",
];

/// At least two labels of `a-z`, `0-9` and inner `-`, each 1 to 63 characters, 253 in all, with
/// a top-level label that isn't all digits, so an IPv4 address never passes, and isn't a
/// special-use name. No trailing dot, port, path or upper case.
pub fn valid_domain(domain: &str) -> bool {
    let labels: Vec<&str> = domain.split('.').collect();
    let label_ok = |l: &&str| {
        (1..=63).contains(&l.len())
            && l.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !l.starts_with('-')
            && !l.ends_with('-')
    };
    let top = labels.last().copied().unwrap_or_default();
    domain.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(label_ok)
        && !top.bytes().all(|b| b.is_ascii_digit())
        && !SPECIAL_USE.contains(&top)
}
