//! Proof of work on every `POST` and `PUT`: `.stories/2-identity-api/spec.md` "Proof of work"
//! and `docs/adr/0003-proof-on-every-write.md`.
//!
//! The client finds a nonce where
//! `SHA-256("sfab:<METHOD>:<path>:<unix-ts>:<body-hash>:<caller>:<nonce>")` has at least
//! `required_bits` leading zero bits, and sends `Proof: <unix-ts>:<nonce>`.

use sha2::{Digest, Sha256};

/// How old a proof's timestamp may be, in seconds.
pub const MAX_AGE_SECS: i64 = 5 * 60;
/// How far in the future a proof's timestamp may be, in seconds.
pub const MAX_AHEAD_SECS: i64 = 60;
/// The longest nonce, in base64url characters.
pub const MAX_NONCE_LEN: usize = 64;
/// `<caller>` for an anonymous `birth` or federation request.
pub const ANONYMOUS: &str = "-";

/// What the hash covers besides the `Proof` header itself.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// Upper case, such as `POST`.
    pub method: &'a str,
    /// The path as the client sent it, starting with `/v1/`, without its query.
    pub path: &'a str,
    /// The ID in `Authorization`, or [`ANONYMOUS`].
    pub caller: &'a str,
    /// The raw request body.
    pub body: &'a [u8],
}

/// A parsed `Proof` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proof<'a> {
    pub timestamp: i64,
    pub nonce: &'a str,
}

/// Why a proof was refused. Every case answers `400` with `required_bits` and the server's time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Missing,
    Malformed,
    Stale,
    Future,
    Insufficient,
}

impl<'a> Proof<'a> {
    /// Splits `<unix-ts>:<nonce>` on its one `:`.
    pub fn parse(header: &'a str) -> Result<Self, Rejection> {
        let (ts, nonce) = header.split_once(':').ok_or(Rejection::Malformed)?;
        let ts_ok = !ts.is_empty() && ts.len() <= 20 && ts.bytes().all(|b| b.is_ascii_digit());
        let nonce_ok = !nonce.is_empty()
            && nonce.len() <= MAX_NONCE_LEN
            && nonce.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        if !ts_ok || !nonce_ok {
            return Err(Rejection::Malformed);
        }
        let timestamp = ts.parse().map_err(|_| Rejection::Malformed)?;
        Ok(Self { timestamp, nonce })
    }
}

/// Lowercase hex SHA-256 of the raw body; an empty body hashes the empty string.
pub fn body_hash(body: &[u8]) -> String {
    hex(&Sha256::digest(body))
}

/// The exact string that is hashed.
pub fn preimage(request: &Request, proof: &Proof) -> String {
    format!(
        "sfab:{}:{}:{}:{}:{}:{}",
        request.method,
        request.path,
        proof.timestamp,
        body_hash(request.body),
        request.caller,
        proof.nonce
    )
}

/// The solution hash.
pub fn solution_hash(request: &Request, proof: &Proof) -> [u8; 32] {
    Sha256::digest(preimage(request, proof).as_bytes()).into()
}

pub fn leading_zero_bits(hash: &[u8]) -> u32 {
    let mut bits = 0;
    for &byte in hash {
        if byte != 0 {
            return bits + byte.leading_zeros();
        }
        bits += 8;
    }
    bits
}

/// Checks a `Proof` header with one hash, before anything is read or written, and returns the
/// solution hash, from which creates take their keys.
pub fn verify(
    header: Option<&str>,
    request: &Request,
    now: i64,
    required_bits: u32,
) -> Result<[u8; 32], Rejection> {
    let proof = Proof::parse(header.ok_or(Rejection::Missing)?)?;
    if proof.timestamp < now - MAX_AGE_SECS {
        return Err(Rejection::Stale);
    }
    if proof.timestamp > now + MAX_AHEAD_SECS {
        return Err(Rejection::Future);
    }
    let hash = solution_hash(request, &proof);
    if leading_zero_bits(&hash) < required_bits {
        return Err(Rejection::Insufficient);
    }
    Ok(hash)
}

