//! What every write passes before its route's own rules: the proof, then the 32 KB body limit,
//! with nothing read or written before either. `.stories/2-identity-api/spec.md` "Proof of work"
//! and `docs/api-design.md` "Writes".

use crate::credentials::caller_of;
use crate::id::Caller;
use crate::problem::Problem;
use crate::proof::{self, Request};

/// The most a `POST` or `PUT` body may hold, so every story's writes keep the bound on stored data.
pub const MAX_BODY_BYTES: usize = 32 * 1024;

/// A call as an institution crate sees it, with no HTTP types.
#[derive(Debug, Clone, Copy)]
pub struct Call<'a> {
    /// Upper case, such as `POST`.
    pub method: &'a str,
    /// The path as the client sent it, starting with `/v1/`, without its query.
    pub path: &'a str,
    pub authorization: Option<&'a str>,
    pub proof: Option<&'a str>,
    pub body: &'a [u8],
}

/// A write that passed the gate.
#[derive(Debug, Clone)]
pub struct Admitted {
    pub caller: Caller,
    /// The solution hash, from which creates take their keys.
    pub hash: [u8; 32],
}

/// The proof, checked with one hash, then the body's size. A malformed `Authorization` can't
/// name a caller, so it gets `401` before the proof; that reads nothing either.
pub fn admit_write(call: &Call, now: i64, required_bits: u32) -> Result<Admitted, Problem> {
    let caller = caller_of(call.authorization).ok_or_else(Problem::unauthorized)?;
    let request = Request {
        method: call.method,
        path: call.path,
        caller: &caller,
        body: call.body,
    };
    let hash = proof::verify(call.proof, &request, now, required_bits)
        .map_err(|r| Problem::proof(r, required_bits, now))?;
    if call.body.len() > MAX_BODY_BYTES {
        return Err(Problem::bad_request(
            "The request body is over 32 KB (32,768 bytes)",
        ));
    }
    Ok(Admitted { caller, hash })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(body: &[u8]) -> Call<'_> {
        Call {
            method: "POST",
            path: "/v1/identity/birth",
            authorization: None,
            proof: Some("1000:x"),
            body,
        }
    }

    #[test]
    fn the_proof_comes_before_the_body_limit() {
        let big = vec![b'a'; MAX_BODY_BYTES + 1];
        let err = admit_write(
            &Call {
                proof: None,
                ..call(&big)
            },
            1000,
            0,
        )
        .unwrap_err();
        assert_eq!(err.extra["required_bits"], 0);
    }

    #[test]
    fn a_body_of_32_kb_passes_and_one_byte_more_does_not() {
        let fits = vec![b'a'; MAX_BODY_BYTES];
        assert!(admit_write(&call(&fits), 1000, 0).is_ok());
        let over = vec![b'a'; MAX_BODY_BYTES + 1];
        let err = admit_write(&call(&over), 1000, 0).unwrap_err();
        assert_eq!(err.status, 400);
        assert!(!err.extra.contains_key("required_bits"));
    }

    #[test]
    fn a_malformed_authorization_gets_401() {
        let c = Call {
            authorization: Some("Bearer nope"),
            ..call(b"")
        };
        assert_eq!(admit_write(&c, 1000, 0).unwrap_err().status, 401);
    }
}
