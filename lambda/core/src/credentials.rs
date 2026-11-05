//! Claims and recovery codes: `.stories/2-identity-api/spec.md` "Credentials".
//!
//! Secrets are 256 random bits, base64url without padding. Only their SHA-256 hashes are stored,
//! compared in constant time. A slow password hash is deliberately not used
//! (`docs/adr/0001-initial-architecture.md`).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::id::{AgentId, Caller};
use crate::proof::hex;

/// The length of an encoded secret: 32 bytes in base64url without padding.
pub const SECRET_LEN: usize = 43;

/// A new random secret, from the operating system's generator.
pub fn new_secret() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the operating system's random generator");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// The lowercase hex SHA-256 of a secret, which is what storage keeps.
pub fn hash_secret(secret: &str) -> String {
    hex(&Sha256::digest(secret.as_bytes()))
}

/// Whether `secret` hashes to `stored`, compared in constant time.
pub fn matches(secret: &str, stored: &str) -> bool {
    hash_secret(secret)
        .as_bytes()
        .ct_eq(stored.as_bytes())
        .into()
}

/// `Authorization: Bearer <id>.<secret>`, where the secret is a claim or, for `reclaim`, a
/// recovery code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bearer {
    pub id: AgentId,
    pub secret: String,
}

impl Bearer {
    pub fn parse(header: &str) -> Option<Self> {
        let token = header.strip_prefix("Bearer ")?;
        let (id, secret) = token.split_once('.')?;
        let secret_ok = secret.len() == SECRET_LEN
            && secret
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        Some(Self {
            id: AgentId::parse(id)?,
            secret: secret_ok.then(|| secret.to_owned())?,
        })
    }
}

/// The proof's `<caller>` from the `Authorization` header: anonymous without one, the ID with a
/// well-formed one, and `None` for a malformed one, which can't name a caller.
pub fn caller_of(authorization: Option<&str>) -> Option<Caller> {
    match authorization {
        None => Some(Caller::Anonymous),
        Some(header) => Bearer::parse(header).map(|b| Caller::Agent(b.id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_secret_is_43_base64url_characters_and_never_repeats() {
        let a = new_secret();
        assert_eq!(a.len(), SECRET_LEN);
        assert!(Bearer::parse(&format!("Bearer abcdefgh2345.{a}")).is_some());
        assert_ne!(a, new_secret());
    }

    #[test]
    fn a_secret_matches_only_its_own_hash() {
        let secret = new_secret();
        let stored = hash_secret(&secret);
        assert_eq!(stored.len(), 64);
        assert!(matches(&secret, &stored));
        assert!(!matches(&new_secret(), &stored));
    }

    #[test]
    fn parses_only_a_well_formed_bearer() {
        let secret = "a".repeat(SECRET_LEN);
        let ok = Bearer::parse(&format!("Bearer abcdefgh2345.{secret}")).unwrap();
        assert_eq!(ok.id.as_str(), "abcdefgh2345");
        for bad in [
            format!("bearer abcdefgh2345.{secret}"),
            format!("Bearer abcdefgh2345{secret}"),
            format!("Bearer ABCDEFGH2345.{secret}"),
            format!("Bearer abcdefgh2345.{}", "a".repeat(SECRET_LEN - 1)),
            format!("Bearer abcdefgh2345.{}=", "a".repeat(SECRET_LEN - 1)),
            format!("Bearer abcdefgh2345.{secret}.x"),
        ] {
            assert!(Bearer::parse(&bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn the_caller_is_the_id_before_the_dot() {
        assert_eq!(caller_of(None), Some(Caller::Anonymous));
        let header = format!("Bearer abcdefgh2345.{}", "a".repeat(SECRET_LEN));
        assert_eq!(
            caller_of(Some(&header)).unwrap().to_string(),
            "abcdefgh2345"
        );
        assert_eq!(caller_of(Some("Bearer nonsense")), None);
    }
}
