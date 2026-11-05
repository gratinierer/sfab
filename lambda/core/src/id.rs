//! Agent IDs and the proof's `<caller>`: `.stories/2-identity-api/spec.md` "Wire contract".

use std::fmt;

const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

/// A validated agent ID: 12 lowercase RFC 4648 base32 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(String);

impl AgentId {
    pub const LEN: usize = 12;

    /// Accepts exactly 12 characters of `a-z` and `2-7`, and nothing else.
    pub fn parse(text: &str) -> Option<Self> {
        let ok = text.len() == Self::LEN && text.bytes().all(|b| ALPHABET.contains(&b));
        ok.then(|| Self(text.to_owned()))
    }

    /// The last 60 bits of a solution hash, because the first bits hold the proof's zeros.
    pub fn from_hash(hash: &[u8; 32]) -> Self {
        let tail = u64::from_be_bytes(hash[24..].try_into().expect("8 bytes"));
        Self(
            (0..Self::LEN)
                .map(|i| ALPHABET[((tail >> (5 * (Self::LEN - 1 - i))) & 31) as usize] as char)
                .collect(),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The proof's `<caller>`: `-` with no `Authorization`, or the ID before its `.`. Only a validated
/// ID can be a caller, so a full `id.claim` or any other string can't reach the preimage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Caller {
    Anonymous,
    Agent(AgentId),
}

impl fmt::Display for Caller {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Caller::Anonymous => f.write_str("-"),
            Caller::Agent(id) => id.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_twelve_base32_characters() {
        assert!(AgentId::parse("abcdefgh2345").is_some());
        for bad in [
            "",
            "abcdefgh234",
            "abcdefgh23456",
            "ABCDEFGH2345",
            "abcdefgh2341",
            "abcdefgh.234",
        ] {
            assert!(AgentId::parse(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn takes_the_last_60_bits_in_base32() {
        let mut hash = [0xff; 32];
        assert_eq!(AgentId::from_hash(&hash).as_str(), "777777777777");
        hash[24..].copy_from_slice(&[0xf0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(AgentId::from_hash(&hash).as_str(), "aaaaaaaaaaaa");
        hash[31] = 1;
        assert_eq!(AgentId::from_hash(&hash).as_str(), "aaaaaaaaaaab");
    }

    #[test]
    fn a_caller_prints_as_the_proof_needs() {
        assert_eq!(Caller::Anonymous.to_string(), "-");
        let id = AgentId::parse("abcdefgh2345").unwrap();
        assert_eq!(Caller::Agent(id).to_string(), "abcdefgh2345");
    }
}
