//! Replies and `application/problem+json` errors: `.stories/2-identity-api/spec.md` "Wire
//! contract" and `docs/api-design.md` "Errors". No HTTP types: the router turns these into
//! responses.

use serde_json::{Map, Value, json};

use crate::proof::Rejection;

/// A successful reply: a status and a JSON body.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub status: u16,
    pub body: Value,
}

impl Reply {
    pub fn ok(body: Value) -> Self {
        Self { status: 200, body }
    }

    pub fn created(body: Value) -> Self {
        Self { status: 201, body }
    }

    /// A message or knock was accepted.
    pub fn accepted(body: Value) -> Self {
        Self { status: 202, body }
    }
}

/// An RFC 9457 problem. `kind` is the `type`: `about:blank` when the status says it all.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub status: u16,
    pub kind: &'static str,
    pub title: String,
    pub extra: Map<String, Value>,
}

pub const ABOUT_BLANK: &str = "about:blank";

impl Problem {
    pub fn new(status: u16, kind: &'static str, title: impl Into<String>) -> Self {
        Self {
            status,
            kind,
            title: title.into(),
            extra: Map::new(),
        }
    }

    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.extra.insert(key.to_owned(), value.into());
        self
    }

    /// A refused proof: `400` with `required_bits` and the server's time, so the agent can
    /// solve again, or fix its clock.
    pub fn proof(rejection: Rejection, required_bits: u32, now: i64) -> Self {
        let title = match rejection {
            Rejection::Missing => "A Proof header is required on every POST and PUT",
            Rejection::Malformed => "The Proof header must be <unix-ts>:<nonce>",
            Rejection::Stale => "The proof's timestamp is more than 5 minutes old",
            Rejection::Future => "The proof's timestamp is more than 1 minute ahead",
            Rejection::Insufficient => "The proof's hash has too few leading zero bits",
        };
        Self::new(400, ABOUT_BLANK, title)
            .with("required_bits", required_bits)
            .with("server_time", now)
    }

    pub fn bad_request(title: impl Into<String>) -> Self {
        Self::new(400, ABOUT_BLANK, title)
    }

    /// Missing or wrong credentials, an unknown ID or a taken-down identity, all alike.
    pub fn unauthorized() -> Self {
        Self::new(401, ABOUT_BLANK, "Missing or wrong credentials")
    }

    pub fn not_found(title: impl Into<String>) -> Self {
        Self::new(404, ABOUT_BLANK, title)
    }

    /// You're not allowed, and `kind` says why, such as `blocked`: don't retry.
    pub fn forbidden(kind: &'static str, title: impl Into<String>) -> Self {
        Self::new(403, kind, title)
    }

    pub fn conflict(kind: &'static str, title: impl Into<String>) -> Self {
        Self::new(409, kind, title)
    }

    /// A failure on our side. The title says nothing about the cause.
    pub fn internal() -> Self {
        Self::new(500, ABOUT_BLANK, "Server error, retry with backoff")
    }

    pub fn body(&self) -> Value {
        let mut body = json!({"type": self.kind, "title": self.title, "status": self.status});
        let map = body.as_object_mut().expect("an object");
        for (k, v) in &self.extra {
            map.insert(k.clone(), v.clone());
        }
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proof_problem_carries_the_bits_and_the_time() {
        let body = Problem::proof(Rejection::Stale, 20, 1_000).body();
        assert_eq!(body["status"], 400);
        assert_eq!(body["type"], "about:blank");
        assert_eq!(body["required_bits"], 20);
        assert_eq!(body["server_time"], 1_000);
    }
}
