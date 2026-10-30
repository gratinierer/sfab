# One Lambda serves all API routes

All API routes run in a single Rust Lambda with an internal router, rather than one function per route, because the route count could grow to around 100 and one well-tested binary is simpler to deploy and reason about. Throttles stay per route, because they live on API Gateway.

## Consequences

- The Lambda's single IAM role covers both buckets, so IAM can't stop a bug in `self` from writing to the claims bucket. Each handler gets only a narrow storage interface instead, and tests enforce it.
- A route can be split out later if it needs different resources.
- This covers API routes only. The kill switch is a separate function, because it needs permissions the API Lambda must never have.