/// An agent's ID: 12 lowercase RFC 4648 base32 characters from the last 60 bits of the solution
/// hash, because the first bits hold the proof's zeros.
pub fn agent_id(hash: &[u8; 32]) -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let tail = u64::from_be_bytes(hash[24..].try_into().expect("8 bytes"));
    (0..12)
        .map(|i| ALPHABET[((tail >> (5 * (11 - i))) & 31) as usize] as char)
        .collect()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQ: Request = Request {
        method: "POST",
        path: "/v1/identity/birth",
        caller: ANONYMOUS,
        body: b"{}",
    };

    #[test]
    fn parses_a_header() {
        assert_eq!(Proof::parse("17:aZ_-9"), Ok(Proof { timestamp: 17, nonce: "aZ_-9" }));
    }

    #[test]
    fn refuses_malformed_headers() {
        for h in ["", "17", ":x", "17:", "-1:x", "1.5:x", "17:a:b", "17:a=", "17:a b"] {
            assert_eq!(Proof::parse(h), Err(Rejection::Malformed), "{h:?}");
        }
        let long = format!("17:{}", "a".repeat(MAX_NONCE_LEN + 1));
        assert_eq!(Proof::parse(&long), Err(Rejection::Malformed));
        assert!(Proof::parse(&format!("17:{}", "a".repeat(MAX_NONCE_LEN))).is_ok());
    }

    #[test]
    fn empty_body_hashes_the_empty_string() {
        assert_eq!(
            body_hash(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn counts_leading_zero_bits() {
        assert_eq!(leading_zero_bits(&[0xff]), 0);
        assert_eq!(leading_zero_bits(&[0x00, 0x01]), 15);
        assert_eq!(leading_zero_bits(&[0x00, 0x00, 0x02]), 22);
        assert_eq!(leading_zero_bits(&[0; 4]), 32);
    }

    #[test]
    fn refuses_missing_stale_and_future_proofs_before_hashing() {
        let now = 1_000_000;
        assert_eq!(verify(None, &REQ, now, 0), Err(Rejection::Missing));
        let stale = format!("{}:x", now - MAX_AGE_SECS - 1);
        assert_eq!(verify(Some(&stale), &REQ, now, 0), Err(Rejection::Stale));
        let future = format!("{}:x", now + MAX_AHEAD_SECS + 1);
        assert_eq!(verify(Some(&future), &REQ, now, 0), Err(Rejection::Future));
        for edge in [now - MAX_AGE_SECS, now + MAX_AHEAD_SECS] {
            assert!(verify(Some(&format!("{edge}:x")), &REQ, now, 0).is_ok());
        }
    }

    #[test]
    fn refuses_too_few_zero_bits() {
        assert_eq!(verify(Some("1000000:x"), &REQ, 1_000_000, 256), Err(Rejection::Insufficient));
    }

    #[test]
    fn a_solution_is_bound_to_its_method_path_caller_and_body() {
        let proof = Proof { timestamp: 1, nonce: "n" };
        let base = solution_hash(&REQ, &proof);
        for other in [
            Request { method: "PUT", ..REQ },
            Request { path: "/v1/identity/evolve", ..REQ },
            Request { caller: "aaaaaaaaaaaa", ..REQ },
            Request { body: b"{ }", ..REQ },
        ] {
            assert_ne!(solution_hash(&other, &proof), base);
        }
    }

    #[test]
    fn agent_id_is_the_last_60_bits_in_base32() {
        let mut hash = [0xff; 32];
        assert_eq!(agent_id(&hash), "777777777777");
        hash[24..].copy_from_slice(&[0xf0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(agent_id(&hash), "aaaaaaaaaaaa");
        hash[31] = 1;
        assert_eq!(agent_id(&hash), "aaaaaaaaaaab");
    }
}
