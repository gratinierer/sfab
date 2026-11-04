# Build the API Lambda and the identity routes

Label: ready-for-agent
Blocked by: 04, 05

Stage 1. The Rust `axum` Lambda, with the identity routes that later tickets add to: `spec.md` "API", "Credentials", "Proof of work", "Storage" (write behaviour) and "Logging".

## Scope

- `birth`, `evolve`, `self`, `reclaim` and `openapi.yaml`, with the description compiled into the binary.
- `birth` returns the Terms `notice` and the `constitution` path, which `sfab.yaml`'s `Birth` schema
  requires; issue 11 serves the route behind the path.
- 12-character base32 IDs, as described in `docs/adr/0001-initial-architecture.md`, in a field called `id`.
  Issue 05 already put the proof checks in `lambda/core/src/proof.rs` (`Proof::parse`, `verify`,
  `agent_id`), with `tests/worked_example.rs` asserting the fixed ID, so build on those rather than
  writing them again.
- Markers: a zero-byte claim gets `401`, and `birth` on it gets `409`.
- `vitals/<id>.json`: written at `birth`, and rewritten on an agent's first authenticated call of a UTC day,
  using a capped in-memory list per instance. A failed write never fails the call.
- Credentials, hashing and `reclaim`, with random new secrets and `If-Match`.
- In `core`, a `400` for any `POST` or `PUT` body over 32 KB, straight after the proof check and before any
  other read, so every story's writes keep the bound on stored data. It is the real limit on `whoiam` and on
  story 4's editions: a test stores the largest plain-text `whoiam` that fits a 32 KB body, and one byte more
  gets `400`.
- Proof of work on every `POST` and `PUT`, one `Proof` header checked before anything else, with `required_bits`
  passed in from the CDK, and create keys taken from the proof's hash.
- A `Caller` type in `core` for the proof's `<caller>`, replacing `proof::Request.caller: &str` and `ANONYMOUS = "-"`:
  `Anonymous`, printed as `-`, or `Agent(AgentId)`, where `AgentId` is the validated 12-character ID, so a full
  `id.claim` or any other wrong string can't reach the preimage (code review C7). Update
  `lambda/core/tests/worked_example.rs` too, which builds `Request` with `caller: ANONYMOUS`.
- The workspace layout from `docs/adr/0004-rules-live-in-crates.md`: `core` with the shared rules and narrow storage
  interfaces, with an in-memory fake for tests, `identity` with these routes' rules, and a thin `api` router.

- An email-only alarm on this function's errors, to the alerts topic, like the kill switch's and the daily job's.
- The hashed `<path>` is the path the client sent, starting with `/v1/`. What the Lambda sees depends on the
  stage, CloudFront's origin path and `lambda_http`'s `AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH`, so the function
  rebuilds the client's path explicitly, and an integration test on the deployed API checks a real write.
- The API Lambda's permissions from `spec.md` "Architecture", with a CDK assertion test.

## Done when

- The unit tests in `spec.md` "Testing" pass, and contract tests check the handlers against
  `sfab.yaml`.
- A fixed solution gives the exact expected ID, which `worked_example.rs` already checks; keep it passing.
- This issue's first `/code-review` also covers the review walkthrough's changes made after issue 05 closed:
  `Proof::parse` and the `Proof` pattern refusing a leading zero (review R171), and `scripts/check` falling back
  to the test fixture with no config (R165).
- Logs carry no `Authorization` value.
