//! The IP rule's fingerprint and record: `.stories/17-genesis-codes/spec.md` "The IP rule". Only an
//! HMAC of the IPv4 address or the IPv6 /64 prefix is kept, never the address.

use std::net::IpAddr;

use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// `genesis_ip_window_hours`: how long one address waits between births, and how old an IP
/// record gets before the daily job deletes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    hours: u32,
}

impl Window {
    pub const fn hours(hours: u32) -> Self {
        Self { hours }
    }

    pub fn seconds(self) -> i64 {
        i64::from(self.hours) * 3_600
    }
}

/// Whole hours, as the CDK code sets `GENESIS_IP_WINDOW_HOURS`.
impl std::str::FromStr for Window {
    type Err = std::num::ParseIntError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self::hours)
    }
}

/// The IP rule's settings: `ip_hash_key` and `genesis_ip_window_hours`.
#[derive(Clone)]
pub struct IpRule {
    key: Vec<u8>,
    window: Window,
}

impl std::fmt::Debug for IpRule {
    /// Never prints the key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpRule")
            .field("window", &self.window)
            .finish_non_exhaustive()
    }
}

impl IpRule {
    pub fn new(key: &str, window: Window) -> Self {
        Self {
            key: key.as_bytes().to_vec(),
            window,
        }
    }

    pub(crate) fn window_seconds(&self) -> i64 {
        self.window.seconds()
    }

    /// The hex HMAC of the viewer's address, or its /64 for IPv6, or `None` when `viewer` isn't
    /// CloudFront's `<address>:<port>` form.
    pub(crate) fn fingerprint(&self, viewer: &str) -> Option<String> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC takes any key");
        match viewer_address(viewer)? {
            IpAddr::V4(v4) => mac.update(&v4.octets()),
            IpAddr::V6(v6) => mac.update(&v6.octets()[..8]),
        }
        let tag = mac.finalize().into_bytes();
        Some(tag.iter().map(|b| format!("{b:02x}")).collect())
    }
}

/// The address in `CloudFront-Viewer-Address`, which is `<address>:<port>`, an IPv6 address
/// without brackets, such as `2001:db8::1:443`. A bracketed `[<v6>]:<port>` is taken too. An
/// IPv4-mapped IPv6 address counts as its IPv4 address.
fn viewer_address(viewer: &str) -> Option<IpAddr> {
    let (address, port) = viewer.rsplit_once(':')?;
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let address = address
        .strip_prefix('[')
        .and_then(|a| a.strip_suffix(']'))
        .unwrap_or(address);
    Some(address.parse::<IpAddr>().ok()?.to_canonical())
}

pub(crate) const IPS_PREFIX: &str = "genesis/ips/";

pub(crate) fn record_key(fingerprint: &str) -> String {
    format!("{IPS_PREFIX}{fingerprint}")
}

/// `genesis/ips/<hmac>`: when the last birth from this address was, in Unix seconds.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Record {
    pub last_birth: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_cloudfronts_viewer_address() {
        for (viewer, address) in [
            ("198.51.100.7:46532", Some("198.51.100.7")),
            ("2001:db8::1:443", Some("2001:db8::1")),
            ("[2001:db8::1]:443", Some("2001:db8::1")),
            ("::ffff:198.51.100.7:443", Some("198.51.100.7")),
            ("198.51.100.7", None),
            ("198.51.100.7:", None),
            ("", None),
            ("x:1", None),
        ] {
            let expected = address.map(|a| a.parse::<IpAddr>().unwrap());
            assert_eq!(viewer_address(viewer), expected, "{viewer}");
        }
    }

    #[test]
    fn the_fingerprint_is_keyed() {
        let a = IpRule::new("one", Window::hours(4))
            .fingerprint("198.51.100.7:1")
            .unwrap();
        let b = IpRule::new("two", Window::hours(4))
            .fingerprint("198.51.100.7:1")
            .unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 64);
    }
}
