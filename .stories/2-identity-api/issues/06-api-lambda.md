# Build the API Lambda and the identity routes

Label: ready-for-agent
Blocked by: 04, 05

Stage 1. The Rust `axum` Lambda, with the identity routes that later tickets add to: `spec.md` "API", "Credentials", "Proof of work", "Storage" (write behaviour) and "Logging".

## Scope

- `birth`, `evolve`, `self`, `reclaim` and `openapi.yaml`, with the description compiled into the binary.
- 12-character base32 IDs, as described in `docs/adr/0001-initial-architecture.md`, in a field called `id`.
- Markers: a zero-byte claim gets `401`, and `birth` on it gets `409`.
- `vitals/<id>.json`: written at `birth`, and rewritten on an agent's first authenticated call of a UTC day,
  using a capped in-memory list per instance. A failed write never fails the call.
- Credentials, hashing and `reclaim`, with random new secrets and `If-Match`.
- Proof of work on every `POST` and `PUT`, one `Proof` header checked before anything else, with `required_bits`
  passed in from the CDK, and create keys taken from the proof's hash.
- Narrow storage interfaces, with an in-memory fake for tests.

- An email-only alarm on this function's errors, to the alerts topic, like the kill switch's and the daily job's.
- The hashed `<path>` is the path the client sent, starting with `/v1/`. What the Lambda sees depends on the
  stage, CloudFront's origin path and `lambda_http`'s `AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH`, so the function
  rebuilds the client's path explicitly, and an integration test on the deployed API checks a real write.

## Done when

- The unit tests in `spec.md` "Testing" pass, and contract tests check the handlers against
  `identity.yaml`.
- A fixed solution gives the exact expected ID.
- Logs carry no `Authorization` value.
