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
- `identity`'s takedown function, which ADR 0004 says every crate exports and issue 08's program calls: its
  locations for an agent and the action on each, so its key formats live only here. The claim becomes a
  zero-byte marker that keeps `born_at` in its S3 metadata, `identities/<id>/` is deleted last, and
  `vitals/<id>.json` is deleted. A unit test against the in-memory fake, filled through `identity`'s own write
  functions, checks the list (review R178).

- The hashed `<path>` is the path the client sent, starting with `/v1/`. What the Lambda sees depends on the
  stage, CloudFront's origin path and `lambda_http`'s `AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH`, so the function
  rebuilds the client's path explicitly, with unit tests for each form it can arrive in. Issue 01 checks a real
  write on the deployed API.
- The CDK side of this function (its errors alarm, its permissions and the deployed write) is issue 01's,
  because the stacks and the alerts topic are built there.

## Done when

- The identity unit tests from `spec.md` "Testing" pass, against the in-memory fake (the rest of that list
  belongs to issues 11, 13, 14 and 15):
  - `reclaim` invalidates the old secrets, and a second use of the old code gets `401`.
  - Every `POST` and `PUT` without a valid proof gets `400` with nothing stored, and a proof made for another
    route, caller or body is rejected.
  - A marker gets `401`, and `birth` on a marker gets `409`.
  - The first call of a day updates `vitals/<id>.json` and later ones don't, and a failed write doesn't fail
    the call.
- Contract tests check the handlers against `sfab.yaml`.
- A fixed solution gives the exact expected ID, which `worked_example.rs` already checks; keep it passing.
- This issue's first `/code-review` starts at or before `39c2bed`, so it also covers the review walkthrough's
  changes in e9e8391, made after issue 05 closed, and findings on them are filed against this issue:
  `Proof::parse` and the `Proof` pattern refusing a leading zero (review R171), and `scripts/check` falling back
  to the test fixture with no config (R165), and `scripts/deploy` refusing to start without `sfab.config.toml`
  (R176), committed after e9e8391.
- Logs carry no `Authorization` value.
